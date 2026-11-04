# MXC native-containment spike

Throwaway evidence harness for wayfinder ticket
[#23](https://github.com/Xaidel/forge/issues/23). It answers one question: does
`mxc-sdk` 1.0.1's native containment tier enforce the six PRD canaries on
macOS 15+, Windows 11 24H2+ x64, and Ubuntu 24.04 LTS?

This code is evidence, not a Forge adapter. Do not merge it. The resolution is
`EVIDENCE.md` on this branch.

## What it does

`mxc-spike` builds a disposable layout:

```
mxc-spike-run/
  worktree/          read-write grant; cwd; contains the canary binary and tmp/
    escape-outside/  host-created junction/symlink to outside/
    escape-credentials/ host-created junction/symlink to credentials/
  outside/           denied; the host proves it is writable before launch
  credentials/       denied; contains a fake private key
```

It launches `canary` through `mxc_sdk::v1::run` with `Containment::Process`
(native default: ProcessContainer on Windows, Seatbelt on macOS, Bubblewrap on
Linux), a single read-write grant, explicit denied paths, deny-all directional
network policy, UI disabled, and an explicit environment allowlist.

The canary attempts:

1. write outside the task worktree;
2. read a credential path;
3. connect to an unlisted network host (`1.1.1.1:443`);
4. spawn a child process that repeats (1) and (2);
5. write and read through a host-created symlink/junction escape;
6. find model keys, cloud secrets, or the host marker
   `FORGE_SPIKE_HOST_SECRET` in its environment.

## Build and run

```sh
cargo build --release --bins
FORGE_SPIKE_HOST_SECRET=leak-me ./target/release/mxc-spike
```

On Windows PowerShell:

```powershell
cargo build --release --bins
$env:FORGE_SPIKE_HOST_SECRET = 'leak-me'
.\target\release\mxc-spike.exe
```

Exit codes: `0` all six pass, `2` a canary failed or was inconclusive, `3` MXC
reports the host unsupported.

## CI

`.github/workflows/mxc-spike.yml` runs the macOS 15 arm64, macOS 15 x64, and
Ubuntu 24.04 x64 legs. The Windows 11 25H2 x64 result is from physical hardware
because hosted Windows runners are Server SKUs, not the PRD's client floor.
