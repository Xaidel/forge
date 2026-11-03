---
status: accepted
date: 2026-11-03
---

# Repo-scoped sequential task IDs

## Context

The PRD uses `T-042` in task examples, `.forge/worktrees/<task-id>`, and `forge/<task-id>`. It derives attestation and override IDs from the task ID (`AT-<task-id>-v<N>`, `OV-<task-id>-v<N>`). Decision and assumption IDs are date-based (`DR-MM-DD-YY-<slug>`, `AS-...`).

The task ID is user-visible and appears in file paths, Git branch names, commit trailers, attestation IDs, override IDs, and review output. It is therefore hard to change after implementation. Forge is local-first, one repo at a time, one active task per repo, and one open task branch per repo. A task ID does not need to be globally unique across repositories, machines, or hosted services.

A UUID would be globally unique but unreadable in paths and transcripts. A date-plus-slug ID could collide within a day and would put mutable text into paths. A monotonic sequence is readable, stable, and matches the PRD’s examples.

## Decision

`TaskId` is `T-` plus a repo-scoped zero-padded monotonic sequence:

- Grammar: `^T-[0-9]{3,}$`.
- The first task is `T-001`; the sequence grows without a fixed upper bound.
- `IdSourcePort::next_task_id` allocates the next ID after `SessionStorePort::acquire_lock` succeeds.
- The adapter derives the next sequence from the existing sessions and attestations, or from a local counter it owns. The counter is local-only and rebuildable; the core never reads it.
- `AttestationId` is `AT-<task-id>-v<N>` and `OverrideId` is `OV-<task-id>-v<N>`.
- `DecisionId` and `AssumptionId` remain date-based, as the PRD requires.
- No UUID or random ID is used for a task.

## Consequences

- Task IDs are readable in the worktree path, branch, transcript, and review.
- One repo cannot have two tasks with the same ID because the active-task lock serializes allocation.
- A cloned repository carries committed attestations and overrides; a new machine’s local counter must not reuse an ID that appears in the committed records. The adapter derives the floor from the highest committed task ID and the local sessions.
- If a local counter is lost, the adapter rebuilds it. A duplicate is a hard error, not a silent reuse.
- A future multi-repo or hosted product that needs globally unique task IDs must introduce a repo identifier or a new ID scheme through a new ADR; it must not reinterpret an existing `T-<n>`.
- The exact counter storage is TDD-0003’s contract; the grammar and derivation rule are fixed here.

## Considered options

- **UUID v4.** Globally unique and allocation-free, but unreadable in paths and impossible to scan in a transcript. Rejected for a local-first product whose IDs are part of the human experience.
- **Date plus slug (`T-2026-11-03-fix-auth`).** Readable but long, collision-prone within a day, and puts mutable prose into branch and worktree names.
- **Random short ID.** Compact, but collision handling and readability are worse than a sequence, and it breaks the PRD’s `T-042` examples.
- **Repo-scoped sequence.** Chosen: readable, deterministic, matches the PRD, and serialized by the active-task lock.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the domain model, ports, and application assembly (TDD-0002)](https://github.com/Xaidel/forge/issues/21)
- `docs/PRD.md` §5.1, §7.2 FR-PLAN-001, §7.7 FR-REV-010, §9.1, D-09
- [TDD-0002](../tdd/0002-domain-model-ports-assembly.md) §5.2, §6.2, §7.2
- [TDD-0003](https://github.com/Xaidel/forge/issues/28) — counter storage and session layout
