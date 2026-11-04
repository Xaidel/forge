# TDD-0005 — Decision Ledger and Records

**Status:** Approved
**Date:** 2026-11-04
**Owner:** Forge architecture
**Sources:** `docs/PRD.md` §5.1, §5.3–5.4, §7.5, §7.7, §8.1, §9, §10.10; D-08, D-16, D-18
**Companions:** [TDD-0002](0002-domain-model-ports-assembly.md), [architecture.md](../../arch_docs/architecture.md), [dependency-rules.md](../../arch_docs/dependency-rules.md), [naming-conventions.md](../../arch_docs/naming-conventions.md)
**ADRs:** [ADR-0006](../adr/0006-ledger-local-stub-and-full-record.md), [ADR-0007](../adr/0007-ledger-substance-hash-and-merge-backlinks.md), [ADR-0008](../adr/0008-tagged-ledger-references.md)
**Ticket:** [Define the decision ledger and record contract (TDD-0005)](https://github.com/Xaidel/forge/issues/22)
**Map:** [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
**Amends:** [TDD-0002](0002-domain-model-ports-assembly.md) (see §17)

---

## 1. Purpose and authority

This document fixes the exact decision and assumption ledger contract for Forge v1:

- the Git-native file model under `.forge/decisions/`, `.forge/assumptions/`, and `.forge/local/`;
- the YAML front matter and Markdown body schemas;
- the `full`, `light`, and `stub` detail levels and their required fields;
- the identity, actor, timestamp, typed-reference, supersession, errata, visibility, and secret-scan rules;
- the canonical serialization, unknown-field preservation, and content/substance hashing rules;
- the `.forge/local/index.json` schema and rebuild behavior;
- the `Forge-Decisions:` commit-trailer grammar and the `implemented_by` / `attestation` backlink contract;
- `forge ledger verify` and `forge ledger migrate`;
- the validation issue taxonomy, fixtures, and requirement-to-test mapping.

`docs/PRD.md` is authoritative for product behavior. `arch_docs/architecture.md`, `arch_docs/dependency-rules.md`, and `arch_docs/naming-conventions.md` are authoritative for boundaries, dependency direction, and names. TDD-0002 is authoritative for the public Rust records and ports; this document amends it where §17 says so. When this document conflicts with a higher authority, the higher authority wins and this document is corrected.

---

## 2. Scope

### 2.1 In scope

- Decision and assumption record files and their local-only counterparts.
- Exact YAML keys, order, types, requiredness, and Markdown body grammar.
- ID generation, slug normalization, collision handling, and duplicate reporting.
- Typed references and their per-kind validation.
- Same-kind and cross-kind supersession, errata, and immutable rationale.
- Canonical YAML, unknown-field preservation, line endings, and body normalization.
- `content_hash`, `safe_hash`, and `substance_hash`.
- Ledger index JSON, rebuild, invalidation, and cross-worktree duplicate checks.
- Commit-trailer parsing, `implemented_by` rebuilding, and merge-time backlinks.
- Secret-scan invocation in the ledger write path.
- `forge ledger verify` and `forge ledger migrate` behavior.
- Public type and port amendments to TDD-0002.
- Fixtures and the requirement-to-test mapping for FR-LEDGER-001..011.

### 2.2 Out of scope

- CLI flags, help text, and exit-code mapping — TDD-0001.
- Session state, pending gates, and deferred decisions in session storage — TDD-0003.
- Policy trigger IDs, policy evaluation, and the decision-map rules — TDD-0004.
- Git adapter mechanics, merge trailers, attestation JSON, signing, hooks, and override mechanics — TDD-0008.
- TUI rendering, terminal sanitization, and review presentation — TDD-0009 and TDD-0010.
- Secret patterns, entropy rules, and the human override flow — TDD-0010.
- Audit-log encoding and dogfood metrics — TDD-0011.
- Implementation code, workspace scaffold, tests, and release artifacts. This document is a contract, not a delivery.

---

## 3. Constraints and dependencies

- Rust 1.93, edition 2024.
- The core remains I/O-free and Serde-free. YAML/JSON parsing, canonical serialization, hashing, and secret scanning live in `infra_local` adapters; the core exchanges plain records.
- `LedgerPort` is the only port that writes `.forge/decisions/`, `.forge/assumptions/`, `.forge/local/decisions/`, `.forge/local/assumptions/`, and `.forge/local/index.json`.
- The host is the only writer. The worker never writes ledger or control paths (INV-05, FR-EXEC-004).
- One ledger per repository. Packages use typed refs or path rules, never per-package records (FR-LEDGER-011, NFR-SCALE-002).
- The active-task lock serializes task writes. `forge ledger migrate` refuses a live lock; `forge ledger verify` runs read-only when a live lock exists.
- `ContentHash` is 64 lowercase hexadecimal characters and names a SHA-256 digest (TDD-0002 §5.2). The `sha256:` prefix used in prose is notation, not stored.
- `Timestamp` is `i64` Unix milliseconds UTC. Record files serialize it as RFC 3339 UTC at second precision.

---

## 4. File model and lifecycle

### 4.1 Paths

| Artifact | Path | Committed |
|---|---|---|
| Decision record | `.forge/decisions/<decision-id>.md` | Yes |
| Assumption record | `.forge/assumptions/<assumption-id>.md` | Yes |
| Local decision record | `.forge/local/decisions/<decision-id>.md` | No |
| Local assumption record | `.forge/local/assumptions/<assumption-id>.md` | No |
| Ledger index | `.forge/local/index.json` | No |
| Migration backup | `.forge/local/migrations/<timestamp>/...` | No |

- The file name is the record ID plus `.md`. No other extension, suffix, or directory nesting is allowed.
- The committed and local forms of a local-only record share the same ID.
- `.forge/local/` is gitignored in full. TDD-0001 owns the `.forge/.gitignore` content.
- A record file is UTF-8 without BOM, uses LF line endings, and ends with exactly one trailing newline.

### 4.2 Detail and visibility

- `detail` is `full`, `light`, or `stub`.
- `visibility` is `repo` or `local`; it is absent on every `stub`.
- A decision with `visibility: local` is always `detail: full`. A local decision must use the full ceremony.
- An assumption with `visibility: local` is always `detail: light`; assumptions have no `full` level.
- A `detail: stub` record is the committed pointer to a local full or local light record. It carries only the safe fields in §6.7 and §7.6.
- A repo record is `detail: full` or `detail: light` and has `visibility: repo`.
- A local record's full content is never committed. Only its stub is.

### 4.3 Write order and crash safety

`write_decision` and `write_assumption` are single logical writes even when they produce two files:

1. Build the full record.
2. Validate it and run the secret scan.
3. For `visibility: local`, write the local full/light record first to a temp file, fsync, and rename it into place.
4. Write the committed record or stub second to a temp file, fsync, and rename it into place.
5. Invalidate `.forge/local/index.json` by removing it. A missing index is rebuilt on demand.

If step 4 fails, the local record remains and the logical write fails. Retrying the same ID overwrites the orphan local record. If step 3 fails, the committed file is untouched. A committed stub with no local full record is a normal state on a clone and is not repaired by creating empty content.

### 4.4 Read model

`LedgerPort::read_decision` and `read_assumption` return a view:

- a local full/light record, when it exists in the current worktree;
- otherwise the committed record, which may be `full`, `light`, or `stub`;
- otherwise `None`.

A committed stub and its local full record are one logical record. Duplicate detection and the index use the ID, not the physical path. A local full record with the same ID as a committed `detail: full` or `detail: light` record is a conflict (`local_repo_conflict`); the local form is only legal when the committed form is a stub.

### 4.5 Commit batch

- The host writes records to the working tree immediately.
- The task's final commit carries the batch with the code and tests; the host creates the commit.
- A pending decision request is never written to the ledger (INV-03, FR-LEDGER-010).
- A cancelled task retains its partial records on the branch but does not merge them.
- The final commit's `Forge-Decisions:` trailer lists the decision IDs in the batch. Assumptions are in the batch but not in the trailer.

---

## 5. Identity and actors

### 5.1 Record IDs

```text
DecisionId  = "DR-" MM "-" DD "-" YY "-" slug
AssumptionId = "AS-" MM "-" DD "-" YY "-" slug
slug         = [a-z0-9] [a-z0-9-]{0,63}
```

- `MM` is 01–12, `DD` is 01–31, `YY` is 00–99.
- The date is the UTC date of `decided_at` for a decision and `created_at` for an assumption.
- The regex is the TDD-0002 grammar: `^DR-[0-9]{2}-[0-9]{2}-[0-9]{2}-[a-z0-9][a-z0-9-]{0,63}$` and the `AS-` equivalent.
- The slug is generated once from the title and never changes when the title changes.
- The ID is immutable after the first write. Renaming a title, correcting a typo, or superseding a record never changes an ID.

### 5.2 Slug generation

1. Normalize the title with Unicode NFKD and remove combining marks.
2. Lowercase ASCII `A-Z`.
3. Replace every run of characters outside `[a-z0-9]` with `-`.
4. Trim leading and trailing `-`.
5. Collapse repeated `-`.
6. Truncate to 64 characters at the last `-` boundary; if there is no boundary, truncate to 64.
7. If the result is empty, use `decision` or `assumption`.

The host uses `IdSourcePort::next_decision_id(title, at)` and `next_assumption_id(title, at)`; the adapter applies the same rules and checks the current ledger for collisions. The exact allocation API is TDD-0002; the grammar and normalization are fixed here.

### 5.3 Collision handling

- Within one worktree's ledger, a duplicate ID is never silently reused. The next free suffix is `-2`, `-3`, and so on; the base slug is shortened so the total slug stays at most 64 characters.
- A duplicate ID across worktrees is reported by `forge ledger verify`. It is not auto-renamed.
- Two physical records with the same ID are a true collision only when their `safe_hash` differs (§10.6). Identical copies of the same record are not duplicates.
- Date-based IDs are not globally unique; the one-open-task-branch rule keeps the normal path linear, and `verify` reports the edge case (FR-LEDGER-005, FR-LEDGER-009).

### 5.4 Timestamps

All timestamps are RFC 3339 UTC at second precision, for example `2026-11-04T10:04:12Z`. `Timestamp` is `i64` Unix milliseconds internally.

| Field | Meaning |
|---|---|
| `created_at` | When the host created the record object. |
| `decided_at` | When the human's decision answer was accepted. Decisions only. |
| `updated_at` | When the record was last rewritten, including errata and backlink repairs. |
| `rationale_provenance.captured_at` | When the human rationale was captured. |
| `reviewed_at` | When a human review was recorded. Assumptions only. |
| `Erratum.date` | The date in the errata heading, parsed as midnight UTC. |

Rules:

- `created_at <= decided_at <= updated_at`.
- `created_at <= updated_at` for assumptions.
- `reviewed_at` is present exactly when `reviewed_by` is present, and `reviewed_at <= updated_at`.
- The ID date is derived from `decided_at` (decision) or `created_at` (assumption), not from `updated_at`.
- A missing or malformed timestamp is an error, not a default.

### 5.5 Human identity and reserved aliases

- `owner`, `approver`, `rationale_provenance.author`, and `reviewed_by` are human display names.
- Every human field is filled from `git config user.name` at write time. There is no configured-identity fallback and no OS-username fallback.
- If `git config user.name` is missing or empty, the write fails with `missing_human_identity`.
- `owner` and `approver` are both the same human in v1; the schema keeps them distinct for later use.
- `proposer` is `agent` or `human`. `logger` is `agent` or `human`.
- The values `agent`, `assistant`, `forge`, `model`, and `system` (case-insensitive, trimmed) are forbidden in every human field. Violation is `agent_identity_forbidden`.
- The agent never drafts, prefills, paraphrases, or generates a rationale (INV-04, FR-LEDGER-003). The host captures the human text verbatim.

---

## 6. Decision records

### 6.1 Front matter order and types

The front matter starts with `---` on line 1 and ends with `---` followed by one blank line and the body. Keys are emitted in this order. Optional absent keys are omitted; the writer does not emit `null`.

```yaml
---
schema_version: 1
id: DR-11-04-26-use-postgres
kind: decision
detail: full
title: Use PostgreSQL for the durable ledger index
status: accepted
created_at: 2026-11-04T10:00:00Z
decided_at: 2026-11-04T10:04:12Z
updated_at: 2026-11-04T10:04:12Z
owner: Karl Abechuela
approver: Karl Abechuela
proposer: agent
rationale_provenance:
  author: Karl Abechuela
  method: typed
  captured_at: 2026-11-04T10:04:12Z
reversibility: hard
blast_radius: system
task: T-042
refs:
  - kind: commit
    sha: 0123456789abcdef0123456789abcdef01234567
    path: crates/infra_local/src/ledger.rs
  - kind: issue
    url: https://github.com/Xaidel/forge/issues/22
    title: Define the decision ledger and record contract (TDD-0005)
chosen_option: postgres
policy:
  version: 7
  hash: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
policy_triggers:
  - schema-migration
supersedes: []
visibility: repo
---
```

### 6.2 Field table

`R` = required, `O` = optional, `—` = absent or forbidden.

| Field | Type | full | light | stub |
|---|---|:--:|:--:|:--:|
| `schema_version` | integer `1` | R | R | R |
| `id` | `DecisionId` | R | R | R |
| `kind` | `decision` | R | R | R |
| `detail` | `full` \| `light` \| `stub` | R | R | R |
| `title` | single-line string, 1–200 chars | R | R | R |
| `status` | decision status | R | R | R |
| `created_at` | RFC 3339 UTC | R | R | R |
| `decided_at` | RFC 3339 UTC | R | R | R |
| `updated_at` | RFC 3339 UTC | R | R | R |
| `owner` | human name | R | R | R |
| `approver` | human name | R | R | R |
| `proposer` | `agent` \| `human` | R | R | — |
| `rationale_provenance` | map | R | R | — |
| `reversibility` | `easy` \| `hard` | R | R | — |
| `blast_radius` | `local` \| `module` \| `system` \| `external` | R | R | — |
| `task` | `TaskId` | R | R | — |
| `refs` | list of typed refs | O | O | — |
| `chosen_option` | option label | R | O | — |
| `policy` | `{version, hash}` | R | R if trigger fired | — |
| `policy_triggers` | list of trigger IDs | O | O | — |
| `adr` | repo-relative path | O | — | — |
| `attestation` | `{id, commit}` | O | O | — |
| `status_reason` | single-line string, 1–500 chars | O | O | — |
| `revisit_if` | single-line string, 1–500 chars | O | O | — |
| `supersedes` | list of `RecordId` | O | O | O |
| `superseded_by` | `RecordId` | O | O | O |
| `implemented_by` | list of `CommitSha` | O | O | — |
| `visibility` | `repo` \| `local` | R | R | — |

`rationale_provenance`:

```yaml
rationale_provenance:
  author: <human name>
  method: typed | dictated | imported
  captured_at: <RFC 3339 UTC>
```

`policy`:

```yaml
policy:
  version: <u32>
  hash: <ContentHash>
```

`attestation`:

```yaml
attestation:
  id: AT-T-042-v1
  commit: <CommitSha>
```

### 6.3 Detail-level rules

A record is `full` when any of these is true:

- `policy_triggers` is non-empty;
- `reversibility` is `hard`;
- `blast_radius` is `system` or `external`.

Otherwise `light` is allowed. A `light` record whose `policy_triggers` is non-empty is an error; the host must upgrade it to `full`.

A `full` record requires:

- `policy` (version and hash) even when no trigger fired;
- `chosen_option`;
- body sections `## Decision`, `## Rationale`, `## Context`, `## Options considered`, and `## Consequences`.

A `light` record requires:

- body sections `## Decision` and `## Rationale`;
- `policy` exactly when `policy_triggers` is non-empty.

A `stub` record:

- carries only the safe fields in §6.7;
- has no body;
- is valid only as the committed pointer to a local full record. A stub with `visibility` or any substantive field is invalid.

A `visibility: local` decision must be `detail: full`; a local `light` decision is `local_requires_full`.

### 6.4 Status and lifecycle

Status values: `accepted`, `rejected`, `deferred`, `superseded`, `deprecated`.

| Status | Created by v1 gate flow | Required | Forbidden |
|---|---|---|---|
| `accepted` | Yes, by the gate flow | `chosen_option` per detail; `implemented_by` / `attestation` only after merge | `superseded_by`; `revisit_if` |
| `superseded` | No; lifecycle only | `superseded_by` | `revisit_if` |
| `deprecated` | No; lifecycle only | `status_reason` | `superseded_by`; `revisit_if` |
| `rejected` | No; read-only | — | `superseded_by`; `attestation`; `implemented_by` |
| `deferred` | No; read-only | `revisit_if` | `superseded_by`; `attestation`; `implemented_by` |

Rules:

- The gate flow creates only `accepted` records. A normal `reject` answer returns to planning and writes no record (FR-GATE-002). A deferred decision stays in session state (FR-LEDGER-010); the ledger's `deferred` status exists for read/import compatibility and is not written by v1.
- `superseded` is applied when a later record's `supersedes` names this record. `superseded_by` is the forward-link backlink; `forge ledger verify` repairs it.
- `deprecated` is a lifecycle status for imported or forward-compatible records. V1 has no command that creates it; a live record becomes `superseded`, not `deprecated`. When present, it requires a one-line reason.
- `attestation` is present only after a normal attested merge. `implemented_by` is present after a normal merge or an override merge. `attestation` requires `implemented_by`; `implemented_by` may exist without `attestation` for an override.
- Both `attestation` and `implemented_by` are forbidden on `rejected` and `deferred`.
- A record's status is never silently changed back to `accepted`.
- `LedgerPort::write_decision` and `write_assumption` are upserts used by the gate flow, merge, and verify. They accept every valid status for an existing record; the application restricts new gate-created records to `accepted`. `status_not_writable` is an application-level guard, not a port parse error.

### 6.5 Body grammar

```markdown
## Decision

<required non-stub: one or more paragraphs>

## Rationale

<required non-stub: the human's verbatim words; non-empty>

## Context

<required full; optional light>

## Options considered

<required full; optional light>

### <option label>

<description>

**Rejected because:** <reason>
<required for every non-chosen option in a full record; optional in light>

## Consequences

<required full; optional light>

## Notes

<optional>

## Errata

<optional; append-only>
### E-001 — 2026-11-04

**Source:** <single-line source>

**Evidence:** <verbatim evidence; may continue on following lines>

**Changed:** <what changed>

**Decision changed:** no
```

Rules:

- Headings are exactly `## Decision`, `## Rationale`, `## Context`, `## Options considered`, `## Consequences`, `## Notes`, `## Errata`.
- Sections appear in the order above. Out-of-order known sections produce a `body_section_order` warning; the writer re-emits canonical order.
- A required section with no non-whitespace content is `body_section_empty`.
- A duplicate section is `duplicate_body_section`.
- An unknown top-level `##` section is `unknown_body_section`; the writer never drops human prose silently.
- `###` headings are structural only inside `## Options considered` and `## Errata`. A `###` heading elsewhere is `unknown_body_section`.
- `## Rationale` is the human's verbatim text. The writer does not trim interior whitespace; it trims only leading and trailing blank lines.
- `## Notes` is free prose.
- The body may contain Markdown; it is untrusted when rendered and passes through the terminal sanitizer (TDD-0010).

### 6.6 Options and `chosen_option`

- Each option is a `### <label>` heading followed by a description.
- Labels are unique within the record, trimmed, single-line, 1–120 characters, and contain no leading `#`.
- `chosen_option` is the exact label of one option. A full record with no options, or with a `chosen_option` that does not match a label, is `chosen_option_unknown`.
- In a full record, every non-chosen option requires a `**Rejected because:**` paragraph. In a light record the reason is optional.
- The chosen option has no `**Rejected because:**` paragraph; its justification is the rationale.
- `DecisionOptionRecord.label` is the heading text; `description` is the prose before the rejected reason; `rejected_reason` is the rejected paragraph.

### 6.7 Stub schema

A decision stub has exactly these fields, in this order:

```yaml
---
schema_version: 1
id: DR-11-04-26-use-postgres
kind: decision
detail: stub
title: Use PostgreSQL for the durable ledger index
status: accepted
created_at: 2026-11-04T10:00:00Z
decided_at: 2026-11-04T10:04:12Z
updated_at: 2026-11-04T10:04:12Z
owner: Karl Abechuela
approver: Karl Abechuela
supersedes: []
---
```

- No body follows the closing `---`; the file ends after the newline.
- `task`, `refs`, `policy`, `chosen_option`, `rationale`, `context`, `options`, `consequences`, `notes`, `errata`, `attestation`, `implemented_by`, `visibility`, `status_reason`, and `revisit_if` are forbidden.
- `superseded_by` may appear when the stub points to a superseded local record.
- The stub's safe fields must match the local full record's corresponding fields. A mismatch is `stub_local_mismatch`.

---

## 7. Assumption records

### 7.1 Front matter order and types

```yaml
---
schema_version: 1
id: AS-11-04-26-existing-tests-cover-the-index
kind: assumption
detail: light
title: The existing ledger tests cover the new index
status: assumed
created_at: 2026-11-04T10:00:00Z
updated_at: 2026-11-04T10:00:00Z
logger: agent
risk: low
revisit_if: The index schema changes or a ledger test fails
task: T-042
refs: []
supersedes: []
visibility: repo
---
```

### 7.2 Field table

| Field | Type | light | stub |
|---|---|:--:|:--:|
| `schema_version` | integer `1` | R | R |
| `id` | `AssumptionId` | R | R |
| `kind` | `assumption` | R | R |
| `detail` | `light` \| `stub` | R | R |
| `title` | single-line string, 1–200 chars | R | R |
| `status` | assumption status | R | R |
| `created_at` | RFC 3339 UTC | R | R |
| `updated_at` | RFC 3339 UTC | R | R |
| `logger` | `agent` \| `human` | R | R |
| `reviewed_by` | human name | O | O |
| `reviewed_at` | RFC 3339 UTC | R iff `reviewed_by` | R iff `reviewed_by` |
| `risk` | `low` \| `medium` \| `high` | R | — |
| `revisit_if` | single-line string, 1–500 chars | R | — |
| `task` | `TaskId` | R | — |
| `refs` | list of typed refs | O | — |
| `supersedes` | list of `RecordId` | O | O |
| `superseded_by` | `RecordId` | O | O |
| `visibility` | `repo` \| `local` | R | — |

### 7.3 Detail-level rules

- Assumptions are `light` or `stub` only. `detail: full` is `detail_level_mismatch`.
- `revisit_if` is required on every light assumption. A low-risk belief still needs a condition that would change it.
- A `visibility: local` assumption is `detail: light`; its committed stub carries the safe fields in §7.6.
- `risk: high` is valid but writes a warning that the belief should have been a decision (FR-LEDGER-004).

### 7.4 Status and lifecycle

Status values: `assumed`, `validated`, `invalidated`, `superseded`.

| Status | Required | Forbidden |
|---|---|---|
| `assumed` | `## Assumption`, `## Basis`; `## Validation` optional | `superseded_by` |
| `validated` | `## Validation` with evidence | `superseded_by` |
| `invalidated` | `## Validation` with evidence | `superseded_by` |
| `superseded` | `superseded_by` | — |

Rules:

- A new assumption is created as `assumed`.
- `assumed` may become `validated` or `invalidated` with evidence.
- Any status may become `superseded` by a later assumption or by a decision that escalates it. Cross-kind supersession uses `RecordId`.
- `reviewed_by` records a human review, not a status change. "Seen and still assumed" is valid.
- `reviewed_at` is required exactly when `reviewed_by` is present.
- `Validation` is required when status is `validated` or `invalidated`; it is optional for `assumed` and `superseded`.

### 7.5 Body grammar

```markdown
## Assumption

<required non-stub>

## Basis

<required non-stub>

## Validation

<required when validated or invalidated; optional otherwise>

## Notes

<optional>

## Errata

<optional; same append-only format as decisions>
```

- Fixed order; unknown or duplicate sections are errors as in §6.5.
- `## Assumption` is the belief; `## Basis` is the evidence or reasoning that supports it.
- `## Validation` records the evidence that confirmed or contradicted it.

### 7.6 Stub schema

An assumption stub has exactly these fields:

```yaml
---
schema_version: 1
id: AS-11-04-26-existing-tests-cover-the-index
kind: assumption
detail: stub
title: The existing ledger tests cover the new index
status: assumed
created_at: 2026-11-04T10:00:00Z
updated_at: 2026-11-04T10:00:00Z
logger: agent
supersedes: []
---
```

- `reviewed_by` and `reviewed_at` may appear when present.
- `risk`, `revisit_if`, `task`, `refs`, and `visibility` are forbidden.
- No body follows the closing `---`.

---

## 8. Typed references

### 8.1 Model

`TypedRef` is a tagged enum. The YAML `kind` selects the variant; each variant has its own fields.

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RefKind {
    Task,
    Plan,
    Commit,
    Diff,
    File,
    Issue,
    Adr,
    Url,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedRef {
    Task { id: TaskId },
    Plan { id: PlanVersion, path: Option<PathBuf> },
    Commit {
        sha: CommitSha,
        path: Option<PathBuf>,
        symbol: Option<String>,
        blob: Option<ContentHash>,
    },
    Diff {
        base: CommitSha,
        head: CommitSha,
        path: Option<PathBuf>,
    },
    File {
        path: PathBuf,
        blob: Option<ContentHash>,
        symbol: Option<String>,
    },
    Issue { url: String, title: Option<String> },
    Adr { path: PathBuf, commit: Option<CommitSha> },
    Url { url: String, title: Option<String> },
    Unknown { kind: String, value: String },
}

impl TypedRef {
    pub fn kind(&self) -> RefKind;
}
```

### 8.2 YAML forms

```yaml
refs:
  - kind: task
    id: T-042
  - kind: plan
    id: 3
    path: .forge/local/sessions/T-042/plans/3.md
  - kind: commit
    sha: 0123456789abcdef0123456789abcdef01234567
    path: crates/infra_local/src/ledger.rs
    symbol: LedgerWriter::write
    blob: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  - kind: diff
    base: 0123456789abcdef0123456789abcdef01234567
    head: fedcba9876543210fedcba9876543210fedcba98
    path: crates/app_core/src/domain/ledger.rs
  - kind: file
    path: docs/tdd/0005-decision-ledger-records.md
    blob: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
  - kind: issue
    url: https://github.com/Xaidel/forge/issues/22
    title: Define the decision ledger and record contract (TDD-0005)
  - kind: adr
    path: docs/adr/0007-ledger-substance-hash-and-merge-backlinks.md
    commit: 0123456789abcdef0123456789abcdef01234567
  - kind: url
    url: https://example.com/design-note
    title: Design note
  - kind: future-thing
    value: opaque-value
```

### 8.3 Validation and resolution

- `task` — `TaskId` grammar.
- `plan` — `PlanVersion` is a positive integer; `path` is repo-relative when present.
- `commit` — `sha` is a 40- or 64-character lowercase hex Git object ID; `path` is repo-relative; `symbol` is single-line; `blob` is a `ContentHash`.
- `diff` — `base` and `head` are commit SHAs; `path` is repo-relative.
- `file` — `path` is repo-relative; `blob` is a `ContentHash`; `symbol` is single-line.
- `issue` / `url` — an absolute `http` or `https` URL; `title` is single-line.
- `adr` — a repo-relative path ending `.md`; `commit` is optional.
- Unknown kinds are tolerated with an `unknown_ref_kind` warning; `value` must be non-empty. The writer preserves the original kind string.
- A malformed known ref is `invalid_reference`; the record is rejected at write.
- A known but unresolvable ref is `unresolvable_ref`, a `verify` warning, not a write block. This covers a commit no longer in the repository, a missing file, and a missing ADR.
- Paths are repo-relative, use `/`, contain no `..`, no absolute prefix, no drive letter, and no NUL. Violations are `invalid_path`.
- `implemented_by` is not a `TypedRef`; it is a separate list of commit SHAs.

---

## 9. Supersession, errata, and immutability

### 9.1 Record references

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RecordId {
    Decision(DecisionId),
    Assumption(AssumptionId),
}
```

YAML:

```yaml
supersedes:
  - kind: decision
    id: DR-11-04-26-use-postgres
superseded_by:
  kind: decision
  id: DR-11-10-26-use-sqlite
```

### 9.2 Forward and back links

- `supersedes` is written on the new record and is authoritative. It may contain same-kind or cross-kind IDs.
- `superseded_by` is the backlink on the old record. It is derived, repairable, and excluded from `substance_hash`.
- When a new record's `supersedes` names an old record, the old record's status becomes `superseded` and its `superseded_by` points at the new record.
- The host repairs the old record eagerly during the write when it is in the same worktree; otherwise `forge ledger verify` repairs it.
- A forward link to a missing record is `broken_forward_link`, an error in `verify`.
- A missing backlink is `missing_backlink`, a warning that `verify` can repair.
- A record cannot supersede itself; a cycle in `supersedes` is `broken_forward_link`.

### 9.3 Cross-kind escalation

An assumption may be escalated to a decision:

- the decision's `supersedes` contains an `Assumption` `RecordId`;
- the assumption's `superseded_by` contains a `Decision` `RecordId`;
- the assumption's status becomes `superseded`;
- the decision's rationale is the human's verbatim escalation rationale.

No new field is needed; `RecordId` carries both kinds.

### 9.4 Errata

Errata are append-only corrections inside `## Errata`. They never rewrite the rationale, chosen option, or any other substance. Each entry has:

- an `### E-<three digits> — <YYYY-MM-DD>` heading;
- `**Source:**` — a single line naming the source;
- `**Evidence:**` — verbatim evidence; continuation lines are allowed until the next label;
- `**Changed:**` — what changed;
- `**Decision changed:**` — `yes` or `no`.

Rules:

- E-numbers start at `E-001` and increase by one; gaps and duplicates are `errata_sequence`.
- A malformed entry is `errata_malformed`.
- Existing entries are never edited or removed. A correction is a new entry.
- `Erratum.date` is the heading date at midnight UTC; `source`, `evidence`, `changed`, and `decision_changed` are parsed from the labels.
- `updated_at` changes when an erratum is added. `substance_hash` changes because errata are substance; a historical attestation still binds the record at `C`.

### 9.5 Immutability

- `rationale`, `chosen_option`, `decided_at`, `owner`, `approver`, and `rationale_provenance` are immutable once written.
- `supersedes`, `superseded_by`, `implemented_by`, `attestation`, `updated_at`, `adr`, and `schema_version` are metadata and may be rewritten by the host.
- `status` may move forward through the lifecycle in §6.4 or §7.4; it never moves back to `accepted`/`assumed`.
- A rewrite never changes the body text except to append errata.

---

## 10. Canonical serialization and hashing

### 10.1 Canonical YAML

- UTF-8 without BOM; LF line endings; exactly one trailing newline.
- Front matter delimiters are `---` on their own line at byte 0 and before the body.
- Keys are emitted in the schema order in §6.1, §6.2, §7.1, and §7.2.
- Two-space indentation; list items use `- `; nested mappings are indented two spaces.
- Absent optional scalar and map keys are omitted. The writer does not emit `null`.
- `refs`, `policy_triggers`, `supersedes`, and `implemented_by` are list fields. The writer emits `[]` when empty; a reader treats an absent list as `[]`.
- Timestamps are unquoted RFC 3339 UTC.
- Strings are plain when unambiguous; double-quoted when they contain `: `, `#`, leading/trailing whitespace, or a YAML-looking scalar such as `true`, `false`, `null`, or a number.
- Multiline strings are not used in front matter; prose belongs in the body.
- Unknown keys are preserved value-wise and emitted after known keys, in their original insertion order. This applies at the top level and inside `policy`, `rationale_provenance`, `attestation`, and each ref mapping.
- Comments, anchors, aliases, and original formatting are not preserved. Readers resolve anchors; writers never emit them.
- Duplicate keys are a parse error.
- A hand-edited file with CRLF, a BOM, or trailing whitespace is readable; the next write normalizes it. A read does not rewrite.

### 10.2 Body normalization

- Fixed section order; one blank line between headings and prose; no trailing spaces.
- Headings are `## ` and `### ` with one space.
- The writer emits only the sections present in the record. Required sections are never empty.
- The writer preserves the human's text verbatim, including Markdown, except for leading/trailing blank lines.
- Unknown top-level sections are rejected; the writer never drops prose.

### 10.3 `content_hash`

`content_hash` is the SHA-256 of the effective record file bytes exactly as stored (UTF-8, LF). For a local-only record, it is the local full/light file; for a repo record or a clone's stub, it is the committed file. It is used by the index and by file-integrity checks.

### 10.4 `substance_hash`

`substance_hash` is the SHA-256 of the canonical JSON projection in §10.7. It is the hash the attestation binds as the decision's substantive content. The following fields are excluded:

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

Everything else in the record is included, including `revisit_if`, `errata`, `notes`, `refs`, `policy_triggers`, `visibility`, and `detail`.

Consequences:

- A merge-time backlink rewrite does not change `substance_hash`.
- A migration that changes only `schema_version` does not change `substance_hash`.
- A later supersession changes `status`, `superseded_by`, and `updated_at`; all three are excluded, so `substance_hash` stays stable. The historical attestation binds the record at `C`; a later lifecycle change does not retroactively invalidate it.
- An erratum changes `substance_hash` because it is substance; this is intentional.

### 10.5 Stub `substance_hash`

A stub has no body and no substantive fields. Its `substance_hash` is the SHA-256 of the canonical JSON of its safe-field projection:

```json
{
  "id": "DR-11-04-26-use-postgres",
  "kind": "decision",
  "detail": "stub",
  "title": "Use PostgreSQL for the durable ledger index",
  "created_at": "2026-11-04T10:00:00Z",
  "decided_at": "2026-11-04T10:04:12Z",
  "owner": "Karl Abechuela",
  "approver": "Karl Abechuela",
  "supersedes": []
}
```

`status`, `updated_at`, and `superseded_by` are excluded, matching the full-record exclusion set.

### 10.6 `safe_hash`

`safe_hash` is the SHA-256 of the canonical JSON of the fields a stub carries. It is used only for cross-worktree identity:

```json
{
  "id": "...",
  "kind": "decision",
  "title": "...",
  "status": "...",
  "created_at": "...",
  "decided_at": "...",
  "owner": "...",
  "approver": "...",
  "supersedes": [],
  "superseded_by": null
}
```

For an assumption, the equivalent fields are `id`, `kind`, `title`, `status`, `created_at`, `logger`, `reviewed_by`, `reviewed_at`, `supersedes`, and `superseded_by`.

`safe_hash` excludes `detail`, `visibility`, `schema_version`, `updated_at`, and all content. It lets a full record and its stub compare equal even though their `detail` and `visibility` differ. Two paths with the same ID and the same `safe_hash` are the same logical record; different `safe_hash` values are a divergence.

### 10.7 Canonical JSON projection

The projection is RFC 8785 JSON Canonicalization Scheme: UTF-8, keys sorted lexicographically, no insignificant whitespace, arrays in record order, JSON strings, integers, booleans, and `null`.

Decision projection:

```json
{
  "id": "...",
  "kind": "decision",
  "detail": "full",
  "title": "...",
  "created_at": "...",
  "decided_at": "...",
  "owner": "...",
  "approver": "...",
  "proposer": "agent",
  "rationale_provenance": {
    "author": "...",
    "method": "typed",
    "captured_at": "..."
  },
  "reversibility": "hard",
  "blast_radius": "system",
  "task": "T-042",
  "refs": [],
  "chosen_option": "postgres",
  "policy": {
    "version": 7,
    "hash": "..."
  },
  "policy_triggers": ["schema-migration"],
  "revisit_if": null,
  "supersedes": [],
  "visibility": "repo",
  "body": {
    "decision": "...",
    "rationale": "...",
    "context": "...",
    "options": [
      {
        "label": "postgres",
        "description": "...",
        "rejected_reason": null
      }
    ],
    "consequences": "...",
    "notes": null,
    "errata": [
      {
        "date": "2026-11-04T00:00:00Z",
        "source": "...",
        "evidence": "...",
        "changed": "...",
        "decision_changed": false
      }
    ]
  }
}
```

Assumption projection:

```json
{
  "id": "...",
  "kind": "assumption",
  "detail": "light",
  "title": "...",
  "created_at": "...",
  "logger": "agent",
  "reviewed_by": null,
  "reviewed_at": null,
  "risk": "low",
  "revisit_if": "...",
  "task": "T-042",
  "refs": [],
  "supersedes": [],
  "visibility": "repo",
  "body": {
    "assumption": "...",
    "basis": "...",
    "validation": null,
    "notes": null,
    "errata": []
  }
}
```

The ledger adapter computes `content_hash`, `substance_hash`, and `safe_hash` and exposes them through `LedgerPort::fingerprint` (§17). The core never parses YAML or JSON.

### 10.8 Unknown fields on rewrite

Every `write_decision` and `write_assumption` is an upsert:

- If a file with the same ID already exists, the adapter reads it, carries forward unknown keys at the top level and inside known maps, and overlays the known fields from the incoming record.
- Unknown keys inside a ref mapping are carried forward for the same ref identity (kind plus primary value).
- Unknown top-level keys whose names collide with a known key are ignored with a `duplicate_key` parse error; the known schema wins.
- A new record has no unknown keys.
- The index is invalidated after any write.

---

## 11. Validation

### 11.1 Pipeline

1. Read bytes; reject a BOM, invalid UTF-8, or a missing final newline.
2. Split front matter and body; require the opening and closing `---`.
3. Parse YAML; reject duplicate keys.
4. Read `schema_version`; a newer version is `schema_too_new` and the record is read-only.
5. Deserialize known fields; retain unknown fields.
6. Parse body sections and errata.
7. Validate IDs, timestamps, enums, required fields, detail level, status, refs, and body.
8. Validate the stub/local pair when both are present.
9. Validate cross-record links and duplicates.
10. Run the secret scan on the canonical serialization (write only).
11. Write atomically.

A parse failure stops that file; other files continue. Validation collects all issues it can and returns them together.

### 11.2 Issue model

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LedgerSeverity { Error, Warning }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerIssue {
    pub code: LedgerIssueCode,
    pub severity: LedgerSeverity,
    pub record_id: Option<String>,
    pub path: Option<PathBuf>,
    pub field: Option<String>,
    pub message: String,
}
```

- `path` is repo-relative.
- `field` is a YAML path such as `rationale_provenance.author` or a body section such as `## Rationale`.
- `message` is human-readable and never contains a secret value.
- Issues are sorted by `path`, then errors before warnings, then `field`.

### 11.3 Issue codes

| Code | Severity | Meaning |
|---|---|---|
| `missing_front_matter` | error | The file does not start with `---`. |
| `front_matter_unterminated` | error | The closing `---` is missing. |
| `yaml_parse_error` | error | The front matter is not valid YAML. |
| `duplicate_key` | error | A YAML mapping contains a duplicate key. |
| `missing_field` | error | A required field is absent. |
| `unknown_field` | warning | An unknown key is tolerated and preserved. |
| `unknown_body_section` | error | A `##` or `###` heading is not in the schema. |
| `body_section_order` | warning | Known sections are out of order; the writer reorders. |
| `body_section_missing` | error | A required body section is absent. |
| `body_section_empty` | error | A required body section is empty. |
| `duplicate_body_section` | error | A section appears more than once. |
| `schema_too_new` | error | `schema_version` is newer than supported. |
| `schema_unsupported` | error | `schema_version` is missing, zero, or malformed. |
| `invalid_field` | error | A field has an invalid value or type. |
| `invalid_id` | error | The ID does not match its grammar. |
| `id_mismatch` | error | The front-matter ID does not match the file name. |
| `duplicate_id` | error | The same ID appears twice in one worktree. |
| `duplicate_id_divergent` | error | The same ID appears across worktrees with different `safe_hash`. |
| `invalid_timestamp` | error | A timestamp is not RFC 3339 UTC or violates its ordering. |
| `invalid_enum` | error | A closed enum has an unknown value. |
| `invalid_path` | error | A path is absolute, contains `..`, or uses a drive letter. |
| `invalid_url` | error | A URL is not absolute `http`/`https`. |
| `missing_human_identity` | error | `git config user.name` is missing or empty. |
| `agent_identity_forbidden` | error | A reserved agent alias appears in a human field. |
| `detail_level_mismatch` | error | The detail level does not match the risk/trigger rules. |
| `status_not_writable` | error | The gate flow was asked to create `rejected` or `deferred`. |
| `status_field_mismatch` | error | A field is present or absent for the wrong status. |
| `forbidden_field` | error | A field is not allowed at this detail level or visibility. |
| `local_requires_full` | error | A local decision is not `full`. |
| `stub_local_mismatch` | error | A stub's safe fields differ from its local full record. |
| `local_repo_conflict` | error | A local record and a committed non-stub record share an ID. |
| `duplicate_option_label` | error | Two options have the same label. |
| `chosen_option_unknown` | error | `chosen_option` does not name an option. |
| `errata_malformed` | error | An errata block is missing a label or has a bad heading. |
| `errata_sequence` | error | Errata numbers are duplicated or have gaps. |
| `unknown_ref_kind` | warning | A ref uses an unknown kind; it is preserved. |
| `invalid_reference` | error | A known ref is malformed. |
| `unresolvable_ref` | warning | A known ref cannot be resolved in this repository. |
| `broken_forward_link` | error | A `supersedes` link names a missing record or forms a cycle. |
| `missing_backlink` | warning | A derived `superseded_by` is absent; repairable. |
| `missing_implemented_by` | warning | A trailer-derived `implemented_by` is absent; repairable. |
| `implemented_by_mismatch` | error | The existing `implemented_by` disagrees with the rebuilt value. |
| `missing_attestation_backlink` | warning | A merge-derived `attestation` is absent. |
| `attestation_mismatch` | error | The existing `attestation` disagrees with the merge commit. |
| `secret_detected` | error | The scanner blocked the record. |
| `secret_scan_unavailable` | error | The scanner could not run; fail closed. |
| `index_corrupt` | warning | The index cannot be parsed; it is rebuilt. |
| `index_stale` | warning | The index does not match the ledger; it is rebuilt. |
| `dirty_ledger` | warning | `verify` skipped repairs because ledger paths are dirty. |
| `active_task` | warning | `verify` skipped repairs because a live task holds the lock. |
| `trailer_malformed` | warning | A `Forge-Decisions` trailer is non-canonical. |
| `misplaced_trailer` | warning | A `Forge-Decisions` line is outside the final trailer block. |
| `migration_substance_changed` | error | A migration changed `substance_hash`; aborted. |
| `migration_failed` | error | A migration step failed. |
| `unsupported_migration` | error | No migration path exists. |
| `record_not_found` | error | A requested record does not exist. |
| `write_failed` | error | An atomic write failed. |
| `io_error` | error | A read failed. |

A warning never blocks a write. An error blocks the write and is reported by `verify`. `schema_too_new` is an error for writes and rewrites; readers return the typed error.

The Rust `LedgerIssueCode` variants in TDD-0002 map one-to-one to the snake_case strings in this table. `infra_local` performs the mapping; the core never parses or prints an external code.

### 11.4 Detail and status validation matrix

Decision:

- `full` requires `policy`, `chosen_option`, `task`, `visibility`, and the five body sections.
- `light` requires `task`, `visibility`, and the two core body sections; `policy` is required iff `policy_triggers` is non-empty; `adr` is forbidden.
- `stub` allows only §6.7 fields.
- `status: accepted` requires no `superseded_by` and no `revisit_if`.
- `status: superseded` requires `superseded_by`.
- `status: deprecated` requires `status_reason`; `superseded_by` is forbidden.
- `status: rejected` and `deferred` are read-only; `deferred` requires `revisit_if`.
- For a read-only `rejected` or `deferred` record, `chosen_option` is optional even when `detail: full`, and `## Context`, `## Options considered`, and `## Consequences` are optional; `## Decision` and `## Rationale` remain required.
- `attestation` requires `implemented_by`; both are forbidden on `rejected`/`deferred`.
- `visibility: local` requires `detail: full`.

Assumption:

- `light` requires `risk`, `revisit_if`, `task`, `visibility`, `## Assumption`, and `## Basis`.
- `validated` / `invalidated` require `## Validation`.
- `superseded` requires `superseded_by`.
- `reviewed_at` is required iff `reviewed_by` is present.
- `visibility: local` requires `detail: light`.

### 11.5 Write versus verify

- A write validates the whole record and fails with `LedgerError::Validation { issues }` if any error is present. Warnings are returned in the report and do not block.
- `verify` validates every record in the current worktree and every `.forge/worktrees/*/` ledger it can read. It repairs derived backlinks but never rewrites substance or invalid fields.
- `read_decision` / `read_assumption` validate enough to parse and return the record; a structural error is returned as a typed `LedgerError`.

### 11.6 Exit codes

- `forge ledger verify`: `0` when no errors remain (warnings allowed), `2` when errors remain.
- `forge ledger migrate`: `0` on success or nothing to migrate, `2` on error or refusal.
- A write failure returns `2` through the CLI mapping.

Exact CLI help and flag mapping are TDD-0001.

---

## 12. Ledger index

### 12.1 Location and scope

`.forge/local/index.json` is a per-worktree, generated, gitignored cache. It indexes:

- `.forge/decisions/`
- `.forge/assumptions/`
- `.forge/local/decisions/`
- `.forge/local/assumptions/`

It is never the source of truth. Deleting it loses no data. A missing, corrupt, or unknown-version index is rebuilt.

### 12.2 JSON schema

```json
{
  "index_version": 1,
  "generated_at": "2026-11-04T10:04:12Z",
  "entries": [
    {
      "record_id": "DR-11-04-26-use-postgres",
      "kind": "decision",
      "detail": "full",
      "visibility": "repo",
      "committed_path": ".forge/decisions/DR-11-04-26-use-postgres.md",
      "local_path": null,
      "content_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    {
      "record_id": "AS-11-04-26-existing-tests-cover-the-index",
      "kind": "assumption",
      "detail": "light",
      "visibility": "local",
      "committed_path": ".forge/assumptions/AS-11-04-26-existing-tests-cover-the-index.md",
      "local_path": ".forge/local/assumptions/AS-11-04-26-existing-tests-cover-the-index.md",
      "content_hash": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    }
  ]
}
```

- `index_version` is `1`. A different value is rebuilt, never migrated.
- `generated_at` is RFC 3339 UTC.
- Entries are sorted by `(kind, record_id)`, with `decision` before `assumption`.
- Paths are repo-relative and use `/`.
- One logical entry per record ID.
- `detail` is the effective detail: the local full/light detail when a local record exists, otherwise the committed detail (including `stub`).
- `visibility` is `local` for a local record or a stub-only entry, and `repo` for a repo record.
- `committed_path` always exists.
- `local_path` is present only when the local full/light record exists on this machine.
- `content_hash` is the hash of the effective file: the local record when present, otherwise the committed record.
- `substance_hash` and `safe_hash` are not stored; they are computed on demand by `LedgerPort::fingerprint`.

### 12.3 Rebuild and invalidation

- `rebuild_index` scans all four directories, parses each record enough to identify its ID, kind, detail, visibility, and path, and atomically replaces the index.
- The write path invalidates the index by removing it after a successful record write. The next reader or `verify` rebuilds.
- A corrupt index is a warning (`index_corrupt`) and is rebuilt. It never blocks a task.
- A stale index is detected by comparing entry paths and `content_hash`; it is a warning (`index_stale`) and is rebuilt.
- A newer `index_version` is a warning and is rebuilt; the index is a cache, so the record-level read-only rule does not apply.

### 12.4 Cross-worktree duplicate checks

`LedgerPort::verify` scans the current worktree and every `.forge/worktrees/*/` worktree it can read. It groups records by ID:

- Two physical records with the same ID in one worktree and different `safe_hash` values: `duplicate_id`.
- Two worktrees with the same ID and different `safe_hash` values: `duplicate_id_divergent`.
- The same ID with the same `safe_hash` in multiple worktrees is the same logical record and is not reported.
- A committed stub and its local full record are the same logical record; their `safe_hash` values must match. A mismatch is `stub_local_mismatch`.
- A stub with no local record is normal on a clone and is not reported.
- Duplicates are reported, never auto-renamed. The one-open-task-branch rule keeps the normal path linear.

### 12.5 Local and repo conflict

A local record is legal only when the committed form is a stub. If a local full/light record and a committed `detail: full` or `detail: light` record share an ID, the result is `local_repo_conflict`. The writer refuses to create that state; `verify` reports it.

---

## 13. Git trailers and backlinks

### 13.1 Trailer grammar

Canonical writer:

```text
Forge-Decisions: DR-11-04-26-use-postgres, DR-11-04-26-keep-sqlite
```

Grammar:

```text
trailer      = "Forge-Decisions:" SP decision-id *( "," SP decision-id )
decision-id  = "DR-" 2DIGIT "-" 2DIGIT "-" 2DIGIT "-" slug
slug         = [a-z0-9] [a-z0-9-]{0,63}
```

Rules:

- The trailer block is the final paragraph of the commit message, after a blank line, with one trailer line per line.
- The canonical writer emits exactly one `Forge-Decisions` line, IDs sorted lexicographically and deduplicated, comma-plus-space separated, no trailing comma.
- The trailer lists decision IDs only. Assumptions are in the commit but not in the trailer.
- If the batch contains no decisions, the trailer is omitted.
- The key is case-sensitive on write.
- The reader tolerates multiple `Forge-Decisions` lines, case-insensitive keys, arbitrary spaces around `:`, comma or whitespace separators, duplicate IDs, and a trailing comma. Non-canonical forms produce a `trailer_malformed` warning, not a parse failure.
- A `Forge-Decisions` line outside the final trailer block produces `misplaced_trailer` and is ignored for rebuilding.
- An `AS-` ID in the trailer produces `trailer_malformed` and is ignored for `implemented_by`.

### 13.2 `implemented_by` rebuild

`implemented_by` is a list of commit SHAs, oldest first. It is rebuilt as follows:

1. Read the current worktree's HEAD.
2. Walk first-parent history from HEAD to the root, newest first.
3. For each commit:
   - if it has a `Forge-Decisions` trailer, record every decision ID against that commit's SHA;
   - if it is a merge commit with a `Forge-Subject` trailer, read the subject commit `C` and parse `C`'s `Forge-Decisions` trailer; record every ID against `C`.
4. A merge commit is never attributed directly from its own `Forge-Decisions` trailer; it points at `C`, the commit that carries the ledger batch.
5. Reverse each ID's list to chronological order and deduplicate.

The normal case is one entry, the task's final commit `C`. Multiple entries are legal when more than one commit names the same decision. The `Forge-Subject` pointer is required by FR-MERGE-003 and is owned by TDD-0008; without it, a merged record's trailer cannot be reached from the base branch's first-parent history.

If a rebase or squash loses the trailer:

- `forge ledger verify` reports `missing_implemented_by` as a warning.
- It searches the commit trees for the record's blob hash and reports candidate commits as a diagnostic.
- It never guesses or writes a backlink from the candidate list.
- A blob-hash diagnostic is not a substitute for the trailer; the human resolves the history.

`forge merge` writes the rebuilt list into the affected records in the merge commit. If a record already has a non-empty `implemented_by` that differs from the rebuilt list, the merge aborts with `implemented_by_mismatch`.

### 13.3 `attestation` backlink

The decision record's `attestation` is `{ id, commit }`:

- `id` is `AT-<task-id>-v<N>`.
- `commit` is `C2`, the signed attestation commit.

It is written only by `forge merge`, from the task's current attestation. It is excluded from `substance_hash`.

- A normal merge writes both `implemented_by` and `attestation`.
- An override merge may write `implemented_by` without `attestation`; the task remains `stopped`.
- `forge reopen` creates a new attestation version; the next merge updates `attestation` to the new version. The old merge commit remains in history.
- `forge ledger verify` repairs a missing `attestation` from the merge commit's `Forge-Attestation` and `Forge-Attestation-Commit` trailers. It reports `missing_attestation_backlink` when the merge commit cannot supply them. An existing value that disagrees with the merge commit is `attestation_mismatch`. When a task was reopened and merged again, the newest merge commit wins.
- An override merge has no attestation by contract; its records carry `implemented_by` without `attestation`, and `verify` does not warn about the missing attestation.
- The exact merge trailer values and signing are owned by TDD-0008.

### 13.4 Merge-time write

`forge merge` performs a `--no-ff` merge and creates the merge commit with the ledger backlinks:

1. For a normal merge, verify the task is `done`, the attestation is valid, `HEAD == C2`, `C2^ == C`, and the merge is clean. An override merge requires the signed override record and skips the attestation check.
2. Start a `--no-commit` merge of `C2` (normal) or `C` (override) into the base.
3. Read the decision IDs from `C`'s `Forge-Decisions` trailer.
4. For each ID, read the record from the merged tree.
5. Compute the rebuilt `implemented_by` list and set `attestation` from the task's attestation; an override merge leaves `attestation` absent.
6. Verify that each record's `substance_hash` is unchanged.
7. For a repo record, rewrite the committed file in the base worktree.
8. For a local record, rewrite the local full/light file; the committed stub is unchanged.
9. If a local record's local full file is missing, abort with `record_not_found`; a stub-only clone cannot perform this merge on the origin's behalf.
10. Stage only the ledger files that changed.
11. Create the merge commit with the TDD-0008 trailers.
12. If any step fails, abort the merge and leave the base unchanged.

The merge commit is the durable backlink record. `C2` remains attestation-only and is never rewritten.

### 13.5 Verification and repair

`forge ledger verify` repairs:

- `superseded_by` and the old record's `status` when a forward `supersedes` link exists;
- `implemented_by` from first-parent trailers;
- `attestation` from merge-commit trailers.

It never repairs substance, duplicate IDs, broken forward links, invalid fields, or a newer schema. Repairs set `updated_at` to the current time and invalidate the index.

---

## 14. Secret scan boundary

The ledger write path invokes the secret scanner owned by TDD-0010. TDD-0005 owns the invocation boundary, not the patterns.

- The scanner runs after canonical serialization and validation, before any file is written.
- It scans the entire artifact: front matter and body, including title, rationale, context, options, consequences, notes, errata, and string-valued refs.
- It runs for decisions and assumptions, `full`/`light`/`stub`, committed and local.
- Local records are not exempt. `.forge/local/` can leak through backups, opt-in model context, or a future export, and the PRD's write-time block is unconditional.
- A hit returns `LedgerError::SecretDetected { location, rule_id }`. The error never contains the secret. The record is not written.
- If the scanner is unavailable or errors, the write fails closed with `secret_scan_unavailable`. The human override path is TDD-0010's.
- A successful override is audited by TDD-0010 and does not add metadata to the ledger record. The record is written only after the scanner returns pass.
- `forge ledger verify` does not scan for secrets. The PRD makes this a write-time control, not a historical audit.

---

## 15. `forge ledger verify`

### 15.1 Capability

`VerifyLedger` is the seventeenth inbound capability. It has two operations:

```rust
#[async_trait::async_trait]
pub trait VerifyLedger: Send + Sync {
    async fn verify_ledger(
        &self,
        request: VerifyLedgerRequest,
    ) -> Result<LedgerVerificationReport, ApplicationError>;

    async fn migrate_ledger(
        &self,
        request: MigrateLedgerRequest,
    ) -> Result<LedgerMigrationReport, ApplicationError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifyLedgerRequest {
    pub repair: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrateLedgerRequest {
    pub confirmed: bool,
}
```

`VerifyLedgerUseCase` depends on `LedgerPort`, `SessionStorePort`, `GitPort`, `AuditPort`, and `ClockPort`. `SessionStorePort::active_task` supplies the lock check; stale-lock detection is TDD-0003's contract.

### 15.2 Checks

`verify_ledger` performs:

1. Read the active-task lock. If a live process holds it, set `repair = false` and add an `active_task` warning.
2. Check that the tracked worktree has no uncommitted ledger changes. If it does, set `repair = false` and add a `dirty_ledger` warning.
3. Scan the current worktree and every readable `.forge/worktrees/*/` ledger.
4. Validate every record using §11.
5. Check duplicate IDs and safe-hash divergence.
6. Check stub/local pairs.
7. Check forward and back links.
8. If repairs are enabled, rebuild the index if missing, corrupt, stale, or unknown-version; otherwise leave it for the next safe run.
9. Return the report.

`LedgerPort::verify` is the filesystem-only portion: parse, field validation, duplicate detection. Index rebuild is a separate `rebuild_index` call. `VerifyLedgerUseCase` adds Git-dependent trailer and attestation checks and all repairs.

### 15.3 Repairs

Only when `repair == true`:

- `superseded_by` and the old record's `status` from a forward `supersedes` link.
- `implemented_by` from first-parent trailers.
- `attestation` from merge-commit trailers.
- `updated_at` is set to the current time for each rewritten record.
- The index is rebuilt after repairs.
- Each repair appends an audit event through `AuditPort`; the exact encoding is TDD-0011.

Repairs are metadata-only. `substance_hash` must be unchanged; if it changes, the repair aborts with `implemented_by_mismatch` or `attestation_mismatch`. Repairs apply only to the current worktree; other worktrees are reported, never rewritten.

Never repaired:

- duplicate IDs;
- broken forward links;
- invalid fields or body sections;
- newer schemas;
- secrets;
- missing local records on a clone.

### 15.4 Safety gates

- A live active-task lock: read-only checks, no repairs; the index is left as-is and rebuilt on the next safe run.
- Dirty ledger paths: read-only checks, no repairs.
- A newer-schema record: `schema_too_new`, no rewrite.
- A merge/attestation disagreement: report, do not overwrite.
- A missing or corrupt index: rebuild when repairs are enabled; this is not a repair of record content.

### 15.5 Report

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerVerificationReport {
    pub issues: Vec<LedgerIssue>,
    pub duplicate_ids: Vec<String>,
    pub broken_links: Vec<String>,
    pub missing_backlinks: Vec<String>,
    pub repaired: Vec<LedgerRepair>,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerRepair {
    pub record_id: String,
    pub path: PathBuf,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}
```

- `issues` is sorted as in §11.2.
- `duplicate_ids`, `broken_links`, and `missing_backlinks` are compatibility projections of `issues`.
- `repaired` lists every rewrite.
- `errors` and `warnings` are counts.

### 15.6 Exit behavior

- `0` when `errors == 0`; warnings are allowed.
- `2` when `errors > 0`.
- `verify` does not commit. It prints a summary and the changed paths; the human or the next task's ledger batch commits the result.
- Exact CLI output and flags are TDD-0001.

---

## 16. `forge ledger migrate`

### 16.1 Versions

- Current `schema_version` is `1`; supported versions are `{1}`.
- A missing, zero, or malformed `schema_version` is `schema_unsupported`.
- A newer `schema_version` is `schema_too_new`; readers are read-only and `migrate` refuses.
- Migrations are ordered, pure `vN -> vN+1` steps. Downgrades do not exist.
- The index has its own `index_version`; it is rebuilt, never migrated.

### 16.2 Preconditions

`migrate_ledger` requires:

- `confirmed == true`; the CLI asks `y/N` with default no;
- a clean tracked worktree (`git status --porcelain --untracked-files=no` empty);
- no live active-task lock;
- no newer-schema record anywhere in the current worktree.

A live lock or a dirty worktree produces a refusal with a clear message. A migration lock at `.forge/local/ledger-migrate.lock` prevents concurrent migrations; a stale lock is reported, never silently removed. The command migrates only the current worktree; another worktree's records are migrated when that worktree is checked out.

### 16.3 Steps

1. Scan the current worktree's committed and local records.
2. Group by `schema_version`. If all are current, print “nothing to migrate” and return exit `0`.
3. Build the migration chain from the oldest version to `1`. If a step is missing, `unsupported_migration`.
4. Apply the chain in memory to every record.
5. Compute `substance_hash` before and after. If any changes, abort with `migration_substance_changed`.
6. Print a per-file unified diff of the front matter and a summary. The body is unchanged.
7. Ask for confirmation in the CLI. The use case receives `confirmed`; if false, return the report without writing.
8. Back up every local record to `.forge/local/migrations/<timestamp>/<relative-path>`. Committed records are protected by Git.
9. Write each record atomically: temp file, fsync, rename. The body is copied byte-for-byte.
10. Preserve unknown fields.
11. Stage only ledger files and create one commit:
    `forge: migrate ledger schema <from> -> <to>`
12. Remove and rebuild the index.
13. Append a `ledger_migrate` audit event through `AuditPort`; the exact encoding is TDD-0011.
14. Return the report.

### 16.4 Failure and resumability

- Each file is atomic: it is either the old version or the new version, never corrupt.
- If a step fails after some files were written, the command stops and reports the completed files. The operation is idempotent; rerunning continues.
- Local backups are kept for the human.
- Committed files can be restored with Git.
- A migration never changes substance, body text, or the human's words.

### 16.5 Report

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerMigrationReport {
    pub from_version: u32,
    pub to_version: u32,
    pub files: Vec<PathBuf>,
    pub preview: Vec<MigrationFileDiff>,
    pub commit: Option<CommitSha>,
    pub local_backup: Option<PathBuf>,
    pub issues: Vec<LedgerIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationFileDiff {
    pub path: PathBuf,
    pub diff: String,
}
```

- `from_version` is the oldest version found; `to_version` is `1`.
- `files` lists the migrated record paths.
- `preview` is the unified front-matter diff computed before writing; the CLI renders it and asks for confirmation. It is populated whether or not `confirmed` is true.
- `commit` is the dedicated migration commit, or `None` when nothing changed or confirmation was declined.
- `local_backup` is the backup directory, or `None` when there are no local records.

---

## 17. Amendments to TDD-0002

This document amends TDD-0002 as follows. The TDD-0002 file is edited in the same change and carries an “Amended by TDD-0005” note.

### 17.1 Capability count

- The inbound capability count changes from 16 to 17.
- `VerifyLedger` is added; it owns `verify_ledger` and `migrate_ledger`.
- `VerifyLedgerUseCase` depends on `LedgerPort`, `SessionStorePort`, `GitPort`, `AuditPort`, and `ClockPort`.

### 17.2 Public records

- `TypedRef` becomes the tagged enum in §8.1; `RefKind` remains the kind discriminator.
- `RecordId` is added.
- `AttestationBacklink { id: AttestationId, commit: CommitSha }` is added.
- `DecisionRecord` gains `decided_at: Timestamp`, `policy_triggers: Vec<String>`, `chosen_option: Option<String>`; `attestation` becomes `Option<AttestationBacklink>`; `supersedes`/`superseded_by` become `RecordId`.
- `AssumptionRecord` gains `reviewed_at: Option<Timestamp>`; `reviewer` is renamed `reviewed_by`; `supersedes`/`superseded_by` become `RecordId`.
- `DecisionStub` and `AssumptionStub` are added.
- `LedgerRecordView<F, S>` is added; `DecisionView = LedgerRecordView<DecisionRecord, DecisionStub>` and `AssumptionView = LedgerRecordView<AssumptionRecord, AssumptionStub>`.
- `LedgerRecordFingerprint` is added.
- `LedgerIssue`, `LedgerSeverity`, `LedgerIssueCode`, and `LedgerRepair` are added.
- `LedgerVerificationReport` gains `issues`, `repaired`, and `warnings`; `errors` becomes a `usize` count; `duplicate_ids`, `broken_links`, and `missing_backlinks` remain.
- `LedgerIndexEntry` gains `detail`, `visibility`, `committed_path`, and `local_path`; `path` is replaced by `committed_path`.
- `LedgerIndex` gains `index_version: u32`.
- `LedgerBinding` gains `substance_hash: ContentHash`.
- `ReviewPacket.decisions` and `ReviewPacket.assumptions` become `Vec<DecisionView>` and `Vec<AssumptionView>` so a local-only record can be referenced without exposing its content.
- Private `Decision` mirrors `chosen_option: Option<String>` and `decided_at: Timestamp`.
- `LedgerMigrationReport` gains `preview: Vec<MigrationFileDiff>`; `MigrationFileDiff` is added.

### 17.3 Ports

`LedgerPort` becomes:

```rust
#[async_trait::async_trait]
pub trait LedgerPort: Send + Sync {
    async fn write_decision(&self, record: &DecisionRecord) -> Result<(), LedgerError>;
    async fn write_assumption(&self, record: &AssumptionRecord) -> Result<(), LedgerError>;
    async fn read_decision(&self, id: &DecisionId) -> Result<Option<DecisionView>, LedgerError>;
    async fn read_assumption(&self, id: &AssumptionId) -> Result<Option<AssumptionView>, LedgerError>;
    async fn fingerprint(&self, id: &RecordId) -> Result<LedgerRecordFingerprint, LedgerError>;
    async fn verify(&self) -> Result<LedgerVerificationReport, LedgerError>;
    async fn rebuild_index(&self) -> Result<LedgerIndex, LedgerError>;
}
```

`GitPort` gains:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitInfo {
    pub sha: CommitSha,
    pub parents: Vec<CommitSha>,
    pub message: String,
}

#[async_trait::async_trait]
pub trait GitPort {
    // ... existing methods ...
    async fn current_worktree(&self) -> Result<Worktree, GitError>;
    async fn first_parent_log(&self, to: &CommitSha) -> Result<Vec<CommitInfo>, GitError>;
}
```

`current_worktree` returns the main worktree path, current branch, and base commit. `first_parent_log` returns first-parent ancestors of `to`, newest first, and is implemented by TDD-0008.

### 17.4 Errors

`LedgerError` gains:

```rust
Validation { issues: Vec<LedgerIssue> },
MissingHumanIdentity { field: &'static str },
SecretScanUnavailable { message: String },
MigrationFailed { path: PathBuf, message: String },
```

`SecretDetected` gains `rule_id: String` alongside `location`.

`DirtyLedger` and `ActiveTask` are report warnings (`LedgerIssueCode`), not fatal errors. `LedgerError` remains `#[non_exhaustive]`.

### 17.5 Application

`Application` gains a `verify_ledger` accessor and a `pub(crate) verify_ledger` field. `build_application` wires the new use case. No new outbound port is added; the count remains 11.

---

## 18. Fixtures

Fixtures live under `tests/fixtures/ledger/` when implementation begins. This section fixes their names and expected outcomes so the tests do not invent cases.

| Fixture | Expected |
|---|---|
| `decision-full.md` | Parses; `full`; all §6.2 fields present; `substance_hash` stable. |
| `decision-light.md` | Parses; `light`; Decision and Rationale only; no `policy` when no trigger. |
| `decision-stub.md` | Parses as `DecisionStub`; safe fields only; no body. |
| `decision-local-full.md` | Parses as local `full`; `visibility: local`; paired with `decision-stub.md`. |
| `assumption-light.md` | Parses; `light`; Assumption, Basis; `revisit_if` present. |
| `assumption-stub.md` | Parses as `AssumptionStub`; safe fields only. |
| `decision-superseded.md` | Parses; `status: superseded`; `superseded_by` set; old substance unchanged. |
| `decision-errata.md` | Parses; E-001/E-002; rationale unchanged; `substance_hash` changes. |
| `decision-duplicate-a.md` | Same ID as B; different `safe_hash`; `verify` reports `duplicate_id_divergent`. |
| `decision-duplicate-b.md` | Same ID as A; different title/status. |
| `decision-schema-2.md` | `schema_version: 2`; `schema_too_new`; read-only. |
| `decision-missing-rationale.md` | `body_section_missing` for `## Rationale`. |
| `decision-trigger-light.md` | Non-empty `policy_triggers` with `detail: light`; `detail_level_mismatch`. |
| `decision-stub-unsafe.md` | Stub with `rationale`; `forbidden_field`. |
| `decision-agent-author.md` | `rationale_provenance.author: agent`; `agent_identity_forbidden`. |
| `decision-bad-ref.md` | Commit ref with a malformed SHA; `invalid_reference`. |
| `decision-unresolvable-ref.md` | Commit ref not in the repo; `unresolvable_ref` warning. |
| `decision-bad-errata.md` | Missing `Decision changed`; `errata_malformed`. |
| `assumption-high-risk.md` | `risk: high`; warning that it should have been a decision. |
| `assumption-missing-validation.md` | `validated` with no `## Validation`; `body_section_missing`. |
| `trailer-canonical.txt` | One sorted line; parses cleanly. |
| `trailer-tolerant.txt` | Multiple lines, flexible spaces, duplicate IDs; `trailer_malformed` warnings. |
| `trailer-misplaced.txt` | Trailer outside the final block; `misplaced_trailer`. |
| `index-v1.json` | Valid index; round-trips. |
| `index-corrupt.json` | `index_corrupt`; rebuilt. |
| `index-newer.json` | `index_version: 2`; rebuilt. |

Each fixture has one primary assertion and may have secondary assertions for the fields it exercises. The implementation tests must use the exact issue codes in §11.3.

---

## 19. Requirement-to-test mapping

| Requirement | Test | Layer |
|---|---|---|
| FR-LEDGER-001 — record fields | `ledger_schema::given_full_record_when_parsed_then_all_fields_present` | adapter |
| FR-LEDGER-001 — missing field | `ledger_schema::given_missing_required_field_when_written_then_validation_error` | adapter |
| FR-LEDGER-001 — unknown field | `ledger_schema::given_unknown_field_when_rewritten_then_value_preserved` | adapter |
| FR-LEDGER-001 — newer schema | `ledger_schema::given_schema_two_when_read_then_schema_too_new` | adapter |
| FR-LEDGER-002 — full trigger | `ledger_schema::given_trigger_when_written_then_full_required` | domain |
| FR-LEDGER-002 — hard reversibility | `ledger_schema::given_hard_reversibility_when_light_then_rejected` | domain |
| FR-LEDGER-002 — system blast | `ledger_schema::given_system_blast_radius_when_light_then_rejected` | domain |
| FR-LEDGER-002 — local stub | `ledger_schema::given_local_record_when_written_then_only_safe_stub_committed` | adapter |
| FR-LEDGER-002 — stub fields | `ledger_schema::given_stub_when_parsed_then_no_substantive_field` | adapter |
| FR-LEDGER-003 — verbatim rationale | `ledger_schema::given_human_rationale_when_written_then_verbatim` | adapter |
| FR-LEDGER-003 — agent author | `ledger_schema::given_agent_author_when_written_then_agent_identity_forbidden` | domain |
| FR-LEDGER-003 — missing rationale | `ledger_schema::given_blank_rationale_when_written_then_rejected` | domain |
| FR-LEDGER-003 — imported | `ledger_schema::given_imported_rationale_when_written_then_method_imported` | adapter |
| FR-LEDGER-003 — errata | `ledger_schema::given_erratum_when_written_then_rationale_unchanged` | adapter |
| FR-LEDGER-004 — basis | `ledger_schema::given_assumption_without_basis_when_written_then_rejected` | domain |
| FR-LEDGER-004 — validation | `ledger_schema::given_validated_assumption_when_written_then_validation_required` | domain |
| FR-LEDGER-004 — high risk | `ledger_schema::given_high_risk_assumption_when_written_then_warning` | adapter |
| FR-LEDGER-004 — cross-kind | `ledger_schema::given_assumption_escalated_when_written_then_record_id_links` | domain |
| FR-LEDGER-004 — reviewed_by | `ledger_schema::given_assumption_reviewed_when_written_then_status_unchanged` | domain |
| FR-LEDGER-005 — append-only | `ledger_schema::given_supersession_when_written_then_old_rationale_unchanged` | adapter |
| FR-LEDGER-005 — forward link | `ledger_schema::given_supersedes_when_written_then_new_record_names_old` | adapter |
| FR-LEDGER-005 — backlink | `ledger_schema::given_forward_link_when_verified_then_superseded_by_repaired` | application |
| FR-LEDGER-005 — duplicate | `ledger_schema::given_duplicate_id_when_verified_then_reported` | adapter |
| FR-LEDGER-005 — broken link | `ledger_schema::given_missing_forward_target_when_verified_then_error` | adapter |
| FR-LEDGER-005 — migrate | `ledger_schema::given_old_schema_when_migrated_then_substance_hash_stable` | application |
| FR-LEDGER-006 — final trailer | `ledger_git::given_decision_batch_when_committed_then_trailer_present` | application |
| FR-LEDGER-006 — host committer | `ledger_git::given_worker_capability_when_writing_ledger_then_denied` | application |
| FR-LEDGER-006 — pending | `ledger_git::given_pending_request_when_committed_then_not_in_ledger` | application |
| FR-LEDGER-006 — rebuild | `ledger_git::given_trailer_when_verified_then_implemented_by_rebuilt` | application |
| FR-LEDGER-006 — merge pointer | `ledger_git::given_merge_commit_when_verified_then_forge_subject_points_to_c` | application |
| FR-LEDGER-007 — secret block | `ledger_secret::given_known_secret_when_written_then_blocked` | adapter |
| FR-LEDGER-007 — local stub | `ledger_secret::given_local_record_when_written_then_stub_only_safe_fields` | adapter |
| FR-LEDGER-007 — local gitignored | `ledger_secret::given_local_record_when_committed_then_path_absent` | adapter |
| FR-LEDGER-007 — override | `ledger_secret::given_false_positive_override_when_written_then_audited` | adapter |
| FR-LEDGER-008 — policy full | `ledger_schema::given_full_record_when_written_then_policy_pinned` | adapter |
| FR-LEDGER-008 — policy light trigger | `ledger_schema::given_light_trigger_when_written_then_policy_pinned` | adapter |
| FR-LEDGER-008 — missing policy | `ledger_schema::given_full_record_without_policy_when_written_then_rejected` | adapter |
| FR-LEDGER-009 — index rebuild | `ledger_index::given_missing_index_when_read_then_rebuilt` | adapter |
| FR-LEDGER-009 — unknown field | `ledger_index::given_unknown_field_when_rewritten_then_preserved` | adapter |
| FR-LEDGER-009 — newer schema | `ledger_index::given_schema_two_when_verified_then_not_rewritten` | adapter |
| FR-LEDGER-009 — corrupt index | `ledger_index::given_corrupt_index_when_verified_then_rebuilt` | adapter |
| FR-LEDGER-009 — cross-worktree | `ledger_index::given_same_id_in_two_worktrees_when_verified_then_divergence_reported` | application |
| FR-LEDGER-010 — pending | `ledger_schema::given_pending_request_when_written_then_not_in_ledger` | application |
| FR-LEDGER-010 — deferred | `ledger_schema::given_deferred_decision_when_answered_then_session_only` | application |
| FR-LEDGER-010 — cite by ID | `ledger_schema::given_resolved_decision_when_reviewed_then_cited_by_id` | application |
| FR-LEDGER-010 — cancelled | `ledger_schema::given_cancelled_task_when_committed_then_records_not_merged` | application |
| FR-LEDGER-011 — package ref | `ledger_schema::given_package_ref_when_written_then_typed_ref` | adapter |
| FR-LEDGER-011 — one ledger | `ledger_schema::given_monorepo_when_initialized_then_one_ledger` | application |
| FR-LEDGER-011 — cross-package | `ledger_schema::given_cross_package_decision_when_written_then_one_record` | adapter |
| INV-03 — pending not in ledger | `ledger_schema::given_pending_gate_when_written_then_ledger_untouched` | application |
| INV-15 — immutable substance | `ledger_schema::given_superseded_record_when_read_then_rationale_unchanged` | adapter |
| INV-16 — newer schema | `ledger_schema::given_schema_two_when_written_then_read_only` | adapter |
| INV-20 — no secrets | `ledger_secret::given_secret_when_written_then_not_committed` | adapter |
| FR-MERGE-004 — binding | `ledger_git::given_attestation_when_merged_then_ledger_hashes_match_c` | application |
| FR-MERGE-010 — backlinks | `ledger_git::given_merge_when_committed_then_backlinks_in_merge_commit` | application |
| FR-REV-010 — ledger binding | `ledger_git::given_attestation_when_built_then_substance_hash_bound` | application |
| NFR-SCALE-002 — repo-level | `ledger_schema::given_monorepo_when_records_written_then_one_ledger_and_index` | adapter |
| D-08 — ledger schema | §6, §7, §8, §9, §10 | inspection |
| D-16 — local state | §4, §11, §12 | inspection |
| D-18 — secret and visibility | §6.7, §7.6, §14 | inspection |

---

## 20. Seams with later TDDs

| TDD | This document fixes | The later TDD owns |
|---|---|---|
| TDD-0001 | Ledger file model and command semantics | CLI flags, help, exit-code mapping, `.forge/.gitignore` |
| TDD-0003 | Pending decisions and deferred decisions stay out of the ledger | Session-state encoding, event schema, deferred-decision lifecycle |
| TDD-0004 | `policy_triggers` is a list of stable trigger IDs | Trigger IDs, policy evaluation, detail classification rules |
| TDD-0008 | `implemented_by` and `attestation` field shapes, merge-time write | Merge mechanics, merge trailers, `C2`, attestation JSON, signing, override, `current_worktree`, `first_parent_log` |
| TDD-0010 | Secret-scan invocation and fail-closed behavior | Patterns, entropy rules, override flow, terminal sanitization |
| TDD-0011 | Audit event names for ledger writes and repairs | Audit JSONL encoding, metrics, dogfood diary |

TDD-0008 must not change the backlink fields or the `substance_hash` exclusion set without amending this document. TDD-0004 must not change the meaning of `policy_triggers`; it assigns the IDs. TDD-0010 must not silently allow a scan failure; the ledger write is fail-closed.

---

## 21. Traceability

| Source | Covered by |
|---|---|
| PRD §5.1 governance and ledger terms | §4–§9 |
| PRD §5.3 invariants INV-03, INV-15, INV-16, INV-20 | §4.5, §9.5, §10, §14 |
| PRD §5.4 decision and assumption lifecycles | §6.4, §7.4 |
| PRD §7.5 FR-LEDGER-001..011 | §6–§16, §19 |
| PRD §7.7 FR-MERGE-004, FR-MERGE-010 | §13 |
| PRD §8.1 `forge ledger verify`, `forge ledger migrate` | §15, §16 |
| PRD §9.1–9.3 identity, attestation binding, unknown version | §5, §10, §11, §13 |
| PRD §9.4 retention and privacy | §4.1, §6.7, §7.6, §14 |
| PRD §9.5 repo-level state | §3, §12 |
| PRD §10.10 rationale capture and ledger write | §5.5, §6.3, §9.5 |
| D-08 decision ledger schema | §6–§10 |
| D-16 local state and privacy | §4, §12, §14 |
| D-18 secret and visibility posture | §6.7, §7.6, §14 |
| ADR-0006 local stub and full record | §4, §6.7, §7.6, §12 |
| ADR-0007 substance hash and merge backlinks | §10, §13 |
| ADR-0008 tagged typed references | §8 |
| TDD-0002 public records and ports | §17 |

---

## 22. Changelog

| Date | Change |
|---|---|
| 2026-11-04 | Initial contract. Amends TDD-0002 with the decision ledger records, `VerifyLedger`, and the backlink/hash split. |

