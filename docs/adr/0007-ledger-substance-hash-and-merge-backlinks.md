---
status: accepted
date: 2026-11-04
---

# Ledger backlinks are merge-time metadata excluded from the substance hash

## Context

The PRD makes the decision ledger append-only and makes the human's rationale, chosen option, decision date, owner, approver, and rationale provenance immutable (INV-15, FR-LEDGER-005). It also requires `forge merge` to write two backlinks into the ledger: `implemented_by: [<C>]` and `attestation: { id, commit: <C2> }` (FR-MERGE-010). The attestation binds the ledger record IDs and their substantive content hashes (FR-MERGE-004, FR-REV-010).

These requirements pull against each other. The attestation is created before `C2`, and `C2` must contain only the attestation record. If `forge merge` rewrites ledger files after `C2` to add backlinks, a hash over the whole file would change on every legal merge and invalidate the attestation it just recorded. If the hash covers too little, a later writer could change the rationale, chosen option, or risk fields without detection.

The prior ledger resolution said `implemented_by` was filled at attestation. That is circular: the attestation record would have to bind a ledger file that contains a backlink to an attestation commit that does not exist yet, and `C2` cannot carry the ledger change. The merge commit is the first point where both `C` and `C2` exist.

## Decision

`forge merge` writes the backlinks into the affected ledger records in the merge commit, not in `C` or `C2`:

- A normal merge writes `implemented_by: [C]` and `attestation: { id, commit: C2 }`.
- An override merge may write `implemented_by` without `attestation`; the task remains `stopped`.
- `C2` remains attestation-only.
- The merge commit is the durable backlink record. `forge ledger verify` can rebuild the backlinks from the `Forge-Decisions` trailer and the merge-commit attestation trailers.

The attestation binds `substance_hash`, a SHA-256 over a canonical JSON projection of the record. The projection excludes fields that are lifecycle or link metadata rather than the decision's content. The PRD's immutable-fields table names the rationale, chosen option, decision date, owner, approver, and rationale provenance; `status` and `status_reason` are lifecycle metadata and are excluded so `forge ledger verify` can repair a supersession without changing substance.

```text
schema_version
updated_at
status
status_reason
superseded_by
implemented_by
attestation
adr
```

Everything else is substance, including `revisit_if`, `errata`, `notes`, `refs`, `policy_triggers`, `visibility`, and `detail`. `content_hash` remains the SHA-256 of the file bytes for the index and file-integrity checks. `safe_hash` covers only the stub's safe fields for cross-worktree identity.

Migrations must preserve `substance_hash`. A migration that would change it is not a migration; it is a decision and must go through the ledger.

## Consequences

- A legal merge backlink rewrite does not change `substance_hash`, so it cannot invalidate the attestation.
- A later supersession changes `status`, `superseded_by`, and `updated_at`; all three are excluded, so `substance_hash` stays stable. The historical attestation binds the record at `C`, so a later lifecycle change does not retroactively invalidate it. `forge ledger verify` compares a current record to an attestation only when it is checking the attested commit.
- An erratum is substance and changes `substance_hash`; the attestation remains a historical record and is not retroactively invalidated.
- `forge merge` must verify that each record's `substance_hash` is unchanged before it commits the backlinks. A mismatch aborts the merge.
- Adding a new ledger field requires deciding whether it is substance or metadata. The default is substance; excluding a field requires a schema change and an amendment.
- The merge commit is slightly larger: it carries the ledger backlink updates in addition to the merged code and tests. `C2` stays small and attestation-only.
- The attestation record binds `substance_hash` in addition to the ledger record ID; TDD-0008 owns the exact attestation JSON.

## Considered options

- **Write backlinks at attestation time.** Circular: the attestation commit does not exist yet, and `C2` may contain only the attestation record.
- **Put backlinks only in merge-commit trailers and never in the ledger records.** Simpler hashing, but FR-MERGE-010 names the ledger fields and the review/audit surfaces need them in the record.
- **Hash the whole file.** Any merge-time backlink write changes the hash and invalidates the attestation.
- **Hash only the rationale.** Too little: the chosen option, risk fields, refs, and policy provenance would be unprotected.
- **Store backlinks in a separate sidecar file.** Adds a third artifact per record and a new consistency problem; the ledger already has a durable home for them.
- **Canonical substance projection with a fixed metadata exclusion set.** Chosen: preserves immutability where it matters and permits the backlinks the PRD requires.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the decision ledger and record contract (TDD-0005)](https://github.com/Xaidel/forge/issues/22)
- `docs/PRD.md` §5.3 INV-15; §7.5 FR-LEDGER-005, FR-LEDGER-006; §7.7 FR-MERGE-004, FR-MERGE-010; §9.1–9.2; D-08
- [TDD-0005](../tdd/0005-decision-ledger-records.md) §10, §13
- [TDD-0008](https://github.com/Xaidel/forge/issues/32) — merge mechanics, attestation JSON, and trailers
