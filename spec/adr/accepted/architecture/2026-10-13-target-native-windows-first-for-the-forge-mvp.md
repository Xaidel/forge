# ADR: Target native Windows first for the Forge MVP

Type: project
Status: accepted
Classification: architecture

## Problem

Forge is a Rust + Ratatui TUI that drives Pi over `--mode rpc` as a managed child process, with a thin TypeScript gate extension as the enforcement actuator (ticket [Form: How Forge attaches to Pi](https://github.com/Xaidel/forge/issues/3)).
The MVP must run in the developer's environment — native Windows — and the wayfinder map leaves the first-release platform posture open.
Pi's Windows facts are concrete and easy to get wrong: the CLI entry is a JS bundle reached through an npm `.cmd` shim, the built-in `bash` tool depends on Git Bash (or an explicit `shellPath`), only SIGTERM is registered as a shutdown signal, and the `powershell` tool exists only on native Windows.
The form decision therefore needs a concrete spawn, process-control, and shutdown contract before environment work can start.

## Decision

The MVP targets native Windows first, with the core kept POSIX-portable.

- Forge spawns Pi as `node.exe <pi>/dist/bundle/cli.js --mode rpc ...`, never the npm `pi.cmd` shim directly.
- Shutdown is orderly by closing the child's stdin; SIGTERM is the only signal Pi registers on `win32`, so Forge must not rely on SIGHUP, POSIX process groups, or other POSIX-only mechanics.
- The `bash` tool requires Git Bash discovery or an explicit `shellPath` setting, and shell policy treats `bash` (Git Bash) and `powershell` as separate surfaces on Windows.
- Platform-specific process handling lives behind a small Windows seam in the Rust core, so a POSIX port is a new implementation of that seam, not a rewrite.
- MVP evidence and the demo run on native Windows; POSIX is not a supported or tested target for the MVP.

## Alternatives considered

**WSL-first**: run Forge under WSL so Pi sees a POSIX host. Rejected: it inserts a VM boundary between the TUI and the Pi child process, complicating process control and path handling, and moves the demo away from the platform the developer actually uses. It remains a plausible later target behind the same core.

**Cross-platform from the start**: keep Windows and POSIX on par from day one. Rejected: it doubles the spawn/process/signal test matrix and forces POSIX verification for an MVP whose point is the PRD §16 loop; the platform-specific facts above (`.cmd` shim, Git Bash, SIGTERM-only) make "works everywhere" unprovable without running everywhere.

## Consequences

- The demo runs in the developer's real environment with no virtualization layer between Forge and Pi, and the process contract is concrete: `node.exe` + JS bundle, stdin-close shutdown, SIGTERM-only.
- POSIX behavior is unverified in the MVP: a first POSIX port will surface surprises in shell discovery, signal handling, and process trees; test and demo evidence target Windows only.
- Test and tooling choices that assume POSIX shells are out; gate-extension tests, RPC protocol tests, and the sample-repo walkthrough must run on native Windows.
- The platform seam is the price of portability: process handling gains one indirection that Windows-only code would not need.
- Windows Terminal key limitations matter only if a human watches Pi's own interactive TUI; the RPC path needs no TTY.
- If the demo audience needs macOS or Linux, a POSIX effort moves up; the seam keeps that effort bounded.
- Confirmation: the environment stand-up ticket ([Task: Stand up Pi, a provider, and the sample repo](https://github.com/Xaidel/forge/issues/14)) is the first conformance evidence — pinned Pi v1.1.0 (`f1b2e77`) started via `node.exe` under RPC on Windows, accepting a prompt and streaming events.
