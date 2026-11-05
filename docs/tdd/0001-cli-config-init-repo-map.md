# TDD-0001 — CLI, Configuration, Init, and Repo Map

**Status:** Approved
**Date:** 2026-11-05
**Owner:** Forge architecture
**Sources:** `docs/PRD.md` §7.1, §8.1, §8.2, §9, §9.5, §10.11, §11.3, §13.13, §15.2; NFR-DIST-001, NFR-COMPAT-001; D-13, D-16, D-19, D-20
**Companions:** [TDD-0002](0002-domain-model-ports-assembly.md), [architecture.md](../../arch_docs/architecture.md), [dependency-rules.md](../../arch_docs/dependency-rules.md), [naming-conventions.md](../../arch_docs/naming-conventions.md)
**ADRs:** [ADR-0009](../adr/0009-committed-config-confirmed-facts.md)
**Ticket:** [Define the CLI, configuration, init, and repo map contract (TDD-0001)](https://github.com/Xaidel/forge/issues/26)
**Map:** [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
**Amends:** none
**Amended by:** [TDD-0007](0007-model-adapters-providers-cost.md) (2026-11-05) - profile auth/locality/capabilities/session header, price table, token budgets, privacy, and base-URL path rule.

---

## 1. Purpose and authority

This document fixes the exact command-line, configuration, initialization, repository-map, init-report, and distribution contracts for Forge v1:

- the `forge` command tree, flags, aliases, help output, exit codes, and error mapping;
- the `.forge/config` TOML schema, canonical serialization, validation, versioning, unknown-field preservation, and local override file;
- the repo-map facets, statuses, confirmed absence, path rules, version, hash, package-root grouping, docs precedence, and check identity;
- the phased `forge init` flow, its human gates, staged writes, dry-run, re-init, and failure behavior;
- the repo-brief command grammar, layout, grouping, and collapse rules;
- the `.forge/local/init-report.json` schema and evidence grammar;
- the deterministic scan heuristics and their limits;
- the provisional cargo-dist release contract;
- the fixtures and requirement-to-test mapping for FR-ONB-001..019 and NFR-DIST-001.

`docs/PRD.md` is authoritative for product behavior. `arch_docs/architecture.md`, `arch_docs/dependency-rules.md`, and `arch_docs/naming-conventions.md` are authoritative for boundaries, dependency direction, and names. TDD-0002 is authoritative for the public Rust records and ports. This document does not amend TDD-0002; it defines infrastructure-local file and CLI contracts that map onto TDD-0002's public records. When this document conflicts with a higher authority, the higher authority wins and this document is corrected.

---

## 2. Scope

### 2.1 In scope

- The public `forge` command surface, global and per-command flags, aliases, help text, stdout/stderr split, and exit codes.
- The hidden internal `__worker` and `hook` subcommands at the command-tree level; their protocols belong to TDD-0006 and TDD-0008.
- The `.forge/config` TOML schema and its local override file.
- The repo-map facets, statuses, confirmed absence, paths, grouping, version, hash, and base ref.
- The full `forge init` flow, including preflight, scan, repo brief, verification choice, provider setup, execution-policy review handoff, trust/signing/hooks handoff, summary, apply, dry-run, and re-init.
- The repo-brief command grammar and deterministic text layout.
- `.forge/local/init-report.json`, its evidence grammar, warnings, and privacy rules.
- The deterministic scan rule table for the supported ecosystems and the "unknown" fallback.
- The provisional distribution contract from the partial cargo-dist evidence.
- Fixtures and the requirement-to-test mapping.

### 2.2 Out of scope

- Implementation code, workspace scaffold, release artifacts, and CI workflows. This document is a contract, not a delivery.
- The execution-policy JSON schema, host probe internals, canaries, permission store, and worker IPC framing — TDD-0006.
- Provider wire mappings, key-file shape, price table, budget arithmetic, and model catalog — TDD-0007.
- Hook shim content, signing mechanics, attestation JSON, merge mechanics, and allowed-signers content — TDD-0008.
- TUI layout, ANSI tokens, keybindings, live region, and streaming cadence — TDD-0009.
- Security scan patterns, terminal sanitizer, secret patterns, protected-path list, and review flags — TDD-0010.
- Session state, event schema, resume algorithm, and active-task lock format — TDD-0003.
- Plan schema, plan-time map diff trigger, policy evaluation, and gate lifecycle — TDD-0004.
- Ledger record schema, index, verification, and migration — TDD-0005.
- Dogfood instrumentation and metrics — TDD-0011.
- Implementation of `NFR-DOC-001` checker tooling.

---

## 3. Constraints and dependencies

- Rust 1.93, edition 2024, stable toolchain.
- CLI parsing uses `clap` 4.x with the derive API. The parser is an `infra_local` dependency and never appears in a public port signature.
- TOML reading uses `toml` with `serde` in `infra_local`; preservation writes use `toml_edit`. JSON reading uses `serde_json`. The core remains Serde-free and receives plain records through `ConfigPort`.
- `cli` never constructs an adapter. `wiring` composes `InitRuntime`, the scan source, the config adapter, and the task runtime; `cli` drives prompts and output.
- `forge init` is an infrastructure-local host command, not a new core inbound capability. It uses `ConfigPort` for reads and an adapter-level `ConfigWriter` for writes.
- The scan is deterministic, read-only, and never executes repository code, a package manager, build-system introspection, or a model.
- Committed files contain repo-relative POSIX paths. Local-only files may contain the absolute repo root.
- One repository has one config, one repo map, one policy, one execution policy, and one trust declaration. Packages are entries, not state domains (NFR-SCALE-002).
- The release contract is provisional. The cargo-dist spike was deferred by the owner; this document records the evidence and the revisit trigger, not an accepted release policy.
- No Node, npm, or Bun runtime is used by Forge or its release artifacts.

---

## 4. Vocabulary

The PRD's terms are canonical. This section fixes the terms that are easy to confuse.

| Term | Meaning |
|---|---|
| **Config file** | The committed `.forge/config` TOML file. It contains project settings and the confirmed repo map. |
| **Repo map** | The five confirmed facets inside `.forge/config`: `checks`, `entry_points`, `module_roots`, `docs`, `package_roots`. It is not the whole config file. |
| **Facet** | One of the five repo-map categories. Each has a status and may have confirmed entries or confirmed absence. |
| **Confirmed fact** | An entry a human accepted, edited and accepted, or explicitly added. Only confirmed facts enter `.forge/config`. |
| **Proposal / candidate** | A scan result that is not yet confirmed. Proposals live only in `.forge/local/init-report.json` and the repo brief. |
| **Confirmed absence** | A human statement that a facet legitimately has no entries, such as a library with no binary entry point. |
| **Repo brief** | The human-facing, inline rendering of the scan and current map during `forge init`. It is a view, not a stored artifact. |
| **Init report** | The local-only `.forge/local/init-report.json` evidence record produced by the scan and replaced by a confirmed re-init. |
| **Check** | A command proposed at init and confirmed in `checks[]`, with a stable integer ID. |
| **Verification choice** | The repo-level choice of `automated` checks, a `manual` method, or `unconfigured`. |
| **Profile** | A provider endpoint and protocol definition in `[profiles.<name>]`. |
| **Slot** | A routing binding in `[slots.<name>]` to a profile and model. The slots are `primary`, `fast`, and `local`. |
| **Template** | An init-time starting point for a profile: OpenCode Go, Anthropic direct, OpenAI direct, or custom OpenAI-compatible. The committed artifact is a profile, not a template. |
| **Dry run** | The non-interactive `forge init --dry-run` mode that prints the scan and proposed files without writing, prompting, validating the network, or running canaries. |
| **Re-init** | Running `forge init` in a repository that already has `.forge/config`. It shows the current map, new candidates, and a diff. |
| **Base ref** | The Git ref or commit a task branches from. The name is `base_ref`, never `base_branch`. |
| **Task goal** | The human's task description. It is the `TaskGoal` value, never a "prompt". |

The private and public Rust names in TDD-0002 remain authoritative. This document adds only infrastructure-local types.

---

## 5. CLI contract

### 5.1 Command tree

```text
forge
├── init
├── run
├── resume <TASK>
├── status [TASK]
├── doctor
├── sandbox
│   └── probe
├── exec --host <COMMAND> --cwd <DIR> --reason <TEXT>
├── attest
│   └── show [TASK]
├── merge [--override --reason <TEXT>] <TASK>
├── reopen --reason <TEXT> <TASK>
├── hooks
│   ├── status
│   ├── install
│   └── repair
├── audit [TASK]
├── transcript <TASK>
├── task
│   └── clean [TASK]
├── ledger
│   ├── verify [--repair]
│   └── migrate [--dry-run]
├── hook                  (hidden, internal; TDD-0008)
└── __worker              (hidden, internal; TDD-0006)
```

`forge help` and `forge help <COMMAND>` are provided by the parser. `hook` and `__worker` never appear in `forge --help`, subcommand lists, or the public contract.

### 5.2 Global options

Global options are accepted **before** the subcommand. They are not repeated in per-command help. This keeps each subcommand's help block focused on its own flags and makes the golden text stable.

| Option | Value | Default | Behavior |
|---|---|---|---|
| `--plain` | none | off | Force plain output. No ANSI, spinners, cursor movement, or live region. Also set by `FORGE_PLAIN=1`. |
| `--color <WHEN>` | `auto`, `always`, `never` | `auto` | Color policy for the TUI. `--plain` wins over `--color`. |
| `--theme <THEME>` | `default`, `high-contrast`, `mono` | `default` | Semantic theme for the TUI. Ignored in plain mode. |
| `--reduced-motion` | none | off | Disable spinners, cursor blink, and animated progress. Also set by `FORGE_REDUCED_MOTION=1`. |
| `--timestamps` | none | off | Show timestamps on every transcript line. By default timestamps appear only on gates, receipts, and system events. |
| `--show-thinking` | none | off | Show reasoning in plain mode. The TUI still hides reasoning by default and uses `Ctrl-T` for the live toggle. |
| `-h`, `--help` | none | — | Print help and exit `0`. |
| `-V`, `--version` | none | — | Print `forge <package-version>` and exit `0`. |

No other short flags exist. Destructive and irreversible commands have no short forms (NAME-22d).

### 5.3 Command synopses and flags

| Command | Synopsis | Notes |
|---|---|---|
| `init` | `forge init [--dry-run] [--base-ref <REF>] [--no-network] [--no-hooks]` | Interactive unless `--dry-run`. |
| `run` | `forge run [--base-ref <REF>] [--guarded] [GOAL]` | `GOAL` is one positional string. On a TTY, omission prompts. On a non-TTY, omission is a usage error. |
| `resume` | `forge resume <TASK>` | `TASK` is a `TaskId` such as `T-042`. |
| `status` | `forge status [TASK]` | No task means the active task. No active task prints `No active task.` and exits `0`. |
| `doctor` | `forge doctor [--no-network]` | `--no-network` skips provider endpoint validation. |
| `sandbox probe` | `forge sandbox probe` | Read-only diagnostic. TDD-0006 owns the report. |
| `exec` | `forge exec --host <COMMAND> --cwd <DIR> --reason <TEXT>` | `--host` is one exact command string. Forge parses it into argv and never invokes a shell. `--cwd` is required. `--reason` is required for the audit log. |
| `attest show` | `forge attest show [TASK]` | Prints the current attestation as Markdown. TDD-0008 owns the record. |
| `merge` | `forge merge <TASK> [--override --reason <TEXT>]` | `--override` requires `--reason`. No `--force`. |
| `reopen` | `forge reopen <TASK> --reason <TEXT>` | The only path out of `done`. |
| `hooks status` | `forge hooks status` | Installed, missing, outdated, or conflicting. |
| `hooks install` | `forge hooks install` | Explains the hooks, then installs after confirmation. Never overwrites. |
| `hooks repair` | `forge hooks repair` | Repairs missing or outdated managed hooks. Never overwrites a conflicting hook. |
| `audit` | `forge audit [TASK]` | Overrides, hook state, and unattested merges. |
| `transcript` | `forge transcript <TASK>` | Full session log to stdout, sanitized plain text, no pager. |
| `task clean` | `forge task clean [TASK]` | Inspects or cleans orphaned worktrees. Never forces. |
| `ledger verify` | `forge ledger verify [--repair]` | TDD-0005 owns the checks and report; `--repair` sets `VerifyLedgerRequest.repair`. |
| `ledger migrate` | `forge ledger migrate [--dry-run]` | `--dry-run` prints the preview without writing. A real migration requires typed confirmation. TDD-0005 owns the mechanics. |

`forge run --guarded` is a deliberate execution-mode downgrade. It requires a typed confirmation and is refused for an untrusted repository (FR-EXEC-010). It is not a general `--no-sandbox` flag.

`forge exec --host` is the only host-exec escape hatch. It does not bypass the decision policy. The parsed command, working directory, mode, reason, and scopes are shown before approval (FR-EXEC-007; PRD §11.6).

### 5.4 Help contract

The exact help output is part of the contract. The following blocks are the golden text; a change to a command, flag, value, or description changes this document and the golden fixtures together.

`forge --help`:

```text
Forge is a local-first coding agent harness with a human decision loop

Usage: forge [OPTIONS] <COMMAND>

Commands:
  init        Initialize a repository
  run         Start a task
  resume      Resume a paused or interrupted task
  status      Show task state and the active mode
  doctor      Check the host and repository prerequisites
  sandbox     Diagnose the sandbox tier without changing state
  exec        Run one approved command on the host
  attest      Show or manage the attestation record
  merge       Perform the enforced merge
  reopen      Reopen a done task for a new attestation
  hooks       Install, inspect, or repair managed hooks
  audit       Show overrides, hook state, and enforcement gaps
  transcript  Print the full session log
  task        Inspect or clean orphaned worktrees
  ledger      Check, repair, or migrate the decision ledger
  help        Print this message or the help of the given subcommand(s)

Options:
      --plain             Force plain output; no ANSI, spinners, or live region
      --color <WHEN>      Color mode [default: auto] [possible values: auto, always, never]
      --theme <THEME>     Theme [default: default] [possible values: default, high-contrast, mono]
      --reduced-motion    Disable spinners, cursor blink, and animated progress
      --timestamps        Show timestamps on every transcript line
      --show-thinking     Show reasoning in plain mode
  -h, --help              Print help
  -V, --version           Print version
```

`forge init --help`:

```text
Initialize a repository, scan, confirm the repo map, configure providers, review the execution policy, run canaries, and set up signing and hooks

Usage: forge init [OPTIONS]

Options:
      --dry-run           Print the scan and proposed files without writing
      --base-ref <REF>    Confirm the base ref instead of prompting
      --no-network        Skip provider validation
      --no-hooks          Do not install managed hooks; enforcement is degraded
  -h, --help              Print help
```

`forge run --help`:

```text
Start a task, plan, execute, verify, review, and attest

Usage: forge run [OPTIONS] [GOAL]

Arguments:
  [GOAL]  Task goal; if omitted on a TTY, Forge prompts for it

Options:
      --base-ref <REF>  Confirm the base ref for this task
      --guarded         Use Guarded - not sandboxed after typed confirmation
  -h, --help            Print help
```

`forge resume --help`:

```text
Resume a paused or interrupted task

Usage: forge resume <TASK>

Arguments:
  <TASK>  Task id, for example T-042
```

`forge status --help`:

```text
Show task state, mode, enforcement, budget, and pending gate

Usage: forge status [TASK]

Arguments:
  [TASK]  Task id; defaults to the active task
```

`forge doctor --help`:

```text
Check Git, Rust, platform, MXC, provider, policy, trust, signing, and hooks

Usage: forge doctor [OPTIONS]

Options:
      --no-network  Skip provider endpoint validation
  -h, --help        Print help
```

`forge sandbox --help`:

```text
Diagnose the sandbox tier without changing state

Usage: forge sandbox <COMMAND>

Commands:
  probe  Run the sandbox probe
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

`forge exec --help`:

```text
Run one approved command on the host

Usage: forge exec --host <COMMAND> --cwd <DIR> --reason <TEXT>

Options:
      --host <COMMAND>  Exact command to run; parsed as argv, never through a shell
      --cwd <DIR>       Working directory for the command
      --reason <TEXT>   Human reason recorded in the audit log
  -h, --help            Print help
```

`forge attest --help`:

```text
Show or manage the attestation record

Usage: forge attest <COMMAND>

Commands:
  show  Print the current attestation as Markdown
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

`forge attest show --help`:

```text
Print the current attestation as Markdown

Usage: forge attest show [TASK]

Arguments:
  [TASK]  Task id; defaults to the active task
```

`forge merge --help`:

```text
Perform the enforced merge

Usage: forge merge [OPTIONS] <TASK>

Arguments:
  <TASK>  Task id, for example T-042

Options:
      --override       Bypass the attestation gate; requires --reason
      --reason <TEXT>  Reason for the override
  -h, --help           Print help
```

`forge reopen --help`:

```text
Reopen a done task for a new attestation

Usage: forge reopen --reason <TEXT> <TASK>

Arguments:
  <TASK>  Task id, for example T-042

Options:
      --reason <TEXT>  Reason for reopening the task
  -h, --help           Print help
```

`forge hooks --help`:

```text
Install, inspect, or repair managed hooks

Usage: forge hooks <COMMAND>

Commands:
  status   Report installed, missing, outdated, or conflicting hooks
  install  Install the managed hooks after explaining them
  repair   Repair missing or outdated managed hooks
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

`forge audit --help`:

```text
Show overrides, hook state, and enforcement gaps

Usage: forge audit [TASK]

Arguments:
  [TASK]  Task id; defaults to the active task
```

`forge transcript --help`:

```text
Print the full session log

Usage: forge transcript <TASK>

Arguments:
  <TASK>  Task id, for example T-042
```

`forge task --help`:

```text
Inspect or clean orphaned worktrees

Usage: forge task <COMMAND>

Commands:
  clean  Inspect or clean orphaned worktrees
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

`forge task clean --help`:

```text
Inspect or clean orphaned worktrees

Usage: forge task clean [TASK]

Arguments:
  [TASK]  Task id; defaults to all orphaned worktrees
```

`forge ledger --help`:

```text
Check, repair, or migrate the decision ledger

Usage: forge ledger <COMMAND>

Commands:
  verify   Check and repair ledger backlinks and duplicate IDs
  migrate  Migrate older ledger schemas with a diff and confirmation
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

`forge ledger verify --help`:

```text
Check and repair ledger backlinks and duplicate IDs

Usage: forge ledger verify [OPTIONS]

Options:
      --repair  Apply safe metadata repairs; otherwise report only
  -h, --help    Print help
```

`forge ledger migrate --help`:

```text
Migrate older ledger schemas with a diff and confirmation

Usage: forge ledger migrate [OPTIONS]

Options:
      --dry-run  Print the migration preview without writing
  -h, --help     Print help
```

`forge help --help`:

```text
Print this message or the help of the given subcommand(s)

Usage: forge help [COMMAND]...

Arguments:
  [COMMAND]...  Print help for the subcommand(s)
```

The hidden commands have no public help blocks. Their internal argument contracts belong to TDD-0006 and TDD-0008.

### 5.5 Exit codes

The exit codes are fixed by I-CLI and FR-REC-007.

| Outcome | Code |
|---|---|
| Success, help, version, dry run | `0` |
| Successful read-only query (`status`, `audit`, `hooks status`, `transcript`, `sandbox probe`) | `0` |
| Usage, malformed config, or runtime error | `2` |
| Human gate pending, non-TTY stop, user decline, EOF at a prompt | `3` |
| Stopped or blocked: missing `run` prerequisites, merge conflict, `doctor` with a failed required prerequisite, refused `--guarded` | `4` |
| Cancelled task or SIGINT | `130` |
| Broken pipe (`EPIPE` or Windows `ERROR_BROKEN_PIPE`) | `0` |

Command-specific mapping:

| Command | `0` | `2` | `3` | `4` | `130` |
|---|---|---|---|---|---|
| `init` | Initialized, no changes, or dry run | Preflight/config/write error | Declined, EOF, uncommitted `.forge` proposal | Git below floor or blocked host probe | SIGINT |
| `run` | Task done | Malformed config or runtime error | First gate reached non-interactively | Missing prerequisites, live lock, active task wait/refusal | Cancelled |
| `resume` | Resumed and running | Malformed state/config | Gate pending | No resumable task, worktree/branch mismatch, hash drift | Cancelled |
| `status` | Report printed | Cannot read state | — | — | — |
| `doctor` | All required checks pass | Doctor could not run | — | A required prerequisite failed | — |
| `sandbox probe` | Probe ran | Probe could not run | — | — | — |
| `exec` | Approved command executed (its own exit status is printed) | Parse or policy error | Non-TTY confirmation needed | Denied or not run | — |
| `attest show` | Attestation printed | Invalid task id | — | No attestation exists | — |
| `merge` | Merged | Malformed record or Git error | Non-TTY confirmation needed | Not attestable, conflict, or override refused | — |
| `reopen` | Reopened | Malformed state | Non-TTY confirmation needed | Task is not `done` | — |
| `hooks status` | Report printed | Cannot inspect | — | — | — |
| `hooks install/repair` | Installed or repaired | Write error | Non-TTY or declined | Conflicting hook | — |
| `audit` | Report printed | Cannot read | — | — | — |
| `transcript` | Transcript printed | Missing task or unreadable log | — | — | — |
| `task clean` | Cleaned or nothing to do | State error | Non-TTY or declined | Dirty or unmerged worktree retained | — |
| `ledger verify` | No errors remain | Errors remain or read failure | — | — | — |
| `ledger migrate` | Migrated or nothing to migrate | Refused or failed | Non-TTY confirmation needed | — | — |

`forge exec` prints the executed command's own exit status but does not return it, because the `forge` exit-code contract is fixed. A command that runs and exits non-zero is still a successful `forge exec` (`0`); a denied or unexecuted command is `4`.

### 5.6 CLI error type

`infra_local::cli` defines one error type and no `anyhow`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{message}")]
    Usage { message: String },
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Init(#[from] InitError),
    #[error("blocked: {reasons:?}")]
    Blocked {
        reasons: Vec<String>,
        next: Option<String>,
    },
    #[error("a human gate is pending for {task_id}")]
    GatePending {
        task_id: TaskId,
        next: String,
    },
    #[error("cancelled")]
    Cancelled { task_id: Option<TaskId> },
    #[error("{message}")]
    Runtime { message: String },
}
```

`CliError` implements:

```rust
impl CliError {
    pub fn exit_code(&self) -> std::process::ExitCode;
}
```

Mapping rules:

- `Usage`, `Config`, `Runtime`, and init read/write failures map to `2`.
- `InitError::Declined`, `InitError::Interrupted`, and `InitError::ExistingUncommitted` map to `3`.
- `InitError::GitTooOld` maps to `4`.
- `Blocked` maps to `4`.
- `GatePending` maps to `3`.
- `Cancelled` maps to `130`.
- `BrokenPipe` is not an error; the writer returns a sentinel that `main` maps to `0`.

No `Box<dyn Error>`, `anyhow::Error`, or `Debug`-formatted internal error crosses the CLI boundary. Port and domain errors are converted at the command boundary into `CliError` with a named operation and a next command.

### 5.7 Output conventions

- Transcript and command results go to **stdout**.
- Diagnostics, warnings, progress, and next-command hints go to **stderr**.
- `--plain` output is linear, has no ANSI, no spinners, and no live region. It uses the same central sanitizer as the TUI for untrusted strings (TDD-0010).
- `--color=never`, `NO_COLOR`, or `TERM=dumb` disable ANSI. `--color=always` forces color unless `--plain` is set. `--color=auto` detects the terminal.
- `--theme=high-contrast` and `--theme=mono` change semantic tokens only. Meaning never depends on color alone.
- `--reduced-motion` and `FORGE_REDUCED_MOTION=1` disable animation.
- `--timestamps` adds timestamps to every transcript line; otherwise they appear only on gates, receipts, and system events.
- `--show-thinking` affects plain mode only. The TUI hides reasoning by default.
- No JSON output mode exists in v1. A JSON event stream is v1.1 (FR-TUI-010).
- English is the only v1 language. No localization framework is added.

Stable read-only output:

```text
$ forge status
Task: T-042
State: review
Mode: Sandboxed
Enforcement: okay
Budget: 1.23 USD / 2.00 soft; 5.00 hard (estimated)
Gate: review (pending)
Branch: forge/T-042
Worktree: .forge/worktrees/T-042
```

With no active task:

```text
No active task.
```

`forge doctor` prints one row per check, in this order: Git, Rust, platform, MXC, execution policy, trust, signing, hooks, provider, repository config. Each row starts with `[ok]`, `[warn]`, or `[fail]`, then the check name, detail, and an optional next command. A `[fail]` on a required prerequisite exits `4`; a `[warn]` does not. Signing missing is a `[warn]` because `forge run` may start, but attestation and merge will fail.

`forge audit` prints sections in this order: overrides, hook health, unattested merges. It exits `0` because it is a report; missing events are printed as gaps.

`forge hooks status` prints:

```text
pre-merge-commit: installed
reference-transaction: missing
Enforcement: degraded
```

`forge attest show` prints the Markdown view. A missing attestation prints `No attestation for T-042.` and exits `4`.

### 5.8 Hidden commands

- `forge __worker` is hidden and internal. It is spawned by the host inside MXC. Direct invocation without the host handshake fails closed with exit `2`. The handshake, frames, and limits are TDD-0006's contract.
- `forge hook` is hidden and internal. It is called by the managed Git hooks. Its arguments, trailer checks, and fail-closed behavior are TDD-0008's contract.
- Neither command appears in help, completion, or public documentation.
- Neither command may be invoked from the worker as an approval path.

### 5.9 Not in v1

No shell completions, man pages, `--json`, `--quiet`, `--verbose`, `--yes`, `--force`, `--no-sandbox`, or `--skip-permissions`. No mouse, images, in-app pager, or browser UI. These are v1.1 or never, per PRD §15.3.

---

## 6. `.forge/config`

### 6.1 File model

- Path: `<repo>/.forge/config`.
- Committed, UTF-8, no BOM, LF line endings, final newline.
- Maximum size: 1 MiB. A larger file is `ConfigIssueCode::TooLarge`.
- One file per repository. No per-package config.
- Writers: `forge init`, a confirmed re-init, and a host-applied plan-time map diff (TDD-0004). No other component writes it.
- The file is the source of truth. There is no hidden config store.

### 6.2 Complete example

```toml
# Forge repository configuration.
# Committed. Change it through `forge init`, a reviewed plan-time map diff, or an explicit human edit.

schema_version = 1

[project]
name = "forge"
base_ref = "main"

[map]
version = 1

[map.facets]
checks = { status = "confirmed", confirmed_absence = false }
entry_points = { status = "confirmed", confirmed_absence = false }
module_roots = { status = "confirmed", confirmed_absence = false }
docs = { status = "confirmed", confirmed_absence = false }
package_roots = { status = "confirmed", confirmed_absence = false }

[context]
entry_points = [
  { path = "src/main.rs", kind = "binary", primary = true },
]
module_roots = [
  { path = "src" },
]
docs = [
  { path = "AGENTS.md", precedence = 0 },
  { path = "README.md", precedence = 20 },
]
package_roots = [
  { path = ".", container = true },
]

[[checks]]
id = 1
kind = "build"
command = "cargo build --workspace"
cwd = "."
required = true
source = "manifest:Cargo.toml"

[[checks]]
id = 2
kind = "test"
command = "cargo test --workspace"
cwd = "."
required = true
source = "manifest:Cargo.toml"

[[checks]]
id = 3
kind = "lint"
command = "cargo clippy --workspace --all-targets -- -D warnings"
cwd = "."
required = true
source = "manifest:Cargo.toml"

[[checks]]
id = 4
kind = "format"
command = "cargo fmt --all -- --check"
cwd = "."
required = false
source = "manifest:Cargo.toml"

[verification]
mode = "automated"
checks = [1, 2, 3]

[profiles.opencode-go]
protocol = "openai-chat"
base_url = "https://opencode.ai/inference/go/openai/v1"
api_key_env = "OPENCODE_API_KEY"
default_model = "deepseek-v4.1-flash"

[slots.primary]
profile = "opencode-go"
model = "deepseek-v4.1-flash"

[slots.fast]
profile = "opencode-go"
model = "deepseek-v4.1-flash"

[routing]
planning = "primary"
coding = "primary"
review = "primary"
attestation = "primary"
title = "fast"
summary = "fast"
compaction = "fast"
classification = "fast"

[budgets]
soft_usd = "2.00"
hard_usd = "5.00"
```

### 6.3 Top-level fields

| Key | Type | Required | Rules |
|---|---|---|---|
| `schema_version` | integer | yes | Must be `1`. Missing is `MissingField`; greater is read-only `SchemaTooNew`; zero or negative is `SchemaUnsupported`. |
| `project` | table | yes | See §6.4. |
| `map` | table | yes | See §6.5. |
| `context` | table | yes | May contain empty arrays. See §6.6. |
| `checks` | array of tables | no | Default empty. See §6.7. |
| `verification` | table | yes | See §6.8. |
| `profiles` | table | no | Default empty. See §6.9. |
| `slots` | table | no | Default empty. See §6.10. |
| `routing` | table | no | Missing means the default routing table. See §6.11. |
| `budgets` | table | no | Missing means defaults `2.00` and `5.00`. See §6.12. |

Unknown top-level keys are warnings and are preserved. Unknown tables are preserved with their contents. A known key with the wrong type is an error.

### 6.4 `[project]`

| Key | Type | Required | Rules |
|---|---|---|---|
| `name` | string | yes | 1–64 printable Unicode scalar values, no control characters, no leading or trailing whitespace. Default at init is the repository directory name. |
| `base_ref` | string | yes | A branch name, tag, or full commit SHA. No leading `-`, no whitespace, no control characters, no `..`, no `@{`. Must resolve at init and at `forge run`. |

The base ref is a ref, not a snapshot. A branch may advance; the task records `base_before` and `base_after` at merge (TDD-0008). A full commit SHA pins the base.

### 6.5 `[map]` and `[map.facets]`

```toml
[map]
version = 1

[map.facets]
checks = { status = "confirmed", confirmed_absence = false }
entry_points = { status = "confirmed", confirmed_absence = false }
module_roots = { status = "confirmed", confirmed_absence = false }
docs = { status = "confirmed", confirmed_absence = false }
package_roots = { status = "confirmed", confirmed_absence = false }
```

| Key | Type | Required | Rules |
|---|---|---|---|
| `version` | integer | yes | `>= 1`. Increments only when confirmed map content changes. |
| `facets` | table | yes | Exactly the five facet keys. No duplicates. |
| `facets.<name>.status` | string | yes | `confirmed` or `unknown`. `proposed` is invalid in the committed file. |
| `facets.<name>.confirmed_absence` | boolean | yes | `true` only when `status = "confirmed"` and the facet has no entries. |

Facet invariants:

- A facet with entries must have `status = "confirmed"` and `confirmed_absence = false`.
- A facet with `status = "confirmed"` and no entries must have `confirmed_absence = true`.
- A facet with `status = "unknown"` must have no entries and `confirmed_absence = false`.
- `checks` is `confirmed` when automated checks exist or when manual verification is configured. It is `confirmed` with `confirmed_absence = true` only for manual verification with no checks. It is `unknown` when verification is unconfigured.
- `entry_points` may be confirmed absent for a library or a service with no local entry point.
- `module_roots` may be confirmed absent only when the repository has no human-authored source; the summary then reports the repository as outside v1's code-task scope (FR-ONB-019).
- `docs` may be confirmed absent; init warns and `forge run` is not blocked.
- `package_roots` may be confirmed absent; a repo-level root/container group is still used for review.

The five facets are always present in `[map.facets]`, even when unknown. This keeps the file shape stable and makes the human's unknowns explicit.

### 6.6 `[context]`

| Array | Entry fields | Rules |
|---|---|---|
| `entry_points` | `path` (string, required), `kind` (string, required), `primary` (boolean, default false) | `path` is repo-relative POSIX, points to an existing file at init, no `..`, no absolute path. `kind` is a non-empty lowercase kebab-case word; scan proposes `binary`, `library`, `service`, `cli`, `script`, `package`, `module`, or `other`. At most one entry has `primary = true`. |
| `module_roots` | `path` (string, required) | Repo-relative POSIX, points to an existing directory at init, no `..`, no absolute path. |
| `docs` | `path` (string, required), `precedence` (integer, required) | Repo-relative POSIX, points to an existing file at init, no `..`, no absolute path. `precedence` is 0–100; lower is higher. `AGENTS.md` is canonical among instruction docs on conflict. |
| `package_roots` | `path` (string, required), `container` (boolean, default false) | Repo-relative POSIX, points to an existing directory at init, no `..`, no absolute path. `container = true` means the root contains other confirmed roots; a root may be both a container and a package. |

Rules:

- Arrays are sorted by `path` then `kind` in canonical output.
- Duplicate `(path, kind)` entry points, duplicate module-root paths, duplicate doc paths, and duplicate package-root paths are errors.
- A confirmed entry's status is `Confirmed` when mapped to TDD-0002; `Proposed` never appears in config.
- No `source`, `confidence`, `stale_reference`, or `status` field is stored per entry. Evidence stays in the init report. The only `source` in the config is on checks, where FR-ONB-011 requires it.
- A module root or package root that escapes the repository through a symlink is rejected. Paths are lexically normalized and then checked against the canonical repo root.

### 6.7 `[[checks]]`

| Key | Type | Required | Rules |
|---|---|---|---|
| `id` | integer | yes | `>= 1`, unique, stable. Removed IDs are reserved and never reused. |
| `kind` | string | yes | `build`, `test`, `lint`, `format`, `typecheck`, or `setup`. |
| `command` | string | yes | 1–4096 printable characters, no NUL, no newline. The exact command Forge runs. |
| `cwd` | string | yes | Repo-relative POSIX, default `.`, must exist and be a directory. A path outside the repo is an error. A path outside confirmed package roots is a warning, not an error. |
| `required` | boolean | yes | Required checks are part of the full verification set. |
| `source` | string | yes | Evidence token from §10.4, for example `manifest:Cargo.toml`. It must not contain a secret. |

Checks are sorted by `id`. Unknown fields are preserved.

Check identity across re-init is `(kind, command, cwd)`. If a scan proposes the same identity, the existing ID is kept. New checks receive `max(existing ids) + 1`. A removed check's ID is not reused, even if a later scan proposes the same command again; the human can re-add it with a new ID or keep the old one through an explicit edit.

### 6.8 `[verification]`

| Key | Type | Required | Rules |
|---|---|---|---|
| `mode` | string | yes | `automated`, `manual`, or `unconfigured`. |
| `checks` | array of integers | conditional | Required and non-empty when `mode = "automated"`; each ID must exist in `[[checks]]`. Must be absent or empty for `manual` and `unconfigured`. |
| `method` | string | conditional | Required when `mode = "manual"`; one line, 1–200 printable characters, no newline. Must be absent for `automated` and `unconfigured`. |

`automated` with no checks is invalid. `automated` must include every check with `required = true`; optional checks may also be included. `manual` with no method is invalid. `unconfigured` is valid at init and produces a `forge run` blocker (FR-ONB-004, FR-ONB-018).

### 6.9 `[profiles.<name>]`

Profile names are lowercase kebab-case: `[a-z0-9][a-z0-9-]*`, 1–64 characters.

| Key | Type | Required | Rules |
|---|---|---|---|
| `protocol` | string | yes | `openai-chat` or `anthropic-messages`. |
| `base_url` | string | yes | Absolute URL. `https` for any host; `http` only for loopback (`localhost`, `127.0.0.1`, `::1`). No userinfo, query, or fragment. A trailing slash is normalized away. |
| `api_key_env` | string | conditional | Environment variable name matching `[A-Za-z_][A-Za-z0-9_]*`. Required for `opencode-go`, `anthropic`, and `openai` templates; optional for a custom profile with no auth. Empty is invalid. |
| `default_model` | string | yes | 1–128 printable characters, no control characters. Free text; the curated default belongs to TDD-0007. |
| `extra_headers` | table of strings | no | Header names `[A-Za-z0-9-]+`; values printable, no CR or LF. Forbidden names: `authorization`, `proxy-authorization`, `x-api-key`, `api-key`, `cookie`, `set-cookie`. |
| `allowance_key` | string | no | Dotted key such as `opencode.allowance`. Used by the OpenCode Go profile. |

Committed profiles never contain secret values. `api_key_env` names the environment variable; the key file is `.forge/local/keys/providers.toml` and is TDD-0007's contract. `extra_headers` must not carry a secret.

The four init templates write these starting profiles:

| Template | Protocol | Base URL | Key variable | Default model |
|---|---|---|---|---|
| OpenCode Go | `openai-chat` | `https://opencode.ai/inference/go/openai/v1` | `OPENCODE_API_KEY` | `deepseek-v4.1-flash` |
| Anthropic direct | `anthropic-messages` | `https://api.anthropic.com` | `ANTHROPIC_API_KEY` | TDD-0007's curated default |
| OpenAI direct | `openai-chat` | `https://api.openai.com/v1` | `OPENAI_API_KEY` | TDD-0007's curated default |
| Custom OpenAI-compatible | `openai-chat` | user-supplied | optional | user-supplied |

A custom profile may use `http://127.0.0.1:<port>/v1` for a local runtime. Forge does not install, start, download, or manage that runtime.

### 6.10 `[slots.<name>]`

Only `primary`, `fast`, and `local` exist.

```toml
[slots.primary]
profile = "opencode-go"
model = "deepseek-v4.1-flash"

[slots.fast]
profile = "opencode-go"
model = "deepseek-v4.1-flash"
```

| Key | Type | Required | Rules |
|---|---|---|---|
| `profile` | string | yes | Must name a profile in the committed config or the local patch. |
| `model` | string | yes | 1–128 printable characters. Free text. |

`primary` is required for `forge run`. `fast` is optional; when absent, the effective fast slot is `primary`. `local` is optional and is used only when explicitly selected or named by routing. There is no automatic cross-profile fallback.

### 6.11 `[routing]`

```toml
[routing]
planning = "primary"
coding = "primary"
review = "primary"
attestation = "primary"
title = "fast"
summary = "fast"
compaction = "fast"
classification = "fast"
```

The keys are exactly `planning`, `coding`, `review`, `attestation`, `title`, `summary`, `compaction`, and `classification`. Each value is `primary`, `fast`, or `local`. If the table is present, all eight keys are required. If absent, the default table above applies. A referenced slot must exist.

Routing chooses a slot, not a provider. A provider failure pauses; it never switches profiles (FR-MODEL-006).

TDD-0007 owns the model-call taxonomy and may add a new call kind only by amending this table and this document. TDD-0001 fixes the v1 keys.

### 6.12 `[budgets]`

```toml
[budgets]
soft_usd = "2.00"
hard_usd = "5.00"
```

| Key | Type | Required | Rules |
|---|---|---|---|
| `soft_usd` | string | yes when the table is present | `"none"` or a decimal string with 1–6 fractional digits, `0` to `1000000`. No sign, exponent, comma, or whitespace. |
| `hard_usd` | string | yes when the table is present | Same grammar. |

If `[budgets]` is absent, defaults are `2.00` soft and `5.00` hard. If the table is present, both keys are required; use `"none"` to disable one. When both are numeric, `soft <= hard`. Values convert exactly to `UsdMicros`; no floating-point parsing.

### 6.13 Canonical serialization

- Generated files start with the two-line comment in §6.2.
- Top-level order: `schema_version`, `[project]`, `[map]`, `[context]`, `[[checks]]`, `[verification]`, `[profiles.*]`, `[slots.*]`, `[routing]`, `[budgets]`.
- Table order inside `[map.facets]`: `checks`, `entry_points`, `module_roots`, `docs`, `package_roots`.
- Context arrays: `entry_points`, `module_roots`, `docs`, `package_roots`, each sorted by `path` then `kind`.
- Checks sorted by `id`; profiles sorted by name; slots in `primary`, `fast`, `local` order; routing in the fixed key order.
- Strings are double-quoted. No multiline strings. Paths use `/`.
- Array formatting is one inline entry per line with a trailing comma, as in the example.
- No timestamps, no hostnames, no machine paths.
- `toml_edit` preserves comments, blank lines, and unknown keys. Forge inserts new known keys at their canonical position and updates existing values in place. A rewrite that changes no value leaves the file byte-identical.
- Unknown keys are warnings and are preserved. An unknown table is preserved with its contents.

### 6.14 Validation

Validation runs in this order:

1. Read the file; fail on read error, non-UTF-8, or size over 1 MiB.
2. Parse TOML syntax.
3. Check `schema_version`; missing, newer, or unsupported.
4. Deserialize known fields with `serde` and `toml`.
5. Collect unknown keys by comparing against the known key set; each is a warning.
6. Validate every field rule, cross-field invariant, path rule, and reference.
7. Merge `.forge/local/config.local.toml` if present.
8. Re-run the cross-field validation on the effective config.
9. Compute the effective config hash and map to `ConfigSnapshot`.

`ConfigIssue` is an infrastructure-local record:

```rust
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[error("{code:?} at {path}: {message}")]
pub struct ConfigIssue {
    pub code: ConfigIssueCode,
    pub path: String,       // dotted key, for example "verification.checks[0]"
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigIssueCode {
    Parse,
    TooLarge,
    MissingField,
    InvalidType,
    InvalidValue,
    UnknownField,
    DuplicateCheckId,
    DuplicateEntry,
    UnknownCheck,
    UnknownProfile,
    UnknownSlot,
    InvalidPath,
    InvalidBudget,
    SchemaTooNew,
    SchemaUnsupported,
}
```

`UnknownField` is a warning. Every other code is an error for load. `ConfigPort::load` maps errors to TDD-0002's `ConfigError`:

- `Parse`, `InvalidType`, `InvalidValue`, `Duplicate*`, `Unknown*`, `InvalidPath`, `InvalidBudget` → `ConfigError::Parse { path, message }` with the dotted path and issue code in the message.
- `TooLarge` → `ConfigError::Parse { path, message }`.
- `MissingField` → `ConfigError::MissingField { field }`.
- `SchemaTooNew` → `ConfigError::SchemaTooNew { found, supported }`.
- `SchemaUnsupported` → `ConfigError::Parse { path, message }`.
- Read failures → `ConfigError::ReadFailed`.

`ConfigPort::load` returns the snapshot; warnings are logged and surfaced by `forge doctor` and init. `forge run` refuses on any error with exit `2`.

`forge init` uses the richer `ConfigIssue` for the brief and summary. It does not expose a new public error type.

### 6.15 Versioning and unknown fields

- `schema_version` is `1`.
- A newer schema is read-only. `forge run` refuses; `forge init` prints the version and migration path and exits `2`; `forge doctor` reports it.
- An older schema migrates only with a diff and confirmation. There is no older v1 schema; the migration path is the same mechanism future versions use.
- Unknown fields are warnings and are preserved on rewrite. Unknown enum values are errors, because Forge cannot safely guess their meaning.
- A missing required field is an error, never a default.
- An unreadable config in `forge init` is an error; in an already-initialized repo it blocks `forge run`.

### 6.16 Local override file

Path: `.forge/local/config.local.toml`. Local-only, gitignored, optional, UTF-8, TOML.

- It carries `schema_version = 1`; it may not change the value.
- It may override only `[profiles.*]`, `[slots.*]`, `[routing]`, and `[budgets]`.
- It may add a profile or override a profile by name. The committed profile remains in the file; the local one wins for this machine.
- It may override a slot or routing key. It may not delete a slot or routing key.
- It may not contain `[project]`, `[map]`, `[context]`, `[[checks]]`, or `[verification]`. Presence is an error.
- A missing local file means no overrides.
- Unknown fields are preserved; the same schema rules apply.
- Precedence: built-in defaults → committed config → local config → CLI flags that are explicitly single-command overrides (`--base-ref` at init/run).
- Secrets never enter the committed config. A local profile still uses `api_key_env`; the key file is TDD-0007's contract.

### 6.17 Mapping to TDD-0002

`ConfigSnapshot` is built as follows:

| `ConfigSnapshot` field | Source |
|---|---|
| `schema_version` | Config `schema_version`. |
| `repo_root` | Canonical repository root, resolved from the config path. |
| `config_hash` | SHA-256 of the canonical effective config JSON, including local overrides and `map.version`. |
| `base_ref` | `project.base_ref`. |
| `repo_map` | See §7.10. |
| `checks` | `checks[]` mapped to `CheckDefinition`; `CheckId` is the integer ID. |
| `verification` | `Automated { checks }`, `Manual { method }`, or `Unconfigured`. |
| `budgets` | `Budget { soft, hard }` in `UsdMicros`. |
| `slots` | Each effective slot mapped to `ModelSlotBinding`. When `fast` is absent, the snapshot contains a `fast` binding equal to `primary`; when `local` is absent, no local binding is present. |

`project.name`, profiles, and routing stay in the infrastructure-local `EffectiveConfig`; the core does not need them. `wiring` uses them to construct model adapters. This is why TDD-0002 is not amended.

`EntryPoint.source` and `ModuleRoot.source` are set to `human-confirmed`; `status` is `FacetStatus::Confirmed`. `ContextDoc.stale_reference` is `false` in the config snapshot because stale references are local scan evidence, not committed map facts. `PackageRoot.source` is `human-confirmed`. `CheckDefinition.source` uses the config `source`.

### 6.18 Config hash

The effective config hash is SHA-256 over this canonical JSON projection:

```json
{
  "budgets": { "hard_usd": "5.00", "soft_usd": "2.00" },
  "checks": [],
  "map": { "facets": {}, "version": 1 },
  "profiles": {},
  "project": { "base_ref": "main", "name": "forge" },
  "routing": {},
  "schema_version": 1,
  "slots": {},
  "verification": {}
}
```

Rules:

- Keys are sorted lexicographically; arrays are sorted by their documented identity.
- `none` budgets become `null`; numeric budgets remain strings.
- No whitespace, no comments, UTF-8, no trailing newline.
- The hash is 64 lowercase hexadecimal characters.
- It includes local overrides because it is the effective config a task runs with.
- It includes `map.version`; the separate map hash excludes it.

### 6.19 Write API and composition

`config` is an outbound adapter. It exposes:

```rust
pub struct TomlConfig { /* repo_root */ }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded<T> {
    pub value: T,
    pub warnings: Vec<ConfigIssue>,
}

impl TomlConfig {
    pub fn load_committed(&self) -> Result<Loaded<ConfigFile>, ConfigIssue>;
    pub fn load_local(&self) -> Result<Loaded<Option<ConfigPatch>>, ConfigIssue>;
    pub fn load_effective(&self) -> Result<Loaded<EffectiveConfig>, ConfigIssue>;
    pub fn propose(&self, next: &ConfigFile) -> Result<ConfigDiff, ConfigIssue>;
    pub fn write(&self, next: &ConfigFile) -> Result<WriteReport, ConfigIssue>;
}
```

`TomlConfig` implements `ConfigPort` by returning `ConfigSnapshot`. It never writes in `load`. `Loaded.warnings` carries `UnknownField` issues; `Loaded.value` is the parsed file. `ConfigPort::load` logs the warnings and returns the snapshot.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigDiff {
    pub changed: bool,
    pub unified: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteReport {
    pub written: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanReport {
    pub report: InitReport,
    pub proposals: ProposedMap,
    pub warnings: Vec<ScanWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedMap {
    pub entry_points: Vec<Candidate<EntryPointCandidate>>,
    pub module_roots: Vec<Candidate<ModuleRootCandidate>>,
    pub package_roots: Vec<Candidate<PackageRootCandidate>>,
    pub docs: Vec<Candidate<DocCandidate>>,
    pub checks: Vec<Candidate<CheckCandidate>>,
}
```

`ConfigDraft` is the mutable in-memory brief state: the current `ConfigFile`, the proposed rows, and the verification choice. `InitProposal` is the fully validated result of the brief and setup phases: the next `ConfigFile`, the `InitReport`, the decision-policy proposal (TDD-0004/0006), the execution-policy proposal (TDD-0006), the trust declaration (TDD-0006), the signing plan (TDD-0008), the hook plan (TDD-0008), the optional `ConfigDiff`, and the list of `run` blockers. The policy, execution-policy, signing, and hook proposal types are opaque here and owned by their TDDs; `InitRuntime::apply` passes them through.

The scan source is an infrastructure-local `GitQuery` trait, injected by `wiring`. It is not an outbound adapter; `scan` is a host service module:

```rust
pub trait GitQuery: Send + Sync {
    fn repo_root(&self) -> Result<PathBuf, ScanError>;
    fn head(&self) -> Result<CommitSha, ScanError>;
    fn current_branch(&self) -> Result<Option<BranchName>, ScanError>;
    fn default_branch(&self) -> Result<Option<BranchName>, ScanError>;
    fn is_dirty(&self) -> Result<bool, ScanError>;
    fn list_files(&self) -> Result<Vec<PathBuf>, ScanError>;
    fn read_file(&self, path: &Path) -> Result<Option<Vec<u8>>, ScanError>;
    fn remotes(&self) -> Result<Vec<RemoteInfo>, ScanError>;
    fn submodules(&self) -> Result<Vec<PathBuf>, ScanError>;
    fn nested_repos(&self) -> Result<Vec<PathBuf>, ScanError>;
    fn hooks_present(&self) -> Result<Vec<PathBuf>, ScanError>;
}
```

`GitCli` implements `GitQuery` in TDD-0008. The scan module must not import the `git`, `config`, or `cli` adapter modules; `wiring` injects the implementation.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub direction: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("Git query failed: {message}")]
    Git { message: String },
    #[error("failed to read {path:?}: {message}")]
    Read { path: PathBuf, message: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate<T> {
    pub row_id: String,
    pub confidence: Confidence,
    pub source: String,
    pub evidence: Vec<String>,
    pub value: T,
}
```

`Confidence` has one variant in v1, `Proposed`. The JSON form in §10.4 flattens `value` into the candidate object.

`wiring` exposes:

```rust
pub struct InitRuntime { /* private */ }

impl InitRuntime {
    pub fn new(repo_root: PathBuf) -> Result<Self, InitError>;
    pub async fn scan(&self) -> Result<ScanReport, InitError>;
    pub fn current_map(&self) -> Result<Option<ConfigFile>, InitError>;
    pub fn propose(&self, draft: &ConfigDraft) -> Result<InitProposal, InitError>;
    pub async fn probe_sandbox(&self) -> Result<SandboxProbeReport, InitError>;
    pub async fn apply(&self, proposal: &InitProposal) -> Result<WriteReport, InitError>;
}
```

`cli::init` drives the prompts and formatting. It never names `TomlConfig`, `GitCli`, `MxcSandbox`, or any other concrete adapter. The scan source is injected by `wiring`; `cli` sees only `ScanReport`.

### 6.20 `.forge/.gitignore`

Init always writes this committed file:

```gitignore
local/
worktrees/
active-task.lock
```

It does not edit the root `.gitignore`. It does not ignore `decisions/`, `assumptions/`, `attestations/`, `overrides/`, `trust/`, `config`, `policy`, `execution-policy.json`, or itself.

---

## 7. Repo map contract

### 7.1 Facets and statuses

The five facets are `checks`, `entry_points`, `module_roots`, `docs`, and `package_roots`.

| Status | Meaning | In config | In init report |
|---|---|---|---|
| `confirmed` | A human confirmed the entries or the absence. | Yes | No |
| `proposed` | A scan candidate that is not confirmed. | Never | Yes |
| `unknown` | The human rejected the candidates or the scan found none. | Yes | Yes |

`confirmed_absence` is a boolean modifier on a confirmed facet. It is valid only when the facet has no entries. It is how a library confirms no entry point and how a human confirms no docs or no package roots.

### 7.2 Path rules

- Committed paths are repo-relative, POSIX-separated, no leading `./`, no trailing `/`, no `.` or `..` component, no drive letter, no UNC path, no backslash.
- The repo root is `.` only in `package_roots`.
- A file path must exist at init. A directory path must exist at init.
- A path is lexically normalized and then checked against the canonical repo root. A symlink or junction that resolves outside the root is rejected.
- Paths in the local init report are also repo-relative. Only `repo_root` is absolute.
- Case sensitivity follows the filesystem. Forge does not fold case.

### 7.3 Version and hash

- `[map] version` starts at `1`.
- It increments by one when a confirmed map change alters any of: facets, entry points, module roots, docs, package roots, check identity, or verification choice.
- It does not increment for a formatting-only rewrite, comment change, local override, or a re-init with no map change.
- The map hash is SHA-256 over the canonical JSON of the confirmed map content:

```json
{
  "checks": [],
  "entry_points": [],
  "facets": [],
  "module_roots": [],
  "package_roots": [],
  "docs": [],
  "verification": {}
}
```

- Keys are sorted; arrays are sorted by path then kind; `status` is `"confirmed"` or `"unknown"`; `confirmed_absence` is included.
- The hash excludes `map.version`, `project`, profiles, slots, routing, and budgets. It includes the confirmed checks and the verification choice because `checks` is one of the five facets. It is the map content identity, not the whole config identity.
- The hash is 64 lowercase hexadecimal characters.
- `RepoMapSnapshot.hash` is this value. `RepoMapSnapshot.version` is `[map] version`.

### 7.4 Entry points

- An entry point is a file that starts a program, service, library, script, or package surface.
- `kind` is proposed by the scan from a closed v1 set; a human may edit it to another lowercase kebab-case word.
- At most one entry point is `primary`. The primary marker is optional.
- A confirmed entry point may be `proposed` in the report but becomes `confirmed` in config only through the brief.
- A library may confirm `entry_points` absent; `confirmed_absence = true`.
- Doc-only entry points stay `proposed` and are not used as confirmed context (FR-ONB-008).

### 7.5 Module roots

- A module root is a directory that owns source files the plan can name as an owning unit.
- It must contain at least one tracked source file at confirmation time.
- Generated and vendored trees are never proposed as module roots unless the human explicitly adds them.
- A module root may be the repo root `.` for a flat project.
- A module root is not a package root unless it also has a recognized manifest.

### 7.6 Package roots

- A package root is a directory containing a recognized manifest (`Cargo.toml`, `package.json`, `pyproject.toml`, `setup.py`, `setup.cfg`, `go.mod`).
- Workspace declarations are evidence only. A workspace member without its own manifest is `unknown`, not a package root.
- A root that contains other confirmed roots defaults to `container = true`. The human may also confirm it as a package; the two are not exclusive.
- The deepest confirmed root owns a changed file. A file outside every confirmed root belongs to the outside-map group.
- A package root move or rename is a repo-map change. It does not create a new ledger, policy, execution policy, worktree, branch, or trust declaration.
- No per-package config, ledger, policy, worktree, branch, lock, or trust is created.

### 7.7 Docs

- Root docs are `AGENTS.md`, `CLAUDE.md`, `README*`, and `CONTRIBUTING*`.
- Markdown under `docs/` is proposed with lower precedence.
- Other agent rule files (for example `.cursorrules`, `.github/copilot-instructions.md`, `GEMINI.md`) are reported as non-authoritative evidence. They enter `context.docs` only if the human adds them.
- Precedence: `AGENTS.md` = 0, `CLAUDE.md` = 10, `README*` = 20, `CONTRIBUTING*` = 30, `docs/**/*.md` = 40. Lower is higher.
- `AGENTS.md` remains canonical among instruction docs on conflict.
- Docs are context, never authority. They are labeled untrusted repository data (PRD §11.9).
- A broken relative link is `stale-reference`. It is local evidence and does not enter config.
- A potential conflict is two or more instruction docs at the same or adjacent precedence with different content hashes. The brief shows them side by side. It never silently picks one.
- Docs are never rewritten, merged, or generated by Forge.

### 7.8 Checks

- A check is a command with `kind`, `command`, `cwd`, `required`, and `source`.
- `kind` is one of `build`, `test`, `lint`, `format`, `typecheck`, `setup`.
- `required` defaults: build/test/lint/setup `true`; format/typecheck `false`. The human may change it.
- A check is proposed only when a manifest declares the script or a conventional target is justified by tracked files. Forge never invents a command for an unrecognized build system.
- Forge never runs a check during init.
- The full verification set is the `required` checks plus any optional checks the human adds to `[verification].checks`.
- A package with no check is valid at init. A touched package with no matching check is handled at plan time (FR-VER-010); it is not an init error.
- A required check that cannot run is a decision at plan time, not a skip.

### 7.9 Verification choice

- `automated`: one or more checks; `[verification].checks` lists them.
- `manual`: a one-line method; the first task expands it into a concrete checklist at the plan gate (FR-VER-008).
- `unconfigured`: init finishes with a warning; `forge run` refuses until configured (FR-ONB-004, FR-ONB-018).
- The choice is recorded in `[verification]` and reflected in `[map.facets].checks`.
- Init never scaffolds tests.

### 7.10 Base ref

- `project.base_ref` is required.
- Init proposes the current branch, then the remote default branch, then local `main` or `master` only if those refs exist. It never assumes `main` or a GitHub remote.
- A detached HEAD proposes the exact commit SHA.
- `--base-ref` supplies the value non-interactively. It is resolved at init; an invalid ref is an error.
- `forge run --base-ref` supplies a one-time base ref when config is missing one; it is recorded in the task, not written to config.
- The base ref is resolved to a commit at task intake. The task records the base commit. A branch may advance; merge records `base_before` and `base_after`.

### 7.11 Deepest root and container

- A changed file belongs to the deepest confirmed package root that contains it.
- If the file is outside every confirmed root, it belongs to the outside-map group.
- If a root is confirmed as a container, it is still a package root; the container flag is descriptive, not exclusive.
- The root/container group and the outside-map group are deterministic and shared by the plan, verification selection, and review (FR-REV-005).

---

## 8. Init flow

### 8.1 Phases

`forge init` runs these phases in order:

1. Preflight
2. Scan
3. Base-ref confirmation
4. Repo brief
5. Verification choice
6. Provider setup
7. Execution-policy probe and review
8. Trust declaration
9. Signing setup
10. Hooks setup
11. Summary and final confirmation
12. Apply
13. Done

`--dry-run` runs preflight, scan, a read-only probe, and a proposal summary, then stops. It never prompts, writes, validates the network, or runs canaries.

The flow is in-memory. No phase state is persisted between processes. Aborting before Apply writes nothing.

### 8.2 Preflight

Preflight is read-only and runs first.

| Check | Behavior on failure |
|---|---|
| Locate the repository root with `git rev-parse --show-toplevel` | Not a Git repository: `NotGitRepository`, exit `2`, no writes. |
| Resolve `HEAD` with `git rev-parse --verify HEAD` | No commits: `NoCommits`, exit `2`, no writes. |
| Check Git version against 2.39.4 | Below floor: `GitTooOld`, exit `4`, no writes. |
| Check the working tree with `git status --porcelain` | Dirty: warn and continue. The first task creates its own worktree from the base commit. |
| Inspect an existing `.forge/` | See §8.11. |
| Detect stale `forge-init-*` staging directories | Warn; offer removal only after confirmation. Never delete user files. |
| Resolve `--base-ref` if supplied | Invalid: exit `2`, no writes. |

Preflight does not probe MXC, install anything, or read repository file contents. The scan does that.

### 8.3 Scan

The scan is the deterministic procedure in §11. It produces an in-memory `ScanReport` and a proposed `ConfigFile`. It writes nothing. It does not network. It does not execute repository code.

The scan reads only tracked files reported by `git ls-files -z`. It parses only the file types in §11.4. It caps individual manifest reads at 1 MiB and treats an unreadable or unparseable file as a warning, never a silent skip.

### 8.4 Base-ref confirmation

If `--base-ref` was supplied and valid, use it. Otherwise show the candidates and ask:

```text
Base ref
  1. main (current branch)
  2. origin/main (remote default)
  3. 1a2b3c4 (detached HEAD)
Enter a ref or a number:
```

There is no default and no `none`. `project.base_ref` is required, and init cannot apply a config without it. A missing answer aborts with exit `3`. A hand-edited config that loses `base_ref` is handled by `forge run`, which asks for a ref and accepts `--base-ref`; the config loader reports the missing field and the run command opens the prompt rather than treating it as a fatal error.

### 8.5 Repo brief

The repo brief is the grammar in §9. It shows current facts (on re-init) and proposals. It never writes a fact without an explicit human command. `a all` is allowed but explicit; Enter never accepts a proposal.

At the end of the brief, the unresolved proposals are not written. A facet with no confirmed entries becomes `unknown`; a facet with confirmed entries becomes `confirmed`. The human may confirm absence with `none <facet>`.

### 8.6 Verification choice

After the brief, init asks:

```text
Verification
  1. Automated checks
  2. Manual method
  3. Unconfigured (forge run will refuse)
Choose 1-3:
```

There is no default.

- `1` lists the confirmed checks and asks which are in the verification set. The default selection is all `required` checks; the human may deselect. If no check is confirmed, option 1 is unavailable.
- `2` asks for one line. Empty is invalid.
- `3` warns that `forge run` will refuse until verification is configured.

The choice writes `[verification]` and `[map.facets].checks` at apply.

### 8.7 Provider setup

Provider setup asks for a template or "skip":

```text
Provider
  1. OpenCode Go (default)
  2. Anthropic direct
  3. OpenAI direct
  4. Custom OpenAI-compatible
  5. Skip (forge run will refuse)
Choose 1-5:
```

There is no default. For a template, init asks for a profile name (default: the template slug), confirms the base URL, and reads the key in this order:

1. The profile's `api_key_env` environment variable.
2. `.forge/local/keys/providers.toml`.
3. A hidden interactive prompt.

A key from the environment or local file is never copied to the committed config. A key entered interactively may be saved to `.forge/local/keys/providers.toml` only after an explicit `yes`; the default is no. The key file shape is TDD-0007's contract.

Validation uses `GET /models` where available, or a consented one-token request. `--no-network` skips validation. A missing key, invalid key, or failed validation leaves init finished with a warning; `forge run` refuses until the provider is configured. Init never sends repository content to a provider.

If the human skips, no `[profiles]` or `[slots]` are written. The summary names the provider as unconfigured. `forge run` refuses.

### 8.8 Execution-policy probe and review

TDD-0006 owns the probe, the canaries, the `.forge/execution-policy.json` schema, and the mode logic. Init owns the handoff:

1. Run the deterministic host probe.
2. Generate a baseline policy with no model inference.
3. Show the proposed policy and the probe result.
4. Ask the human to accept, edit the file, or skip. Accepting is a setup gate, not a ledger decision (FR-EXEC-011).
5. Run the negative canary suite after acceptance.
6. Record the resulting mode: `Sandboxed`, `Sandboxed — network denied`, or `Guarded — not sandboxed`.
7. A trusted repo with a failed probe or canary may continue in `Guarded` with the persistent label. An untrusted repo without `Sandboxed` is a `forge run` blocker; init still finishes and names it.
8. `--dry-run` does not run canaries; it prints the proposed policy summary and "canaries: not run (dry run)".

Init never auto-accepts the execution policy, never describes `Guarded` as a sandbox, and never uses `--force`.

### 8.9 Trust declaration

Init asks explicitly:

```text
Repository trust
  1. Trusted (your own repository or one you have reviewed)
  2. Untrusted (cloned from an unknown source, a contribution target, or unreviewed)
Choose 1-2:
```

There is no default. An unanswered or missing declaration is untrusted (FR-ONB-014). The declaration is stored in `.forge/local/repo-trust.json` at apply, not committed. Changing it later is a human-only audited action; a mid-task change pauses the task. TDD-0006 owns the file shape and audit.

### 8.10 Signing and hooks

Signing:

- Prefer an existing SSH or GPG signing key.
- If none exists, offer to generate a repo-local SSH Ed25519 key under `.forge/local/keys/` using repo-local `-c` overrides; never change global Git config.
- Offer to skip. Skipping finishes init but blocks attestation and merge; the summary says so.
- `.forge/trust/allowed_signers` is committed when a key is configured. TDD-0008 owns its content and the signing mechanics.

Hooks:

- Explain `pre-merge-commit` and `reference-transaction` once.
- Install after confirmation.
- `--no-hooks` opts out.
- Never overwrite an existing hook. Print the exact lines to add and leave enforcement degraded.
- `forge hooks status` reports installed, missing, outdated, or conflicting.
- TDD-0008 owns the shim content, installation mechanics, and fail-closed behavior.

### 8.11 Summary and final confirmation

Before any write, init prints:

```text
Init summary
  Repository: /home/dev/acme (main @ 1a2b3c4)
  Files: .forge/config (new), .forge/policy (new), .forge/execution-policy.json (new),
         .forge/.gitignore (new), .forge/local/init-report.json (new),
         .forge/local/repo-trust.json (new), .forge/trust/allowed_signers (new)
  Confirmed: entry_points 2, module_roots 1, package_roots 1, docs 2, checks 4
  Unknown: none
  Unconfigured: none
  Verification: automated [1, 2, 3]
  Execution mode: Sandboxed
  Trust: trusted
  Signing: existing SSH key (SHA256:...)
  Hooks: installed
  Blocks forge run: none

Type "apply" to write these files, or "cancel":
```

`apply` is typed in full; `y`, `yes`, or Enter do not apply. `cancel` exits `3` with no writes. EOF exits `3`. Ctrl-C exits `130`.

After apply:

```text
Initialized /home/dev/acme. Wrote 7 files.
Next: forge run "your task goal"
```

If there are blockers, the last line names them:

```text
forge run will refuse until: verification is unconfigured; provider key is missing.
```

Re-init prints a diff and the same typed `apply`. If nothing changed, it prints `No changes.` and exits `0` without writing.

### 8.12 Apply and rollback

Fresh init:

1. Build every file's content in memory.
2. Create a staging directory `forge-init-<pid>-<timestamp>` at the repository root.
3. Write all committed and local files into the staging directory with the final relative layout.
4. Fsync files and the directory where supported.
5. Rename the staging directory to `.forge` on the same volume.
6. Install hooks last.
7. Remove the staging directory if any pre-rename step fails.

If the rename succeeds but a later local key or hook write fails, `.forge` remains, the summary reports the degraded piece, and init exits `0` only when `forge run` is still possible. A failure that prevents a runnable config exits `2` with the exact path and a next command.

Re-init:

1. Refuse `apply` if any tracked or untracked `.forge` path has uncommitted changes (`git status --porcelain -- .forge`). Print the proposal/diff and exit `3`.
2. Write changed files to a staging directory.
3. Back up each file that will be replaced under `.forge/local/init-backup-<timestamp>/`.
4. Rename the staged files into place one at a time.
5. On a mid-apply failure, restore the backups and remove the staging directory; if restore also fails, print the exact paths and exit `2`.
6. Install hooks last.
7. Remove the backup directory only after a successful apply; keep it on failure for inspection.

Init never commits. The human commits the result.

### 8.13 Dry run

`forge init --dry-run`:

- Runs preflight and scan.
- Runs the host probe but not canaries.
- Uses `--base-ref` if supplied; otherwise prints the candidates and "base ref: not confirmed".
- Prints the proposed `.forge/config` TOML, the proposed `.forge/.gitignore`, the proposed policy summary, the execution-mode summary, and a list of decisions that would be asked.
- Prints "No files were written."
- Exits `0` unless preflight fails.
- Works with a non-TTY.
- Never prompts, never reads a key interactively, never validates the network, never writes an init report.
- If an existing `.forge/config` is present, prints a unified diff instead of the full file.

### 8.14 Re-init

Running `forge init` when `.forge/config` exists enters re-init.

- Show the current confirmed map, new candidates, and a unified diff.
- If `.forge/` exists without `.forge/config`, treat it as an incomplete prior init: show a proposal, require `apply`, and refuse to overwrite any existing path. It is never treated as a fresh init.
- If `.forge/config` has a newer schema, print the version and migration path and exit `2`; do not rewrite.
- If `.forge` has uncommitted changes, print the proposal/diff and exit `3`; do not write.
- If the schema is older, print the migration diff and require `apply`; v1 has no older schema.
- Preserve check IDs by identity; new checks get `max + 1`; removed IDs are reserved.
- Increment `[map] version` only if confirmed map content changes.
- Do not commit.
- If there are no changes, print `No changes.` and exit `0`.

### 8.15 Init state machine

| State | Next | On failure |
|---|---|---|
| `Preflight` | `Scan` | `Failed` (exit `2` or `4`) |
| `Scan` | `BaseRef` | `Failed` (exit `2`) |
| `BaseRef` | `Brief` | `Aborted` (exit `3`) |
| `Brief` | `Verification` | `Aborted` (exit `3`) |
| `Verification` | `Provider` | `Aborted` (exit `3`) |
| `Provider` | `ExecutionPolicy` | `Aborted` (exit `3`); a failed validation is a warning |
| `ExecutionPolicy` | `Trust` | `Aborted` (exit `3`); a probe failure records a mode or a blocker |
| `Trust` | `Signing` | `Aborted` (exit `3`) |
| `Signing` | `Hooks` | `Aborted` (exit `3`); a skipped key is a warning |
| `Hooks` | `Summary` | `Aborted` (exit `3`); a conflict is degraded |
| `Summary` | `Apply` or `Done` | `Aborted` (exit `3`) |
| `Apply` | `Done` | `Failed` (exit `2`; rollback attempted) |
| `Done` | — | — |

`--dry-run` goes `Preflight → Scan → ProbeSummary → Done`.

### 8.16 What init never does

- Commit.
- Overwrite or modify existing docs, tests, CI, manifests, build files, user code, the root `.gitignore`, global Git config, existing hooks, accepted `.forge` artifacts, or credentials.
- Auto-accept the execution policy or the trust declaration.
- Install toolchains or dependencies.
- Execute repository code, a package manager, build-system introspection, or a model.
- Send repository content to a provider during validation.
- Write secrets to committed files.
- Create a task.
- Scaffold tests or source.
- Assume `main`, a GitHub remote, a provider key, a default branch, or a clean tree.

---

## 9. Repo brief grammar

### 9.1 Row model and IDs

A brief row is:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BriefRow {
    pub row_id: String,          // E1, M1, P1, D1, C1
    pub facet: FacetKind,
    pub origin: RowOrigin,       // Current | Proposed
    pub path: PathBuf,
    pub kind: Option<String>,    // entry point or check kind
    pub status: RowStatus,       // Current | Proposed | Changed
    pub source: String,
    pub fields: BriefFields,
}
```

- Row IDs use a facet prefix and a per-brief sequence: `E1`, `M1`, `P1`, `D1`, `C1`.
- IDs are assigned in display order after deduplication. They are not stable across runs; they are a prompt handle, not a persisted identity. Check IDs in config are the stable identity.
- Deduplication key: `(facet, path, kind)`. If a proposed row matches a current row, it becomes one row with `origin = Current`, `status = Changed`, and the proposed fields shown as the diff.
- A current row is already confirmed; `a` on it is a no-op with a notice.
- A proposed row is not confirmed until `a` or `e …` plus `a`.

### 9.2 Commands

| Command | Meaning |
|---|---|
| `a <id>` | Accept one proposed row as confirmed. On a current row, no-op. |
| `a all` | Accept every remaining proposed row. Explicit; Enter is not acceptance. |
| `e <id> field=value …` | Edit one row. Values with spaces use double quotes. Validated immediately; an invalid edit leaves the row unchanged. |
| `r <id>` | Reject a proposed row. A current row becomes removed from the proposed map. |
| `u <id>` | Mark a proposed row unknown. |
| `none <facet>` | Confirm absence for a facet. Clears that facet's proposed rows; current rows must be rejected first. |
| `primary <id>` | Mark an entry point as primary and unset any other primary. |
| `show <group\|facet>` | Expand a collapsed group or facet. |
| `diff` | Print the unified diff of the proposed config against the current config, or "no current config". |
| `done` | Finish the brief and go to verification. |
| `?` | Print the command list. |

Editable fields by facet:

- Entry point: `path`, `kind`, `primary`.
- Module root: `path`.
- Package root: `path`, `container`.
- Doc: `path`, `precedence`.
- Check: `kind`, `command`, `cwd`, `required`, `source`.

`e` never changes a check's integer ID. A check ID is assigned at apply.

### 9.3 Rendering, groups, and collapse

The brief is plain text, rendered into the existing terminal flow. It is not a new screen, tab, or web surface.

Header:

```text
Forge init — repository brief
Repository: /home/dev/acme (main @ 1a2b3c4), clean
Scan: 128 tracked files, 3 manifests, 2 lockfiles, 1 warning
```

Groups:

- Group by deepest confirmed package root, or by the proposed package root if none is confirmed, or by the repo root when there is no package root.
- Group order: repo root first if it is a container, then package roots by path lexicographic.
- Within a group, facet order: `entry_points`, `module_roots`, `package_roots`, `docs`, `checks`.
- Within a facet, rows by path lexicographic, then kind.

Row format:

```text
  [E1] proposed  src/main.rs  (binary)  source: path:src/main.rs
  [C2] proposed  test  cargo test --workspace  cwd: .  required  source: manifest:Cargo.toml
  [D1] proposed  AGENTS.md  precedence: 0  source: path:AGENTS.md
```

Collapse:

- Show all current and changed rows.
- Show all `unknown` rows.
- Show at most 20 proposed rows per group and facet.
- If more, print `… N more proposed; show <group>/<facet> to expand`.
- `show` expands in the same scrollback; the brief never paginates and never requires a second screen.
- A group with more than 100 rows prints a group summary first and requires `show <group>` to list rows.

Docs conflicts:

```text
  Docs conflict
    AGENTS.md  precedence 0  sha256:...
    CLAUDE.md  precedence 10  sha256:...
    AGENTS.md is canonical among instruction docs on conflict.
```

Stale references:

```text
  Stale references
    README.md:12 -> docs/missing.md  (stale-reference)
```

Warnings are printed once after the groups, sorted by code then path.

### 9.4 Verification prompt

After the brief:

```text
Verification
  1. Automated checks [1, 2, 3, 4]
  2. Manual method
  3. Unconfigured (forge run will refuse)
Choose 1-3:
```

- Option 1 is unavailable when no check is confirmed.
- Option 1 asks which check IDs are in the verification set; the default is all `required` checks.
- Option 2 asks for one line; an empty line is invalid.
- Option 3 prints a warning.
- No default; Enter alone is invalid.

### 9.5 Brief validation

- Unknown row ID: print `unknown row <id>` and re-prompt.
- Unknown command: print the command list and re-prompt.
- Invalid edit: print the field error and keep the old value.
- `primary` on a non-entry row: error.
- `none` on a facet with current rows: error, list the rows to reject first.
- `done` with automated verification and no checks: error, choose manual or unconfigured.
- EOF at any prompt: abort init with exit `3`; no writes.
- Ctrl-C: exit `130`; no writes.

### 9.6 Example

```text
Forge init — repository brief
Repository: /home/dev/acme (main @ 1a2b3c4), clean
Scan: 128 tracked files, 3 manifests, 2 lockfiles, 0 warnings

[crates/app_core] (package)
  Entry points
    [E1] proposed  crates/app_core/src/lib.rs  (library)  source: manifest:crates/app_core/Cargo.toml
  Module roots
    [M1] proposed  crates/app_core/src  source: heuristic:crates/app_core/src
  Checks
    [C1] proposed  test  cargo test -p forge-harness-core  cwd: crates/app_core  required  source: manifest:crates/app_core/Cargo.toml

[crates/infra_local] (package)
  Entry points
    [E2] proposed  crates/infra_local/src/main.rs  (binary, primary)  source: manifest:crates/infra_local/Cargo.toml
  Module roots
    [M2] proposed  crates/infra_local/src  source: heuristic:crates/infra_local/src

[repo root] (container)
  Docs
    [D1] proposed  AGENTS.md  precedence: 0  source: path:AGENTS.md
    [D2] proposed  README.md  precedence: 20  source: path:README.md
  Package roots
    [P1] proposed  .  container  source: manifest:Cargo.toml

Commands: a <id> accept · e <id> field=value · r <id> reject · u <id> unknown · none <facet> · primary <id> · show <group|facet> · diff · done · ?

> a E1
accepted E1
> primary E2
E2 is primary
> a all
accepted 5 rows
> done
```

---

## 10. Init report

### 10.1 Path and lifecycle

- Path: `.forge/local/init-report.json`.
- Local-only, gitignored, UTF-8 JSON, no BOM.
- One report per confirmed scan; a confirmed re-init replaces it. `--dry-run` writes none.
- A newer `schema_version` is read-only; init refuses to overwrite it.
- A different `scan_version` means the scan rules changed; the report is stale and may be replaced after confirmation.
- Atomic write: temp file in the same directory, fsync, rename.

### 10.2 Top-level schema

```json
{
  "schema_version": 1,
  "scan_version": 1,
  "repo_root": "/home/dev/acme",
  "scanned_at": "2026-11-05T12:00:00Z",
  "git": {
    "head": "1a2b3c4d5e6f7890",
    "current_branch": "main",
    "default_branch": "origin/main",
    "dirty": false,
    "remotes": [
      {
        "name": "origin",
        "url": "https://github.com/Xaidel/forge.git",
        "direction": "fetch"
      }
    ],
    "submodules": [],
    "nested_repos": []
  },
  "scan": {
    "file_count": 128,
    "file_list_hash": "<64 lowercase hex>",
    "truncated": false
  },
  "languages": [
    { "name": "Rust", "files": 120, "source": "heuristic:*.rs" }
  ],
  "manifests": [
    { "path": "Cargo.toml", "kind": "cargo", "source": "path:Cargo.toml" }
  ],
  "lockfiles": [
    { "path": "Cargo.lock", "kind": "cargo", "source": "path:Cargo.lock" }
  ],
  "toolchains": [
    { "name": "rust", "version": "1.93", "source": "manifest:Cargo.toml#rust-version" }
  ],
  "entry_points": [],
  "module_roots": [],
  "package_roots": [],
  "docs": [],
  "checks": [],
  "generated_trees": [],
  "vendored_trees": [],
  "env": {
    "examples": [".env.example"],
    "required_vars": ["DATABASE_URL"]
  },
  "devcontainer": {
    "present": false,
    "paths": []
  },
  "security_findings": [],
  "warnings": [],
  "stale_references": [],
  "summary": {
    "candidate_count": 0,
    "warning_count": 0,
    "security_finding_count": 0,
    "outside_v1_scope": false,
    "unknown_facets": []
  }
}
```

| Field | Type | Required | Rules |
|---|---|---|---|
| `schema_version` | integer | yes | `1`. |
| `scan_version` | integer | yes | `1` for this rule table. |
| `repo_root` | string | yes | Absolute path. It may contain the OS home directory; no separate username or hostname field is stored. |
| `scanned_at` | string | yes | RFC 3339 UTC. |
| `git` | object | yes | See §10.3. |
| `scan` | object | yes | `file_count`, `file_list_hash`, `truncated`. |
| `languages` | array | yes | May be empty. |
| `manifests` | array | yes | May be empty. |
| `lockfiles` | array | yes | May be empty. |
| `toolchains` | array | yes | May be empty. |
| `entry_points` | array | yes | Proposed candidates only. |
| `module_roots` | array | yes | Proposed candidates only. |
| `package_roots` | array | yes | Proposed candidates only. |
| `docs` | array | yes | Proposed candidates only. |
| `checks` | array | yes | Proposed candidates only. |
| `generated_trees` | array | yes | Reported, excluded from proposals. |
| `vendored_trees` | array | yes | Reported, excluded from proposals. |
| `env` | object | yes | Names only; no values. |
| `devcontainer` | object | yes | Present or not; v1 reports it unsupported. |
| `security_findings` | array | yes | TDD-0010 owns the patterns. |
| `warnings` | array | yes | Stable codes. |
| `stale_references` | array | yes | Doc links only. |
| `summary` | object | yes | Counts and scope. |

### 10.3 Git facts

```json
{
  "head": "1a2b3c4d5e6f7890",
  "current_branch": "main",
  "default_branch": "origin/main",
  "dirty": false,
  "remotes": [
    { "name": "origin", "url": "https://github.com/Xaidel/forge.git", "direction": "fetch" }
  ],
  "submodules": [],
  "nested_repos": []
}
```

- `head` is the full commit SHA.
- `current_branch` is `null` for a detached HEAD.
- `default_branch` is `null` when no remote default is detectable.
- Remote URLs redact userinfo: `https://user:token@host/path` becomes `https://host/path`.
- `dirty` is `true` when `git status --porcelain` is non-empty.
- `submodules` and `nested_repos` are repo-relative paths.
- No hostname, username, or credential is stored as a field.

### 10.4 Candidate records and evidence

Each facet candidate has the common fields:

```json
{
  "row_id": "E1",
  "confidence": "proposed",
  "source": "path:src/main.rs",
  "evidence": ["path:src/main.rs"]
}
```

plus the facet fields:

| Facet | Fields |
|---|---|
| `entry_points` | `path`, `kind` |
| `module_roots` | `path` |
| `package_roots` | `path`, `container` |
| `docs` | `path`, `precedence`, `potential_conflict` |
| `checks` | `kind`, `command`, `cwd`, `required` |

`confidence` is always `"proposed"` in the report. Confirmed facts live in config, never in the report.

Evidence grammar:

| Kind | Form | Example |
|---|---|---|
| `manifest` | `manifest:<path>[#<locator>]` | `manifest:Cargo.toml#12`, `manifest:Cargo.toml#rust-version` |
| `path` | `path:<path>` | `path:src/main.rs` |
| `git` | `git:<query>` | `git:ls-files` |
| `heuristic` | `heuristic:<rule>` | `heuristic:src` |
| `gitattributes` | `gitattributes:<path>` | `gitattributes:dist` |
| `ignore` | `ignore:<path>` | `ignore:target` |
| `security` | `security:<rule>` | `security:tracked-env` |

`<path>` is repo-relative POSIX. `#<locator>` is a line number or a manifest key; `#` is used because `:` is already the kind separator. Evidence never contains a secret, file content, or absolute path.

### 10.5 Warnings and findings

Warnings:

| Code | Meaning |
|---|---|
| `unreadable-file` | A tracked file could not be read; skipped with evidence. |
| `unparseable-manifest` | A manifest could not be parsed; its evidence is still reported. |
| `scan-error` | A scan step failed; the report is partial and names the step. |
| `no-candidates` | A facet had no candidates and remains unknown unless confirmed absent. |
| `no-source` | No human-authored source was found; outside v1 code-task scope. |
| `no-docs` | No docs were found; docs remain unknown; run is not blocked. |
| `no-tests` | No test command was found; verification must be manual or unconfigured. |
| `unsupported-devcontainer` | A devcontainer was found; v1 does not use it. |
| `nested-repo` | A nested repository was skipped and is opaque. |
| `submodule-skipped` | A submodule was skipped and is opaque. |
| `generated-tree` | A generated tree was excluded from proposals. |
| `vendored-tree` | A vendored tree was excluded from proposals. |
| `stale-reference` | A doc's relative link target is missing. |
| `scan-truncated` | A scan limit was reached; counts are a lower bound. |
| `stale-init-staging` | A previous init staging directory remains. |
| `config-unknown-field` | A config key is unknown and was preserved. |

Security findings:

```json
{
  "kind": "hook",
  "path": ".git/hooks/pre-commit",
  "severity": "warn",
  "detail": "existing hook is not managed by Forge",
  "source": "git:hooks"
}
```

`kind` is one of `hook`, `filter`, `attribute`, `submodule`, `cargo-config`, `npm-config`, `script`, `ci`, `symlink`, `binary`, `instruction-file`, `env-file`, or `other`. TDD-0010 owns the patterns and severity rules; TDD-0001 owns the record. A finding is shown, never silently blocking (NFR-SEC-007).

### 10.6 Summary

```json
{
  "candidate_count": 12,
  "warning_count": 3,
  "security_finding_count": 1,
  "outside_v1_scope": false,
  "unknown_facets": ["checks"]
}
```

`outside_v1_scope` is `true` for a docs-only, data-only, or generated-only repository (FR-ONB-019). `unknown_facets` lists facets with no candidates. The post-apply summary printed by init is separate.

### 10.7 Unknown fields and versions

- Unknown JSON fields are tolerated by readers and preserved where the report is rewritten; a re-init writes a fresh report, so unknown fields from an older report are not carried forward.
- A newer `schema_version` is read-only; init refuses to overwrite it and exits `2`.
- A different `scan_version` is stale; the report may be replaced after confirmation.
- A missing required field is an error for readers; init writes a complete report.

### 10.8 Privacy

- No secrets, environment values, file contents, diffs, prompts, or model output.
- `env.required_vars` is names only; `.env.example` values are ignored.
- `repo_root` is absolute and local-only; it is not committed.
- Remote URLs redact userinfo.
- The report is gitignored by `.forge/.gitignore`.

---

## 11. Deterministic scan

### 11.1 Inputs and rule

The scan uses:

- the canonical repository root;
- the tracked file list from `git ls-files -z`;
- the contents of tracked manifests, docs, and candidate entry files, read with a per-file cap;
- Git facts (head, branches, remotes, dirty, submodules, nested repos, hooks);
- no environment values, no network, no model, no repository code execution.

The scan is a pure function of the repository contents, the Git facts, and the scan rule version. Given the same inputs and `scan_version`, it produces the same candidates in the same order. `scanned_at` is metadata and does not affect the candidate set.

### 11.2 File selection and order

- Use tracked files only. Untracked, ignored, and generated files are not scanned; they are reported when they are tracked.
- Sort paths by their UTF-8 byte sequence, then by kind.
- Do not follow symlinks outside the repository. A symlink that points outside is a security finding and is skipped.
- Skip files larger than 1 MiB when parsing manifests or docs; report `unreadable-file` or `unparseable-manifest` with the path. The file still counts in the language inventory.
- A file that cannot be read is a warning, never a silent skip.

### 11.3 Exclusions

Generated and vendored trees are excluded from entry-point, module-root, package-root, and check proposals by default. They are reported in `generated_trees` or `vendored_trees`.

Detection:

- Fixed path segments: `target`, `node_modules`, `vendor`, `dist`, `build`, `out`, `.venv`, `venv`, `__pycache__`, `.next`, `.nuxt`, `.turbo`, `coverage`.
- `.gitattributes` marks: `linguist-generated=true`, `linguist-vendored=true`.
- A nested `.git` directory or file: the directory is a nested repo and is opaque.
- A submodule from `.gitmodules`: skipped and opaque.

A human may explicitly add a generated or vendored path to `context.module_roots` or `context.package_roots`; the brief flags it.

### 11.4 Ecosystem table

| Ecosystem | Manifests | Lockfiles / toolchains | Entry-point candidates | Module-root candidates | Check candidates |
|---|---|---|---|---|---|
| Rust | `Cargo.toml` | `Cargo.lock`, `rust-toolchain.toml`, `rust-toolchain`, `.cargo/config.toml` | `[[bin]]` targets, `src/main.rs`, `src/bin/*.rs`, `src/lib.rs` | `src`, each package's `src` | `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` |
| Node | `package.json` | `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, `bun.lockb`, `.nvmrc`, `.node-version` | `bin`, `main`, `exports`, `src/index.*`, `src/main.*` | `src`, `lib`, `app` if present | declared scripts `build`, `test`, `lint`, `format`, `typecheck`, `setup`; use the detected package manager |
| Python | `pyproject.toml`, `setup.py`, `setup.cfg`, `requirements*.txt`, `Pipfile` | `poetry.lock`, `uv.lock`, `Pipfile.lock`, `.python-version` | `[project.scripts]`, `__main__.py`, `manage.py`, `app.py`, `wsgi.py`, `asgi.py` | dirs with `__init__.py`, `src` | declared `pytest`, `ruff`, `mypy`, `black` only when configured or present as a dependency |
| Go | `go.mod` | `go.sum`, `go.mod` `go` directive | `main.go`, `cmd/*/main.go` | `.`, `cmd`, `internal`, `pkg` if present | `go build ./...`, `go test ./...`, `go vet ./...` |
| Make | `Makefile`, `GNUmakefile`, `makefile` | none | none | none | declared targets `build`, `test`, `lint`, `check`, `fmt` |

Notes:

- Rust workspace-level checks are repo-level checks with `cwd = "."`. Package roots are still proposed from each member `Cargo.toml`.
- Node package manager detection order: `packageManager` field, then `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`/`bun.lockb`, `package-lock.json`, then npm. A check command uses the detected manager: `npm run <script>`, `pnpm <script>`, `yarn <script>`, or `bun run <script>`.
- Node scripts are proposed only when `package.json` declares them. Forge never invents `npm test` for a missing script.
- Python checks are proposed only when the tool is declared in `pyproject.toml`, a requirements file, or `tox.ini`; `pytest` may be proposed when a `tests/` tree exists and pytest is a declared dependency. No guessed `python -m unittest`.
- Go has no default format check because `gofmt -l` exits `0` even when it lists files; a human may add a shell check.
- Make targets are proposed only when the target name appears in a rule line; Forge does not run `make -n`.
- Other ecosystems report languages and manifest evidence only. They produce no guessed entry points, module roots, package roots, or checks. The human may add them.

### 11.5 Entry-point rules

- Rust: parse `[[bin]]` name/path; default `src/main.rs`; `src/bin/*.rs`; `src/lib.rs` as a `library` candidate.
- Node: `bin` (string or object), `main`, `exports` string targets; common `src/index.*` and `src/main.*` when present.
- Python: `[project.scripts]` values; `__main__.py`; `manage.py`, `app.py`, `wsgi.py`, `asgi.py` at the repo root or a package root.
- Go: `main.go` containing `package main`; `cmd/*/main.go`.
- Make: none.
- A doc-only entry point stays `proposed` and never becomes confirmed context automatically.
- A generated or vendored path is never proposed.

### 11.6 Module-root rules

- Rust: `src` for each package root; a workspace member's `src`.
- Node: `src`, `lib`, or `app` if they contain tracked source files.
- Python: directories with `__init__.py`; `src` if present.
- Go: the module root `.`, plus `cmd`, `internal`, `pkg` if present.
- Generic: the parent of a confirmed entry point if it contains source files.
- A module root must contain at least one tracked source file. A directory with only generated files is excluded.
- Module roots are not package roots unless they contain a recognized manifest.

### 11.7 Package-root rules

- A candidate is a directory containing a recognized manifest: `Cargo.toml`, `package.json`, `pyproject.toml`, `setup.py`, `setup.cfg`, `go.mod`.
- Workspace declarations (`[workspace] members`, `workspaces`, `pnpm-workspace.yaml`, `go.work`) are evidence only. They never create a root without a manifest.
- A root that contains another candidate root defaults to `container = true`.
- A root may be both a container and a package.
- Nested repos and submodules are not proposed.
- A package root with no check is valid.

### 11.8 Docs rules

- Root docs: `AGENTS.md`, `CLAUDE.md`, `README*`, `CONTRIBUTING*`.
- Markdown under `docs/` up to depth 3, excluding generated and vendored trees.
- Precedence as in §7.7.
- Potential conflict: two or more instruction docs at precedence 0 or 10 with different content hashes.
- Stale reference: a relative Markdown link whose target is missing. Skip URLs, anchors, `mailto:`, and absolute paths.
- Other agent rule files are `other-rule-file` evidence, not authoritative.

### 11.9 Check rules

- A candidate check is created only from a declared manifest script, a conventional ecosystem target justified by tracked files, or a declared Makefile target.
- `source` identifies the manifest and key or the rule. `evidence` repeats the source.
- `required` defaults as in §7.8.
- `cwd` is the package root for package scripts and `.` for repo-level checks.
- A command containing a newline is rejected; a command over 4096 characters is rejected.
- Checks are sorted by `kind` order (`build`, `test`, `lint`, `typecheck`, `format`, `setup`), then command, then cwd.
- Check IDs are assigned at apply, not during the scan.

### 11.10 Environment, toolchains, Git, and devcontainer

- `.env.example`, `.env.sample`, `.env.template`: parse variable names only.
- Toolchains: `rust-version` from `Cargo.toml`, `package.json` `engines`, `.nvmrc`, `.node-version`, `rust-toolchain.toml`, `go.mod` `go`, `pyproject.toml` `requires-python`.
- Git: head, current branch, default branch from `refs/remotes/origin/HEAD`, remotes, dirty, submodules, nested repos, existing hooks.
- Devcontainer: `.devcontainer/devcontainer.json` or `.devcontainer.json`; report `unsupported-devcontainer`.
- No environment values are read or stored.

### 11.11 Security findings

The scan invokes a read-only dangerous-pattern scan over tracked files. TDD-0010 owns the exact patterns. TDD-0001 fixes:

- the finding record shape in §10.5;
- the finding kinds;
- the rule that findings are shown, never silently blocking;
- the rule that a scan error is a warning, not a skipped check;
- the rule that no repository code is executed.

Findings do not by themselves block `forge init`. The execution-policy and trust gates decide run eligibility; a high-severity finding may open a decision during a task.

### 11.12 Base-ref candidates

Order:

1. Current branch, if attached.
2. Remote default branch, if `refs/remotes/origin/HEAD` resolves.
3. Local `main` or `master` only if the ref exists.
4. Exact `HEAD` SHA for a detached HEAD.

Forge never assumes `main` or a GitHub remote. A missing candidate requires typed input. `--base-ref` bypasses the prompt.

### 11.13 No source and outside scope

- If no tracked human-authored source file exists, or all source is generated/vendored, the report sets `summary.outside_v1_scope = true`.
- A docs-only, data-only, or generated-only repo is outside v1's code-task scope (FR-ONB-019).
- Init may finish with a warning; `forge run` refuses with the reason; no task is created.
- Forge never scaffolds or invents source.

### 11.14 Limits

- Per-file parse cap: 1 MiB.
- Manifest parse failures are warnings, not fatal.
- A scan that reaches an implementation guard sets `scan.truncated = true` and `scan-truncated`; counts are a lower bound. The guard is an implementation constant, not a product limit; it must be high enough for normal monorepos and must not silently drop candidates.
- No network, no code execution, no build-system introspection.

---

## 12. Distribution (provisional)

### 12.1 Status and revisit trigger

This section is **provisional**. The owner deferred [Spike cargo-dist packaging and release artifacts](https://github.com/Xaidel/forge/issues/25) until Forge is built and functional. The partial evidence report is not an accepted distribution contract. Every value below is the starting contract and may be confirmed or adjusted when the spike resumes; an adjustment is not a product-scope change.

Revisit trigger: the cargo-dist spike completes and records macOS x64/arm64 builds, the final cargo-dist pin, the Windows arm64 decision, and the crates.io publish order.

### 12.2 Packages and binary

| Item | Value |
|---|---|
| Published binary package | `forge-harness` |
| Binary target | `forge` |
| Published core package | `forge-harness-core` |
| Core library crate | `app_core` |
| License | MIT |
| Versioning | Lockstep; both packages share one version |
| Publish order | `forge-harness-core`, then `forge-harness` |

`cargo install forge-harness` installs the `forge` binary. The core is published because Cargo will not publish a package that depends on an unpublished path dependency.

### 12.3 cargo-dist configuration

Root `dist-workspace.toml`:

```toml
[workspace]
members = ["cargo:crates/infra_local"]

[dist]
cargo-dist-version = "0.34.0"
ci = "github"
hosting = "github"
targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
]
installers = ["shell", "powershell", "msi", "homebrew"]
tap = "Xaidel/homebrew-forge"
formula = "forge"
publish-jobs = ["homebrew"]
github-attestations = true
install-path = "CARGO_HOME"
pr-run-mode = "plan"
install-updater = false
```

- `allow-dirty` is not set. The generated release workflow is owned by cargo-dist.
- `members = ["cargo:crates/infra_local"]` selects the binary package. The deferred spike confirms the exact cargo-dist member selector for the two-crate workspace.
- `install-updater = false`: no auto-update in v1.
- The release workflow triggers on a `v*` tag and publishes a GitHub Release.

### 12.4 Targets and artifacts

Default matrix:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`

Windows arm64 is **best effort**: a separate native `windows-11-arm` job builds `forge-harness-aarch64-pc-windows-msvc.zip` only, with no MSI promise. It is not part of the default MSI matrix.

Expected artifact names (package-based, not binary-based):

```text
source.tar.gz
source.tar.gz.sha256
forge-harness-installer.sh
forge-harness-installer.ps1
forge.rb
sha256.sum
forge-harness-aarch64-apple-darwin.tar.xz
forge-harness-aarch64-apple-darwin.tar.xz.sha256
forge-harness-x86_64-apple-darwin.tar.xz
forge-harness-x86_64-apple-darwin.tar.xz.sha256
forge-harness-x86_64-unknown-linux-gnu.tar.xz
forge-harness-x86_64-unknown-linux-gnu.tar.xz.sha256
forge-harness-x86_64-pc-windows-msvc.zip
forge-harness-x86_64-pc-windows-msvc.zip.sha256
forge-harness-x86_64-pc-windows-msvc.msi
forge-harness-x86_64-pc-windows-msvc.msi.sha256
```

Each archive contains `forge` (`forge.exe` on Windows), `README.md`, and `LICENSE`. The MSI embeds the same `forge.exe`.

The `forge-harness-*` names are accepted. There is no cargo-dist setting that renames archives or installers to `forge-*`; a custom release-time rename is not v1.

### 12.5 Installers and channels

| Channel | Evidence |
|---|---|
| Shell installer | `forge-harness-installer.sh`; default `$CARGO_HOME/bin` or `$HOME/.cargo/bin`; requires `curl`/`wget` and `tar`; honors `INSTALLER_DOWNLOAD_URL` and `FORGE_HARNESS_DOWNLOAD_URL`. |
| PowerShell installer | `forge-harness-installer.ps1`; default `$CARGO_HOME\bin` or `$HOME\.cargo\bin`. |
| MSI | `forge-harness-x86_64-pc-windows-msvc.msi`; Windows x64 only. |
| Homebrew | Tap `Xaidel/homebrew-forge`; formula `forge`; `brew install Xaidel/forge/forge`; primary macOS channel. |
| crates.io | `cargo install forge-harness`; separate publish, not cargo-dist. |

### 12.6 Checksums and attestations

- Every archive and installer has a `.sha256` file.
- `sha256.sum` aggregates checksums.
- GitHub artifact attestations use `actions/attest@v4` with `attestations: write` and `id-token: write`.
- v1 is unsigned: no macOS notarization, no Windows Authenticode.
- macOS direct download may hit Gatekeeper; Homebrew is primary.
- Windows may show SmartScreen; Smart App Control may block unsigned binaries.
- Windows arm64 is best effort and zip-only.

### 12.7 Version output

`forge --version` prints:

```text
forge 1.0.0
```

The version is the package version. The command set is versioned by this output; flags are additive; a breaking change requires a major version (I-CLI).

### 12.8 What remains unconfirmed

- macOS x64/arm64 end-to-end release legs.
- The final cargo-dist pin (`0.34.0` is the current provisional choice).
- The Windows arm64 MSI decision; v1 makes no arm64 MSI promise.
- The crates.io publish order and credentials.
- The Homebrew tap ownership and formula publication.
- The exact release-workflow file after cargo-dist generation.

A change to any of these is a distribution adjustment, not a product-scope change. It updates this section and the release checklist; it does not reopen the PRD.

---

## 13. Error taxonomy

### 13.1 Init errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("not a Git repository")]
    NotGitRepository,
    #[error("the repository has no commits")]
    NoCommits,
    #[error("Git {found} is below the required {required}")]
    GitTooOld { found: String, required: String },
    #[error(transparent)]
    Config(#[from] ConfigIssue),
    #[error("schema version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("uncommitted .forge changes: {paths:?}")]
    ExistingUncommitted { paths: Vec<PathBuf> },
    #[error("stale init staging directory: {path:?}")]
    StaleStaging { path: PathBuf },
    #[error("scan failed: {message}")]
    ScanFailed { message: String },
    #[error("provider setup failed: {message}")]
    ProviderSetup { message: String },
    #[error("sandbox probe failed: {message}")]
    SandboxProbe { message: String },
    #[error("signing setup failed: {message}")]
    Signing { message: String },
    #[error("existing hooks conflict: {paths:?}")]
    HookConflict { paths: Vec<PathBuf> },
    #[error("failed to write {path:?}: {message}")]
    WriteFailed { path: PathBuf, message: String },
    #[error("initialization declined")]
    Declined,
    #[error("initialization interrupted")]
    Interrupted,
}
```

`InitError` is infrastructure-local; it never crosses a port.

### 13.2 Scan errors

- A per-file read or parse failure is a warning, not a fatal error.
- A failure to list tracked files, resolve the repo root, or read Git facts is fatal (`ScanFailed`), because the scan cannot be trusted.
- A security-scan failure is a warning, not a skipped check (NFR-SEC-007).
- A stale staging directory is a warning until the human confirms removal.

### 13.3 Message style

- Developer-facing error messages are lower-case, have no trailing period, and name the failed operation (NAME-28a).
- User-facing CLI messages are sentence case, name the next command, and avoid internal jargon (NAME-28b).
- No internal type names, provider payloads, or stack traces in normal output.
- `--plain` and `RUST_BACKTRACE=1` do not change the exit code.

### 13.4 Error-to-exit mapping

| Error | Code |
|---|---|
| `InitError::NotGitRepository`, `NoCommits` | `2` |
| `InitError::GitTooOld` | `4` |
| `InitError::Config`, `SchemaTooNew`, `WriteFailed`, `ScanFailed` | `2` |
| `InitError::ExistingUncommitted`, `Declined`, `Interrupted` | `3` |
| `InitError::StaleStaging` | `0` with a warning; removal failure is `2` |
| `InitError::HookConflict` | `0` with degraded status |
| `InitError::ProviderSetup`, `SandboxProbe`, `Signing` | `0` with a warning and a run blocker, unless they prevent a runnable config |
| `CliError::Blocked` | `4` |
| `CliError::GatePending` | `3` |
| `CliError::Cancelled` | `130` |
| `CliError::Usage`, `Config`, `Runtime` | `2` |

---

## 14. Fixtures

Fixtures live under `tests/fixtures/init/`, `tests/fixtures/config/`, `tests/fixtures/scan/`, and `tests/fixtures/cli/` when implementation begins. This section fixes their names and expected outcomes so tests do not invent cases.

| Fixture | Expected |
|---|---|
| `single-rust-repo/` | Cargo manifest, `src/main.rs`, lockfile, `AGENTS.md`, `README.md`, tests; entry point, module root, package root, docs, and checks proposed; base ref `main`. |
| `library/` | `src/lib.rs` only; library entry point proposed; `none entry_points` accepted; init finishes. |
| `monorepo/` | Workspace root plus two member manifests; three package roots; root container; deepest root wins; per-package checks where declared. |
| `no-tests-no-docs/` | No test command, no docs; `no-tests` and `no-docs` warnings; manual or unconfigured verification; init finishes. |
| `unclear-map/` | Source in an unusual directory with no manifest; entry points and module roots unknown; init finishes; run is allowed with a full-risk orientation plan. |
| `unrecognized-build-system/` | `build.zig` or `meson.build`; manifest evidence reported; no guessed check; human supplies manual verification. |
| `existing-forge/` | Committed config; re-init shows the current map and a diff; `apply` updates; no commit. |
| `existing-forge-dirty/` | Uncommitted `.forge` change; proposal/diff only; exit `3`; no write. |
| `existing-newer-schema/` | `schema_version = 2`; read-only; exit `2`; no write. |
| `existing-hooks/` | `.git/hooks/pre-merge-commit` exists; not overwritten; lines printed; enforcement degraded. |
| `no-source/` | Docs-only or data-only; `outside_v1_scope = true`; init finishes with a warning; `forge run` refuses. |
| `generated-vendored/` | `target/`, `node_modules/`, `dist/` and `.gitattributes` marks; reported, excluded from proposals; explicit override accepted with a flag. |
| `docs-conflict/` | `AGENTS.md` and `CLAUDE.md` with different hashes; side-by-side; `AGENTS.md` canonical. |
| `stale-doc-link/` | `README.md` links to a missing file; `stale-reference`. |
| `detached-head/` | Detached HEAD; base ref prompt offers the SHA; no assumed branch. |
| `dirty-tree/` | Dirty working tree; warning; init proceeds. |
| `offline/` | `--no-network`; provider validation skipped; init finishes; summary names the unvalidated provider. |
| `provider-missing-key/` | No key; init finishes; `forge run` refuses; `forge doctor` names the missing key. |
| `provider-invalid-key/` | Invalid key or endpoint; init finishes with a warning; `forge doctor` names it. |
| `untrusted-no-sandbox/` | Untrusted declaration plus failed sandbox probe; init finishes; `forge run` refuses; `Guarded` is not offered. |
| `trust-unanswered/` | Missing declaration; treated as untrusted; run refuses `Guarded`. |
| `hooks-declined/` | `--no-hooks`; init finishes; `forge hooks status` and `forge audit` report degraded. |
| `large-monorepo/` | More than 20 proposed rows in a group; collapse and `show` behavior; no pagination. |
| `config-unknown-fields/` | Unknown top-level and nested keys; warning; preserved on rewrite; file byte-identical when no known value changes. |
| `config-invalid/` | Missing field, duplicate check ID, unknown verification check, bad budget, path escape; each has a stable issue code. |
| `config-local-override/` | Local patch overrides profiles/slots/routing/budgets; map/checks/verification unchanged; forbidden section rejected. |
| `config-newer-schema/` | `schema_version = 2`; `SchemaTooNew`; run refuses; init read-only. |
| `init-report-schema-2/` | Newer report; init refuses to overwrite; exit `2`. |
| `dry-run/` | No writes, no prompts, no network, no canaries; proposed files printed; exit `0`. |
| `non-tty-init/` | First prompt printed to stderr; exit `3`; no writes. |
| `reinit-no-change/` | Same map; `No changes.`; `map.version` unchanged; no write. |
| `reinit-map-change/` | New entry point; `map.version` increments; check IDs stable; diff shown. |
| `check-id-reuse/` | Removed check ID is not reused; new check gets `max + 1`. |
| `cli-help/` | Every golden help block in §5.4 matches byte-for-byte. |
| `cli-exit-codes/` | Every row in §5.5 is exercised. |
| `dist-plan/` | The `dist-workspace.toml` plan lists the expected artifacts; Windows arm64 is zip-only. |
| `dist-no-node/` | Generated installers and archives contain no Node/npm/Bun runtime. |
| `dist-checksums/` | Per-archive `.sha256`, `sha256.sum`, and attestation steps are present. |

---

## 15. Requirement-to-test mapping

| Requirement | Test | Layer |
|---|---|---|
| FR-ONB-001 — phased init | `init_flow::given_git_repo_when_init_runs_then_phases_in_order` | process |
| FR-ONB-001 — dry run | `init_flow::given_dry_run_when_run_then_no_writes` | process |
| FR-ONB-001 — no commits | `init_flow::given_no_commits_when_init_then_exit_2_no_forge` | process |
| FR-ONB-001 — existing `.forge` | `init_flow::given_existing_forge_with_changes_then_diff_only` | process |
| FR-ONB-002 — deterministic scan | `scan::given_same_inputs_when_scanned_twice_then_identical_candidates` | unit |
| FR-ONB-002 — source and confidence | `scan::given_every_candidate_then_source_and_confidence_present` | unit |
| FR-ONB-002 — unreadable file | `scan::given_unreadable_file_when_scanned_then_warning` | unit |
| FR-ONB-002 — no candidates | `scan::given_no_candidates_then_facet_unknown` | unit |
| FR-ONB-003 — confirmation | `brief::given_proposed_rows_when_accepted_then_config_contains_confirmed_only` | process |
| FR-ONB-003 — no Enter accept | `brief::given_enter_when_pressed_then_no_fact_accepted` | process |
| FR-ONB-003 — reject all | `brief::given_reject_all_then_facets_unknown_and_init_finishes` | process |
| FR-ONB-003 — confirmed absence | `brief::given_library_then_none_entry_points_accepted` | process |
| FR-ONB-004 — manual verification | `init_flow::given_no_check_when_manual_chosen_then_recorded` | process |
| FR-ONB-004 — run refuses | `run_gate::given_unconfigured_verification_then_exit_4` | process |
| FR-ONB-005 — candidates | `scan::given_entry_candidates_then_kind_path_source_confidence` | unit |
| FR-ONB-005 — primary | `config::given_two_entry_points_then_at_most_one_primary` | unit |
| FR-ONB-006 — generated trees | `scan::given_generated_tree_then_excluded_and_reported` | unit |
| FR-ONB-006 — override | `brief::given_explicit_generated_root_then_accepted_with_flag` | process |
| FR-ONB-007 — unrecognized build | `scan::given_unrecognized_build_system_then_no_guessed_check` | unit |
| FR-ONB-007 — supplied command | `brief::given_supplied_check_then_recorded` | process |
| FR-ONB-008 — docs conflict | `scan::given_docs_conflict_then_side_by_side` | unit |
| FR-ONB-008 — stale reference | `scan::given_broken_link_then_stale_reference` | unit |
| FR-ONB-008 — AGENTS canonical | `config::given_agents_and_claude_then_agents_precedence_zero` | unit |
| FR-ONB-009 — monorepo roots | `scan::given_monorepo_then_roots_from_manifests` | unit |
| FR-ONB-009 — workspace evidence | `scan::given_workspace_only_entry_then_not_a_root` | unit |
| FR-ONB-010 — container default | `brief::given_root_containing_members_then_default_container` | process |
| FR-ONB-010 — deepest root | `config::given_nested_roots_then_deepest_owns_file` | unit |
| FR-ONB-011 — check fields | `config::given_check_when_written_then_id_kind_command_cwd_source_required` | unit |
| FR-ONB-011 — no guessed command | `scan::given_unrecognized_manifest_then_no_check` | unit |
| FR-ONB-012 — provider setup | `init_flow::given_provider_missing_key_then_init_finishes_and_run_refuses` | process |
| FR-ONB-012 — offline | `init_flow::given_no_network_then_validation_skipped` | process |
| FR-ONB-012 — no secrets | `config::given_profile_then_no_secret_value_committed` | unit |
| FR-ONB-013 — policy accepted | `init_flow::given_execution_policy_then_human_accept_required` | process |
| FR-ONB-013 — untrusted | `init_flow::given_untrusted_no_sandbox_then_run_blocked` | process |
| FR-ONB-014 — trust default | `init_flow::given_no_trust_answer_then_untrusted` | process |
| FR-ONB-014 — Guarded refused | `init_flow::given_untrusted_guarded_then_refused` | process |
| FR-ONB-015 — hooks | `init_flow::given_existing_hook_then_not_overwritten` | process |
| FR-ONB-015 — declined hooks | `init_flow::given_no_hooks_then_degraded_status` | process |
| FR-ONB-016 — no overwrite | `init_flow::given_user_files_then_unchanged` | process |
| FR-ONB-016 — abort | `init_flow::given_abort_then_no_forge_directory` | process |
| FR-ONB-016 — newer schema | `init_flow::given_newer_schema_then_read_only` | process |
| FR-ONB-017 — re-init | `reinit::given_current_map_then_diff_and_apply` | process |
| FR-ONB-017 — uncommitted | `reinit::given_uncommitted_forge_then_proposal_only` | process |
| FR-ONB-017 — migration | `reinit::given_older_schema_then_migration_diff` | process |
| FR-ONB-018 — prerequisites | `run_gate::given_missing_prerequisites_then_exit_4_names_all` | process |
| FR-ONB-018 — incomplete map | `run_gate::given_incomplete_map_then_run_starts` | process |
| FR-ONB-018 — base ref | `run_gate::given_missing_base_ref_then_prompt` | process |
| FR-ONB-019 — outside scope | `scan::given_docs_only_then_outside_v1_scope` | process |
| FR-ONB-019 — no task | `run_gate::given_no_source_then_refused_no_task` | process |
| NFR-DIST-001 — artifacts | `release::given_dist_config_when_planned_then_expected_artifacts` | release |
| NFR-DIST-001 — installs | `release::given_supported_target_then_installer_present` | release |
| NFR-DIST-001 — checksums | `release::given_release_when_built_then_checksums_and_attestations` | release |
| NFR-DIST-001 — no Node | `release::given_artifacts_then_no_node_runtime` | release |
| NFR-DIST-001 — arm64 | `release::given_windows_arm64_then_zip_only_best_effort` | release |
| NFR-COMPAT-001 — floors | `doctor::given_supported_host_then_floors_reported` | process |
| NFR-COMPAT-001 — old Git | `doctor::given_old_git_then_exit_4` | process |
| I-CLI — help | `cli_help::given_golden_files_when_rendered_then_match` | unit |
| I-CLI — exit codes | `exit_codes::given_matrix_when_exercised_then_expected` | process |
| I-CONFIG — schema | `config::given_example_when_parsed_then_round_trips` | unit |
| I-CONFIG — unknown fields | `config::given_unknown_field_when_rewritten_then_preserved` | unit |
| I-CONFIG — newer schema | `config::given_schema_two_when_loaded_then_read_only` | unit |
| I-INITREPORT — schema | `init_report::given_example_when_parsed_then_round_trips` | unit |
| I-INITREPORT — newer schema | `init_report::given_schema_two_when_read_then_read_only` | unit |
| I-INITREPORT — unknown fields | `init_report::given_unknown_field_when_read_then_tolerated` | unit |
| D-13 — distribution | §12; `release::given_dist_config_when_planned_then_expected_artifacts` | release |
| D-16 — local state | §6.16, §8.9, §10.8; `config::given_local_patch_then_only_allowed_sections` | unit |
| D-19 — confirmed map | §7.1, §9, `brief::given_reject_all_then_facets_unknown_and_init_finishes` | process |
| D-20 — monorepo | §7.6, §11.7, `scan::given_monorepo_then_roots_from_manifests` | unit |
| NFR-SEC-007 — findings | `scan::given_security_finding_then_reported_not_blocking` | unit |
| NFR-OBS-001 — output | `cli_output::given_status_doctor_audit_then_stable_fields` | process |

---

## 16. Seams with later TDDs

| TDD | This document fixes | The later TDD owns |
|---|---|---|
| TDD-0002 | `ConfigSnapshot` mapping, `RepoMapSnapshot` mapping, no public amendments | Public records, ports, error categories |
| TDD-0003 | Init creates no session state; CLI `resume` flags and exit mapping | Session schema, events, resume, active-task lock format |
| TDD-0004 | `[map] version`/hash, `forge run --base-ref`, plan-time map diff writes the same config file | Plan schema, policy engine, plan gate, map-diff trigger |
| TDD-0005 | `forge ledger verify [--repair]` and `forge ledger migrate [--dry-run]` command surface | Ledger schema, validation, index, migration mechanics |
| TDD-0006 | Init's probe/review handoff, `--guarded`, trust declaration prompt, hidden `__worker` entry | Execution-policy JSON, probe internals, canaries, permission store, worker IPC |
| TDD-0007 | Profile/slot/routing/budget config keys and templates | Wire mappings, key-file shape, price table, budget arithmetic |
| TDD-0008 | `forge hooks` surface, hidden `forge hook`, signing/hook handoff | Hook shim, signing, `C2`, attestation JSON, merge, allowed_signers |
| TDD-0009 | Repo-brief content grammar and deterministic text layout | TUI layout, ANSI tokens, keybindings, plain-mode rendering |
| TDD-0010 | Init report finding record and scan invocation | Security patterns, sanitizer, secret patterns, protected paths, review flags |
| TDD-0011 | No dogfood event encoding | Audit JSONL, metrics, dogfood diary |

Boundary rules:

- TDD-0004 must not change the config schema or map hash without amending this document.
- TDD-0006 must not change the execution-policy path or the trust prompt semantics without amending this document.
- TDD-0007 must not add a secret to the committed config; key material stays local.
- TDD-0008 must not change `forge hooks` or `forge hook` names without amending this document.
- TDD-0009 must not change the brief's facts or command grammar; it may style the text.
- TDD-0010 must not silently allow a security-scan failure; it is a warning with evidence.
- No later TDD may add a public core record for an infra-only config type.

No separate ADR is required for the staged init transaction. The no-partial-write guarantee is a PRD requirement (FR-ONB-016 E3), not a free architectural choice; [ADR-0009](../adr/0009-committed-config-confirmed-facts.md) covers the committed config/repo-map design.

---

## 17. Traceability

| Source | Covered by |
|---|---|
| PRD §7.1 FR-ONB-001..019 | §8, §9, §10, §11, §14, §15 |
| PRD §8.1 CLI command surface | §5 |
| PRD §8.2 config and policy boundary | §6, §7, §8.12 |
| PRD §9 data model and persistence | §6, §7, §10 |
| PRD §9.4 retention and privacy | §6.16, §8.9, §10.8 |
| PRD §9.5 repo-level state | §7.6, §7.11 |
| PRD §10.11 policy change control | §8.8 |
| PRD §11.3 platform and mode matrix | §8.8, §8.11 |
| PRD §13.13 definition of done | §14, §15 |
| PRD §15.2 TDD-owned technical questions | §5–§12 |
| NFR-DIST-001 | §12, §15 |
| NFR-COMPAT-001 | §5.7, §8.2, §15 |
| NFR-SCALE-001, NFR-SCALE-002 | §6, §7.6, §7.11 |
| NFR-SEC-003, NFR-SEC-007, NFR-SEC-008 | §8.8, §8.9, §10.5, §11.11 |
| NFR-OBS-001 | §5.7, §15 |
| D-13 stack and distribution | §3, §12 |
| D-16 providers and onboarding | §6.9–6.12, §8.7, §8.9 |
| D-19 unclear architecture | §7.1, §7.7, §8.5, §10 |
| D-20 monorepo | §7.6, §7.11, §11.7 |
| ADR-0009 committed config | §6, §7 |
| TDD-0002 public records | §6.17 |

---

## 18. Changelog

| Date | Change |
|---|---|
| 2026-11-05 | Initial contract. Fixes the CLI surface, `.forge/config`, repo map, init flow, repo brief, init report, deterministic scan, provisional distribution, fixtures, and requirement-to-test mapping. Does not amend TDD-0002. |
