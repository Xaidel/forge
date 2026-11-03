---
status: accepted
date: 2026-11-03
---

# Executor-agnostic, I/O-free core

## Context

The core owns the hard, consequential behavior: the decision loop, gates, ledger rules, attestation, merge policy, and task state. That behavior must be testable without a running model, a sandbox, a Git repository, a clock, or a network. NFR-MAINT-001 fixes the core's baseline dependencies as `async-trait` and `thiserror` and forbids Tokio, Serde, Ratatui, reqwest, MXC, Git, environment, and provider types.

The core also needs IDs and timestamps, and its tests need to drive async ports. Those needs are easy to satisfy by pulling in `uuid`, `time`, and a test runtime, but that would make the core depend on the environment and on a specific executor.

## Decision

`app_core` is executor-agnostic and I/O-free:

- No `serde`, `uuid`, `chrono`/`time`, `rand`, `tokio`, `std::env`, `std::fs`, `std::net`, `std::process`, `SystemTime::now`, or `Instant::now`.
- Time and IDs arrive through `Clock` and `IdSource` ports.
- Serialization and external representation mapping live in `infra_local`; the core's records are plain Rust types.
- Normal dependencies are exactly `async-trait` and `thiserror`.
- `futures-core` is optional and feature-gated. It is added only if TDD-0002 proves the streaming model port needs a `Stream` return.
- The only dev-dependency is `pollster`, a minimal executor shim for async use-case tests. Tokio is forbidden even in dev-dependencies.
- Async ports use `async-trait`; they are object-safe, `Send`, and cancellation-safe. No lock is held across `.await`.

## Consequences

- The core can be tested deterministically with fake clocks, fake IDs, and fake ports.
- The runtime, serializer, time source, and ID source can change without touching the core.
- Adapters carry more mapping code: port records to domain records, domain records to port records, and port records to external DTOs.
- The core cannot use convenience derives from Serde or a UUID/time crate. Newtypes and conversions are written by hand.
- The architecture test enforces the dependency allowlist and the forbidden `std` paths.
- A future need for a new core dependency requires an ADR and an update to the architecture test.

## Considered options

- **Tokio in the core.** Simpler async tests and task spawning, but locks the core to a runtime and violates NFR-MAINT-001.
- **Serde derives on port records.** Less mapping code, but couples the public contract to a serialization format and pulls Serde into the core.
- **Allow `uuid` and `time` as pure value crates.** Convenient, but expands the audit surface and weakens the "baseline dependencies" rule.
- **Allow `anyhow` for core errors.** Less boilerplate, but erases the stable error categories the ledger and the CLI depend on.
- **Use a std-only `block_on` instead of `pollster`.** Avoids the dev-dependency, but adds a hand-written executor to the test surface and is harder to review.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the architecture, dependency, and naming rules](https://github.com/Xaidel/forge/issues/20)
- `docs/PRD.md` §6.5, NFR-MAINT-001, D-13
- `arch_docs/architecture.md`, `arch_docs/dependency-rules.md`
