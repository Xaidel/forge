# Sandboxing and permission models for Forge v1

**Ticket:** `03-research-sandboxing` · **Status:** research complete · **Date:** 2026-10-29
**Audience:** ticket 06 (sandbox and permission model), ticket 07 (scope, stack, distribution)

**Scope and method.** This brief compares practical ways to isolate a local coding agent's shell, file, and network access on macOS, Linux, and Windows. It prioritises primary sources: OS and kernel documentation, first-party product documentation, source repositories, and engineering posts from the teams that shipped these systems. It does **not** make the product decision; that belongs to ticket 06.

---

## Bottom line

The practical v1 is not "build a sandbox." It is **adopt a cross-platform process-sandbox runtime and put the whole tool-execution worker inside it**. In late 2026, the strongest option found in this survey is **Microsoft Execution Containers (MXC)**: OS-enforced filesystem, network, and UI policy on Windows, macOS, and Linux behind one TypeScript API, without Docker, WSL2, a separate Windows user account, or manual firewall rules on supported hosts. Use it behind a Forge-owned interface, fail closed on unsupported hosts, and keep permission prompts for escalation and consequential actions rather than as the boundary.

**The one-sentence recommendation:** ship v1 with MXC as the only sandbox engine, sandboxing the entire tool-execution worker (not just shell commands), require MXC's native process-containment tier, and fall back to an explicitly labelled approval-only `Guarded` mode when that tier is unavailable.

---

## 1. Executive recommendation

### 1.1 The decision

1. **Use MXC as the v1 sandbox engine.** One policy model, three native backends: Windows `ProcessContainer` (PSEC/BaseContainer), macOS `Seatbelt`, Linux `Bubblewrap` [S1][S3][S5]. MXC is MIT-licensed with a Node/TypeScript SDK [S1][S2][S4].
2. **Sandbox the worker, not just the shell.** The trusted host — CLI/TUI, model client, decision ledger, policy engine, and approval UI — stays outside. The worker that runs shell commands, file edits, patches, tests, and git runs inside MXC and communicates over a narrow stdio/IPC protocol. This closes the gap that existing agents leave open: Claude Code's sandbox covers Bash only, while its file tools, MCP servers, and hooks run outside [S6].
3. **Require the native tier and fail closed.** On Windows, require MXC to report native PSEC/`BaseContainer` containment via `getPlatformSupport()`/`probe()`; do not accept an AppContainer fallback or silently weaken the policy [S4][S5]. On macOS require Seatbelt (MXC requires macOS 15+); on Linux require bubblewrap/unprivileged user namespaces [S5].
4. **If the native tier is unavailable, run `Guarded` mode.** No autonomous execution. Every shell command, network access, and write outside the task worktree requires approval. The UI must show `Sandboxed` vs `Guarded` persistently; never downgrade silently.
5. **Do not build native sandbox code in v1.** No Seatbelt profiles from scratch, no Landlock/seccomp composition, no Windows AppContainer/user-account/WFP helper. That is a multi-month security engineering project, especially on Windows; the evidence is the teams that tried it (see §3).
6. **Do not make Docker, Podman, devcontainers, WSL2, Firecracker, or gVisor the v1 default.** Keep a container backend as an explicit escape hatch for repos that already have a devcontainer, and defer full devcontainer support.
7. **Move v1 to Node 24 LTS.** MXC's Node SDK requires Node 24; Windows stdio transfer requires 24.21.0+ (or 26.8.0+) [S4]. If Node 22 is immovable, invoke the `wxc-exec` executor binary instead, or use `srt` as the engine [S1][S7].
8. **Budget four weeks, not ten, for the sandbox.** Week 1 is a hard spike on the target OS matrix; weeks 2–4 cover worker isolation, policy compilation, the egress proxy, and the approval/audit path. If the spike fails on Windows, pivot rather than sink the schedule into native Windows work.

**Keep the layers separate.** The sandbox is the execution boundary; the decision ledger is the governance layer. A sandbox denial is a capability fact, not a product decision; a decision request is not a permission prompt. Forge should record grants in an execution audit log and decisions in the ledger, and never let one silently substitute for the other.

### 1.2 Why this option

**Windows is the binding constraint.** Windows has no lightweight, unprivileged, developer-compatible sandbox primitive comparable to Seatbelt or bubblewrap [S9]. Every serious coding agent has had to choose one of five expensive paths:

- **Build a native Windows sandbox.** Codex's current design uses dedicated lower-privilege sandbox users, a restricted-token command runner, Windows Firewall/WFP rules, ACLs, and a separate elevated setup binary. OpenAI's engineering post describes AppContainer as "the wrong shape" for open-ended developer workflows, Windows Sandbox as too heavy and unavailable on Home, and Mandatory Integrity Control as too broad because it relabels the real workspace [S9][S11].
- **Run the Linux sandbox inside WSL2.** This is what Cursor shipped on Windows; its engineering post says "building an equivalent native Windows sandbox is significantly harder because most existing sandboxing primitives are tailored to browsers and do not support general-purpose developer tools" [S13].
- **Run as a separate local user with a firewall fence.** Anthropic's `srt` Windows support (alpha) creates an `srt-sandbox` account, a WFP egress fence keyed to its SID, a write-restricted token, and ACLs; it requires a one-time elevated install and has documented limitations [S7].
- **Use low-integrity ACLs.** Gemini CLI's Windows native sandbox uses `icacls` to set Low Mandatory Level on writable paths, with persistent filesystem side effects after the session [S15].
- **Use Windows Sandbox.** It is a disposable Hyper-V VM, but it is unavailable on Windows Home and supports only one instance at a time; it cannot act on the user's real checkout without bridging [S24].

