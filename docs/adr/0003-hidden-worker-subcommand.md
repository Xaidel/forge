---
status: accepted
date: 2026-11-03
---

# Worker as a hidden `forge __worker` subcommand

## Context

Forge separates the trusted host from the untrusted worker. The worker executes shell commands, file edits, patches, tests, package installs, and any MCP, plugin, or hook processes inside MXC. The host owns the model client, the policy engine, gates, Git, the ledger, attestations, and merges. The worker is untrusted by construction; the host is the trusted computing base.

Forge ships as one standalone binary, `forge`, through `forge-harness`. The worker must run in a separate process so MXC can contain it, but a second binary target or a separate crate would complicate packaging, installation, and version alignment.

## Decision

The worker is the hidden `forge __worker` subcommand in the same binary.

- There is exactly one binary target: `forge`.
- `__worker` has two leading underscores, is marked hidden in the CLI parser, and is absent from `forge --help` and the public CLI contract.
- The host spawns `forge __worker` as a separate process inside MXC.
- There is no `forge-worker` binary, no worker-only package, and no worker-only crate.
- The worker may import `sandbox::protocol` (the IPC contract) and `app_core::ports`. It must not import host-only adapters: `cli`, `tui`, `wiring`, `model`, `git`, `ledger`, `session`, `policy`, `attest`, or `audit`.
- The worker and the host share the same binary but not the same trust boundary. The worker never writes host-owned paths such as `.git`, `.forge/attestations`, `.forge/trust`, `.forge/local/keys`, `.forge/local/sessions`, `.forge/decisions`, or `.forge/assumptions`.

## Consequences

- Installation and version alignment stay simple: one binary, one package, one version.
- The CLI parser must hide `__worker` and must not expose it in help, completions, or the public contract.
- The worker's tests are process-level, not library-level.
- `sandbox::protocol` is the single shared module between the sandbox adapter and the worker. It is a protocol contract, not adapter behavior.
- The architecture test asserts that `forge` is the only binary target and that `worker` does not import host adapters.
- A future need for a separate worker artifact requires a new ADR.

## Considered options

- **Separate `forge-worker` binary.** Cleaner process separation, but adds a second packaging artifact, a second version to align, and a second installation path.
- **Worker as a library entry point.** Avoids a hidden subcommand, but runs the worker in the host process and defeats MXC's whole-worker isolation.
- **Worker in a separate crate.** Enforces the boundary at compile time, but adds a third workspace crate and a third published package for no product benefit in v1.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the architecture, dependency, and naming rules](https://github.com/Xaidel/forge/issues/20)
- `docs/PRD.md` §6.2, §6.4, NFR-MAINT-001, D-12, D-13
- `arch_docs/architecture.md`, `arch_docs/dependency-rules.md`
