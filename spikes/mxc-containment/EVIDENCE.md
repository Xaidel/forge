# MXC native-containment spike: evidence

- **Ticket:** [Spike the MXC native containment tier on macOS, Windows, and Ubuntu](https://github.com/Xaidel/forge/issues/23)
- **Map:** [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- **Date:** 2026-11-04
- **SDK:** `mxc-sdk` 1.0.1 (crates.io, Microsoft OSS release, MIT)
- **Spike branch:** `spike/mxc-containment` (throwaway; do not merge)
- **Workflow:** [`.github/workflows/mxc-spike.yml`](https://github.com/Xaidel/forge/blob/spike/mxc-containment/.github/workflows/mxc-spike.yml)
- **Harness:** [`spikes/mxc-containment`](https://github.com/Xaidel/forge/tree/spike/mxc-containment/spikes/mxc-containment)
- **CI run:** [41826409053](https://github.com/Xaidel/forge/actions/runs/41826409053)

## Verdict

| Platform | Native backend | Mode label | Result |
|---|---|---|---|
| Windows 11 25H2 build 26200 x64 | `processcontainer` (`base-container`) | `Sandboxed` | **PASS**: all six PRD canaries are explicit denials; host unchanged. |
| macOS 15.8.2 build 24H232 arm64 | `seatbelt` | `Sandboxed` | **PASS**: all six are explicit denials; host unchanged. |
| macOS 15.7.9 build 24G830 x64 | `seatbelt` | `Sandboxed` | **PASS**: all six are explicit denials; host unchanged. |
| Ubuntu 24.04.5 LTS x64, stock | `bubblewrap` | `Guarded` only | **FAIL to launch**: AppArmor's `kernel.apparmor_restrict_unprivileged_userns=1` blocks Bubblewrap; `bwrap` exits on `loopback: Failed RTM_NEWADDR: Operation not permitted`. |
| Ubuntu 24.04.5 LTS x64, user namespaces permitted, allowlist-only policy | `bubblewrap` | `Sandboxed` | **PASS**: all six canaries are explicit denials (`ENOENT` / `ENETUNREACH`); host unchanged. |
| Ubuntu 24.04.5 LTS x64, user namespaces permitted, redundant explicit denies | `bubblewrap` | `Sandboxed` with a canary caveat | **CONTAINED**: no host mutation, but two write canaries are masked by a writable tmpfs rather than returning `EACCES`. See [Ubuntu finding](#ubuntu-2404-findings). |

The engine decision does not need to be reopened: MXC's native tier works on all three platforms. Ubuntu needs a user-namespace prerequisite, and Forge's policy compiler should use allowlist-only for paths outside the worktree so the canaries are true denials. The writable-tmpfs behavior only appears when a redundant explicit `deniedPaths` entry masks a path that was never granted.

## What was built

A two-binary Rust spike:

- `mxc-spike` (host): creates the fixtures, builds the MXC request, calls
  `mxc_sdk::v1::run`, parses the canary output, and checks the host fixtures
  after the run.
- `canary` (worker): attempts each denial from inside the sandbox. It is copied
  into the task worktree before launch.

### Fixture layout

```text
mxc-spike-run/
  worktree/                    read-write grant; cwd
    canary                     worker binary
    tmp/                       TEMP/TMP/TMPDIR
    escape-outside/            host-created junction/symlink -> outside/
    escape-credentials/        host-created junction/symlink -> credentials/
  outside/                     not granted
    host-sentinel.txt          "HOST-SENTINEL-UNCHANGED"; host checks after run
  credentials/                 not granted
    id_rsa                     fake private key
```

On Windows the escape links are directory junctions created by `mklink /J`
before launch, so the test measures traversal, not link-creation privilege. On
macOS and Linux they are symlinks created before launch.

### MXC API used

```rust
use mxc_sdk::v1::{
    self, ContainerRequest, Containment, WaitResult,
};
use mxc_sdk::v1::policy::{
    FilesystemPolicy, NetworkAction, NetworkEgressPolicy,
    NetworkIngressPolicy, NetworkPolicy, UiPolicy,
};

let support = v1::platform_support();
let backends = v1::available_backends();
#[cfg(windows)]
let probe = v1::probe(Some(&request));

let request = ContainerRequest {
    containment: Containment::Process, // native default per host
    filesystem: Some(FilesystemPolicy {
        readwrite_paths: vec![worktree],
        readonly_paths,                 // OS toolchain/runtime paths
        denied_paths,                   // outside + credentials, or empty
        clear_policy_on_exit: Some(true),
    }),
    network: Some(NetworkPolicy {
        egress: Some(NetworkEgressPolicy {
            default: Some(NetworkAction::Deny),
            ..Default::default()
        }),
        ingress: Some(NetworkIngressPolicy {
            default: Some(NetworkAction::Deny),
            host_loopback: Some(NetworkAction::Deny),
        }),
        ..Default::default()
    }),
    ui: Some(UiPolicy::default()),       // UI disabled, clipboard none
    timeout_ms: Some(120_000),
    working_directory: Some(worktree),
    environment: Some(worker_env),       // explicit allowlist
    inherit_default_environment: Some(false),
    ..ContainerRequest::new(command)
};

let result = v1::run(request, Default::default())?;
```

`Containment::Process` resolves to `processcontainer` on Windows, `seatbelt` on
macOS, and `bubblewrap` on Linux. No `mxc-exec` process is spawned for the
native tier: the Rust crate builds the engine and selected backend into the
host binary.

### Environment allowlist

Windows: `PATH`, `SystemRoot`, `windir`, `ComSpec`, `PATHEXT`,
`PROCESSOR_ARCHITECTURE`, `OS`, `LOCALAPPDATA`, `APPDATA`, `ProgramData`,
`SystemDrive`, `USERPROFILE`, `HOMEDRIVE`, `TEMP`, `TMP`, `TMPDIR`, and
`FORGE_SPIKE_WORKTREE`.

macOS/Linux: `PATH`, `HOME`, `USERPROFILE`, `TEMP`, `TMP`, `TMPDIR`, `LANG`,
`LC_ALL`, `TERM`, and `FORGE_SPIKE_WORKTREE`.

The host exported `FORGE_SPIKE_HOST_SECRET=leak-me` and
`ANTHROPIC_API_KEY=sk-ant-spike-should-not-leak` before the run. Neither reached
the worker.

## Build and run

Windows (physical host, Rust 1.96.0 MSVC):

```powershell
cargo build --bins
$env:FORGE_SPIKE_HOST_SECRET = 'leak-me'
$env:ANTHROPIC_API_KEY = 'sk-ant-spike-should-not-leak'
.\target\debug\mxc-spike.exe
```

macOS and Ubuntu (GitHub-hosted, Rust 1.93.0):

```sh
cargo build --release --bins
FORGE_SPIKE_HOST_SECRET=leak-me ./target/release/mxc-spike
```

Ubuntu additionally needs `sudo apt-get install -y bubblewrap` (0.9.0 on the
runner). The stock-host run is expected to fail; the workflow then sets
`kernel.apparmor_restrict_unprivileged_userns=0` and reruns.

### Asset path answer

- **Rust dependency:** `mxc-sdk = "=1.0.1"`; default features; no `microvm`,
  `hyperlight`, `isolation_session`, `wslc`, or `link-wslcsdk`.
- **Build output:** the native engine and backend are linked into the host
  binary. Cargo's build-script output lives under
  `target/<profile>/build/mxc-sdk-<hash>/out/` and contains generated version
  resources/telemetry consent data, not a standalone MXC executable.
- **Windows:** no `wxc-exec.exe` asset is needed. The optional
  `wxc-windows-sandbox-daemon`, `wxc-windows-sandbox-guest`, and
  `wxc-wslc-daemon` binaries are feature-gated and were not built.
- **macOS:** no `mxc-exec-mac` asset; Seatbelt is applied in-process with
  `sandbox_init`.
- **Linux:** the only external asset is the host's `bwrap` (0.9.0 tested; MXC
  requires 0.5.0+), found on `PATH` or the standard system directories.
- **Toolchain:** 1.93.0 is sufficient (tested on macOS/Linux CI); 1.96.0 also
  builds on Windows.

## Canary results

`Denied` means the operation failed. `Contained` means a write succeeded into a
private MXC mask but the host fixture was unchanged. `Fail` means the host
fixture changed or a secret leaked.

| # | Canary | Windows 25H2 x64 | macOS 15.8.2 arm64 | macOS 15.7.9 x64 | Ubuntu 24.04, explicit denies | Ubuntu 24.04, allowlist only |
|---|---|---|---|---|---|---|
| 1 | Write outside the task worktree | Denied (`PermissionDenied`, os 5) | Denied (`EPERM`) | Denied (`EPERM`) | **Contained** (write landed in a writable tmpfs mask) | Denied (`ENOENT`) |
| 2 | Read a credential path | Denied (`PermissionDenied`, os 5) | Denied (`EPERM`) | Denied (`EPERM`) | Denied (`ENOENT` from the empty mask) | Denied (`ENOENT`) |
| 3 | Unlisted network host (`1.1.1.1:443`) | Denied (`WSAEACCES` 10013) | Denied (`EPERM`) | Denied (`EPERM`) | Denied (`ENETUNREACH`) | Denied (`ENETUNREACH`) |
| 4 | Child process inherits denials | Denied (child write + read `PermissionDenied`) | Denied (child write + read `EPERM`) | Denied (child write + read `EPERM`) | **Contained** (child write landed in the mask; child read `ENOENT`) | Denied (child write + read `ENOENT`) |
| 5 | Symlink/junction escape | Denied (write + read `PermissionDenied`) | Denied (write + read `EPERM`) | Denied (write + read `EPERM`) | **Contained** (write landed in the mask; read `ENOENT`) | Denied (write + read `ENOENT`) |
| 6 | Secret-free worker environment | Denied/absent | Denied/absent | Denied/absent | Denied/absent | Denied/absent |
| | Host sentinel after run | Unchanged | Unchanged | Unchanged | Unchanged; no `canary-write.txt` or `child-write.txt` on host | Unchanged; no host writes |

Raw logs: [Windows 11 25H2 x64](https://github.com/Xaidel/forge/blob/spike/mxc-containment/spikes/mxc-containment/evidence-windows-11-25h2-x64.txt);
the macOS and Ubuntu logs are attached to run
[41826409053](https://github.com/Xaidel/forge/actions/runs/41826409053) as
`evidence-*.log` and are also in the `spikes/mxc-containment` tree when
downloaded from that run.

### Windows details

- Host: Windows 11 Home Single Language, version 10.0.26200, build 26200 (25H2),
  x64.
- `platform_support`: `is_supported=true`, `methods=["processcontainer"]`.
- `available_backends`: `processcontainer`, tier `base-container`,
  capabilities `[CaptureDenials, FilesystemDeniedPaths,
  FilesystemEnumeratePaths, IngressHostLoopbackAllow,
  IdentitylessLoopbackProxy]`.
- `probe`: `tier=base-container`, `needs_dacl_augmentation=false`,
  `base_container_api_present=true`, `native_capture_available=true`,
  `base_container_supports_deny_paths=true`,
  `base_container_supports_enumerate_paths=true`.
- All writes and reads failed with `Access is denied` (os error 5); the network
  connect failed with `WSAEACCES` (10013). No warnings, no DACL fallback.

### macOS details

- arm64: macOS 15.8.2, build 24H232, Darwin 24.6.0, Rust 1.93.0.
- x64: macOS 15.7.9, build 24G830, Darwin 24.6.0, Rust 1.93.0.
- `platform_support`: `is_supported=true`, `methods=["seatbelt"]`.
- `available_backends`: `seatbelt`, tier `None`, capabilities `[]`, no warnings.
- Every denial surfaced as `Operation not permitted` (`EPERM`); no MXC
  warnings. Seatbelt's `deniedPaths` and deny-by-default baseline both held.

### Ubuntu 24.04 findings

#### 1. Stock AppArmor restriction blocks Bubblewrap

The runner is Ubuntu 24.04.5 LTS, kernel `7.0.0-1017-azure` x86_64,
`bubblewrap 0.9.0`. `kernel.unprivileged_userns_clone=1`, but
`kernel.apparmor_restrict_unprivileged_userns=1`. No `bwrap` AppArmor profile
was present. `unshare --user --net true` failed, and the first canary launch
failed with:

```text
bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted
```

MXC's `platform_support()` still reported `is_supported=true` with
`methods=["bubblewrap"]`; only `available_backends()` warned that unprivileged
user and network namespaces were refused. This is a trap for Forge's mode
labeling: **`platform_support()` alone is not a launch probe.**

MXC's own `prepare-linux-host.sh` for its Bubblewrap CI explicitly relaxes
`kernel.apparmor_restrict_unprivileged_userns` on ephemeral hosts, confirming
this is a known host prerequisite, not an MXC-only quirk. Forge must not
silently relax a security sysctl on a developer machine. Options for TDD-0006:

1. Detect the restriction at `forge init`/session start and run `Guarded` on
   trusted repos, refusing untrusted repos (fail closed).
2. Document and verify an AppArmor profile that grants `userns` to `bwrap` only,
   so the developer can opt in without disabling the sysctl globally.
3. Accept the prerequisite as an Ubuntu 24.04 install step and re-run the
   canaries; only then label `Sandboxed`.

Option 1 is the only one that preserves the PRD's "no silent downgrade" rule
without changing host security settings. It may require a PRD/TDD decision
because `NFR-COMPAT-001` currently lists Ubuntu 24.04 as a supported
`Sandboxed` platform.

#### 2. Denied directories are a writable tmpfs, not an explicit denial

With user namespaces permitted and explicit `deniedPaths`, the canaries that
write outside the worktree **succeeded at the syscall level**:

```text
CANARY write-outside-worktree FAIL write unexpectedly succeeded: .../outside
CANARY child-process-inherits-denials FAIL ... CHILD write ALLOWED .../outside/child-write.txt
CANARY symlink-or-junction-escape FAIL write through escape link succeeded: .../escape-outside/canary-write.txt
```

But the host was unchanged:

```text
host mutation check: sentinel_changed=false canary_write_created=false child_write_created=false
```

The writes landed in an empty tmpfs that MXC mounts over each denied directory.
The MXC Bubblewrap guide says a denied directory is masked with `--tmpfs`, then
remounted read-only; the 1.0.1 command builder emits only `--tmpfs`, which is
writable. The mask is private and discarded, so this is **containment, not an
escape**, but it is not an explicit `EACCES` denial. Consequences:

- A Forge canary that only checks the syscall result would fail Ubuntu 24.04
  even though the host is safe.
- A Forge canary that checks a host sentinel after the run correctly passes.
- A sandboxed process can still fill that private tmpfs (memory/disk pressure);
  it cannot modify or read the host path.

Recommendation for TDD-0006: define the write canary as "the host fixture is
unchanged after the attempt", and record whether the syscall was denied or
masked. Prefer an allowlist-only policy for paths outside the grant; use
explicit `deniedPaths` only for sensitive paths nested inside a granted root,
where the tmpfs mask is actually needed. If the PRD's "denied" is interpreted
literally as `EACCES`, Ubuntu 24.04 is Guarded-only until MXC remounts the mask
read-only.

#### 3. Allowlist-only comparison

The workflow also runs the same fixture with `denied_paths: []`, relying on the
Bubblewrap default-deny baseline. With user namespaces permitted, **all six
canaries pass as true denials**:

```text
CANARY write-outside-worktree PASS .../outside: NotFound: No such file or directory (os error 2)
CANARY read-credential-path PASS .../credentials/id_rsa: NotFound: No such file or directory (os error 2)
CANARY unlisted-network-host PASS 1.1.1.1:443: NetworkUnreachable: Network is unreachable (os error 101)
CANARY child-process-inherits-denials PASS ... CHILD write DENIED NotFound ... | CHILD read DENIED NotFound ...
CANARY symlink-or-junction-escape PASS write through escape link denied (NotFound ...); read through escape link denied (NotFound ...)
CANARY secret-free-worker-environment PASS ...
host mutation check: sentinel_changed=false canary_write_created=false child_write_created=false
SPIKE RESULT: ALL SIX CANARIES DENIED
```

The paths are invisible because they were never granted, not because they were
masked. This is the posture Forge's default policy generator should use for
paths outside the task worktree. Explicit `deniedPaths` are still useful for a
sensitive path nested inside a granted root (for example, a credential file
under a read-only toolchain directory), but they should not be used to restate
the default-deny boundary.

## TDD-0006 inputs

- Use `mxc_sdk::v1::run` with `Containment::Process`; no separate MXC process or
  asset path is needed on any of the three platforms.
- Probe with a real short launch, not just `platform_support()`. On Ubuntu the
  latter returns supported while Bubblewrap cannot start under the stock
  AppArmor restriction.
- Include `bwrap` version (>= 0.5.0) and user-namespace/AppArmor readiness in
  `forge doctor`.
- Model the canaries as host-mutation checks with a post-run sentinel, and
  classify raw syscall outcomes separately as `denied`, `masked`, or `allowed`.
  The allowlist-only run on Ubuntu proves the clean path: all six are `denied`.
- Prefer allowlist-only for paths outside the worktree. Reserve explicit
  `deniedPaths` for sensitive paths nested inside a granted root; a redundant
  deny of an already-invisible directory becomes a writable tmpfs mask on
  Bubblewrap.
- Do not gate `deniedPaths` on `available_backends().capabilities`: Seatbelt and
  Bubblewrap report an empty capability list but support the field.
- The Windows `probe` result is the authoritative native-tier check; `tier` was
  `base-container` and no DACL augmentation was needed.
- `Sandboxed` on Ubuntu 24.04 requires the userns prerequisite; otherwise the
  honest mode is `Guarded — not sandboxed` (trusted repos only).

## Open questions / follow-ups

1. Does the PRD's canary 1/5 "denied" require `EACCES`, or is host
   non-mutation sufficient? This changes whether Ubuntu 24.04 can ship
   `Sandboxed` on a stock host.
2. Should Forge ship or document a scoped AppArmor profile for `bwrap`, or
   treat the stock restriction as a Guarded-only platform?
3. File an upstream MXC issue for the docs/code mismatch: denied directories
   are documented as read-only tmpfs masks but are emitted as writable tmpfs in
   1.0.1. The security impact is limited to in-sandbox resource use, but the
   mismatch breaks a literal denial canary.
4. Re-run the Windows leg on a 24H2 build if a 25H2 host is not accepted as
   representative; the PRD's floor is 26100.9278+ and the tested host is 26200.

## Reproduction

```text
Branch:  spike/mxc-containment (latest commit)
CI:      gh run view 41826409053 --repo Xaidel/forge
Logs:    gh run download 41826409053 --repo Xaidel/forge
```

The spike code is intentionally not merged. The branch is evidence only.
