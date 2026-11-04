---
status: accepted
date: 2026-11-04
---

# Typed ledger references are a tagged enum

## Context

The PRD fixes eight reference kinds for the ledger: `task`, `plan`, `commit`, `diff`, `file`, `issue`, `adr`, and `url` (FR-LEDGER-001, §9.1). Several kinds carry optional metadata: a commit ref may name a path, symbol, and blob; a file ref may name a blob and symbol; an issue or URL may carry a title; an ADR may name a commit.

TDD-0002 originally fixed `TypedRef { kind, value }`. That is enough for a single locator string but not for the optional fields. Encoding `path`, `symbol`, `blob`, and `title` into `value` would make the YAML hard to read, hard to validate, and easy to break when a path contains a separator. It would also force every consumer to parse a private mini-format.

The reference list is small, closed, and part of the public core contract. Unknown reference kinds must still be tolerated because a newer schema may add one.

## Decision

`TypedRef` is a tagged enum with one variant per known kind:

```rust
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
```

The YAML mapping's `kind` selects the variant; each variant has its own fields. `RefKind` remains the discriminator, including `RefKind::Unknown(String)`.

Validation is per kind. A malformed known ref is an error. An unknown kind is a warning and is preserved as `Unknown { kind, value }`. A known but unresolvable ref (a commit no longer present, a missing file, a missing ADR) is a `verify` warning, not a write block.

Adding a new known kind is an amendment to this ADR and to TDD-0005. It is not an open-ended map.

## Consequences

- The YAML is explicit and reviewable: each ref kind shows exactly the fields it supports.
- The Rust core has one typed representation per kind; adapters do not parse a private string format.
- The public `TypedRef` shape changes from TDD-0002's `{ kind, value }`. TDD-0002 is amended in the same change.
- The adapter needs a per-kind serializer and validator. This is more code than a single string but is straightforward and testable.
- Unknown kinds survive a rewrite as `Unknown`; they do not crash an older reader.
- A future kind can be added without changing the existing variants, but it requires a schema amendment.

## Considered options

- **Keep `{ kind, value }` and encode optional fields in the value string.** Unreadable, hard to validate, and fragile for paths and symbols.
- **Add `attributes: BTreeMap<String, String>`.** Flexible, but loses type safety and makes every consumer validate string keys at runtime.
- **Split optional metadata into separate refs of the same kind.** Ambiguous: a commit and its path would become two unrelated entries, and a title would have nowhere to live.
- **Store refs as free-form YAML.** No stable contract; the core cannot type them.
- **Tagged enum with per-kind fields and an unknown fallback.** Chosen: readable, type-safe, and forward-tolerant.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the decision ledger and record contract (TDD-0005)](https://github.com/Xaidel/forge/issues/22)
- `docs/PRD.md` §7.5 FR-LEDGER-001; §9.1; D-08
- [TDD-0005](../tdd/0005-decision-ledger-records.md) §8
- [TDD-0002](../tdd/0002-domain-model-ports-assembly.md) — amended by TDD-0005