MXC is a vendor-supported abstraction that packages the hard Windows work — PSEC/BaseContainer process isolation, filesystem policy, network policy, UI restrictions — behind a typed SDK, and it is the backend OpenAI recommends on compatible Windows devices [S1][S3][S11]. On macOS and Linux it delegates to the same primitives Forge would otherwise hand-roll (Seatbelt and bubblewrap) [S1][S5].

**The alternatives trade away something Forge needs:**

- **Containers** are the mature cross-platform fallback, but they break the "use my installed toolchain" promise: the container image must contain Node/Python/Go/Rust/Java/.NET and the project's native dependencies, or the repo must ship a devcontainer. Docker Desktop also adds a VM, a runtime prerequisite, and (for larger organisations) a paid licence [S25][S26]. A local coding agent that cannot run the developer's `npm test` without rebuilding an image is a hard sell.
- **MicroVMs** are stronger but Linux-only and operationally heavy: Firecracker requires Linux/KVM and a guest kernel/root filesystem [S31][S32]; gVisor requires Linux [S33]. Neither runs natively on macOS or Windows, and both add image/network plumbing that a 10–12-week v1 cannot absorb.
- **Permission prompts and worktrees alone** are not a boundary. Prompts suffer approval fatigue, and prefix rules are explicitly fragile: Claude Code's own docs say Bash permission patterns are "not a security boundary around the program" and do not match `/usr/bin/curl` or `sh -c '...'` [S39]. A worktree isolates branch state, not secrets or network access, and shares the repository's `.git` directory [S36].
- **Raw OS primitives** are the right ingredients but the wrong v1 project. Landlock and seccomp are unprivileged and near-native, but composing them correctly (ABI versions, user namespaces, mount plans, seccomp filters) is a security project [S17][S18][S19]. Seatbelt is deprecated by Apple and uses an idiosyncratic profile language [S20]. Windows is worse (above).

### 1.3 What v1 must build

| Component | Minimum v1 behaviour |
|---|---|
| **Sandbox adapter** | A Forge-owned `Sandbox` interface (`run`, `spawn`, `probe`, `capabilities`) with one MXC implementation. The rest of Forge must never assume a sandbox exists. |
| **Worker isolation** | The tool-execution worker runs inside MXC; the model client, ledger, policy engine, and UI stay outside. Tool results cross a typed, size-limited IPC boundary. MXC's one-shot process backends require per-command `cwd`/env or a long-lived shell inside one sandbox; prefer per-command for policy clarity. |
| **Policy compiler** | Translate `.forge/policy` + repo facts into an MXC request: `readwritePaths`, `readonlyPaths`, `deniedPaths`, `network.egress`, `network.runtimeConfig.networkProxy`, explicit `process.env`, `ui.disable`. Use MXC's `getUserProfilePolicy`, `getTemporaryFilesPolicy`, and `getAvailableToolsPolicy` to generate baseline grants, then subtract credential paths [S4]. |
| **Egress proxy** | A local HTTP/CONNECT proxy with a domain allowlist, DNS-rebinding checks (reject loopback/link-local/private/metadata addresses unless explicitly allowed), and per-command attribution. MXC's direct egress rules are CIDR/port-based; domain allowlists need the proxy [S5]. On Windows, MXC's proxy path requires `ingress.default: allow` and either an identity-scoped proxy peer or host loopback; treat the strong peer-scoped path as a Week-1 spike and default to network-deny if it is not practical [S5]. |
| **Secret deny list** | Deny reads of `~/.ssh`, `~/.aws`, `~/.config/gh`, `~/.git-credentials`, `~/.npmrc`, `~/.pypirc`, `~/.docker/config.json`, `~/.kube/config`, browser profiles, and OS keychains; deny `.env*` except `.env.example`. |
| **Env allowlist** | `inheritDefaultEnv: false`; pass only `PATH`, `HOME`/`USERPROFILE`, `SystemRoot`, `TEMP`/`TMP`, locale, shell, and explicit per-repo variables. Never pass model keys, cloud credentials, `SSH_AUTH_SOCK`, or `DOCKER_HOST`. Windows ProcessContainer requires `SYSTEMROOT` and `LOCALAPPDATA` [S5]. |
| **Approval + audit** | A permission request is distinct from a decision request. Show the exact command/path/host, the reason (policy trigger or sandbox denial), and the scope (once/session). Persistent grants require a visible `.forge/policy` diff. Log every grant/denial. |
| **Worktree integration** | One linked worktree per task; agent writes only there. Treat the worktree as a review/rollback mechanism, not a security boundary; the sandbox must still deny writes outside it, including the main repo's `.git`. |
| **Fail-closed gating** | `Sandboxed` only when `probe()` reports the native tier and a negative canary test passes (write outside workspace denied, `~/.ssh` read denied, unlisted host denied). Otherwise `Guarded`. |

### 1.4 Recommended policy defaults (for ticket 06 to accept or reject)

