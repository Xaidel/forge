---
status: accepted
date: 2026-11-05
---

# Model cost is estimated in the core from a config-carried price table

## Context

Forge must track provider-reported token usage per request, estimate USD from a versioned built-in price table with per-profile and model overrides, label the result as an estimate, and pause at a local hard budget (FR-MODEL-004, FR-MODEL-005, NFR-COST-001; D-16). TDD-0002 already gives the core `Usage`, `Budget`, `UsdMicros`, and a task aggregate, but it deliberately adds no cost type. TDD-0001 fixes `.forge/config` without price keys.

Three placements are possible: the adapter computes USD and returns it; the core computes USD from a table carried through `ConfigPort`; or a new `PriceTablePort` supplies the table. The choice determines whether fakes, the TUI, and the budget guardrail share one cost definition or several.

## Decision

Cost is a core concern. The built-in catalog and TOML overrides are loaded by `infra_local`, but the effective table is a core value object carried in `ConfigSnapshot`. A pure core function estimates cost from normalized `Usage`, the effective model binding, and the table; the application loop records the resulting `CostRecord`; `ModelPort` returns only normalized `Usage`.

- The price table is a top-level `[prices]` table in `.forge/config`, with complete-row `[[prices.models]]` entries and `[[prices.profile_models]]` overrides. Precedence is profile+model > model > built-in; a missing row means USD `unknown`, never zero.
- `CostRecord.usd` is `Option<UsdMicros>`. Unknown price and local-model cost are `None`, not zero. Estimated usage is labeled separately from provider-reported usage.
- When a provider reports the request cost (OpenCode Go reports `usage.cost` and `usage.opencode.list_cost`), that value is carried in `Usage.reported_cost` and is authoritative for the request. The price-table estimate remains available as a cross-check. The one exception is a profile with `locality = "local"`: FR-MODEL-004 E2 wins, the task shows tokens only, and a reported cost is retained for diagnostics but never used as USD or for the budget.
- Cache read, cache write, output, and reasoning tokens are priced once. Reasoning tokens are a subset of output unless a profile explicitly declares a separate reasoning rate.
- The budget guardrail reads `CostRecord.usd` and the task token aggregate. It runs after each request and before the next; it never interrupts an in-flight request.

## Consequences

- Fakes and the application use the same cost function as production; no adapter can invent a cost.
- A new provider does not need a new cost path; it needs profile data and a price-table row or override.
- The core stays free of Serde, `reqwest`, and provider types; it receives plain records through `ConfigPort`.
- Adding `prices` to `ConfigSnapshot` and `CostRecord`/`TaskCost` to the core amends TDD-0002; adding `[prices]` and token budgets to config amends TDD-0001. Both amendments are recorded in TDD-0007.
- An unknown price makes the USD total a lower bound, not a complete bill. The known lower bound can still pause at the hard threshold; the UI labels the total `incomplete` and shows the unknown count. Token thresholds remain the honest guardrail for local models, and USD is never shown as zero.
- A provider-reported cost can differ from the estimate; the UI shows the source so the developer is never told an estimate is a bill.

## Considered options

- **Adapter computes USD and returns it in `ModelResponse`.** Rejected: it duplicates budget math in every adapter, makes fakes inconsistent, and lets provider-specific pricing leak into the transport layer.
- **New `PriceTablePort`.** Rejected: one more outbound port for a small, read-only value that already belongs to the config snapshot; `ConfigPort` is the only port that reads `.forge/config`.
- **Estimate only in the TUI.** Rejected: the budget pause is a task-state decision, not a rendering decision; the core must own it.
- **Treat an unknown price as zero.** Rejected by FR-MODEL-004 E1 and INV-19.
- **Core value object, config-carried table, pure core cost function.** Chosen: one definition, testable without I/O, and consistent across fakes, TUI, and budget.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the model adapters, provider profiles, and cost contract (TDD-0007)](https://github.com/Xaidel/forge/issues/27)
- `docs/PRD.md` §7.9 FR-MODEL-004, FR-MODEL-005; NFR-COST-001; D-16
- [TDD-0001](../tdd/0001-cli-config-init-repo-map.md) §6.12, §6.13
- [TDD-0002](../tdd/0002-domain-model-ports-assembly.md) §6.3, §6.10, §7.2, §7.4
- [TDD-0007](../tdd/0007-model-adapters-providers-cost.md) §10–§11, §17
