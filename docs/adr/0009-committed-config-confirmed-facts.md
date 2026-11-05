---
status: accepted
date: 2026-11-05
---

# Committed config stores confirmed repo-map facts only

## Context

`.forge/config` is the one committed repository map (D-19, D-20; PRD §8.2 and §9). Init scans deterministically, but the scan only proposes: every entry point, module root, package root, doc, and check is a candidate until the human accepts, edits, rejects, or marks it unknown (FR-ONB-002, FR-ONB-003, FR-ONB-010, FR-ONB-011).

Two different artifacts therefore hold two different kinds of truth:

- the local `.forge/local/init-report.json` holds raw candidates, evidence, confidence, and warnings;
- the committed `.forge/config` holds only the facts the human confirmed.

The review packet also needs a repo-map version and hash (FR-REV-004), and a rewrite must not discard fields a newer Forge added (I-CONFIG). A design that stores proposals in the committed file, or recomputes a hash from a pretty-printed file, would make confirmation and drift ambiguous.

## Decision

`.forge/config` is a single committed TOML file whose repo-map sections contain **confirmed facts only**.

- The five facets are `checks`, `entry_points`, `module_roots`, `docs`, and `package_roots`.
- Each facet carries `status = "confirmed" | "unknown"` and `confirmed_absence`. `proposed` is a report-only status and is invalid in the committed file.
- A confirmed facet with no entries is valid only when `confirmed_absence = true`.
- `[map] version` is a monotonically increasing revision counter. It increments only when the confirmed map content changes. It never increments for a formatting-only rewrite, a comment change, or a re-init with no map change.
- The repo-map hash is computed at load time from the canonical confirmed map content. It is not stored in `.forge/config`. `[map] version` covers revision identity; the hash covers content identity.
- `[[checks]]` keep stable integer IDs. A removed check's ID is reserved and never reused.
- Unknown fields and comments survive a rewrite through `toml_edit`. Forge inserts or updates only the fields it owns.
- A newer `schema_version` is read-only. A missing required field is an error. A `proposed` status in the committed file is an error.
- `.forge/local/config.local.toml` is a sparse machine-local patch. It may override profiles, slots, routing, and budgets. It may not override the map, checks, context, verification, or the base ref.

## Consequences

- The committed file is reviewable: every entry is a fact the human accepted, not a scan dump.
- Re-init diffs are meaningful: current facts versus new candidates, with proposals staying local.
- The review packet can show a stable version and a content hash without a self-referential file.
- A formatting-only rewrite cannot masquerade as a map change; a real map change always increments the version.
- The config adapter needs two parsers: a typed reader for validation and a document editor for preservation. This is more code than a single `serde` round-trip but keeps comments and unknown fields.
- A human who wants to keep a proposal must confirm it; the proposal is never silently promoted.

## Considered options

- **Store proposals in `.forge/config` with a status field.** Rejected: the committed map would mix evidence with confirmed facts, and every reader would need to filter proposals. It also contradicts the PRD rule that raw confidence stays local.
- **Split the repo map into a separate `.forge/map` file.** Rejected: the PRD names `.forge/config` as the one committed config and repo-map record; a second file adds a second writer and a synchronization problem.
- **Use JSON or SQLite.** Rejected: the file must be human-editable and diff-reviewable; the PRD fixes TOML.
- **Store the map hash in the file.** Rejected: a content hash inside the content it hashes is self-referential. The version covers revision identity; the hash is derived.
- **Rewrite with a plain `serde` round-trip.** Rejected: it loses comments and unknown fields, violating I-CONFIG's preservation rule.
- **Confirmed-facts-only TOML, version plus computed hash, `toml_edit` preservation.** Chosen: reviewable, deterministic, and forward-tolerant.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the CLI, configuration, init, and repo map contract (TDD-0001)](https://github.com/Xaidel/forge/issues/26)
- `docs/PRD.md` §7.1 FR-ONB-002, FR-ONB-003, FR-ONB-010, FR-ONB-011, FR-ONB-017; §8.2; §9; D-19, D-20
- [TDD-0001](../tdd/0001-cli-config-init-repo-map.md) §6–§7
- [TDD-0002](../tdd/0002-domain-model-ports-assembly.md) §6.9
