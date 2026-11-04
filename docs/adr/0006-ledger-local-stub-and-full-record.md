---
status: accepted
date: 2026-11-04
---

# Local-only ledger records use a committed stub and a same-ID local full record

## Context

The PRD requires a `visibility: local` ledger record to keep its full content out of Git while still committing a pointer that teammates can read (FR-LEDGER-007). The pointer must carry only safe fields: schema version, ID, kind, title, status, dates, owner or approver, and supersession links. The full record stays under `.forge/local/`, which is gitignored.

A single Markdown file cannot satisfy both halves. If the full record is committed, the sensitive rationale leaks. If only the local file exists, a teammate who clones the repository sees no record at all and the plan, review, and attestation references dangle. A separate ID for the local copy would break every reference and duplicate-detection rule.

The ledger must also keep one logical record per ID for the index, cross-worktree duplicate checks, and `forge ledger verify`. The committed pointer and the local record are two physical artifacts of one logical decision.

## Decision

A local-only record is two files with the same ID:

- `.forge/decisions/<id>.md` or `.forge/assumptions/<id>.md` — the committed `detail: stub` pointer, carrying only the safe fields.
- `.forge/local/decisions/<id>.md` or `.forge/local/assumptions/<id>.md` — the full local record, `detail: full` for a decision and `detail: light` for an assumption, with `visibility: local`.

`LedgerPort::read_*` returns a `LedgerRecordView`: the local full/light record when it exists, otherwise the committed stub. The index has one logical entry per ID with a `committed_path` and an optional `local_path`. `safe_hash` compares the stub's safe fields with the local record's safe fields, so the pair is one record for duplicate detection and a genuine divergence is still reported.

For writes, the local file is written first and the committed stub second. A crash between them leaves an orphan local record, which is safe and retryable; it never leaves a committed stub that points at missing content on the machine that wrote it. A clone sees only the stub, which is a normal state, not an error.

## Consequences

- A teammate who clones the repository sees the decision ID, title, status, dates, actors, and supersession links, but not the rationale or context.
- `forge ledger verify` does not report a stub without a local record as an error; it reports a mismatch when both exist.
- A local file can be lost independently of the committed stub. The stub remains a valid pointer; the content is gone. Migration backs up local files before rewriting them.
- Duplicate detection uses `safe_hash`, not the raw file hash, so a full local record and its stub compare equal.
- The review packet can cite the record ID without exposing local content (FR-REV-001 E2).
- `visibility: local` requires `detail: full` for decisions and `detail: light` for assumptions; a light decision cannot be made local without upgrading its ceremony.
- The model-context builder must exclude local records unless the human opts in (FR-MODEL-008, TDD-0007).

## Considered options

- **One committed file with local-only fields omitted.** The local content has nowhere to live; either the rationale is lost or it is committed.
- **Encrypt the sensitive fields in the committed record.** Encryption, key management, and teammate access are out of scope for v1 and still leave ciphertext in Git.
- **Use a different ID for the local record.** Breaks plan, review, attestation, supersession, and index references; also makes the pair impossible to reconcile.
- **Keep only the local file and commit no pointer.** A clone loses the record entirely; review and attestation references cannot resolve.
- **Two files with the same ID, one stub and one local full record.** Chosen: preserves references, protects content, and keeps one logical record.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the decision ledger and record contract (TDD-0005)](https://github.com/Xaidel/forge/issues/22)
- `docs/PRD.md` §7.5 FR-LEDGER-002, FR-LEDGER-007; §9.1, §9.4; D-08, D-16, D-18
- [TDD-0005](../tdd/0005-decision-ledger-records.md) §4, §6.7, §7.6, §10.6, §12
