# Forge v1 Naming Conventions

**Status:** Approved  
**Date:** 2026-11-03  
**Owner:** Forge architecture  
**Sources:** `docs/PRD.md` §5, §6.5, §6.7; NFR-MAINT-001; D-13; [Define the architecture, dependency, and naming rules](https://github.com/Xaidel/forge/issues/20)  
**Companions:** [architecture.md](architecture.md), [dependency-rules.md](dependency-rules.md)  
**ADRs:** [ADR-0001](../docs/adr/0001-two-crate-ports-only-core.md), [ADR-0002](../docs/adr/0002-executor-agnostic-io-free-core.md), [ADR-0003](../docs/adr/0003-hidden-worker-subcommand.md)

---

## 1. Scope

This document fixes the names used across Forge: Cargo packages and targets, Rust modules, files, types, traits, ports, use cases, adapters, events, records, errors, services, tests, CLI flags, environment variables, configuration keys, and paths.

The PRD's **ubiquitous language** is canonical for product terms. This document does not invent new product terms. It fixes how those terms appear in code.

When a name in this document conflicts with the PRD, the PRD term wins and this document is corrected.

---

## 2. Rule index

| ID | Rule | Enforced by |
|---|---|---|
| **NAME-01** | Package, crate, and target names follow the table in §3. | Manifest inspection; architecture test |
| **NAME-02** | Modules and files use `snake_case`. | Review; source scan |
| **NAME-03** | Use `foo.rs` + `foo/`, never `mod.rs`. | Review; architecture test |
| **NAME-04** | One primary type per file. Module roots re-export; they do not own multiple types. | Review |
| **NAME-05** | Types use `PascalCase`; acronyms are words, not all-caps. | Review |
| **NAME-06** | Domain concepts use the PRD's exact term. | Review |
| **NAME-07** | Port records use `*Snapshot`, `*Record`, or `*Report` by role. | Review |
| **NAME-08** | Events are past-tense facts. | Review |
| **NAME-09** | Errors use `<Area>Error`. | Review |
| **NAME-10** | Services are named by responsibility, never `*Manager` or `*Service`. | Review |
| **NAME-11** | Outbound traits use `<Noun>Port`. | Review |
| **NAME-12** | Inbound traits are verb phrases with no suffix. | Review |
| **NAME-13** | Private use cases use `<Verb><Noun>UseCase`. | Review |
| **NAME-14** | Adapters use `<Tech><Role>`. | Review |
| **NAME-15** | Functions and methods are verb phrases; no vague `handle`, `process`, or `do`. | Review |
| **NAME-16** | Getters, setters, and conversions follow the Rust conventions in §7. | Review |
| **NAME-17** | Boolean predicates start with `is_`, `has_`, `can_`, or `needs_`. | Review |
| **NAME-18** | Fields and variables use `snake_case` with role suffixes. | Review |
| **NAME-19** | Collections, maps, and options use the forms in §9. | Review |
| **NAME-20** | Constants use `SCREAMING_SNAKE_CASE`; global mutable state is forbidden. | Review |
| **NAME-21** | Environment variables use `FORGE_*`, except provider key names. | Review |
| **NAME-22** | CLI flags are kebab-case; subcommands are lowercase; `__worker` is hidden. | CLI tests; review |
| **NAME-23** | Configuration keys are snake_case; paths use the PRD's exact names. | Review |
| **NAME-24** | Test modules and files follow §12. | Review |
| **NAME-25** | Test functions use `given_<context>_when_<action>_then_<outcome>`. | Review |
| **NAME-26** | Test doubles use `Fake`, `Stub`, or `Spy` prefixes. | Review |
| **NAME-27** | Public items have doc comments; module roots have module docs. | `#![deny(missing_docs)]` in the core |
| **NAME-28** | Error messages are lower-case, actionable, and period-free. | Review |
| **NAME-29** | The banned names in §14 never appear in committed code. | Review; source scan |
| **NAME-30** | Only the approved abbreviations in §15 are allowed. | Review |

---

## 3. Crates, packages, and targets

| Logical name | Cargo package | Cargo target | Crate name in code | Published |
|---|---|---|---|---|
| Core | `forge-harness-core` | `[lib] name = "app_core"` | `app_core` | Yes |
| Infrastructure | `forge-harness` | `[[bin]] name = "forge"` | `forge` | Yes |
| Worker | same `forge-harness` package | hidden subcommand `__worker` | `forge` | Yes |

Rules:

- **NAME-01a:** The workspace has exactly two packages: `forge-harness-core` and `forge-harness`.
- **NAME-01b:** There is no `infra_local` Cargo package and no `infra_local` library target. `infra_local` is the source directory and the name of the infrastructure layer.
- **NAME-01c:** The binary target is `forge`. There is no `forge-worker` target.
- **NAME-01d:** Both packages share one version. The harness dependency is a path plus the exact shared version.
- **NAME-01e:** The package `forge-harness-core` builds the library crate `app_core`, not a crate named `forge_harness_core`. The `[lib] name` is part of the public contract.

```toml
# crates/app_core/Cargo.toml
[package]
name = "forge-harness-core"

[lib]
name = "app_core"
```

```toml
# crates/infra_local/Cargo.toml
[package]
name = "forge-harness"

[[bin]]
name = "forge"
```

---

## 4. Modules and files

### 4.1 Rules

- **NAME-02a:** Module and file names are `snake_case`.
- **NAME-02b:** A module with submodules uses `foo.rs` plus a `foo/` directory. Never `foo/mod.rs`.
- **NAME-02c:** The module root (`foo.rs`) contains only `mod`, `pub mod`, `pub use`, and module-level documentation. It contains no business logic.
- **NAME-02d:** A leaf file is named after its primary type or responsibility: `task.rs` for `Task`, `start_task.rs` for `StartTaskUseCase`, `session_store.rs` for `SessionStorePort`.
- **NAME-02e:** Directories are plural only for collections: `value_objects/`, `entities/`, `events/`, `services/`, `records/`, `inbound/`, `outbound/`, `error/`.
- **NAME-02f:** No `utils.rs`, `helpers.rs`, `common.rs`, `misc.rs`, or `shared.rs`.

### 4.2 Core file layout

```text
crates/app_core/src/
  lib.rs
  domain.rs
  domain/
    value_objects/
      task_id.rs
      task_state.rs
    events/
      task_started.rs
      plan_approved.rs
    entities/
      task.rs
      plan.rs
    services/
      policy_evaluator.rs
      risk_classifier.rs
  ports.rs
  ports/
    inbound.rs
    inbound/
      start_task.rs
      answer_gate.rs
    outbound.rs
    outbound/
      session_store.rs
      ledger.rs
      git.rs
      model.rs
      sandbox.rs
      clock.rs
      id_source.rs
    error.rs
    error/
      session_error.rs
      ledger_error.rs
      git_error.rs
      application_error.rs
    records.rs
    records/
      task_snapshot.rs
      session_snapshot.rs
      decision_record.rs
      status_report.rs
  application.rs
  application/
    start_task.rs
    answer_gate.rs
    resume_task.rs
  assembly.rs
```

- The exact record and trait set is TDD-0002's contract. The pattern is fixed here.
- `ports.rs` re-exports the modules and `ApplicationPorts` / `build_application`.
- `ports/inbound.rs` and `ports/outbound.rs` are module roots; each trait has its own file under the matching directory.
- `ports/error.rs` is a module root; each error enum has its own file under `error/`.
- `ports/records.rs` is a module root; `ports/records/` contains one record per file.

### 4.3 Infrastructure file layout

```text
crates/infra_local/src/
  main.rs
  cli.rs
  cli/
    init.rs
    run.rs
    status.rs
  tui.rs
  tui/
    app.rs
    transcript.rs
    theme.rs
  sandbox.rs
  sandbox/
    mxc.rs
    canary.rs
    protocol.rs
  worker.rs
  worker/
    execute.rs
    protocol.rs
  model.rs
  model/
    anthropic.rs
    openai_compat.rs
    profile.rs
  git.rs
  ledger.rs
  session.rs
  policy.rs
  attest.rs
  audit.rs
  config.rs
  wiring.rs
  wiring/
    adapters.rs
```

- `main.rs` is the crate root. It declares the modules and calls `cli::run()`.
- `sandbox/protocol.rs` is the shared worker IPC contract. It is the only module both `sandbox` and `worker` may import.
- `model/openai_compat.rs` keeps the PRD's path; it contains the `OpenAiChatClient` adapter.
- The exact infrastructure submodule set is owned by the relevant TDD. The file pattern is fixed here.

---

## 5. Types

### 5.1 General

- **NAME-05a:** Types use `PascalCase`.
- **NAME-05b:** Acronyms are words: `JsonlSessionStore`, `TomlConfig`, `HttpClient`, `Url`, `ApiToken`, `Tui`, `Cli`, `MxcSandbox`, `GitCli`. Not `JSONLSessionStore`, `TOMLConfig`, `HTTPClient`, `URL`, `API`, `TUI`, `CLI`.
- **NAME-05c:** Trait names never start with `I`; no `Abstract`, `Base`, or `Impl` prefixes/suffixes.
- **NAME-05d:** Newtypes are named for the value, not the wrapper: `TaskId`, `CommitSha`, `BranchName`, `PathPattern`, `RiskClass`. Not `TaskIdWrapper`.
- **NAME-05e:** Enums are named for the classification: `TaskState`, `GateKind`, `RiskClass`, `DecisionStatus`. Variants are `PascalCase` noun phrases.

### 5.2 Domain concepts

Use the PRD's exact term:

| PRD term | Rust type |
|---|---|
| Task | `Task` |
| Session | `Session` |
| Run | `Run` |
| Step | `Step` |
| Attempt | `Attempt` |
| Gate | `Gate` |
| Decision request | `DecisionRequest` |
| Permission request | `PermissionRequest` |
| Grant | `Grant` |
| Review packet | `ReviewPacket` |
| Attestation | `Attestation` |
| Task branch | `TaskBranch` |
| Worktree | `Worktree` |
| Base ref | `BaseRef` |
| Subject commit | `SubjectCommit` |
| Attestation commit | `AttestationCommit` |
| Assumption | `Assumption` |
| Decision record | `DecisionRecord` |
| Assumption record | `AssumptionRecord` |
| Deferred decision | `DeferredDecision` |
| Decision map | `DecisionMap` |
| Risk class | `RiskClass` |
| Override | `Override` |

Do not rename a PRD concept in code. Do not use `TaskData`, `TaskInfo`, `TaskDto`, or `TaskModel`.

### 5.3 Port records

| Suffix | Use for | Examples |
|---|---|---|
| `*Snapshot` | A persisted, point-in-time state record exchanged across a repository-like port. | `TaskSnapshot`, `SessionSnapshot`, `PlanSnapshot`, `RepoMapSnapshot` |
| `*Record` | A ledger or attestation artifact with a stable schema. | `DecisionRecord`, `AssumptionRecord`, `AttestationRecord`, `OverrideRecord` |
| `*Report` | A read-only query output assembled for a caller. | `StatusReport`, `DoctorReport`, `AuditReport` |
| no suffix | A PRD concept used directly as the port record. | `ReviewPacket`, `Gate`, `DecisionRequest` |

Rules:

- **NAME-07a:** A repository-like port exchanges `*Snapshot`, never a private aggregate.
- **NAME-07b:** A ledger port exchanges `*Record`.
- **NAME-07c:** A query port exchanges `*Report`.
- **NAME-07d:** Do not use `*Dto`, `*Data`, `*Info`, or `*Model` for a port record.
- **NAME-07e:** A snapshot is immutable plain data. Mutation produces a new snapshot.

### 5.4 Events

- **NAME-08a:** Events are past-tense facts: `TaskStarted`, `TaskPlanned`, `PlanApproved`, `GateOpened`, `GateAnswered`, `DecisionRecorded`, `StepFailed`, `VerificationPassed`, `AttestationCreated`, `MergeCompleted`.
- **NAME-08b:** No `Event` suffix. The type lives in the `events` module; the name is the fact.
- **NAME-08c:** Event fields use the same names as the corresponding record fields. Do not invent a second vocabulary.

### 5.5 Errors

- **NAME-09a:** Error enums use `<Area>Error`: `DomainError`, `ApplicationError`, `SessionError`, `LedgerError`, `GitError`, `ModelError`, `SandboxError`, `PolicyError`, `AttestationError`, `ConfigError`, `TerminalError`.
- **NAME-09b:** Error variants are `PascalCase` noun phrases: `TaskNotFound`, `GateAlreadyAnswered`, `CommitRejected`, `ProviderUnavailable`.
- **NAME-09c:** Public error enums are `#[non_exhaustive]`.
- **NAME-09d:** No `Error` suffix on traits or modules. No `Result` aliases that hide the error type.

### 5.6 Services

- **NAME-10a:** Domain services are named by the rule they implement: `PolicyEvaluator`, `RiskClassifier`, `PlanValidator`, `DecisionCoverage`, `GateScheduler`.
- **NAME-10b:** Never `*Manager`, `*Service`, `*Helper`, `*Util`, `*Processor`, or `*Handler` for a domain service.
- **NAME-10c:** A service is stateless unless the PRD defines state. State lives in an entity or a port record.

---

## 6. Traits, ports, use cases, and adapters

### 6.1 Outbound ports

- **NAME-11a:** `<Noun>Port`: `SessionStorePort`, `LedgerPort`, `GitPort`, `ModelPort`, `SandboxPort`, `PolicyPort`, `AttestationPort`, `AuditPort`, `ConfigPort`, `ClockPort`, `IdSourcePort`.
- **NAME-11b:** The noun names the capability, not the technology. `SessionStorePort`, not `JsonlPort`.
- **NAME-11c:** Port methods are verb phrases: `load`, `save`, `append`, `commit`, `merge`, `stream`, `now`, `next_id`.
- **NAME-11d:** A port method returns `Result<T, <Area>Error>` or a port-specific error. It never returns `anyhow::Error` or `Box<dyn Error>`.

### 6.2 Inbound capabilities

- **NAME-12a:** Verb phrases, no suffix: `StartTask`, `AnswerGate`, `ResumeTask`, `CancelTask`, `ReviewTask`, `AttestTask`, `MergeTask`, `ReopenTask`, `OverrideMerge`, `QueryStatus`, `RunDoctor`, `QueryAudit`.
- **NAME-12b:** The trait name is the capability, not the implementation. `StartTask`, not `TaskStarter` or `StartTaskUseCase`.
- **NAME-12c:** The trait method uses the same verb phrase in `snake_case`: `start_task`, `answer_gate`, `resume_task`.

### 6.3 Private use cases

- **NAME-13a:** `<Verb><Noun>UseCase`: `StartTaskUseCase`, `AnswerGateUseCase`, `ResumeTaskUseCase`, `AttestTaskUseCase`.
- **NAME-13b:** Use cases are private (`pub(crate)`). They are never re-exported.
- **NAME-13c:** A use case implements exactly one inbound capability. If it needs two, split it.

### 6.4 Adapters

- **NAME-14a:** `<Tech><Role>`: `GitCli`, `JsonlSessionStore`, `MarkdownLedger`, `AnthropicMessagesClient`, `OpenAiChatClient`, `MxcSandbox`, `RatatuiTerminal`, `TomlConfig`, `OsClock`, `UuidIdSource`.
- **NAME-14b:** The technology comes first, the role second. `GitCli`, not `CliGit`.
- **NAME-14c:** The adapter name may differ from the file name where the PRD fixes the path. `model/openai_compat.rs` contains `OpenAiChatClient`.
- **NAME-14d:** An adapter implements one outbound port. If it implements two, split it or introduce a narrower port.
- **NAME-14e:** Adapter constructors are `new` or `from_*`; they take resolved configuration, not a global config object.

---

## 7. Functions and methods

- **NAME-15a:** Functions and methods are verb phrases: `load_task`, `save_snapshot`, `evaluate_policy`, `merge_branch`, `render_transcript`.
- **NAME-15b:** No vague `handle`, `process`, `do`, `run`, or `execute` unless the type makes the action obvious. `StartTaskUseCase::execute` is allowed because the type supplies the verb; `GitCli::run` is not.
- **NAME-15c:** A function that can fail uses a `Result` return and a name that says what it does, not `try_*` for every fallible operation. Use `try_*` only for a fallible conversion paired with an infallible alternative (`try_from` vs `from`).
- **NAME-15d:** A function that blocks must say so only when the distinction matters; async functions use `async fn`. Do not mix blocking I/O into an async port.

### 7.1 Getters, setters, and conversions

- **NAME-16a:** A getter is named after the value: `fn state(&self) -> TaskState`, `fn id(&self) -> &TaskId`. Not `get_state` or `get_id`.
- **NAME-16b:** A setter is `set_<field>` only when mutation is part of the public contract. Prefer a consuming builder or a new value.
- **NAME-16c:** `into_*` consumes; `as_*` borrows; `to_*` allocates. Follow the standard library convention.
- **NAME-16d:** `from_*` constructs; `try_from_*` is a fallible conversion. Do not use `parse_*` for a conversion that is not a parser.
- **NAME-16e:** A constructor is `new` when it has no fallible preconditions. Use `try_new` when it validates.

### 7.2 Predicates

- **NAME-17a:** Boolean predicates start with `is_`, `has_`, `can_`, or `needs_`: `is_done`, `has_pending_gate`, `can_merge`, `needs_review`.
- **NAME-17b:** No `should_*` in the core. A policy decision is a service result, not a boolean naming convention.

---

## 8. Fields and variables

- **NAME-18a:** Fields and variables use `snake_case`.
- **NAME-18b:** Identifiers use `*_id`: `task_id`, `decision_id`, `attestation_id`.
- **NAME-18c:** Timestamps use `*_at`: `created_at`, `updated_at`, `resolved_at`.
- **NAME-18d:** Counts use `*_count`: `attempt_count`, `step_count`.
- **NAME-18e:** Paths use `*_path`: `worktree_path`, `ledger_path`.
- **NAME-18f:** References use `*_ref`: `base_ref`, `subject_ref`.
- **NAME-18g:** Hashes use `*_hash`: `policy_hash`, `config_hash`.
- **NAME-18h:** Versions use `*_version`: `schema_version`, `policy_version`.
- **NAME-18i:** Raw bytes use `*_bytes`; text uses `*_text`; a file name uses `*_file_name`; a directory uses `*_dir`.
- **NAME-18j:** Booleans use the predicate forms in NAME-17.
- **NAME-18k:** Avoid Hungarian notation (`str_name`, `b_flag`, `vec_items`). The type is in the signature.

---

## 9. Collections, maps, and options

- **NAME-19a:** Collections are plural nouns: `tasks`, `steps`, `decisions`.
- **NAME-19b:** Maps are `<value>_by_<key>`: `tasks_by_id`, `checks_by_kind`.
- **NAME-19c:** A `Vec` is plural; a `HashMap` is `<value>_by_<key>`; a `HashSet` is plural or `<value>_set` when the set semantics matter.
- **NAME-19d:** An `Option<T>` uses the plain noun; do not prefix with `maybe_` or `opt_`. The type carries the optionality.
- **NAME-19e:** An iterator variable is singular: `for task in tasks`.
- **NAME-19f:** A slice parameter is plural; a single item is singular.

---

## 10. Constants and statics

- **NAME-20a:** Constants use `SCREAMING_SNAKE_CASE`: `MAX_LIVE_REGION_ROWS`, `DEFAULT_SOFT_BUDGET_USD`, `WORKER_SUBCOMMAND`.
- **NAME-20b:** `const` is for compile-time values. `static` is only for immutable process-wide data.
- **NAME-20c:** Global mutable state is forbidden. No `lazy_static`, `OnceLock<Mutex<_>>`, or `static mut` in the core. Infrastructure may use a process-wide immutable value when wiring requires it; document why.
- **NAME-20d:** Do not encode a configurable value as a constant. Configuration lives in `.forge/config` or the relevant policy file.

---

## 11. Environment variables, CLI, configuration, and paths

### 11.1 Environment variables

- **NAME-21a:** Forge-owned variables use `FORGE_*`: `FORGE_PLAIN`, `FORGE_REDUCED_MOTION`, `FORGE_LOG`.
- **NAME-21b:** Provider keys keep their ecosystem names: `OPENCODE_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`. Do not rename them to `FORGE_*`.
- **NAME-21c:** Environment variables are read only in `infra_local`. The core receives resolved values through ports or constructors.
- **NAME-21d:** Do not add a new `FORGE_*` variable when a configuration file field is the right home. Environment variables are for machine-local overrides and accessibility.

### 11.2 CLI

- **NAME-22a:** Flags are kebab-case: `--base-ref`, `--no-hooks`, `--execution-policy`, `--no-network`.
- **NAME-22b:** Subcommands are lowercase and match the PRD §8.1 surface: `init`, `run`, `resume`, `status`, `doctor`, `sandbox`, `exec`, `attest`, `merge`, `reopen`, `hooks`, `audit`, `transcript`, `task`, `ledger`. Nested commands are lowercase: `sandbox probe`, `task clean`, `ledger verify`, `ledger migrate`. Exact flags and help copy are TDD-0001's contract.
- **NAME-22c:** The hidden worker subcommand is exactly `__worker`, with two leading underscores. It is absent from `forge --help` and the public CLI contract.
- **NAME-22d:** Short flags are reserved for common, unambiguous actions. Do not assign a short flag to a destructive or irreversible action.
- **NAME-22e:** CLI names use the PRD's term: `--base-ref`, not `--base-branch`; `--execution-policy`, not `--exec-policy`.

### 11.3 Configuration and paths

- **NAME-23a:** TOML and JSON keys use `snake_case`: `schema_version`, `base_ref`, `entry_points`, `package_roots`, `reduced_motion`.
- **NAME-23b:** Committed file names use the PRD's exact names: `.forge/config`, `.forge/policy`, `.forge/execution-policy.json`, `.forge/.gitignore`.
- **NAME-23c:** Local-only paths use the PRD's exact names under `.forge/local/`: `config.local.toml`, `execution.local.json`, `keys/`, `sessions/`, `index.json`, `attestations/`, `dogfood/`.
- **NAME-23d:** New file names use kebab-case when the PRD does not fix the name: `init-report.json`, `repo-trust.json`.
- **NAME-23e:** Directory names are lowercase: `decisions/`, `assumptions/`, `attestations/`, `overrides/`, `trust/`, `worktrees/`, `plans/`, `artifacts/`.
- **NAME-23f:** A Rust module name may differ from a file name only when the PRD fixes the path. Document the exception at the module root.

---

## 12. Tests

### 12.1 Placement

- **NAME-24a:** Unit tests live in `#[cfg(test)] mod tests` at the bottom of the file they test.
- **NAME-24b:** Integration tests live in `tests/<area>.rs`. The test binary is named `<area>`.
- **NAME-24c:** No `tests/mod.rs`. Shared test helpers live in `tests/support/` and are imported with `mod support;`.
- **NAME-24d:** The architecture test is `tests/architecture.rs`. It is an integration test of the binary package and does not import the `forge` crate.
- **NAME-24e:** A test module for a specific submodule is named `tests` inside that file, not `test_<module>`.

### 12.2 Test names

- **NAME-25a:** Behavior tests use `given_<context>_when_<action>_then_<outcome>`:
  - `given_pending_gate_when_answered_then_task_resumes`
  - `given_untrusted_repo_when_guarded_requested_then_refused`
  - `given_duplicate_decision_when_recorded_then_rejected`
- **NAME-25b:** Property tests use `prop_<property>` when a property-testing crate is used: `prop_snapshot_round_trip_preserves_identity`.
- **NAME-25c:** Regression tests use `regression_<issue>_<behavior>`: `regression_42_truncated_event_tail_is_ignored`.
- **NAME-25d:** No `test_` prefix. The `#[test]` attribute already says it is a test.
- **NAME-25e:** A test name states the expected outcome, not the implementation detail.

### 12.3 Test doubles

- **NAME-26a:** `Fake<Port>` is a working in-memory implementation: `FakeSessionStore`, `FakeGitPort`.
- **NAME-26b:** `Stub<Port>` returns canned responses and does not record calls: `StubModelPort`.
- **NAME-26c:** `Spy<Port>` records calls for assertions: `SpyLedgerPort`.
- **NAME-26d:** Test doubles live in the test module that uses them, or in `tests/support/` when shared. They never ship in the production binary.
- **NAME-26e:** A fake does not contain business rules. If it does, it belongs in a test fixture, not a port fake.

---

## 13. Documentation and messages

### 13.1 Documentation

- **NAME-27a:** Every public item in `app_core::ports` has a `///` doc comment. `#![deny(missing_docs)]` enforces it.
- **NAME-27b:** Every module root has a `//!` module doc explaining its responsibility and boundary.
- **NAME-27c:** A doc comment states the contract, the error cases, and any cancellation or idempotency guarantee. It does not restate the type signature.
- **NAME-27d:** Link the PRD requirement ID or ADR when a rule is not obvious: `/// Implements NFR-MAINT-001.`
- **NAME-27e:** Comments explain why, not what. Delete a comment that repeats the code.

### 13.2 Error and CLI messages

- **NAME-28a:** Developer-facing error messages are lower-case, have no trailing period, and name the failed operation: `failed to load session snapshot`, `policy version is newer than this binary`.
- **NAME-28b:** A user-facing CLI message is sentence case, names the next command, and avoids internal jargon: `Forge is not initialized in this repository. Run forge init.`
- **NAME-28c:** Do not include secrets, raw model output, absolute home paths, or provider response bodies in a user-facing error.
- **NAME-28d:** Error variants carry structured data; the `Display` message is for humans. Do not parse `Display` output in code.
- **NAME-28e:** Do not use `unwrap()` or `expect()` to produce a user-facing error. Map the failure to a typed error.

---

## 14. Banned names

The following never appear in committed code, including test code and doc examples:

| Banned | Use instead |
|---|---|
| `*Dto`, `*DTO` | The PRD term, or `*Snapshot` / `*Record` / `*Report` |
| `*Data`, `*Info` | The PRD term |
| `*Model` for a port record | `*Snapshot` or the PRD term |
| `*Manager`, `*Service` for a domain service | Responsibility name: `PolicyEvaluator`, `RiskClassifier` |
| `*Helper`, `*Util`, `*Utils` | The owning module |
| `*Impl`, `*Abstract`, `*Base` | The concrete name |
| `I<Type>` trait prefix | The capability name |
| `mod.rs` | `foo.rs` + `foo/` |
| `utils.rs`, `helpers.rs`, `common.rs`, `misc.rs`, `shared.rs` | The owning module |
| `get_*` getter | The field name |
| `do_*`, `handle_*`, `process_*` | The action verb |
| `tmp`, `foo`, `bar`, `baz` in committed code | A descriptive name |
| `TaskData`, `TaskInfo`, `TaskDto`, `TaskModel` | `Task`, `TaskSnapshot`, or the PRD term |

A doc example may use `foo` only when the example is explicitly about syntax and not committed production code. Prefer a real domain name.

---

## 15. Approved abbreviations

Prefer a full word. The following abbreviations are allowed because they are standard in the domain or the Rust ecosystem:

`id`, `url`, `http`, `https`, `api`, `ipc`, `tui`, `cli`, `git`, `mcp`, `json`, `toml`, `yaml`, `sha`, `utf8`, `ascii`, `max`, `min`, `len`, `idx`.

Not allowed in new names: `cfg`, `ctx`, `err`, `req`, `res`, `msg`, `val`, `num`, `prev`, `cur`, `tmp`, `buf`, `ptr`, `fn`, `arg`, `param`, `attr`, `config` shortened to `cfg`. Use `configuration` or the specific noun.

`ref` is allowed as a suffix (`base_ref`, `subject_ref`) because it is a Git term. It is not allowed as a standalone variable name.

---

## 16. Examples

### 16.1 Good names

| Kind | Name |
|---|---|
| Package | `forge-harness-core`, `forge-harness` |
| Library crate | `app_core` |
| Binary crate | `forge` |
| Module | `session_store`, `start_task`, `policy_evaluator` |
| Domain entity | `Task`, `Plan`, `Attestation` |
| Value object | `TaskId`, `CommitSha`, `BranchName` |
| Port record | `TaskSnapshot`, `DecisionRecord`, `StatusReport` |
| Event | `TaskStarted`, `PlanApproved`, `GateAnswered` |
| Error | `SessionError`, `GitError`, `ApplicationError` |
| Outbound trait | `SessionStorePort`, `GitPort`, `ClockPort` |
| Inbound trait | `StartTask`, `AnswerGate`, `MergeTask` |
| Use case | `StartTaskUseCase`, `AnswerGateUseCase` |
| Adapter | `JsonlSessionStore`, `GitCli`, `AnthropicMessagesClient`, `OpenAiChatClient`, `MxcSandbox` |
| Service | `PolicyEvaluator`, `RiskClassifier`, `DecisionCoverage` |
| Test | `given_pending_gate_when_answered_then_task_resumes` |

### 16.2 Bad names and their replacements

| Bad | Good | Why |
|---|---|---|
| `TaskDTO` | `TaskSnapshot` | DTO is banned; the role is a snapshot. |
| `SessionManager` | `SessionStorePort` or `SessionService` with a real responsibility | Manager is banned; name the capability. |
| `GitHelper` | `GitCli` | Helper is banned; name the adapter. |
| `IStartTask` | `StartTask` | No `I` prefix. |
| `startTask` | `start_task` | Rust uses snake_case. |
| `get_task` | `task` | No `get_` getter. |
| `do_merge` | `merge` | Name the action. |
| `TaskData` | `Task` or `TaskSnapshot` | Data is banned; use the PRD term or a role suffix. |
| `mod.rs` | `session.rs` + `session/` | No `mod.rs`. |
| `cfg` | `configuration` | No non-approved abbreviation. |
| `OpenAIClient` | `OpenAiChatClient` | Acronyms are words; the role is Chat. |
| `JSONLSessionStore` | `JsonlSessionStore` | Acronym casing. |

---

## 17. Traceability

| Source | Covered by |
|---|---|
| PRD §5 ubiquitous language | NAME-06; §5.2 |
| PRD §6.5 workspace and module boundaries | NAME-01 to NAME-04 |
| PRD §6.7 documentation map | This document; the ADR links |
| NFR-MAINT-001 | NAME-01 to NAME-30 |
| D-13 v1 scope, stack, and distribution | NAME-01, NAME-14, NAME-22 |
| Ticket #20 | All rules |
| ADR-0001 | NAME-01 to NAME-04, NAME-11 to NAME-14 |
| ADR-0002 | NAME-05 to NAME-10, NAME-15 to NAME-20 |
| ADR-0003 | NAME-22 |