| Capability | Default inside the sandbox | Escalation |
|---|---|---|
| Read workspace and toolchain directories | Allow | — |
| Write workspace | Allow | — |
| Write outside workspace | Deny | Ask once, showing the resolved path; no persistent grant without a policy-edit decision |
| Read credential stores | Deny | No blanket grant; one-file, one-time approval with the exact path |
| Read `.env*` | Deny; `.env.example` allowed | Ask per file, showing why the command needs it |
| Network | Deny | Per-repo domain allowlist through the proxy; a new host asks once |
| Package install | Allow if the registry is allowlisted | Otherwise ask |
| `git` read/commit | Allow inside sandbox | — |
| `git push` / private `git fetch` | Ask | Run outside the sandbox with the host's credential helper [S37]; show remote, branch, and commits |
| Destructive workspace commands (`rm -rf`, `git reset --hard`, `git clean -fd`, force push) | Ask even inside the sandbox | The sandbox limits blast radius; it does not infer intent |
| Child processes | Allow; inherit the sandbox | — |
| GUI, clipboard, synthetic input | Deny | Explicit per-run opt-in only |
| Model/API keys and cloud secrets | Never enter the sandbox | Model calls stay in the trusted host; or a host-side broker injects credentials |
| Local dev servers / loopback | Deny by default | Per-repo `allowLocalBinding` decision; on Windows this weakens the proxy fence, so require an explicit warning [S5] |

### 1.5 Schedule and kill criteria

| Week | Work | Gate |
|---|---|---|
| 1 | Spike MXC on Windows 11 24H2 (26100.9278+), macOS 15, Ubuntu 24.04. Run Node, Python, Go, Rust, and .NET installs/tests; `git commit`; verify deny-path, deny-network, child-process inheritance, symlink/junction escape; measure spawn overhead | If Windows `probe()` cannot report native BaseContainer, or the target toolchains cannot run after one week of policy tuning, **stop** and pivot |
| 2–3 | Worker isolation + IPC, policy compiler, egress proxy, secret deny list | Negative canary tests pass on all three OSes |
| 4 | Approval UX, audit log, worktree integration | A permission request and a decision request are visually and structurally distinct |
| 5–6 | Hardening, test matrix, docs, fallback ladder | `Guarded` mode is honest and usable; no silent downgrade |

