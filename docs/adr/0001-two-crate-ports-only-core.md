---
status: accepted
date: 2026-11-03
---

# Two-crate workspace with a ports-only public core

## Context

Forge v1 is a Rust 1.93, edition 2024, local-first coding-agent harness. NFR-MAINT-001 requires exactly two crates, `app_core` and `infra_local`, with only `app_core::ports` public and no workspace dependency from the core. D-13 fixes `forge-harness` as the published binary crate and `forge` as the binary.

The published binary must be installable with `cargo install forge-harness`. Cargo will not publish a package that depends on an unpublished path dependency, so the core must also be published. The names `app-core` and `forge-core` are already taken on crates.io; `forge-harness-core` and `forge-harness` are available.

## Decision

The workspace has exactly two members:

- `crates/app_core` — package `forge-harness-core`, library crate `app_core`, published. Only `app_core::ports` is public. `domain` and `application` are private. `assembly` is private and re-exported only through `ports`.
- `crates/infra_local` — package `forge-harness`, binary crate `forge`, published. There is no library target, so the infrastructure has no second public API. `infra_local` is the source directory and the name of the infrastructure layer.

The two packages share one version and are published core-first. The harness depends on the core with a path plus the exact shared version.

`ports` owns every public record and does not depend on `domain`. `application` maps between private domain types and port records. `domain` depends on nothing in the workspace. The public traits, records, error categories, `ApplicationPorts`, and `build_application` live in, or are re-exported from, `app_core::ports`.

## Consequences

- `cargo install forge-harness` resolves `forge-harness-core` from crates.io.
- The core is a published package even though its API is internal and unstable outside Forge.
- The infrastructure has no library target; adapter tests are unit tests inside the binary crate, and end-to-end tests are process-level.
- Mapping between domain and port records is explicit and lives in `application`.
- An architecture test enforces the two-member workspace, the package/target names, the public surface, and the adapter isolation.
- Adding a third workspace crate requires a demonstrated need and a new ADR.

## Considered options

- **Single crate with modules.** Simplest, but violates NFR-MAINT-001 and loses the compile-time boundary between domain and infrastructure.
- **Publish only `forge-harness` and inline the core.** Preserves one published package, but breaks the two-crate source layout and makes the core impossible to test in isolation.
- **Keep the core unpublished and distribute with `cargo install --git`.** Avoids publishing the core, but weakens NFR-DIST-001's `cargo install forge-harness` path.
- **Add an `infra_local` library target.** Makes adapter integration tests easier, but creates a second public surface and contradicts the PRD's "binary crate" description.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the architecture, dependency, and naming rules](https://github.com/Xaidel/forge/issues/20)
- `docs/PRD.md` §6.5, NFR-MAINT-001, NFR-DIST-001, D-13
- `arch_docs/architecture.md`, `arch_docs/dependency-rules.md`