**If the spike fails:** fallback A is `srt` on macOS/Linux plus WSL2 on Windows (Cursor's approach, Linux toolchains only) [S7][S13]; fallback B is a single Docker/Podman container backend; fallback C is `Guarded` mode only, with sandboxed autonomy deferred to v1.1. Do not start a native Windows sandbox build during v1.

---

## 2. Comparison

### 2.1 Execution boundaries

**Strength legend:** 5 = hardware-virtualisation boundary; 4 = OS kernel/security boundary or user-space kernel with a syscall boundary (Seatbelt, AppContainer/PSEC, Landlock+bwrap, gVisor); 3 = shared-kernel container with hardening; 2 = resource/UI limits only; 1 = policy/UX only. **Cost** is a rough solo-builder estimate for a first working implementation, not a product-quality security review.

| Option | What actually enforces it | Strength | Setup | Platforms | Performance | Toolchain fit | Rough cost | v1 |
|---|---|---|---|---|---|---|---|---|
| **macOS Seatbelt** (`sandbox-exec`) | Kernel MAC profile over the process tree; SBPL allow/deny for file, network, process, Mach operations [S20] | 4 | Low — built in | macOS only | Near-native | High — host tools; profile tuning | 1–2 wks raw; days via MXC/`srt` | Via MXC |
| **Linux Landlock + seccomp** | Landlock LSM for filesystem and TCP/UDP port rules (TCP ABI 4, UDP ABI 10); seccomp BPF for syscall filtering [S17][S18] | 4 | Medium — ABI/version checks, user namespaces | Linux 5.13+; network ABI 4+ (6.7) | Near-native | High | 3–4 wks raw | Via MXC/bwrap |
| **Linux bubblewrap** | User/mount/PID/net namespaces + bind mounts + seccomp [S19] | 4 | Medium — unprivileged userns; Ubuntu AppArmor profile needed on some versions [S10] | Linux only | Near-native | High | 1–2 wks raw | Via MXC |
| **Windows AppContainer** | Package identity + capability SIDs + Low IL + ACL intersection [S21][S22] | 4 | High — packaging/capabilities/ACLs, poor fit for arbitrary CLI tools [S9] | Windows only | Native | Low–medium | 6+ wks | No |
| **Windows Job Objects** | Resource limits, UI restrictions, kill-on-close process tree [S23] | 2 | Low | Windows only | Native | High | Days | Cleanup/timeouts only |
| **Windows dedicated user + ACL + WFP** | Separate local user, restricted token, ACL grants, firewall rules keyed to the user SID [S9][S7] | 4 | Very high — admin, users, firewall, runner binary, ACL propagation | Windows only | Native; ACL setup can be slow [S9] | Medium | 8+ wks | No |
| **MXC** | Windows PSEC/BaseContainer; macOS Seatbelt; Linux bubblewrap — one policy API [S1][S3][S5] | 4 | Low–medium on supported hosts; some hosts need prep | Windows 11 24H2+; macOS 15+; Linux with userns [S5][S11] | Near-native process spawn; no VM/image boot | Medium–high; policy tuning | 2–4 wks | **Yes** |
| **Anthropic `srt`** | Seatbelt/bwrap + host proxy; Windows alpha uses dedicated user + WFP + write-restricted token [S7] | 4 macOS/Linux; 3 alpha Windows | Low macOS; medium Linux; high Windows (one elevated install) [S7] | macOS, Linux, Windows alpha | Near-native | High | 1–3 wks | Alternative/fallback |
| **Docker / Podman** | Linux namespaces/cgroups/seccomp/capabilities; Docker Desktop and Podman machine add a VM on macOS/Windows [S26][S29] | 3 Linux; 4 on Win/mac VM | Medium–high — runtime, image, mounts, networking | All three via VM/WSL2 | Container/VM cold start; bind-mount I/O | Medium — image must match toolchain | 3–5 wks | Optional escape hatch |
| **Devcontainers** | Container + `devcontainer.json` metadata, Features, lifecycle scripts [S30] | 3–4 | High — image build, features, compose | All three | Same as containers | High if the repo has one | 4–6 wks | No — defer |
| **Firecracker microVM** | KVM hardware virtualisation + minimal device model + jailer [S31][S32] | 5 | Very high — Linux/KVM, kernel, rootfs, TAP networking | Linux only [S31] | <125 ms boot; claimed >95% bare-metal CPU (integration test pending) [S32] | Low | 8+ wks | No |
| **gVisor** | User-space kernel (Sentry) + `runsc` [S33][S34] | 4–5 | Medium–high — Linux, runsc, Docker/containerd | Linux only [S33] | Syscall-heavy workloads slower; network/VFS implementation costs [S35] | Medium | 4–6 wks | No |
| **WSL2** | Hyper-V lightweight VM + real Linux kernel; bwrap inside | 4 | Medium — WSL install, distro, toolchains | Windows only (Linux guest) | Native Linux FS; cross-OS FS slow [S38] | Linux toolchains only | 1–3 wks | Fallback only |
| **Worktree + prompts** | Git linked worktree + human approval | 1 | Low | All three | Native | High | 1–2 wks | Required complement |

**Key notes.**

- **MXC's Windows tier is the deciding detail.** `getPlatformSupport()` reports ProcessContainer only when native PSEC is available; current native execution reports `base-container` [S4]. Windows 11 24H2 build 26100.9278+ and 25H2 build 26200.9278+ are the documented ProcessContainer floors [S11][S40]. On older or managed hosts MXC may fall back to an AppContainer tier for compatible requests, but it rejects policy the fallback cannot enforce [S5]. Forge should require `base-container`, not merely "isSupported," and treat everything else as `Guarded`.
- **MXC requires macOS 15+ for Seatbelt, and Linux user namespaces for bubblewrap** [S5]. On Ubuntu 24.04, bubblewrap may need the `bwrap-userns-restrict` AppArmor profile or a sysctl change [S10]. These are onboarding costs, not runtime costs.
- **MXC's Node SDK requires Node 24+** [S4]; its native assets ship per platform via npm [S1]. Forge's Node 22 target is a direct conflict.
- **Codex's Windows evidence is the best public stress test.** OpenAI documents multiple binaries, dedicated sandbox users, WFP rules, ACLs, and an elevated setup step; it also documents why AppContainer, Windows Sandbox, and MIC did not fit [S9]. That is the cost Forge avoids by using MXC.
- **Cursor's Windows evidence is the best public proof of the WSL2 shortcut.** Cursor uses Seatbelt on macOS, Landlock+seccomp on Linux, and runs the Linux sandbox inside WSL2 on Windows [S13].
- **Claude Code's sandbox is shell-only.** Its docs say the sandbox covers Bash, PowerShell, and Monitor commands; file tools, MCP servers, hooks, and plugins run outside [S6]. That is a design gap Forge should not copy.
- **`srt`'s Windows alpha is real but not a v1 foundation for a Windows-primary product.** It requires a one-time elevated install, runs commands as a separate `srt-sandbox` account, uses a WFP egress fence, and documents that Schannel-based tools (system `curl.exe`, .NET, PowerShell web requests) do not work under its write-restricted token [S7].

### 2.2 Control layers (not boundaries)

| Control | What it actually does | Limits / bypasses | Role in v1 |
|---|---|---|---|
| **Git worktree** | Separate working tree and index; shared refs/config [S36] | Not a security boundary; shares `.git`; hooks/config; `cd` out; absolute paths | Required for review/rollback; pair with filesystem policy |
| **Permission prompts** | Human approves an action | Approval fatigue; compound commands; interpreters; `sh -c`; over-broad "always allow" | Required for escalation and destructive actions; not the boundary [S39] |
| **Command allowlist** | Reduces prompts for known prefixes | Prefix/parse bypasses; alternate paths; aliases; wrappers; `find -exec`; `git -c` [S39][S12] | UX only; never a security claim |
| **Environment scrubbing** | Child gets only allowlisted variables [S5] | Does not stop file reads; tools may read config files | Required; `inheritDefaultEnv: false` |
| **Read-deny list** | Blocks credential files and `.env` [S5] | Policy gaps; symlinks/junctions; Windows PSEC fs-deny support; copies elsewhere | Required; default-deny credential stores |
| **Network proxy allowlist** | Domain-level egress through HTTP/CONNECT proxy; blocks direct sockets [S5] | Only works if all egress uses the proxy; non-proxy-aware tools fail; DNS rebinding/IP checks; loopback | Required; default deny; MXC proxy path needs a Windows spike |
| **Credential masking/injection** | Proxy substitutes a real secret after TLS termination [S7] | Complex; MITM CA; mTLS/pinning; not v1 | Defer; use host-side approved `git push` |
| **Audit log** | Records grants, denials, executions | Detective, not preventive | Required for trust and the ledger |
| **Branch protection / attestation** | Governance and review | Not isolation | Required by the product, not by the sandbox |

### 2.3 Permission and worktree patterns in existing agents

| Agent | Default execution mode | Approval model | Sandbox scope / notable gap |
|---|---|---|---|
| **Claude Code** | Sandbox off by default; `/sandbox` enables Seatbelt/bwrap. Permission modes: manual, acceptEdits, plan, auto, dontAsk, bypassPermissions [S6][S39]. | Deny/ask/allow rules; Bash prefix wildcards; "allow for repo" writes to `.claude/settings.local.json`; prompts show the command [S39]. | Sandbox covers shell commands only; file tools, MCP servers, hooks, and plugins run outside [S6]. |
| **Codex** | `workspace-write` sandbox by default; `read-only` and `danger-full-access` modes; the sandbox applies to spawned commands [S10]. | `on-request` or `never`; rules (`prefix_rule`) gate commands outside the sandbox; approvals can go to the user or auto-review [S10][S12]. | Windows uses `mxc` on compatible devices, `elevated`/`unelevated` fallbacks; WSL2 uses the Linux sandbox [S11]. |
| **Cursor** | Auto-run in the sandbox where possible; Auto-review/Allowlist/Run Everything modes [S41]. | `sandbox.json` workspace read/write, network allow/deny; allowlist plus an LLM classifier for out-of-sandbox commands [S14][S41]. | Terminal-scoped sandbox; macOS Seatbelt, Linux Landlock+seccomp, Windows via WSL2 [S13]. |
| **OpenCode** | The permissions docs describe no OS sandbox; permissive defaults [S16]. | `allow`/`ask`/`deny`; granular bash patterns, last matching rule wins; `external_directory` ask; `.env` ask [S16]. | Permission rules are policy/UX, not an OS boundary; approval scope is once/always/reject [S16]. |
| **Gemini CLI** | Sandboxing disabled by default; `-s`/`--sandbox` or `--yolo` enables it [S15]. | Sandbox profiles (`permissive-open`, etc.); `--yolo`; Docker/Podman images [S15]. | Windows native sandbox uses low-integrity ACLs with persistent side effects; gVisor/LXC are Linux-only [S15]. |

**Worktree-only isolation.** Claude Code, Codex, and Cursor all use git worktrees (or equivalent task checkouts) to isolate a task's branch state, and Forge should too: one linked worktree per task, deleted or merged only after review. But a worktree is a review and rollback mechanism, not a security boundary. It shares the repository's refs, config, hooks, and object store; a command that can write to the main `.git` can affect the main checkout, and a command that can read the developer's home directory can still exfiltrate secrets. Worktree-only isolation is a useful complement to a sandbox and a bad substitute for one [S36].

### 2.4 How the options compare on the five criteria

- **Security strength.** A hardware VM (Firecracker, Windows Sandbox) is the strongest boundary; an OS process sandbox (Seatbelt, AppContainer/PSEC, Landlock+bwrap) is next; a shared-kernel container is next; prompts and worktrees are not boundaries. Policy quality matters more than the primitive: a permissive Seatbelt profile or a container with `--privileged`/host network is weaker than a well-scoped process sandbox. MXC, `srt`, and the native primitives land in the same strength class on macOS/Linux; MXC's Windows PSEC tier is a real OS boundary, not a restricted-token advisory.
- **Setup burden.** macOS Seatbelt is built in. Linux needs bubblewrap and user namespaces (plus AppArmor accommodation on some distros). Windows is the outlier: native work needs admin/users/firewall/ACLs; MXC needs a supported Windows build and possibly host prep; WSL2 needs a distro; Docker needs Docker Desktop/Podman and an image. MicroVMs are the heaviest.
- **Cross-platform support.** MXC and `srt` are the only process-sandbox options spanning all three OSes; `srt`'s Windows path is alpha, MXC's is production-backed by Codex. Containers span all three but add a VM/runtime. WSL2 is Windows-only. Firecracker and gVisor are Linux-only. Raw primitives are inherently per-OS.
- **Performance.** Process sandboxes are near-native: kernel checks, no VM boot, no image pull. Containers add VM/engine startup and bind-mount I/O; Firecracker boots in under 125 ms but still needs a kernel/rootfs/network; gVisor adds syscall interception and has known network/VFS implementation costs [S32][S35]. The practical difference for a coding agent is command latency and file-watch/build I/O, not raw CPU.
- **Toolchain compatibility.** Host-native sandboxes win because they run the developer's installed Node/Python/Go/Rust/.NET and their caches. Containers win only if the repo has a working devcontainer or Forge maintains a multi-toolchain image; otherwise the agent cannot run the project's tests. AppContainer is the weakest fit for arbitrary CLI tools [S9]. WSL2 is Linux-only. MXC sits in the middle: host-native, but policy tuning is needed for tools that search upward, write temp/cache dirs, or spawn helpers [S1].

### 2.5 What the evidence says

1. **Autonomy leaders converge on OS-level sandboxing.** Claude Code, Codex, Cursor, and Gemini CLI all ship, or have shipped, OS-level sandboxing on at least macOS/Linux; the common pattern is "sandbox by default, ask to escalate" [S6][S10][S13][S15]. Anthropic's design note explains why filesystem and network isolation must both be present [S8], and Cursor's `sandbox.json` exposes workspace read/write modes plus network allow/deny [S14]. OpenCode documents rich permission rules but no OS sandbox in its permissions docs; that is the permission-only outlier [S16].
2. **Windows is where every team pays.** Codex built a multi-binary elevated sandbox; Cursor chose WSL2; Anthropic's `srt` Windows path is alpha with a dedicated user and WFP; Gemini uses low-integrity ACLs with persistent side effects [S9][S13][S7][S15]. There is no clean, unprivileged, arbitrary-toolchain Windows primitive in 2026 [S9].
3. **Shell-only sandboxing is the common gap.** Claude Code's docs explicitly scope the sandbox to shell commands and leave file tools/MCP/hooks outside [S6]. For a product whose thesis is that the sandbox is the execution boundary, Forge should sandbox the worker, not just `bash`.
4. **Containers are the mature fallback, not the default.** They are cross-platform and well understood, but require a runtime and an image that matches the project; devcontainers fix the image problem only when the repo already has one, and implementing the full spec is a product in itself [S30].
5. **MicroVMs are the wrong shape for a laptop v1.** Firecracker is Linux/KVM-only; gVisor is Linux-only; both are stronger than the threat model needs and add image/networking work that a solo builder cannot absorb in 10–12 weeks [S31][S33].

---

## 3. Strongest counterargument

**The recommendation bets the primary platform on a new Windows sandbox stack that may not exist on the laptops Forge targets.** MXC's Windows ProcessContainer path requires Windows 11 24H2 build 26100.9278+ (or a later Windows 11 release), PSEC/BaseContainer support, and a policy the host can fully enforce; otherwise it rejects the request or falls back to an AppContainer tier that cannot express the full policy [S4][S5][S11]. The fallback is the model OpenAI's own engineering post rejected for open-ended developer workflows [S9]. A large share of Windows developers, and many managed corporate laptops, will not meet that floor. For them Forge falls back to `Guarded` mode — per-command approval — which is exactly the approval-fatigue experience the product is trying to eliminate. If that share is large, v1 ships its headline autonomy only on macOS/Linux and on a subset of Windows machines, while the primary platform gets the least autonomous experience.

MXC is also a 2026 v1.0 dependency. Its Node SDK requires Node 24 and its Seatbelt backend requires macOS 15+, so the supported matrix is narrower than "Windows, macOS, Linux" suggests [S4][S5]. Its policy model is powerful but not turnkey: its own README warns "Your application won't run in the sandbox!" until the policy is tuned, and Windows BaseContainer adds specific failure modes (upward directory traversal, temp/profile grants, package-manager caches) [S1][S5]. A solo builder can burn the 10–12-week budget on Windows access-denied debugging and still ship a sandbox users disable. The mature alternative is a container backend: Docker Desktop works on Windows Home via WSL2, `--network none` gives a clean default, and the boundary is well understood [S26][S27][S28]. It costs a Docker prerequisite and toolchain duplication, but it is predictable. WSL2 is the other proven shortcut, and Cursor ships it [S13].

There is a second, related objection: **a sandbox that wraps only shell commands is security theatre for a coding agent.** The agent's file tools, MCP servers, and model client can read `~/.ssh` and send it to the model API without ever running a shell command. Claude Code's docs make this gap explicit [S6]. The recommendation only works if Forge puts the entire tool-execution worker inside the sandbox; that is a larger architectural change than "add MXC," and it may not fit 10–12 weeks. If Forge ships shell-only sandboxing and calls it the execution boundary, it will have repeated the industry's most common mistake.

**Decision rule.** If the Week-1 spike shows MXC's native BaseContainer tier running the target toolchains on the actual Windows fleet, proceed. If not, do not spend weeks on Windows policy tuning: ship `Guarded` mode on unsupported Windows, use `srt` or MXC on macOS/Linux, and make the platform support matrix explicit. The product can survive "Windows-native autonomy is beta"; it cannot survive a sandbox that silently fails open or an approval prompt on every command.

---

## 4. Open questions for the product decision

1. **Windows floor.** What minimum Windows build can Forge require for sandboxed autonomy (Windows 11 24H2 26100.9278+)? What share of target users are on Windows 10, 23H2, or managed machines where PSEC is unavailable? If the share is material, is `Guarded` mode an acceptable v1 experience, or does v1 need a container/WSL2 fallback wizard?
2. **Node and macOS floors.** Is moving v1 to Node 24 LTS acceptable? Is macOS 15+ acceptable? If not, MXC's SDK is out; the alternatives are `wxc-exec`, `srt`, or a container backend.
3. **Sandbox scope.** Is Forge willing to run the whole tool-execution worker inside the sandbox, with the model client outside? If the worker needs model API access, how does it get it without exposing the API key — host-side model calls, a local broker, or a proxy that injects credentials?
4. **Network policy.** What is the default: deny all, allow package registries only, or per-repo allowlist? Who owns the proxy (Forge process vs separate helper)? Are local dev servers/loopback allowed? On Windows, the MXC proxy path requires `ingress.default: allow` and either an identity-scoped proxy peer or host loopback; the latter is weaker [S5]. Is that acceptable, or should Windows default to network-deny with explicit approved network sessions?
5. **Git credentials.** Do `git push`/private `fetch` run outside the sandbox with the host's credential helper, or through a credential-injecting proxy? Is SSH supported, or HTTPS only? What exactly does the approval show (remote, branch, commits, diffstat)?
6. **Secret policy.** Which paths and env vars are denied by default? Is `.env` always denied, or per-repo? Does v1 need credential masking/injection, or is read-deny + env scrubbing sufficient? (Masking requires TLS termination and is a v1.1 feature at the earliest [S7].)
7. **Fallback UX.** When the sandbox is unavailable, does Forge (a) refuse autonomous mode, (b) offer a guided Docker/WSL2 setup, or (c) run `Guarded` with per-action approval? What is the persistent indicator, and what does the ledger record?
8. **Persistent grants.** Does "allow for this repo" write to `.forge/policy` (committed, reviewable) or a local settings file? Does a persistent grant count as a decision that needs a ledger entry, or only as an execution permission?
9. **Test matrix and CI.** How will Forge test OS sandboxes in CI? GitHub-hosted runners may not expose PSEC/WFP or Seatbelt; a self-hosted Windows 11 24H2+ runner and a macOS runner are likely required. What are the negative canary tests that must pass before `Sandboxed` is displayed?
10. **MXC vendor risk and exit plan.** What is the fallback if MXC changes its API, drops a platform, or a Windows update breaks PSEC? Is there a maintained `srt`/bubblewrap fallback behind the same `Sandbox` interface? What conformance suite proves both implementations enforce the same policy?
11. **Distribution.** MXC ships native assets per platform via npm; what is the package size, signing/notarisation story, and update path for win32-x64/arm64, darwin-x64/arm64, and linux-x64/arm64? Does antivirus/EDR flag the native executor?
12. **Deferred scope.** Is Docker/devcontainer support a v1 escape hatch or a v1.1 feature? Is WSL2 a supported fallback or just a documented workaround? Are microVMs explicitly out of scope for the foreseeable roadmap?

---

## 5. Sources

Primary sources were preferred. Where a vendor page is a mirror of an OS man page, that is noted.

- [S1] Microsoft, *Microsoft eXecution Container (MXC)*, GitHub README — cross-platform backends, policy model, MIT licence, Node SDK, executor binaries. https://github.com/microsoft/mxc
- [S2] Microsoft Learn, *Microsoft Execution Containers overview* (updated 2026-10-15) — policy/containment concepts and supported SDKs. https://learn.microsoft.com/en-us/windows/apps/develop/mxc/
- [S3] Microsoft Learn, *MXC containment options and platform support* (updated 2026-10-15) — Windows ProcessContainer, Linux bubblewrap, macOS Seatbelt; V1 SDK backend matrix. https://learn.microsoft.com/en-us/windows/apps/develop/mxc/platform-support
- [S4] Microsoft, *`@microsoft/mxc-sdk` Node.js/TypeScript SDK README* — Node 24+ requirement, `getPlatformSupport`, `probe`, `run`/`spawn`, PTY, lifecycle. https://github.com/microsoft/mxc/blob/main/sdk/node/README.md
- [S5] Microsoft, *MXC configuration schema* — filesystem `readwritePaths`/`readonlyPaths`/`deniedPaths`, network egress/proxy, env, UI, backend behaviour, macOS 15+, no AppContainer fallback for unsupported policy. https://github.com/microsoft/mxc/blob/main/docs/schema.md
- [S6] Anthropic, *Claude Code sandboxing documentation* — macOS Seatbelt, Linux/WSL2 bubblewrap+socat, native Windows unsandboxed, shell-only scope, default read/write/network boundaries, proxy. https://code.claude.com/docs/en/sandboxing
- [S7] Anthropic, *anthropics/sandbox-runtime* README — Seatbelt/bwrap/WFP implementation, Windows alpha dedicated user, one-time elevated install, known limitations. https://github.com/anthropics/sandbox-runtime
- [S8] Anthropic, *Making Claude Code more secure and autonomous with sandboxing* (2025-10-20) — dual filesystem/network isolation, proxy design, fewer prompts. https://www.anthropic.com/engineering/claude-code-sandboxing
- [S9] OpenAI, *Building a safe, effective sandbox to enable Codex on Windows* (2026-05-13) — AppContainer/Windows Sandbox/MIC evaluation, elevated sandbox with users/ACLs/WFP, four-binary architecture. https://openai.com/index/building-codex-windows-sandbox/
- [S10] OpenAI, *Codex sandbox* — workspace-write default, read-only/full-access modes, platform-native enforcement, Windows native/WSL2, bubblewrap/AppArmor notes. https://developers.openai.com/codex/sandboxing
- [S11] OpenAI, *Codex Windows sandbox* — `mxc` recommended on compatible devices, `elevated`/`unelevated` fallbacks, Windows 11 24H2/25H2 MXC builds, no admin setup for MXC. https://learn.chatgpt.com/docs/windows/windows-sandbox
- [S12] OpenAI, *Codex rules* — `prefix_rule`, tree-sitter splitting of safe compound commands, conservative handling of complex shell. https://developers.openai.com/codex/agent-configuration/rules
- [S13] Cursor, *Implementing a secure sandbox for local agents* (2026-02-18) — macOS Seatbelt, Linux Landlock+seccomp, Windows via WSL2, VM/container trade-offs. https://cursor.com/blog/agent-sandboxing
- [S14] Cursor, *sandbox.json reference* — workspace read/write modes, network allow/deny, protected paths, IPv4/IPv6 private-range blocking. https://cursor.com/docs/reference/sandbox
- [S15] Google, *Sandboxing in Gemini CLI* — macOS Seatbelt profiles, Docker/Podman, Windows native low-integrity ACLs, gVisor/LXC Linux-only. https://geminicli.com/docs/cli/sandbox/
- [S16] OpenCode, *Permissions* — allow/ask/deny, granular bash patterns, last-match-wins, permissive defaults, `external_directory`, `.env` ask. https://opencode.ai/docs/permissions/
- [S17] Linux kernel, *Landlock: unprivileged access control* (August 2026) — filesystem rules, TCP/UDP port rules, ABI versions, unprivileged, stackable, cannot restrict `chdir`/`stat`/`chmod`. https://docs.kernel.org/userspace-api/landlock.html
- [S18] Linux kernel, *Seccomp BPF* — syscall filtering, no pointer dereference, `no_new_privs`, "seccomp is not a sandbox." https://docs.kernel.org/userspace-api/seccomp_filter.html
- [S19] containers/bubblewrap, *README* — unprivileged user namespaces, mount/PID/net namespaces, seccomp, no setuid mode, limitations (D-Bus, TIOCSTI). https://github.com/containers/bubblewrap
- [S20] Apple, *`sandbox-exec(1)` man page* (mirror of the macOS man page) — deprecated, SBPL profiles, process-tree inheritance. https://keith.github.io/xcode-man-pages/sandbox-exec.1.html
- [S21] Microsoft Learn, *AppContainer isolation* — credential/device/file/network/process/window isolation, capability-based least privilege. https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation
- [S22] Microsoft Learn, *Implementing an AppContainer* — package identity, capability SIDs, Low IL, ACL intersection, network capability. https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer
- [S23] Microsoft Learn, *Job Objects* — resource limits, UI restrictions, process-tree assignment/termination; not a filesystem or network boundary. https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- [S24] Microsoft Learn, *Windows Sandbox* — Pro/Enterprise/Education only, not Home; disposable VM; no multiple instances; networking on by default. https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/
- [S25] Docker, *Docker Desktop license agreement* — free for small businesses (<250 employees and <$10M revenue), personal, education, OSS; paid otherwise. https://docs.docker.com/subscription/desktop-license/
- [S26] Docker, *Virtual Machine Manager* — Docker Desktop runs the Linux VM; WSL2/Hyper-V/Docker VMM on Windows, Virtualization framework/Docker VMM on macOS. https://docs.docker.com/desktop/features/vmm/
- [S27] Docker, *Networking overview* — default bridge, drivers, container networking; `none`/`host` behaviour. https://docs.docker.com/engine/network/
- [S28] Docker, *Docker Engine network drivers* — `none` completely isolates a container from the host and other containers. https://docs.docker.com/engine/network/drivers/none/
- [S29] Podman, *Installation* — macOS and Windows use a Podman machine VM; Windows uses WSL2. https://podman.io/docs/installation
- [S30] Development Containers, *Specification* — `devcontainer.json`, image/Dockerfile/Compose, Features, lifecycle. https://containers.dev/implementors/spec/
- [S31] Firecracker, *Design* — Linux hosts and Linux guests, KVM, jailer, TAP networking, no network filtering. https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md
- [S32] Firecracker, *Specification* — <125 ms boot, <5 MiB VMM overhead, >95% bare-metal CPU. https://github.com/firecracker-microvm/firecracker/blob/main/SPECIFICATION.md
- [S33] gVisor, *FAQ* — requires Linux; x86_64/ARM64. https://gvisor.dev/docs/user_guide/faq/
- [S34] gVisor, *Platform guide* — KVM/systrap/ptrace; systrap default; ptrace performance penalty. https://gvisor.dev/docs/architecture_guide/platforms/
- [S35] gVisor, *Performance guide* — syscall, network, VFS implementation costs. https://gvisor.dev/docs/architecture_guide/performance/
- [S36] Git, *git-worktree* — linked worktrees, shared refs/config, per-worktree index/HEAD; multiple checkout is still experimental for submodules. https://git-scm.com/docs/git-worktree
- [S37] Git, *gitcredentials* — credential helpers, OS keychain helpers (osxkeychain, wincred, libsecret, GCM), `store` plaintext caveat. https://git-scm.com/docs/gitcredentials
- [S38] Microsoft Learn, *Comparing WSL versions* — WSL2 managed VM + real Linux kernel; cross-OS filesystem performance poor; store project files on the same OS as the tools. https://learn.microsoft.com/en-us/windows/wsl/compare-versions
- [S39] Anthropic, *Configure permissions* — Bash rules are not a security boundary; alternate paths, `sh -c`, wrappers; use sandbox for enforcement. https://code.claude.com/docs/en/permissions
- [S40] Microsoft, *Windows OS-version support for ProcessContainer* — 24H2 26100.9278+, 25H2 26200.9278+, 26H2, 26H1. https://github.com/microsoft/mxc/blob/main/docs/backends/process-container/os-version-support.md
- [S41] Cursor, *Terminal integration* — Auto-review, Allowlist, and Run Everything modes; sandboxed commands and approval behaviour. https://cursor.com/help/ai-features/terminal
