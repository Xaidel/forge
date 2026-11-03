# TDD-0002 — Domain Model, Ports, and Application Assembly

**Status:** Approved  
**Date:** 2026-11-03  
**Owner:** Forge architecture  
**Sources:** `docs/PRD.md` §5, §6.2–6.5, §8, §9, NFR-MAINT-001; D-08, D-09, D-13, D-16, D-20  
**Companions:** [architecture.md](../../arch_docs/architecture.md), [dependency-rules.md](../../arch_docs/dependency-rules.md), [naming-conventions.md](../../arch_docs/naming-conventions.md)  
**ADRs:** [ADR-0001](../adr/0001-two-crate-ports-only-core.md), [ADR-0002](../adr/0002-executor-agnostic-io-free-core.md), [ADR-0003](../adr/0003-hidden-worker-subcommand.md), [ADR-0004](../adr/0004-callback-model-streaming-cancellation.md), [ADR-0005](../adr/0005-repo-scoped-sequential-task-ids.md)  
**Ticket:** [Define the domain model, ports, and application assembly (TDD-0002)](https://github.com/Xaidel/forge/issues/21)

---

## 1. Purpose and authority

This document fixes the exact core contract for Forge v1:

- the private `app_core::domain` model;
- the public `app_core::ports` surface;
- the port-owned state records and stable error taxonomy;
- the private `app_core::application` use cases and mapping rules;
- `ApplicationPorts`, `build_application`, and `Application`;
- the fakes and requirement-to-test mapping that prove NFR-MAINT-001 and the domain rules.

`docs/PRD.md` is authoritative for product behavior. `arch_docs/architecture.md`, `arch_docs/dependency-rules.md`, and `arch_docs/naming-conventions.md` are authoritative for boundaries, dependency direction, and names. When this document conflicts with a higher authority, the higher authority wins and this document is corrected. When this document conflicts with a later TDD about a port or record it fixes, this document wins until amended.

The exact JSON, YAML, TOML, JSONL, Git, provider, and terminal encodings are owned by TDD-0001, TDD-0003, TDD-0005, TDD-0006, TDD-0007, TDD-0008, TDD-0009, TDD-0010, and TDD-0011. This document fixes the Rust records those encodings carry.

---

## 2. Scope

### 2.1 In scope

- Private domain value objects, entities, events, stateless services, invariants, and the twelve-state task machine.
- Public inbound capabilities and outbound ports with exact method signatures.
- Public port records exchanged across each boundary.
- Public error enums and the `ApplicationError` mapping.
- Private use cases and the domain-to-port mapping.
- `ApplicationPorts`, `build_application`, and the `Application` capability handle.
- Test fakes for plan, gates, resume, review, attestation, merge, reopen, and override.
- Requirement-to-test mapping for NFR-MAINT-001 and the domain rules.

### 2.2 Out of scope

- CLI parsing, help text, exit-code mapping, and init flow — TDD-0001.
- JSON/JSONL session schema, event encoding, resume algorithm, atomic write, and fsync — TDD-0003.
- Plan materiality rules, policy triggers, gate content rules, and the policy engine implementation — TDD-0004.
- Decision and assumption YAML/Markdown schema, validation, index, and trailer parsing — TDD-0005.
- MXC policy JSON, worker IPC framing, permission store, trust file, and canaries — TDD-0006.
- Provider profile schema, wire mappings, price table, and budget arithmetic — TDD-0007.
- Git adapter mechanics, attestation JSON, signing, hooks, merge, and reopen mechanics — TDD-0008.
- TUI layout, ANSI tokens, plain mode, and terminal sanitization — TDD-0009 and TDD-0010.
- Security patterns, caps, secret scan, steering patterns, and egress rules — TDD-0010.
- Dogfood diary and metric formulas — TDD-0011.
- Implementation code, workspace scaffold, tests, and release artifacts. This document is a contract, not a delivery.

---

## 3. Constraints

- Rust 1.93, edition 2024.
- Workspace members: `crates/app_core` (package `forge-harness-core`, library crate `app_core`) and `crates/infra_local` (package `forge-harness`, binary crate `forge`).
- Only `app_core::ports` is public. `domain`, `application`, and `assembly` are private.
- `domain` depends on `std` and `thiserror` only. It performs no I/O and imports no other core module.
- `ports` depends on `std`, `async-trait`, `thiserror`, and optionally `futures-core`. This design does **not** require `futures-core`; the optional dependency is absent.
- `application` depends on `domain`, `ports`, `std`, `async-trait`, and `thiserror`.
- `assembly` depends on `domain`, `ports`, `application`, `std`, `async-trait`, and `thiserror`.
- No Tokio, Serde, UUID, chrono/time, rand, anyhow, tracing, Ratatui, crossterm, reqwest, MXC, Git, env, fs, net, or process types in the core.
- Time and identity arrive through `ClockPort` and `IdSourcePort`.
- Async ports use `#[async_trait]`, are object-safe, return `Send` futures, are cancellation-safe by drop, keep no request-scoped state, and hold no lock across `.await`.
- Test dev-dependency is exactly `pollster`. Fakes are test-only.
- The counts in this contract are **11 outbound ports** and **16 inbound capabilities**. An earlier grilling summary said "12 outbound"; that was an arithmetic slip. `ClockPort`, `IdSourcePort`, `ConfigPort`, `SessionStorePort`, `PolicyPort`, `LedgerPort`, `GitPort`, `ModelPort`, `SandboxPort`, `AttestationPort`, and `AuditPort` are the full set.

---

## 4. Module map

```text
crates/app_core/src/
  lib.rs                         # mod domain; pub mod ports; mod application; mod assembly;
  domain.rs
  domain/
    value_objects/
      task_id.rs
      task_state.rs
      ...
    entities/
      task.rs
      plan.rs
      step.rs
      decision.rs
      assumption.rs
      attestation.rs
      override.rs
      decision_policy.rs
      execution_policy.rs
      repo_map.rs
    events/
      session_event.rs
    services/
      policy_evaluator.rs
      risk_classifier.rs
      plan_validator.rs
      decision_coverage.rs
      gate_scheduler.rs
  ports.rs
  ports/
    application.rs               # public Application capability handle
    inbound.rs
    inbound/
      start_task.rs
      resume_task.rs
      retry_task.rs
      pause_task.rs
      cancel_task.rs
      reopen_task.rs
      merge_task.rs
      override_merge.rs
      answer_gate.rs
      open_review.rs
      attest_task.rs
      switch_model.rs
      increase_budget.rs
      query_status.rs
      query_transcript.rs
      query_audit.rs
    outbound.rs
    outbound/
      clock.rs
      id_source.rs
      config.rs
      session_store.rs
      policy.rs
      ledger.rs
      git.rs
      model.rs
      sandbox.rs
      attestation.rs
      audit.rs
    error.rs
    error/
      domain_error.rs
      session_error.rs
      config_error.rs
      policy_error.rs
      ledger_error.rs
      git_error.rs
      model_error.rs
      sandbox_error.rs
      attestation_error.rs
      audit_error.rs
      id_source_error.rs
      application_error.rs
    records.rs
    records/
      <one file per record in §6.1>
  application.rs
  application/
    mapping.rs
    start_task.rs
    resume_task.rs
    ...
  assembly.rs
```

`lib.rs` is exactly:

```rust
#![forbid(unsafe_code)]
#![deny(unreachable_pub)]
#![deny(missing_docs)]

mod domain;
pub mod ports;
mod application;
mod assembly;
```

`ports.rs` is exactly:

```rust
pub mod application;
pub mod error;
pub mod inbound;
pub mod outbound;
pub mod records;

pub use crate::assembly::{ApplicationPorts, build_application};
pub use application::Application;
```

`ports` never names `crate::domain` or `crate::application`. The only `crate::assembly` use is the `ApplicationPorts` and `build_application` re-exports above. `Application` is defined in `ports::application`, so it is not an `assembly` re-export. The architecture test enforces both.

---

## 5. Domain model

### 5.1 Ubiquitous language

The domain uses the PRD’s exact terms. It does not invent synonyms. The canonical terms are `Task`, `Session`, `Run`, `Step`, `Attempt`, `Gate`, `DecisionRequest`, `PermissionRequest`, `Grant`, `ReviewPacket`, `Attestation`, `TaskBranch`, `Worktree`, `BaseRef`, `SubjectCommit`, `AttestationCommit`, `Assumption`, `DecisionRecord`, `AssumptionRecord`, `DeferredDecision`, `DecisionMap`, `RiskClass`, `Override`, `PolicyTrigger`, `ProtectedPath`, `RepoMap`, `Facet`, `Check`, `ExecutionMode`, and `TrustDeclaration`.

`Session` is the persisted record of a task, 1:1 with it. `Run` is one process invocation and is not a persisted entity. `Attempt` is a count inside a step, verification, or viva record, not a top-level aggregate.

### 5.2 Private value objects

The private domain mirrors the public record vocabulary. The public record in §6 is the stable contract; the private type carries the same invariants and converts through `application::mapping`.

**Name collision rule.** The names in §5 are the **private domain names**. They are distinct Rust types from the public records in §6, even when the spelling is identical. A code block in §5 never imports from `ports`; `application::mapping` converts at the boundary. Every public record in §6 that appears in a domain signature has a private mirror with the same fields and invariants; the mirror set includes `TaskId`, `TaskGoal`, `TaskState`, `RiskClass`, `GateId`, `GateKind`, `GateStatus`, `DecisionId`, `AssumptionId`, `AttestationId`, `OverrideId`, `StepId`, `AcceptanceCriterionId`, `PlanVersion`, `EventSequence`, `Timestamp`, `ContentHash`, `BaseRef`, `BranchName`, `CommitSha`, `UsdMicros`, `Budget`, `Usage`, `Worktree`, `EvidenceRef`, `Gate`, `PlanGate`, `DecisionRequest`, `PermissionRequest`, `ReviewGate`, `GateAnswer`, `PlanAnswer`, `DecisionAnswer`, `PermissionAnswer`, `GateOutcome`, `VerificationEvidence`, `Decision`, `Attestation`, `Override`, `DecisionPolicy`, `ExecutionPolicy`, and `RepoMap`.

| Private type | Grammar or variants | Invariant |
|---|---|---|
| `TaskId` | `^T-[0-9]{3,}$` | Repo-scoped, monotonic, allocated after the active-task lock. |
| `TaskGoal` | non-empty, ≤ 10,000 Unicode scalar values | Trimmed; no control characters except newline and tab. |
| `TaskState` | `Intake`, `Planning`, `PlanReview`, `Executing`, `Decision`, `Permission`, `Verifying`, `Review`, `Paused`, `Stopped`, `Done`, `Cancelled` | Twelve states; `Done` and `Cancelled` are terminal except `Reopen`. |
| `RiskClass` | `Light`, `Full` | `Full` when any policy trigger fires, the task is multi-file or architectural, the approach is unknown, or the target is on a `proposed` or `unknown` facet. |
| `GateId` | `^G-T-[0-9]{3,}-[0-9]{1,}$` | Unique per task; allocated before the gate opens. |
| `GateKind` | `Plan`, `Decision`, `Permission`, `Review` | Matches the gate record. |
| `DecisionId` | `^DR-[0-9]{2}-[0-9]{2}-[0-9]{2}-[a-z0-9][a-z0-9-]{0,63}$` | Unique per ledger; duplicate reported by `LedgerPort::verify`. |
| `AssumptionId` | `^AS-[0-9]{2}-[0-9]{2}-[0-9]{2}-[a-z0-9][a-z0-9-]{0,63}$` | Same as decision IDs. |
| `AttestationId` | `^AT-T-[0-9]{3,}-v[0-9]+$` | Version increases; old versions are never rewritten. |
| `OverrideId` | `^OV-T-[0-9]{3,}-v[0-9]+$` | Version increases; old versions are never rewritten. |
| `StepId` | `S-<u32>` | Unique within a plan version. |
| `AcceptanceCriterionId` | `AC-<u32>` | Unique within a plan version. |
| `PlanVersion` | `u32`, starting at 1 | Monotonic per task; old versions retained. |
| `EventSequence` | `u64`, starting at 1 | Monotonic per task; 0 means “no events yet.” |
| `Timestamp` | `i64` Unix milliseconds UTC | From `ClockPort`; never read from the system clock in the core. |
| `ContentHash` | 64 lowercase hex characters | SHA-256 of the canonical bytes defined by the owning TDD. |
| `BaseRef` | non-empty Git ref name | The branch recorded at intake. |
| `BranchName` | non-empty valid Git ref name | `forge/<task-id>` for a task branch. |
| `CommitSha` | 40 or 64 lowercase hex characters | Git object ID; SHA-1 or SHA-256 repository. |
| `UsdMicros` | `i64` | USD × 1,000,000; negative values rejected. |
| `PathPattern` | non-empty string | A repo-relative path or glob accepted by the execution policy. |
| `ManualVerification` | `{ method, checklist }` | `method` non-empty; `checklist` non-empty when manual verification is selected. |
| `RationaleProvenance` | `{ author, method, captured_at }` | `author` is the human principal; `method` is `Typed`, `Dictated`, or `Imported`. |
| `Reversibility` | `Easy`, `Hard` | `Hard` forces a full decision record. |
| `BlastRadius` | `Local`, `Module`, `System`, `External` | `System` or `External` forces a full decision record. |

### 5.3 Entities

#### 5.3.1 `Task` — aggregate root

`Task` owns identity, lifecycle, pinned hashes, the active plan, the decision map, open gates, assumptions, budget, usage, and terminal references. It is the only aggregate root. It performs no I/O.

```rust
pub(crate) struct Task {
    pub(crate) id: TaskId,
    pub(crate) goal: TaskGoal,
    pub(crate) state: TaskState,
    pub(crate) previous_state: Option<TaskState>, // for pause/resume
    pub(crate) repo_root: PathBuf,
    pub(crate) base_ref: BaseRef,
    pub(crate) base_commit: CommitSha,
    pub(crate) worktree: Worktree,
    pub(crate) risk_class: RiskClass,
    pub(crate) config_hash: ContentHash,
    pub(crate) policy_hash: ContentHash,
    pub(crate) model_hash: ContentHash,
    pub(crate) execution_policy_hash: ContentHash,
    pub(crate) plan: Option<Plan>,
    pub(crate) gates: Vec<Gate>,
    pub(crate) assumptions: Vec<Assumption>,
    pub(crate) budget: Budget,
    pub(crate) usage: Usage,
    pub(crate) attestation_id: Option<AttestationId>,
    pub(crate) merge_commit: Option<CommitSha>,
    pub(crate) override_id: Option<OverrideId>,
    pub(crate) created_at: Timestamp,
    pub(crate) updated_at: Timestamp,
}

pub(crate) struct PinnedHashes {
    pub(crate) config: ContentHash,
    pub(crate) policy: ContentHash,
    pub(crate) model: ContentHash,
    pub(crate) execution_policy: ContentHash,
}
```

Methods:

```rust
impl Task {
    pub(crate) fn start(
        id: TaskId,
        goal: TaskGoal,
        repo_root: PathBuf,
        base_ref: BaseRef,
        base_commit: CommitSha,
        worktree: Worktree,
        pinned: PinnedHashes,
        budget: Budget,
        now: Timestamp,
    ) -> Result<Self, DomainError>;

    pub(crate) fn transition(&mut self, to: TaskState, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn record_plan(&mut self, plan: Plan, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn approve_plan(&mut self, version: PlanVersion, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn open_gate(&mut self, gate: Gate, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn answer_gate(&mut self, gate_id: &GateId, answer: GateAnswer, now: Timestamp)
        -> Result<GateOutcome, DomainError>;

    pub(crate) fn record_decision(&mut self, decision: Decision, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn record_assumption(&mut self, assumption: Assumption, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn record_verification(&mut self, evidence: VerificationEvidence, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn attest(&mut self, attestation_id: AttestationId, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn record_merge(&mut self, merge_commit: CommitSha, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn record_override(&mut self, override_id: OverrideId, now: Timestamp)
        -> Result<(), DomainError>;

    pub(crate) fn reopen(&mut self, reason: &str, now: Timestamp)
        -> Result<(), DomainError>;
}
```

`Task::transition` is the only state mutator. It enforces the table in §5.5. Any other method that changes state calls `transition`.

#### 5.3.2 `Plan`

```rust
pub(crate) struct Plan {
    pub(crate) version: PlanVersion,
    pub(crate) goal: TaskGoal,
    pub(crate) base_commit: CommitSha,
    pub(crate) acceptance_criteria: Vec<AcceptanceCriterion>,
    pub(crate) steps: Vec<Step>,
    pub(crate) risk_class: RiskClass,
    pub(crate) policy_triggers: Vec<PolicyTrigger>,
    pub(crate) decision_map: DecisionMap,
    pub(crate) assumptions: Vec<Assumption>,
    pub(crate) hash: ContentHash,
    pub(crate) cause: PlanRevisionCause,
    pub(crate) previous_version: Option<PlanVersion>,
    pub(crate) created_at: Timestamp,
}
```

`Plan` methods:

```rust
impl Plan {
    pub(crate) fn validate(&self) -> Result<(), DomainError>;
    pub(crate) fn is_fog_clear(&self) -> Result<(), DomainError>;
    pub(crate) fn is_material_revision_against(&self, previous: &Plan) -> bool;
}
```

`Plan::validate` requires every step to name an owning `OwningUnit`, a `VerificationSet`, a status, and evidence refs when complete. A missing owning unit is `DomainError::MissingOwningUnit`; it is a blocking decision, never an inferred owner.

#### 5.3.3 `Step` and `AcceptanceCriterion`

```rust
pub(crate) struct Step {
    pub(crate) id: StepId,
    pub(crate) intent: String,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) owner: OwningUnit,
    pub(crate) verification: VerificationSet,
    pub(crate) status: StepStatus,
    pub(crate) evidence: Vec<EvidenceRef>,
}

pub(crate) struct AcceptanceCriterion {
    pub(crate) id: AcceptanceCriterionId,
    pub(crate) text: String,
    pub(crate) verification: VerificationSet,
}
```

#### 5.3.4 `DecisionMap` and `Assumption`

```rust
pub(crate) struct DecisionMap {
    pub(crate) entries: Vec<DecisionMapEntry>,
}

pub(crate) struct DecisionMapEntry {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) status: DecisionMapStatus,
    pub(crate) blocking: bool,
    pub(crate) trigger: Option<PolicyTrigger>,
    pub(crate) decision_id: Option<DecisionId>,
    pub(crate) revisit_if: Option<String>,
}

pub(crate) struct Assumption {
    pub(crate) id: AssumptionId,
    pub(crate) title: String,
    pub(crate) statement: String,
    pub(crate) basis: String,
    pub(crate) risk: AssumptionRisk,
    pub(crate) revisit_if: String,
    pub(crate) status: AssumptionStatus,
    pub(crate) evidence: Vec<EvidenceRef>,
    pub(crate) created_at: Timestamp,
}

pub(crate) struct Decision {
    pub(crate) id: DecisionId,
    pub(crate) task_id: TaskId,
    pub(crate) title: String,
    pub(crate) chosen_option: String,
    pub(crate) rationale: String,
    pub(crate) provenance: RationaleProvenance,
    pub(crate) reversibility: Reversibility,
    pub(crate) blast_radius: BlastRadius,
    pub(crate) policy: Option<PolicyProvenance>,
    pub(crate) refs: Vec<TypedRef>,
    pub(crate) status: DecisionStatus,
    pub(crate) created_at: Timestamp,
}

pub(crate) struct VerificationEvidence {
    pub(crate) evidence_ref: EvidenceRef,
    pub(crate) check_id: Option<CheckId>,
    pub(crate) outcome: VerificationOutcome,
    pub(crate) command: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) tree_hash: ContentHash,
    pub(crate) started_at: Timestamp,
    pub(crate) finished_at: Timestamp,
    pub(crate) manual: bool,
}
```

`DecisionMap::has_blocking` is true when any entry is `planned`, `requested`, or `deferred` with `blocking: true`. `Execution` cannot start while it is true.

#### 5.3.5 `Attestation` and `Override`

```rust
pub(crate) struct Attestation {
    pub(crate) id: AttestationId,
    pub(crate) task_id: TaskId,
    pub(crate) version: u32,
    pub(crate) subject_commit: CommitSha,
    pub(crate) subject_tree: ContentHash,
    pub(crate) diff_hash: ContentHash,
    pub(crate) plan_version: PlanVersion,
    pub(crate) plan_hash: ContentHash,
    pub(crate) policy_hash: ContentHash,
    pub(crate) ledger_hashes: Vec<LedgerBinding>,
    pub(crate) evidence_hashes: Vec<EvidenceBinding>,
    pub(crate) questions: Vec<VivaQuestion>,
    pub(crate) judge: JudgeMetadata,
    pub(crate) signing: SigningScheme,
    pub(crate) supersedes: Vec<AttestationId>,
    pub(crate) repo_trust: TrustDeclaration,
    pub(crate) execution_mode: ExecutionMode,
    pub(crate) untrusted_content: Vec<UntrustedContentRef>,
    pub(crate) steering_attempts: Vec<SteeringAttempt>,
    pub(crate) protected_paths: Vec<ProtectedPath>,
    pub(crate) fourth_anchor: FourthAnchorState,
    pub(crate) created_at: Timestamp,
}

pub(crate) struct Override {
    pub(crate) id: OverrideId,
    pub(crate) task_id: TaskId,
    pub(crate) version: u32,
    pub(crate) subject_commit: CommitSha,
    pub(crate) base_ref: BaseRef,
    pub(crate) base_commit: CommitSha,
    pub(crate) reason: String,
    pub(crate) attester: AttesterIdentity,
    pub(crate) key_fingerprint: String,
    pub(crate) bypassed_gates: Vec<GateKind>,
    pub(crate) created_at: Timestamp,
}
```

`Attestation` invariants: the subject commit and tree, diff hash, plan hash, policy hash, ledger hashes, evidence hashes, question set, and rubric hash are immutable. A re-attestation creates a new version with `supersedes`. `Override` invariants: the reason, subject, base, attester, and key fingerprint are immutable.

### 5.4 Domain events

The core owns the fact vocabulary. `SessionEvent` is a public port record (§6), not a private domain type, because the session store is a port. The private domain emits `DomainEvent` values that `application` maps to `SessionEvent`. The initial `DomainEvent` set is identical in kind to `SessionEventKind` in §6.11; the application mapping is one-to-one.

### 5.5 Task state machine

Allowed transitions are exactly:

| From | To | Guard |
|---|---|---|
| `Intake` | `Planning` | Task record, base commit, lock, and config resolved |
| `Intake` | `Stopped` | Repo cannot support a task |
| `Intake` | `Cancelled` | Human cancels |
| `Planning` | `PlanReview` | Plan is `Full` risk |
| `Planning` | `Executing` | Plan is `Light` risk and no blocking decision remains |
| `Planning` | `Stopped` | Repeated planning failure |
| `Planning` | `Cancelled` | Human cancels |
| `PlanReview` | `Executing` | Plan approved |
| `PlanReview` | `Planning` | Changes requested |
| `PlanReview` | `Cancelled` | Human cancels |
| `Executing` | `Verifying` | Every step done or explicitly skipped; acceptance evidence collected |
| `Executing` | `Decision` | Policy trigger or agent-raised consequential decision |
| `Executing` | `Permission` | Sandbox permission request |
| `Executing` | `Paused` | Budget, context, or manual stop |
| `Executing` | `Stopped` | Fatal error or unrecoverable tool failure |
| `Executing` | `Cancelled` | Human cancels |
| `Decision` | `Executing` | Answered; plan unchanged |
| `Decision` | `Planning` | Answered; material plan change |
| `Decision` | `Paused` | Deferred |
| `Decision` | `Cancelled` | Human cancels |
| `Permission` | `Executing` | Granted, or an alternative is approved |
| `Permission` | `Decision` | Denial requires an alternative choice |
| `Permission` | `Paused` | Deferred |
| `Permission` | `Cancelled` | Human cancels |
| `Verifying` | `Review` | All checks pass |
| `Verifying` | `Executing` | Retryable failure within budget |
| `Verifying` | `Decision` | Failure needs a choice |
| `Verifying` | `Stopped` | Retries exhausted |
| `Verifying` | `Paused` | Budget, context, or manual stop |
| `Verifying` | `Cancelled` | Human cancels |
| `Review` | `Done` | Attested |
| `Review` | `Executing` | Rejected with minor feedback |
| `Review` | `Planning` | Rejected with material feedback |
| `Review` | `Cancelled` | Human cancels |
| `Paused` | previous state | Resumed |
| `Paused` | `Stopped` | Resume cannot proceed |
| `Paused` | `Cancelled` | Human cancels |
| `Stopped` | `Planning` or `Executing` | Explicit human retry |
| `Stopped` | `Cancelled` | Human cancels |
| `Done` | `Executing` | `forge reopen` with a reason; the only path out of `Done` |

`Cancelled` has no outgoing transition. `Done` has exactly one, `Reopen`. A transition not in this table is `DomainError::InvalidTaskTransition`.

### 5.6 Domain services

All domain services are stateless, synchronous, and I/O-free.

| Service | Responsibility | Contract |
|---|---|---|
| `PolicyEvaluator` | Evaluate a planned or attempted action against `DecisionPolicy` | `fn evaluate(&self, policy: &DecisionPolicy, action: &PlannedAction) -> PolicyEvaluation` |
| `RiskClassifier` | Classify a plan as `Light` or `Full` | `fn classify(&self, plan: &Plan, policy: &DecisionPolicy, map: &RepoMap) -> RiskClassification` |
| `PlanValidator` | Validate plan completeness and fog-clear | `fn validate(&self, plan: &Plan) -> Result<(), DomainError>` |
| `DecisionCoverage` | Check required decisions against the decision map | `fn coverage(&self, plan: &Plan, changed_paths: &[PathBuf]) -> DecisionCoverageReport` |
| `GateScheduler` | Decide the next gate and whether a gate is blocking | `fn next_gate(&self, task: &Task, answer: GateAnswer) -> Result<Option<Gate>, DomainError>` |

`PolicyEvaluator` takes private `DecisionPolicy`, `PlannedAction`, and `PolicyEvaluation`; `application` maps the public `DecisionPolicySnapshot` to `DecisionPolicy`. TDD-0004 owns the exact trigger rules and the `PolicyEvaluation` fields; this document fixes the service boundary and the fact that the model cannot override a policy trigger.

```rust
pub(crate) struct PlannedAction {
    pub(crate) description: String,
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) operation: PlannedOperation,
}

pub(crate) enum PlannedOperation {
    FileWrite,
    ShellCommand,
    NetworkAccess,
    DependencyChange,
    Deletion,
    ConfigChange,
}

pub(crate) struct PolicyEvaluation {
    pub(crate) triggers: Vec<PolicyTrigger>,
    pub(crate) requires_decision: bool,
    pub(crate) full_record: bool,
}

pub(crate) struct RiskClassification {
    pub(crate) risk_class: RiskClass,
    pub(crate) reasons: Vec<String>,
}

pub(crate) struct DecisionCoverageReport {
    pub(crate) required: u32,
    pub(crate) covered: u32,
    pub(crate) missing: Vec<String>,
}
```

`DecisionPolicy`, `ExecutionPolicy`, and `RepoMap` are private entities mapped from `DecisionPolicySnapshot`, `ExecutionPolicySnapshot`, and `RepoMapSnapshot`. Their field sets are the same as the public snapshots plus the domain rules TDD-0004 and TDD-0006 add; they never carry serde or adapter types.

### 5.7 Domain invariants

The domain enforces every invariant that does not require I/O. The application and adapters enforce the rest.

| Invariant | Core enforcement |
|---|---|
| INV-01 one active task per repo | `StartTaskUseCase` acquires `SessionStorePort::acquire_lock` before constructing the task; `Task::start` is only called by that use case |
| INV-02 one open task branch per repo | `StartTaskUseCase` checks the lock and branch before `GitPort::create_worktree` |
| INV-03 pending requests never enter the ledger | `Task::open_gate` stores `Gate`; only `Task::record_decision` produces a `DecisionRecord` |
| INV-04 rationale is human-authored and verbatim | `DecisionAnswer::Choose` requires a non-empty `rationale` and `RationaleProvenance.author`; `DomainError::RationaleAuthoredByAgent` rejects agent provenance |
| INV-05 host writes ledger, commits, attestations, merges | The core assembles the records; only `LedgerPort`, `GitPort`, and `AttestationPort` write |
| INV-06 `Done` requires human attestation | `Task::attest` is only callable from `AttestTask`; no agent capability exists |
| INV-07 attestation binds `C` and `C2` with `C2^ == C` | `Attestation` records both; `MergeTaskUseCase` verifies through `GitPort` and `AttestationPort` |
| INV-08 merge only through `forge merge` or signed override | `MergeTask` and `OverrideMerge` are the only inbound capabilities that call `GitPort::merge` |
| INV-09 no gate timeout or auto-approve | `Gate` has no deadline and no default; time never changes a gate |
| INV-10 `Sandboxed` only after canaries pass | `StartTaskUseCase` requires `SandboxProbeReport` with all six canaries passing |
| INV-11 untrusted repo requires `Sandboxed` | `StartTaskUseCase` rejects `Guarded` with `DomainError::TrustViolation` |
| INV-12 `Guarded` is never called a sandbox | `ExecutionMode::Guarded` displays `Guarded — not sandboxed` |
| INV-13 only human task and system prompt are instruction sources | `ModelRequest` wraps every repository, tool, web, commit-message, and model-generated block in `SourceEnvelope` |
| INV-14 protected-path write requires a decision | `PolicyEvaluator` emits `ProtectedPathWrite`; `Task` refuses to reach `Done` with an unresolved protected-path change |
| INV-15 ledger substance is immutable | `DecisionRecord::rationale` and chosen option are immutable; corrections are `Erratum` or `supersedes` |
| INV-16 newer schema is read-only | Every `*Snapshot` and `*Record` carries `schema_version`; adapters return `SchemaTooNew` |
| INV-17 one repo per session | `TaskSnapshot.base_ref` and `repo_root` are fixed at intake; no cross-repo API exists |
| INV-18 no evidence means no `verified` | `Task::record_verification` requires a non-empty `evidence_ref` and `tree_hash`; `Task` refuses `Review` when a required criterion has no evidence |
| INV-19 unknown usage is explicit | `Usage` uses `Option<u64>` for cache and reasoning tokens; any cost record added by TDD-0007 uses `Option<UsdMicros>` and never zero for unknown |
| INV-20 no telemetry, secrets, reasoning, or customer data in commits | The `LedgerPort` and `AttestationPort` adapters run the secret scan before writing; the core records only redacted excerpts and never stores raw model reasoning in a committed record |

---

## 6. Public port records

### 6.1 Record index and justification

Every record below is public because it crosses a port boundary or appears in `ApplicationPorts`, `Application`, an inbound request/response, or an error. Records that do not cross a boundary stay private.

| Area | Public records |
|---|---|
| Identity and time | `TaskId`, `TaskGoal`, `TaskState`, `RiskClass`, `GateId`, `GateKind`, `DecisionId`, `AssumptionId`, `AttestationId`, `OverrideId`, `StepId`, `AcceptanceCriterionId`, `PlanVersion`, `EventSequence`, `Timestamp`, `ContentHash`, `BaseRef`, `BranchName`, `CommitSha`, `UsdMicros` |
| Task and session | `TaskSnapshot`, `SessionSnapshot`, `TaskProgress`, `ActiveTaskLock`, `EvidenceArtifact`, `ArtifactRef`, `EvidenceRef` |
| Plan | `PlanSnapshot`, `AcceptanceCriterion`, `StepSnapshot`, `StepStatus`, `OwningUnit`, `VerificationSet`, `ManualVerification`, `CheckId`, `DecisionMapSnapshot`, `DecisionMapEntry`, `DecisionMapStatus`, `AssumptionSnapshot`, `AssumptionRisk`, `AssumptionStatus`, `PlanRevisionCause` |
| Gates | `Gate`, `PlanGate`, `DecisionRequest`, `DecisionOption`, `PermissionRequest`, `PermissionAction`, `PermissionScope`, `PermissionScopeOffer`, `Grant`, `ReviewGate`, `GateStatus`, `GateAnswer`, `PlanAnswer`, `DecisionAnswer`, `PermissionAnswer`, `GateOutcome`, `RationaleProvenance`, `RationaleMethod`, `Reversibility`, `BlastRadius` |
| Review | `ReviewPacket`, `ReviewSummary`, `DiffSnapshot`, `DiffFile`, `DiffStatus`, `ReviewFlag`, `PlanVsActual`, `PlanVsActualEntry`, `VerificationEvidence`, `VerificationOutcome`, `PermissionSummary`, `PermissionGrantRecord`, `RepoConfidence`, `FacetConfidence`, `FacetKind`, `FacetStatus`, `CheckConfidence`, `CheckRunStatus`, `RepoMapUpdate`, `PackageGroup`, `CheckStatus`, `UntrustedContentRef`, `UntrustedContentKind`, `SteeringAttempt`, `SteeringClass`, `SteeringDetector`, `ProtectedPath` |
| Attestation | `AttestationRecord`, `AttestationVerification`, `AttesterIdentity`, `VivaQuestionRecord`, `VivaAnchor`, `VivaAttempt`, `VivaVerdict`, `JudgeMetadata`, `SigningScheme`, `SigningKind`, `LedgerBinding`, `EvidenceBinding`, `FourthAnchorState`, `OverrideRecord`, `MergeOutcome`, `MergeResult`, `Trailer` |
| Ledger | `DecisionRecord`, `DecisionOptionRecord`, `DecisionStatus`, `AssumptionRecord`, `RecordKind`, `DetailLevel`, `Visibility`, `TypedRef`, `RefKind`, `PolicyProvenance`, `Erratum`, `LedgerVerificationReport`, `LedgerIndex`, `LedgerIndexEntry` |
| Config and policy | `ConfigSnapshot`, `RepoMapSnapshot`, `PackageRoot`, `EntryPoint`, `ModuleRoot`, `ContextDoc`, `FacetSpec`, `VerificationChoice`, `CheckDefinition`, `CheckKind`, `ModelSlotBinding`, `DecisionPolicySnapshot`, `PolicyRule`, `PolicyTrigger`, `PolicyTriggerKind`, `TriggerSource`, `FullRecordRule`, `ExecutionPolicySnapshot`, `TrustDeclaration`, `SandboxProbeRequest`, `SandboxProbeReport`, `SandboxProbeCheck`, `CanaryKind`, `ExecutionMode` |
| Model | `ModelRequest`, `ModelResponse`, `ModelMessage`, `ModelRole`, `ContentBlock`, `SourceEnvelope`, `ToolDefinition`, `ToolCall`, `ToolCallDelta`, `ModelParameters`, `StopReason`, `ModelEventSink`, `ModelInfo`, `ProfileValidation`, `Usage`, `ProviderAllowance`, `ModelSlot`, `ProfileName`, `ModelId`, `ModelSelection` |
| Sandbox and tools | `ToolRequest`, `ToolResult`, `ToolEventSink`, `ToolEvent`, `OperationId`, `ModelCallId`, `Grant`, `SandboxProbeRequest`, `FileChange` |
| Reports | `StatusReport`, `TranscriptReport`, `AuditReport`, `AuditEntry`, `AuditEntryKind`, `AuditFinding`, `EnforcementStatus`, `HooksStatus`, `HookState`, `HookKind` |
| Inbound requests and responses | `StartTaskRequest`, `ResumeTaskRequest`, `RetryTaskRequest`, `PauseTaskRequest`, `CancelTaskRequest`, `ReopenTaskRequest`, `MergeTaskRequest`, `OverrideMergeRequest`, `AnswerGateRequest`, `OpenReviewRequest`, `StartAttestationRequest`, `AnswerAttestationRequest`, `SubmitAttestationRequest`, `SwitchModelRequest`, `IncreaseBudgetRequest`, `QueryStatusRequest`, `QueryTranscriptRequest`, `QueryAuditRequest`, `VivaPrompt`, `VivaProgress` |
| Assembly and errors | `ApplicationPorts`, `Application`, `DomainError`, `SessionError`, `ConfigError`, `PolicyError`, `LedgerError`, `GitError`, `ModelError`, `SandboxError`, `AttestationError`, `AuditError`, `IdSourceError`, `ApplicationError` |
| Git and worktree | `Worktree`, `WorktreeStatus`, `DiffSummary`, `SignatureVerification`, `MergePreflight`, `CommitRequest`, `AttestationCommitRequest`, `MergeRequest`, `HookInstallRequest`, `HookInstallReport` |

**Constructor rule.** Every identity, name, and hash newtype exposes a validating `try_new` and an `as_str` accessor. Construction from an invalid value is a typed error. The primitive wrappers `Timestamp`, `UsdMicros`, `PlanVersion`, `EventSequence`, `StepId`, `AcceptanceCriterionId`, and `CheckId` are public tuple structs; their owning type validates semantic bounds. `TaskGoal`, `BaseRef`, `BranchName`, `CommitSha`, `ContentHash`, `GateId`, `DecisionId`, `AssumptionId`, `AttestationId`, `OverrideId`, `ProfileName`, `ModelId`, `ModelCallId`, `OperationId`, and `SteeringClass` have private fields. Adapters construct them through `try_new`; they never mutate the inner string.

**Extensibility rule.** Public enums that a later TDD may extend are `#[non_exhaustive]`: `SessionEventKind`, `PolicyTriggerKind`, `ReviewFlag`, `StopReason`, `PermissionAction`, `PlanRevisionCause`, `ModelError`, and the other error enums. `TaskState`, `RiskClass`, `GateKind`, `GateStatus`, `DecisionMapStatus`, `AssumptionStatus`, `ExecutionMode`, `TrustDeclaration`, `Visibility`, `DetailLevel`, `RecordKind`, `RefKind`, `VerificationOutcome`, `CheckKind`, `ModelSlot`, `HookKind`, `HookState`, `CanaryKind`, `SteeringDetector`, `VivaAnchor`, `VivaVerdict`, `SigningKind`, `FourthAnchorState`, `FacetKind`, `FacetStatus`, and `CheckRunStatus` are exhaustive because the PRD fixes their values. Adding a value to an exhaustive enum is an amendment to this document.

### 6.2 Identity and time

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(String);

impl TaskId {
    pub fn try_new(value: impl Into<String>) -> Result<Self, DomainError>;
    pub fn as_str(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskGoal(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaskState {
    Intake,
    Planning,
    PlanReview,
    Executing,
    Decision,
    Permission,
    Verifying,
    Review,
    Paused,
    Stopped,
    Done,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RiskClass { Light, Full }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GateId(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GateKind { Plan, Decision, Permission, Review }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DecisionId(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssumptionId(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttestationId(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OverrideId(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AcceptanceCriterionId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlanVersion(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventSequence(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(pub i64);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContentHash(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseRef(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BranchName(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommitSha(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UsdMicros(pub i64);
```

All `try_new` constructors validate the grammars in §5.2 and return `DomainError::InvalidValue { field, value }` on failure. `Timestamp` has no `now`; only `ClockPort` produces it. `EventSequence` has no constructor that reads storage.

### 6.3 Task, session, and progress

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: BranchName,
    pub base_commit: CommitSha,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    pub soft: Option<UsdMicros>,
    pub hard: Option<UsdMicros>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub provider_allowance: Option<ProviderAllowance>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderAllowance {
    pub label: String,
    pub remaining: Option<String>,
    pub captured_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSnapshot {
    pub task_id: TaskId,
    pub goal: TaskGoal,
    pub state: TaskState,
    pub repo_root: PathBuf,
    pub base_ref: BaseRef,
    pub base_commit: CommitSha,
    pub worktree: Worktree,
    pub risk_class: RiskClass,
    pub config_hash: ContentHash,
    pub policy_hash: ContentHash,
    pub model_hash: ContentHash,
    pub execution_policy_hash: ContentHash,
    pub active_plan_version: Option<PlanVersion>,
    pub budget: Budget,
    pub usage: Usage,
    pub attestation_id: Option<AttestationId>,
    pub merge_commit: Option<CommitSha>,
    pub override_id: Option<OverrideId>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub task: TaskSnapshot,
    pub plan: Option<PlanSnapshot>,
    pub gates: Vec<Gate>,
    pub assumptions: Vec<AssumptionSnapshot>,
    pub sequence: EventSequence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskProgress {
    pub task: TaskSnapshot,
    pub plan: Option<PlanSnapshot>,
    pub gates: Vec<Gate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveTaskLock {
    pub task_id: TaskId,
    pub pid: u32,
    pub host: String,
    pub acquired_at: Timestamp,
    pub heartbeat_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    pub artifact_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub path: PathBuf,
    pub content_hash: ContentHash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceArtifact {
    pub evidence_ref: EvidenceRef,
    pub task_id: TaskId,
    pub check_id: Option<CheckId>,
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
    pub started_at: Timestamp,
    pub finished_at: Timestamp,
    pub tree_hash: ContentHash,
    pub environment_hash: ContentHash,
    pub truncated: bool,
}
```

`TaskSnapshot` is the durable projection. `SessionSnapshot` adds the active plan, open gates, open assumptions, and the event sequence. `TaskProgress` is the standard response for lifecycle capabilities.

### 6.4 Plan

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StepStatus { Pending, InProgress, Done, Skipped, Failed }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwningUnit {
    Module(String),
    EntryPoint(String),
    PackageRoot(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationSet {
    pub checks: Vec<CheckId>,
    pub manual: Option<ManualVerification>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualVerification {
    pub method: String,
    pub checklist: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterion {
    pub id: AcceptanceCriterionId,
    pub text: String,
    pub verification: VerificationSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepSnapshot {
    pub id: StepId,
    pub intent: String,
    pub files: Vec<PathBuf>,
    pub owner: OwningUnit,
    pub verification: VerificationSet,
    pub status: StepStatus,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecisionMapStatus {
    Scheduled,
    Requested,
    Resolved,
    Deferred,
    OutOfScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionMapEntry {
    pub key: String,
    pub title: String,
    pub status: DecisionMapStatus,
    pub blocking: bool,
    pub trigger: Option<PolicyTrigger>,
    pub decision_id: Option<DecisionId>,
    pub revisit_if: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionMapSnapshot {
    pub entries: Vec<DecisionMapEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssumptionRisk { Low, Medium, High }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssumptionStatus { Assumed, Validated, Invalidated, Superseded }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssumptionSnapshot {
    pub assumption_id: AssumptionId,
    pub task_id: TaskId,
    pub title: String,
    pub statement: String,
    pub basis: String,
    pub risk: AssumptionRisk,
    pub revisit_if: String,
    pub status: AssumptionStatus,
    pub evidence: Vec<EvidenceRef>,
    pub supersedes: Vec<AssumptionId>,
    pub superseded_by: Option<AssumptionId>,
    pub visibility: Visibility,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanRevisionCause {
    Initial,
    HumanFeedback { feedback: String },
    Decision { decision_id: DecisionId },
    VerificationFailure { evidence: EvidenceRef },
    PolicyTrigger { trigger: PolicyTrigger },
    RepoMapUpdate { hash: ContentHash },
    Other { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanSnapshot {
    pub task_id: TaskId,
    pub version: PlanVersion,
    pub goal: TaskGoal,
    pub base_commit: CommitSha,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub steps: Vec<StepSnapshot>,
    pub risk_class: RiskClass,
    pub policy_triggers: Vec<PolicyTrigger>,
    pub decision_map: DecisionMapSnapshot,
    pub assumptions: Vec<AssumptionSnapshot>,
    pub hash: ContentHash,
    pub cause: PlanRevisionCause,
    pub previous_version: Option<PlanVersion>,
    pub created_at: Timestamp,
}
```

### 6.5 Gates

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gate {
    Plan(PlanGate),
    Decision(DecisionRequest),
    Permission(PermissionRequest),
    Review(ReviewGate),
}

impl Gate {
    pub fn gate_id(&self) -> &GateId;
    pub fn kind(&self) -> GateKind;
    pub fn status(&self) -> GateStatus;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GateStatus { Awaiting, Deferred }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanGate {
    pub gate_id: GateId,
    pub task_id: TaskId,
    pub plan_version: PlanVersion,
    pub opened_at: Timestamp,
    pub status: GateStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionOption {
    pub label: String,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reversibility { Easy, Hard }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlastRadius { Local, Module, System, External }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionRequest {
    pub gate_id: GateId,
    pub task_id: TaskId,
    pub decision_key: String,
    pub question: String,
    pub why_now: String,
    pub context: String,
    pub options: Vec<DecisionOption>,
    pub recommendation: Option<usize>,
    pub reversibility: Reversibility,
    pub blast_radius: BlastRadius,
    pub opened_at: Timestamp,
    pub status: GateStatus,
    pub deferred_revisit_if: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionAction {
    Command(String),
    NetworkHost(String),
    ExternalRead(PathBuf),
    ExternalWrite(PathBuf),
    CredentialRead(PathBuf),
    DestructiveCommand(String),
    HostExec(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PermissionScope { Once, ThisTask }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionScopeOffer {
    pub scope: PermissionScope,
    pub allowed: bool,
    pub reason_if_not: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionRequest {
    pub gate_id: GateId,
    pub task_id: TaskId,
    pub action: PermissionAction,
    pub working_directory: PathBuf,
    pub execution_mode: ExecutionMode,
    pub reason: String,
    pub denial_consequence: String,
    pub scopes: Vec<PermissionScopeOffer>,
    pub opened_at: Timestamp,
    pub status: GateStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub scope: PermissionScope,
    pub action: PermissionAction,
    pub granted_at: Timestamp,
    pub granted_by: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewGate {
    pub gate_id: GateId,
    pub task_id: TaskId,
    pub packet: ReviewPacket,
    pub opened_at: Timestamp,
    pub status: GateStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateAnswer {
    Plan(PlanAnswer),
    Decision(DecisionAnswer),
    Permission(PermissionAnswer),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanAnswer {
    Approve,
    Reject { feedback: String, material: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionAnswer {
    Choose {
        option_index: usize,
        rationale: String,
        provenance: RationaleProvenance,
    },
    Defer { revisit_if: String },
    Delegate { question: String },
    Reject { feedback: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionAnswer {
    AllowOnce,
    AllowThisTask,
    Deny { reason: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RationaleMethod { Typed, Dictated, Imported }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationaleProvenance {
    pub author: String,
    pub method: RationaleMethod,
    pub captured_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateOutcome {
    PlanApproved { version: PlanVersion },
    PlanRejected { return_to: TaskState },
    DecisionRecorded { decision_id: DecisionId },
    DecisionDeferred { revisit_if: String },
    DecisionDelegated { question: String },
    DecisionRejected { return_to: TaskState },
    PermissionGranted { grant: Grant },
    PermissionDenied { reason: String },
}
```

`DecisionAnswer::Choose` with an empty rationale is `DomainError::RationaleEmpty`. A `RationaleProvenance.author` of `"agent"`, `"assistant"`, `"forge"`, or `"model"` is rejected with `DomainError::RationaleAuthoredByAgent`. `DecisionAnswer::Defer` requires a non-empty `revisit_if`.

### 6.6 Review records

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReviewFlag {
    Binary,
    Large,
    Generated,
    Symlink,
    ModeChange,
    GitAttributes,
    Ci,
    BuildConfig,
    InstructionDoc,
    Manifest,
    Lockfile,
    ProtectedPath,
    BidiOrAnsi,
    Vendored,
    Submodule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffFile {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub status: DiffStatus,
    pub additions: u32,
    pub deletions: u32,
    pub is_binary: bool,
    pub is_large: bool,
    pub flags: Vec<ReviewFlag>,
    pub patch: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSnapshot {
    pub base_commit: CommitSha,
    pub head_commit: CommitSha,
    pub files: Vec<DiffFile>,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSummary {
    pub files: Vec<PathBuf>,
    pub additions: u32,
    pub deletions: u32,
    pub flags: Vec<ReviewFlag>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewSummary {
    pub changed_files: u32,
    pub additions: u32,
    pub deletions: u32,
    pub check_count: u32,
    pub core_diff_lines: u32,
    pub decision_map_summary: String,
    pub assumption_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanVsActualEntry {
    pub step_id: StepId,
    pub planned: String,
    pub actual: String,
    pub status: StepStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanVsActual {
    pub entries: Vec<PlanVsActualEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VerificationOutcome {
    Passed,
    Failed,
    Skipped,
    PreExistingFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub evidence_ref: EvidenceRef,
    pub check_id: Option<CheckId>,
    pub outcome: VerificationOutcome,
    pub command: String,
    pub exit_code: Option<i32>,
    pub tree_hash: ContentHash,
    pub started_at: Timestamp,
    pub finished_at: Timestamp,
    pub manual: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionGrantRecord {
    pub grant: Grant,
    pub requested_action: PermissionAction,
    pub decision: PermissionAnswer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionSummary {
    pub grants: Vec<PermissionGrantRecord>,
    pub denials: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FacetKind { Checks, EntryPoints, ModuleRoots, Docs, PackageRoots }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FacetStatus { Confirmed, Proposed, Unknown }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetConfidence {
    pub facet: FacetKind,
    pub status: FacetStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckRunStatus { Verified, Manual, NeverRun }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckConfidence {
    pub check_id: CheckId,
    pub status: CheckRunStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoMapUpdate {
    pub previous_hash: ContentHash,
    pub proposed_hash: ContentHash,
    pub diff: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoConfidence {
    pub repo_map_version: u32,
    pub repo_map_hash: ContentHash,
    pub facets: Vec<FacetConfidence>,
    pub checks: Vec<CheckConfidence>,
    pub outside_map_files: Vec<PathBuf>,
    pub map_update: Option<RepoMapUpdate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckStatus {
    pub check_id: CheckId,
    pub outcome: VerificationOutcome,
    pub manual: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageGroup {
    pub package_root: PathBuf,
    pub files: Vec<PathBuf>,
    pub checks: Vec<CheckStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UntrustedContentKind {
    RepositoryFile,
    ToolOutput,
    CommitMessage,
    WebContent,
    ModelOutput,
    GeneratedOrVendoredCode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UntrustedContentRef {
    pub source: String,
    pub kind: UntrustedContentKind,
    pub location: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteeringClass(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SteeringDetector { Pattern, Human }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteeringAttempt {
    pub source: String,
    pub class: SteeringClass,
    pub location: Option<String>,
    pub detected_by: SteeringDetector,
    pub action_taken: String,
    pub detected_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedPath {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewPacket {
    pub task_id: TaskId,
    pub plan_version: PlanVersion,
    pub summary: ReviewSummary,
    pub diff: DiffSnapshot,
    pub plan_vs_actual: PlanVsActual,
    pub decisions: Vec<DecisionRecord>,
    pub assumptions: Vec<AssumptionRecord>,
    pub verification: Vec<VerificationEvidence>,
    pub open_questions: Vec<String>,
    pub permissions: PermissionSummary,
    pub execution_mode: ExecutionMode,
    pub repo_confidence: RepoConfidence,
    pub package_groups: Vec<PackageGroup>,
    pub untrusted_content: Vec<UntrustedContentRef>,
    pub steering_attempts: Vec<SteeringAttempt>,
    pub protected_paths: Vec<ProtectedPath>,
}
```

`ReviewPacket` is assembled by `OpenReviewUseCase`. Every section is present or explicitly empty; the TUI renders `N/A` for an empty optional section. The packet never opens a pager.

### 6.7 Attestation, override, merge

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttesterIdentity {
    pub name: String,
    pub email: String,
    pub key_fingerprint: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VivaAnchor {
    HighestRiskDecision,
    RiskiestDiffHunk,
    WeakestEvidence,
    UntrustedContent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VivaAttempt {
    pub answer: String,
    pub missing_points: Vec<String>,
    pub passed: bool,
    pub at: Timestamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VivaVerdict { Passed, NotPassed }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VivaQuestionRecord {
    pub question_id: String,
    pub anchor: VivaAnchor,
    pub prompt: String,
    pub rubric: Vec<String>,
    pub rubric_hash: ContentHash,
    pub attempts: Vec<VivaAttempt>,
    pub verdict: VivaVerdict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeMetadata {
    pub model_id: String,
    pub provider: String,
    pub rubric_hash: ContentHash,
    pub judged_at: Timestamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SigningKind { Ssh, Gpg, GeneratedSshEd25519 }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SigningScheme {
    pub kind: SigningKind,
    pub key_fingerprint: String,
    pub allowed_signers_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerBinding {
    pub record_id: String,
    pub content_hash: ContentHash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBinding {
    pub evidence_ref: EvidenceRef,
    pub content_hash: ContentHash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FourthAnchorState { Required, NotRequired, Answered }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttestationRecord {
    pub attestation_id: AttestationId,
    pub schema_version: u32,
    pub task_id: TaskId,
    pub version: u32,
    pub created_at: Timestamp,
    pub attester: AttesterIdentity,
    pub subject_commit: CommitSha,
    pub subject_tree: ContentHash,
    pub base_ref: BaseRef,
    pub base_commit: CommitSha,
    pub diff_hash: ContentHash,
    pub plan_version: PlanVersion,
    pub plan_hash: ContentHash,
    pub policy_version: u32,
    pub policy_hash: ContentHash,
    pub ledger: Vec<LedgerBinding>,
    pub evidence: Vec<EvidenceBinding>,
    pub questions: Vec<VivaQuestionRecord>,
    pub judge: JudgeMetadata,
    pub signing: SigningScheme,
    pub supersedes: Vec<AttestationId>,
    pub repo_trust: TrustDeclaration,
    pub execution_mode: ExecutionMode,
    pub untrusted_content: Vec<UntrustedContentRef>,
    pub steering_attempts: Vec<SteeringAttempt>,
    pub protected_paths: Vec<ProtectedPath>,
    pub fourth_anchor: FourthAnchorState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverrideRecord {
    pub override_id: OverrideId,
    pub schema_version: u32,
    pub task_id: TaskId,
    pub version: u32,
    pub subject_commit: CommitSha,
    pub base_ref: BaseRef,
    pub base_commit: CommitSha,
    pub reason: String,
    pub created_at: Timestamp,
    pub attester: AttesterIdentity,
    pub key_fingerprint: String,
    pub bypassed_gates: Vec<GateKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trailer {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeResult {
    pub merge_commit: CommitSha,
    pub subject_commit: CommitSha,
    pub attestation_commit: Option<CommitSha>,
    pub base_before: CommitSha,
    pub base_after: CommitSha,
    pub trailers: Vec<Trailer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeOutcome {
    pub task_id: TaskId,
    pub result: MergeResult,
    pub attestation_id: Option<AttestationId>,
    pub override_id: Option<OverrideId>,
}
```

`AttestationRecord` and `OverrideRecord` are the core records. TDD-0008 owns their canonical JSON encoding, file names, and Git binding. The fields here are the minimum binding set from FR-REV-010 and FR-MERGE-006; TDD-0008 may add fields only by amending this document.

### 6.8 Ledger records

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind { Decision, Assumption }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DetailLevel { Full, Light, Stub }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Visibility { Repo, Local }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecisionStatus {
    Accepted,
    Rejected,
    Deferred,
    Superseded,
    Deprecated,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RefKind { Task, Plan, Commit, Diff, File, Issue, Adr, Url, Unknown(String) }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedRef {
    pub kind: RefKind,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyProvenance {
    pub version: u32,
    pub hash: ContentHash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionOptionRecord {
    pub label: String,
    pub description: String,
    pub rejected_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Erratum {
    pub date: Timestamp,
    pub source: String,
    pub evidence: String,
    pub changed: String,
    pub decision_changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionRecord {
    pub schema_version: u32,
    pub decision_id: DecisionId,
    pub kind: RecordKind,
    pub detail: DetailLevel,
    pub title: String,
    pub status: DecisionStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub owner: String,
    pub approver: String,
    pub proposer: String,
    pub rationale_provenance: RationaleProvenance,
    pub reversibility: Reversibility,
    pub blast_radius: BlastRadius,
    pub refs: Vec<TypedRef>,
    pub implemented_by: Vec<CommitSha>,
    pub supersedes: Vec<DecisionId>,
    pub superseded_by: Option<DecisionId>,
    pub visibility: Visibility,
    pub policy: Option<PolicyProvenance>,
    pub task_id: TaskId,
    pub chosen_option: String,
    pub adr: Option<String>,
    pub attestation: Option<AttestationId>,
    pub status_reason: Option<String>,
    pub revisit_if: Option<String>,
    pub rationale: String,
    pub context: Option<String>,
    pub options: Vec<DecisionOptionRecord>,
    pub consequences: Option<String>,
    pub notes: Option<String>,
    pub errata: Vec<Erratum>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssumptionRecord {
    pub schema_version: u32,
    pub assumption_id: AssumptionId,
    pub kind: RecordKind,
    pub detail: DetailLevel,
    pub title: String,
    pub status: AssumptionStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub logger: String,
    pub reviewer: Option<String>,
    pub risk: AssumptionRisk,
    pub revisit_if: String,
    pub task_id: TaskId,
    pub refs: Vec<TypedRef>,
    pub supersedes: Vec<AssumptionId>,
    pub superseded_by: Option<AssumptionId>,
    pub visibility: Visibility,
    pub assumption: String,
    pub basis: String,
    pub validation: Option<String>,
    pub notes: Option<String>,
    pub errata: Vec<Erratum>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerVerificationReport {
    pub duplicate_ids: Vec<String>,
    pub broken_links: Vec<String>,
    pub missing_backlinks: Vec<String>,
    pub repaired: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerIndexEntry {
    pub record_id: String,
    pub kind: RecordKind,
    pub path: PathBuf,
    pub content_hash: ContentHash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerIndex {
    pub entries: Vec<LedgerIndexEntry>,
    pub generated_at: Timestamp,
}
```

TDD-0005 owns the YAML front matter, Markdown body, validation errors, duplicate handling, and index serialization. The Rust fields above are the stable core shape.

### 6.9 Config, policy, and repo map

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryPoint {
    pub path: PathBuf,
    pub kind: String,
    pub source: String,
    pub is_primary: bool,
    pub status: FacetStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleRoot {
    pub path: PathBuf,
    pub source: String,
    pub status: FacetStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextDoc {
    pub path: PathBuf,
    pub precedence: u32,
    pub status: FacetStatus,
    pub stale_reference: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageRoot {
    pub path: PathBuf,
    pub is_container: bool,
    pub source: String,
    pub status: FacetStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetSpec {
    pub kind: FacetKind,
    pub status: FacetStatus,
    pub confirmed_absence: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoMapSnapshot {
    pub schema_version: u32,
    pub version: u32,
    pub hash: ContentHash,
    pub entry_points: Vec<EntryPoint>,
    pub module_roots: Vec<ModuleRoot>,
    pub docs: Vec<ContextDoc>,
    pub package_roots: Vec<PackageRoot>,
    pub facets: Vec<FacetSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckKind { Build, Test, Lint, Format, Typecheck, Setup }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckDefinition {
    pub check_id: CheckId,
    pub kind: CheckKind,
    pub command: String,
    pub cwd: PathBuf,
    pub required: bool,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationChoice {
    Automated { checks: Vec<CheckId> },
    Manual { method: String },
    Unconfigured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelSlot { Primary, Fast, Local }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProfileName(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelId(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSlotBinding {
    pub slot: ModelSlot,
    pub profile: ProfileName,
    pub model_id: ModelId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigSnapshot {
    pub schema_version: u32,
    pub repo_root: PathBuf,
    pub config_hash: ContentHash,
    pub base_ref: BaseRef,
    pub repo_map: RepoMapSnapshot,
    pub checks: Vec<CheckDefinition>,
    pub verification: VerificationChoice,
    pub budgets: Budget,
    pub slots: Vec<ModelSlotBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolicyTriggerKind {
    NewDependency,
    SchemaMigration,
    PublicInterfaceChange,
    CrossModuleRefactor,
    SecurityBoundary,
    Deletion,
    PerformanceSensitivePath,
    DeviationFromPlan,
    ProtectedPathWrite,
    LockfileOnlyChange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriggerSource { Policy, Model, Guard }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyTrigger {
    pub kind: PolicyTriggerKind,
    pub detail: String,
    pub source: TriggerSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyRule {
    pub rule_id: String,
    pub trigger: PolicyTriggerKind,
    pub description: String,
    pub paths: Vec<PathPattern>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullRecordRule {
    pub hard_reversibility: bool,
    pub system_blast_radius: bool,
    pub external_blast_radius: bool,
    pub policy_trigger: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionPolicySnapshot {
    pub schema_version: u32,
    pub version: u32,
    pub hash: ContentHash,
    pub rules: Vec<PolicyRule>,
    pub allowed_assumptions: Vec<PolicyRule>,
    pub full_record: FullRecordRule,
    pub protected_paths: Vec<PathPattern>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPolicySnapshot {
    pub schema_version: u32,
    pub version: u32,
    pub hash: ContentHash,
    pub filesystem_allow: Vec<PathPattern>,
    pub filesystem_deny: Vec<PathPattern>,
    pub network_allow: Vec<String>,
    pub environment_allow: Vec<String>,
    pub process_rules: Vec<String>,
    pub ui_rules: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrustDeclaration { Trusted, Untrusted }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutionMode { Sandboxed, SandboxedNetworkDenied, Guarded }

impl ExecutionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sandboxed => "Sandboxed",
            Self::SandboxedNetworkDenied => "Sandboxed — network denied",
            Self::Guarded => "Guarded — not sandboxed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CanaryKind {
    WriteOutsideWorktree,
    ReadCredentialPath,
    UnlistedNetworkHost,
    ChildProcessInheritsDenials,
    SymlinkOrJunctionEscape,
    ModelKeyAbsentFromWorker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxProbeCheck {
    pub canary: CanaryKind,
    pub passed: bool,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxProbeReport {
    pub mode: ExecutionMode,
    pub checks: Vec<SandboxProbeCheck>,
    pub failed_checks: Vec<CanaryKind>,
    pub probed_at: Timestamp,
}
```

`FacetSpec` is the core representation of a facet’s status. `FacetStatus` is the public enum; the private `domain` has a same-named mirror. The mapping is one-to-one. TDD-0001 owns the TOML form of `ConfigSnapshot` and `RepoMapSnapshot`; TDD-0004 owns the policy rule vocabulary; TDD-0006 owns the execution-policy JSON and the canary implementation.

### 6.10 Model and sandbox

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelRole { System, Human, Assistant, Tool }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEnvelope {
    pub source: String,
    pub kind: UntrustedContentKind,
    pub location: Option<String>,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentBlock {
    Text { text: String },
    Reasoning { text: String },
    ToolCall { call: ToolCall },
    ToolResult { call_id: String, content: String },
    Untrusted { envelope: SourceEnvelope },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelMessage {
    pub role: ModelRole,
    pub blocks: Vec<ContentBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCallDelta {
    pub call_id: String,
    pub name: Option<String>,
    pub arguments_json_delta: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelParameters {
    pub temperature_millis: Option<u16>,
    pub top_p_millis: Option<u16>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRequest {
    pub call_id: ModelCallId,
    pub model_id: ModelId,
    pub system_prompt: String,
    pub messages: Vec<ModelMessage>,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: Option<String>,
    pub parameters: ModelParameters,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
    ContentFilter,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelResponse {
    pub text: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

pub trait ModelEventSink: Send {
    fn on_text_delta(&mut self, delta: &str);
    fn on_reasoning_delta(&mut self, delta: &str);
    fn on_tool_call_delta(&mut self, delta: &ToolCallDelta);
    fn on_usage(&mut self, usage: &Usage);
    fn on_finish(&mut self, stop_reason: &StopReason);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub model_id: ModelId,
    pub display_name: String,
    pub context_window: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileValidation {
    pub profile: ProfileName,
    pub endpoint_reachable: bool,
    pub authenticated: bool,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSelection {
    pub slot: ModelSlot,
    pub profile: ProfileName,
    pub model_id: ModelId,
    pub switched_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelCallId(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OperationId(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolEvent {
    Started { operation_id: OperationId, tool: String },
    StdoutDelta { operation_id: OperationId, delta: String },
    StderrDelta { operation_id: OperationId, delta: String },
    Finished { operation_id: OperationId, exit_code: Option<i32> },
}

pub trait ToolEventSink: Send {
    fn on_event(&mut self, event: &ToolEvent);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRequest {
    pub operation_id: OperationId,
    pub tool: String,
    pub working_directory: PathBuf,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub timeout_ms: Option<u64>,
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolResult {
    pub operation_id: OperationId,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: u64,
    pub file_changes: Vec<FileChange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub before_hash: Option<ContentHash>,
    pub after_hash: Option<ContentHash>,
}
```

`ModelEventSink` and `ToolEventSink` are synchronous, infallible, and `Send`. The adapter drives them; the core never spawns a task. `ModelCallId` and `OperationId` are opaque strings generated by the adapter or the application; they are not IDs that need `IdSourcePort`.

### 6.11 Session events and reports

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEvent {
    pub sequence: EventSequence,
    pub task_id: TaskId,
    pub at: Timestamp,
    pub kind: SessionEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEventKind {
    TaskStarted { goal: TaskGoal, base_ref: BaseRef, base_commit: CommitSha },
    TaskStateChanged { from: TaskState, to: TaskState, reason: String },
    PlanRecorded { version: PlanVersion, hash: ContentHash, risk: RiskClass },
    PlanApproved { version: PlanVersion },
    PlanRevisionRecorded { version: PlanVersion, cause: PlanRevisionCause },
    StepStarted { step_id: StepId },
    StepCompleted { step_id: StepId, status: StepStatus },
    ActionIntentRecorded { operation_id: OperationId, action: String },
    ActionResultRecorded { operation_id: OperationId, result: String },
    GateOpened { gate_id: GateId, kind: GateKind },
    GateAnswered { gate_id: GateId, kind: GateKind, outcome: GateOutcome },
    PermissionRequested { gate_id: GateId, action: PermissionAction },
    PermissionDecided { gate_id: GateId, answer: PermissionAnswer },
    DecisionRecorded { decision_id: DecisionId, gate_id: GateId },
    AssumptionRecorded { assumption_id: AssumptionId },
    VerificationRecorded { check_id: Option<CheckId>, outcome: VerificationOutcome },
    ReviewOpened { plan_version: PlanVersion },
    AttestationCreated { attestation_id: AttestationId, subject_commit: CommitSha },
    MergeCompleted { merge_commit: CommitSha, attestation_id: Option<AttestationId> },
    OverrideCreated { override_id: OverrideId },
    TaskPaused { reason: String },
    TaskResumed { from: TaskState },
    TaskStopped { reason: String },
    TaskCancelled { reason: String },
    TaskReopened { reason: String },
    BudgetWarning { spent: UsdMicros, soft: Option<UsdMicros> },
    BudgetPaused { spent: UsdMicros, hard: Option<UsdMicros> },
    ModelSwitched { slot: ModelSlot, profile: ProfileName, model_id: ModelId },
    PolicyDriftDetected { expected: ContentHash, actual: ContentHash },
    ConfigDriftDetected { expected: ContentHash, actual: ContentHash },
    SteeringDetected { attempt: SteeringAttempt },
    UntrustedContentLabeled { content: UntrustedContentRef },
}
```

`SessionEventKind` is `#[non_exhaustive]`. TDD-0003 owns the JSONL encoding and may add event kinds for resume, audit, or metrics; adding a variant is a minor change, changing an existing field is an amendment to this document.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusReport {
    pub task: Option<TaskSnapshot>,
    pub state: Option<TaskState>,
    pub execution_mode: Option<ExecutionMode>,
    pub enforcement: EnforcementStatus,
    pub budget: Option<Budget>,
    pub usage: Option<Usage>,
    pub pending_gate: Option<Gate>,
    pub attestation_id: Option<AttestationId>,
    pub merge_commit: Option<CommitSha>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnforcementStatus {
    Ok,
    Degraded { reasons: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptReport {
    pub task_id: TaskId,
    pub events: Vec<SessionEvent>,
    pub total_events: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    pub at: Timestamp,
    pub task_id: Option<TaskId>,
    pub kind: AuditEntryKind,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuditEntryKind {
    PermissionGranted,
    PermissionDenied,
    HostExec,
    PolicyEdited,
    TrustChanged,
    OverrideMerged,
    HookInstalled,
    HookConflict,
    ModelSwitched,
    BudgetIncreased,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditFinding {
    pub code: String,
    pub message: String,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditReport {
    pub overrides: Vec<OverrideRecord>,
    pub hooks: HooksStatus,
    pub merged_unattested: Vec<String>,
    pub findings: Vec<AuditFinding>,
}
```

### 6.12 Git and worktree records

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeStatus {
    pub clean: bool,
    pub branch: BranchName,
    pub head: CommitSha,
    pub uncommitted_paths: Vec<PathBuf>,
    pub untracked_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureVerification {
    pub valid: bool,
    pub signer_fingerprint: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergePreflight {
    pub clean: bool,
    pub conflicts: Vec<PathBuf>,
    pub merged_tree_hash: Option<ContentHash>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRequest {
    pub worktree: Worktree,
    pub message: String,
    pub paths: Vec<PathBuf>,
    pub trailers: Vec<Trailer>,
    pub signing: Option<SigningScheme>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttestationCommitRequest {
    pub worktree: Worktree,
    pub attestation_path: PathBuf,
    pub record_hash: ContentHash,
    pub signing: SigningScheme,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeRequest {
    pub base_ref: BaseRef,
    pub subject_commit: CommitSha,
    pub attestation_commit: CommitSha,
    pub message: String,
    pub trailers: Vec<Trailer>,
    pub signing: Option<SigningScheme>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HookKind { PreMergeCommit, ReferenceTransaction }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HookState { Installed, Missing, Outdated, Conflicting }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HooksStatus {
    pub state: HookState,
    pub installed: Vec<HookKind>,
    pub missing: Vec<HookKind>,
    pub outdated: Vec<HookKind>,
    pub conflicting: Vec<HookKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookInstallRequest {
    pub repo_root: PathBuf,
    pub forge_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookInstallReport {
    pub installed: Vec<HookKind>,
    pub skipped_existing: Vec<HookKind>,
    pub lines_to_add: Vec<String>,
}
```

TDD-0008 owns the Git CLI methods that produce these records. `GitPort` methods are in §7.3.

---

## 7. Public ports

### 7.1 Inbound capabilities

Every inbound trait is object-safe, `Send + Sync`, uses `#[async_trait]`, and returns `Result<_, ApplicationError>`.

```rust
#[async_trait::async_trait]
pub trait StartTask: Send + Sync {
    async fn start_task(&self, request: StartTaskRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait ResumeTask: Send + Sync {
    async fn resume_task(&self, request: ResumeTaskRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait RetryTask: Send + Sync {
    async fn retry_task(&self, request: RetryTaskRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait PauseTask: Send + Sync {
    async fn pause_task(&self, request: PauseTaskRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait CancelTask: Send + Sync {
    async fn cancel_task(&self, request: CancelTaskRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait ReopenTask: Send + Sync {
    async fn reopen_task(&self, request: ReopenTaskRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait MergeTask: Send + Sync {
    async fn merge_task(&self, request: MergeTaskRequest) -> Result<MergeOutcome, ApplicationError>;
}

#[async_trait::async_trait]
pub trait OverrideMerge: Send + Sync {
    async fn override_merge(&self, request: OverrideMergeRequest) -> Result<MergeOutcome, ApplicationError>;
}

#[async_trait::async_trait]
pub trait AnswerGate: Send + Sync {
    async fn answer_gate(&self, request: AnswerGateRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait OpenReview: Send + Sync {
    async fn open_review(&self, request: OpenReviewRequest) -> Result<ReviewPacket, ApplicationError>;
}

#[async_trait::async_trait]
pub trait AttestTask: Send + Sync {
    async fn start_attestation(&self, request: StartAttestationRequest) -> Result<VivaPrompt, ApplicationError>;
    async fn answer_attestation(&self, request: AnswerAttestationRequest) -> Result<VivaProgress, ApplicationError>;
    async fn submit_attestation(&self, request: SubmitAttestationRequest) -> Result<AttestationRecord, ApplicationError>;
}

#[async_trait::async_trait]
pub trait SwitchModel: Send + Sync {
    async fn switch_model(&self, request: SwitchModelRequest) -> Result<ModelSelection, ApplicationError>;
}

#[async_trait::async_trait]
pub trait IncreaseBudget: Send + Sync {
    async fn increase_budget(&self, request: IncreaseBudgetRequest) -> Result<TaskProgress, ApplicationError>;
}

#[async_trait::async_trait]
pub trait QueryStatus: Send + Sync {
    async fn query_status(&self, request: QueryStatusRequest) -> Result<StatusReport, ApplicationError>;
}

#[async_trait::async_trait]
pub trait QueryTranscript: Send + Sync {
    async fn query_transcript(&self, request: QueryTranscriptRequest) -> Result<TranscriptReport, ApplicationError>;
}

#[async_trait::async_trait]
pub trait QueryAudit: Send + Sync {
    async fn query_audit(&self, request: QueryAuditRequest) -> Result<AuditReport, ApplicationError>;
}
```

Request and response records:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartTaskRequest {
    pub goal: TaskGoal,
    pub base_ref: Option<BaseRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumeTaskRequest { pub task_id: TaskId }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryTaskRequest { pub task_id: TaskId }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PauseTaskRequest { pub task_id: TaskId }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancelTaskRequest { pub task_id: TaskId, pub confirmed: bool }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReopenTaskRequest { pub task_id: TaskId, pub reason: String }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeTaskRequest { pub task_id: TaskId }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverrideMergeRequest {
    pub task_id: TaskId,
    pub typed_task_id: TaskId,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerGateRequest {
    pub task_id: TaskId,
    pub gate_id: GateId,
    pub answer: GateAnswer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenReviewRequest { pub task_id: TaskId }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartAttestationRequest { pub task_id: TaskId }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerAttestationRequest {
    pub task_id: TaskId,
    pub question_id: String,
    pub answer: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmitAttestationRequest {
    pub task_id: TaskId,
    pub confirmation: String, // must equal "attest"
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VivaPrompt {
    pub question_id: String,
    pub anchor: VivaAnchor,
    pub prompt: String,
    pub attempt_number: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VivaProgress {
    pub question: Option<VivaPrompt>,
    pub passed: bool,
    pub missing_points: Vec<String>,
    pub can_explain: bool,
    pub can_stop: bool,
    pub ready_for_attestation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchModelRequest {
    pub task_id: TaskId,
    pub slot: ModelSlot,
    pub profile: Option<ProfileName>,
    pub model_id: Option<ModelId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncreaseBudgetRequest {
    pub task_id: TaskId,
    pub soft: Option<UsdMicros>,
    pub hard: Option<UsdMicros>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryStatusRequest { pub task_id: Option<TaskId> }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryTranscriptRequest {
    pub task_id: TaskId,
    pub after: Option<EventSequence>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryAuditRequest {}
```

Every request is owned plain data. No request holds an adapter, a lock, a future, or a reference to private state.

### 7.2 Outbound ports

Every outbound trait is object-safe, `Send + Sync`, and uses `#[async_trait]` except `ClockPort`, which is synchronous and infallible.

```rust
pub trait ClockPort: Send + Sync {
    fn now(&self) -> Timestamp;
}

#[async_trait::async_trait]
pub trait IdSourcePort: Send + Sync {
    async fn next_task_id(&self) -> Result<TaskId, IdSourceError>;
    async fn next_gate_id(&self, task_id: &TaskId) -> Result<GateId, IdSourceError>;
    async fn next_decision_id(&self, title: &str, at: Timestamp) -> Result<DecisionId, IdSourceError>;
    async fn next_assumption_id(&self, title: &str, at: Timestamp) -> Result<AssumptionId, IdSourceError>;
}

#[async_trait::async_trait]
pub trait ConfigPort: Send + Sync {
    async fn load(&self) -> Result<ConfigSnapshot, ConfigError>;
}

#[async_trait::async_trait]
pub trait SessionStorePort: Send + Sync {
    async fn load(&self, task_id: &TaskId) -> Result<Option<SessionSnapshot>, SessionError>;
    async fn active_task(&self) -> Result<Option<TaskId>, SessionError>;
    async fn save(
        &self,
        snapshot: &SessionSnapshot,
        expected_sequence: EventSequence,
    ) -> Result<(), SessionError>;
    async fn append(
        &self,
        events: &[SessionEvent],
        expected_sequence: EventSequence,
    ) -> Result<EventSequence, SessionError>;
    async fn read_events(
        &self,
        task_id: &TaskId,
        after: Option<EventSequence>,
        limit: Option<usize>,
    ) -> Result<Vec<SessionEvent>, SessionError>;
    async fn save_plan(&self, task_id: &TaskId, plan: &PlanSnapshot) -> Result<(), SessionError>;
    async fn save_artifact(
        &self,
        task_id: &TaskId,
        artifact: &EvidenceArtifact,
    ) -> Result<ArtifactRef, SessionError>;
    async fn acquire_lock(&self, task_id: &TaskId) -> Result<ActiveTaskLock, SessionError>;
    async fn release_lock(&self, lock: &ActiveTaskLock) -> Result<(), SessionError>;
}

#[async_trait::async_trait]
pub trait PolicyPort: Send + Sync {
    async fn load_decision_policy(&self) -> Result<DecisionPolicySnapshot, PolicyError>;
}

#[async_trait::async_trait]
pub trait LedgerPort: Send + Sync {
    async fn write_decision(&self, record: &DecisionRecord) -> Result<(), LedgerError>;
    async fn write_assumption(&self, record: &AssumptionRecord) -> Result<(), LedgerError>;
    async fn read_decision(&self, id: &DecisionId) -> Result<Option<DecisionRecord>, LedgerError>;
    async fn read_assumption(&self, id: &AssumptionId) -> Result<Option<AssumptionRecord>, LedgerError>;
    async fn verify(&self) -> Result<LedgerVerificationReport, LedgerError>;
    async fn rebuild_index(&self) -> Result<LedgerIndex, LedgerError>;
}

#[async_trait::async_trait]
pub trait GitPort: Send + Sync {
    async fn resolve_base(&self, base_ref: &BaseRef) -> Result<CommitSha, GitError>;
    async fn branch_exists(&self, branch: &BranchName) -> Result<bool, GitError>;
    async fn create_worktree(
        &self,
        task_id: &TaskId,
        base_commit: &CommitSha,
    ) -> Result<Worktree, GitError>;
    async fn remove_worktree(&self, worktree: &Worktree) -> Result<(), GitError>;
    async fn worktree_status(&self, worktree: &Worktree) -> Result<WorktreeStatus, GitError>;
    async fn head(&self, worktree: &Worktree) -> Result<CommitSha, GitError>;
    async fn parents(&self, commit: &CommitSha) -> Result<Vec<CommitSha>, GitError>;
    async fn tree_hash(&self, commit: &CommitSha) -> Result<ContentHash, GitError>;
    async fn commit_message(&self, commit: &CommitSha) -> Result<String, GitError>;
    async fn read_blob(
        &self,
        commit: &CommitSha,
        path: &Path,
    ) -> Result<Option<Vec<u8>>, GitError>;
    async fn diff(
        &self,
        base: &CommitSha,
        head: &CommitSha,
    ) -> Result<DiffSnapshot, GitError>;
    async fn merge_tree_preflight(
        &self,
        base: &CommitSha,
        head: &CommitSha,
    ) -> Result<MergePreflight, GitError>;
    async fn commit(&self, request: &CommitRequest) -> Result<CommitSha, GitError>;
    async fn create_attestation_commit(
        &self,
        request: &AttestationCommitRequest,
    ) -> Result<CommitSha, GitError>;
    async fn merge(&self, request: &MergeRequest) -> Result<MergeResult, GitError>;
    async fn verify_commit_signature(
        &self,
        commit: &CommitSha,
        allowed_signers: &Path,
    ) -> Result<SignatureVerification, GitError>;
    async fn hooks_status(&self) -> Result<HooksStatus, GitError>;
    async fn install_hooks(
        &self,
        request: &HookInstallRequest,
    ) -> Result<HookInstallReport, GitError>;
}

#[async_trait::async_trait]
pub trait ModelPort: Send + Sync {
    async fn stream(
        &self,
        request: ModelRequest,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelError>;
    async fn cancel(&self, call_id: &ModelCallId) -> Result<(), ModelError>;
    async fn list_models(&self, profile: &ProfileName) -> Result<Vec<ModelInfo>, ModelError>;
    async fn validate_profile(
        &self,
        profile: &ProfileName,
    ) -> Result<ProfileValidation, ModelError>;
}

#[async_trait::async_trait]
pub trait SandboxPort: Send + Sync {
    async fn load_execution_policy(&self) -> Result<ExecutionPolicySnapshot, SandboxError>;
    async fn probe(&self, request: SandboxProbeRequest) -> Result<SandboxProbeReport, SandboxError>;
    async fn run_tool(
        &self,
        request: ToolRequest,
        sink: &mut dyn ToolEventSink,
    ) -> Result<ToolResult, SandboxError>;
    async fn cancel(&self, operation_id: &OperationId) -> Result<(), SandboxError>;
    async fn apply_grant(&self, grant: &Grant) -> Result<(), SandboxError>;
    async fn repo_trust(&self) -> Result<TrustDeclaration, SandboxError>;
}

#[async_trait::async_trait]
pub trait AttestationPort: Send + Sync {
    async fn read(&self, task_id: &TaskId) -> Result<Option<AttestationRecord>, AttestationError>;
    async fn write_attestation(
        &self,
        record: &AttestationRecord,
    ) -> Result<CommitSha, AttestationError>;
    async fn verify(
        &self,
        task_id: &TaskId,
        commit: &CommitSha,
    ) -> Result<AttestationVerification, AttestationError>;
    async fn write_override(&self, record: &OverrideRecord) -> Result<(), AttestationError>;
}

#[async_trait::async_trait]
pub trait AuditPort: Send + Sync {
    async fn record(&self, entry: AuditEntry) -> Result<(), AuditError>;
    async fn report(&self, task_id: Option<&TaskId>) -> Result<AuditReport, AuditError>;
}
```

`SandboxProbeRequest` and `AttestationVerification` are public records:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxProbeRequest {
    pub repo_root: PathBuf,
    pub task_worktree: Option<Worktree>,
    pub execution_policy: ExecutionPolicySnapshot,
    pub trust: TrustDeclaration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttestationVerification {
    pub attestation_id: AttestationId,
    pub signature_valid: bool,
    pub subject_matches: bool,
    pub hashes_match: bool,
    pub head_is_attestation_commit: bool,
    pub parent_is_subject: bool,
    pub detail: String,
}
```

### 7.3 Port ownership and statelessness

- `ConfigPort` is the only port that reads `.forge/config`; `PolicyPort` reads `.forge/policy`; `SandboxPort` reads `.forge/execution-policy.json` and `.forge/local/repo-trust.json`. No other port reads those files.
- `SessionStorePort` owns `state.json`, `events.jsonl`, `plans/`, `artifacts/`, and `.forge/active-task.lock`. It is the only port that writes them.
- `LedgerPort` owns `.forge/decisions/`, `.forge/assumptions/`, and `.forge/local/index.json`.
- `GitPort` owns `.forge/worktrees/`, branches, commits, merges, hooks, and signature verification.
- `AttestationPort` owns `.forge/attestations/`, `.forge/local/attestations/`, `.forge/overrides/`, and `.forge/trust/`.
- `AuditPort` owns the read model over session events, hooks, overrides, and merged branches.
- `ModelPort` owns provider HTTP/SSE and normalized model responses. It never reads repository files.
- `SandboxPort` owns MXC policy, the worker process, permission grants, canaries, and tool execution. It never reads the ledger or Git.
- `ClockPort` and `IdSourcePort` are the only sources of time and identity.
- Adapters never call one another. If `AttestationPort` needs Git, it receives `Arc<dyn GitPort>` through wiring; if `AuditPort` needs session events, it receives a session read handle. The core orchestrates through use cases.

### 7.4 Async, cancellation, and concurrency

- Every async port future is `Send`.
- Dropping a future must leave no partial durable state. Mutating methods are atomic or expose an idempotent recovery path:
  - `SessionStorePort::save` writes a temporary file, fsyncs, and renames; it compares `expected_sequence`.
  - `SessionStorePort::append` writes complete lines and fsyncs; a truncated tail is ignored on read.
  - `LedgerPort::write_decision` and `write_assumption` write one record at a time and never partially rewrite a record.
  - `GitPort::commit` and `merge` are single Git operations; a failure leaves the ref unchanged.
- Cancellation of in-process work is by dropping the future. For out-of-process model and tool work, the caller retains the `ModelCallId` or `OperationId` and calls `cancel`; the adapter must stop the provider request or worker process and return `Cancelled`.
- No port stores request-scoped state between calls. A port may hold a connection pool or a resolved configuration value, but not the identity or progress of the current call.
- No lock is held across `.await`. `ApplicationPorts` holds `Arc` values, not mutex guards.

---

## 8. Error taxonomy

Every public error enum is `#[non_exhaustive]`, implements `std::error::Error`, and uses `thiserror`. No `anyhow::Error`, `Box<dyn Error>`, or stringly error crosses a port. Error messages are lower-case, period-free, and name the failed operation.

### 8.1 `DomainError`

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("invalid {field}: {value}")]
    InvalidValue { field: &'static str, value: String },
    #[error("invalid task transition from {from:?} to {to:?}")]
    InvalidTaskTransition { from: TaskState, to: TaskState },
    #[error("gate {gate_id:?} was not found")]
    GateNotFound { gate_id: GateId },
    #[error("gate {gate_id:?} was already answered")]
    GateAlreadyAnswered { gate_id: GateId },
    #[error("a blocking gate is already open: {gate_id:?}")]
    BlockingGateOpen { gate_id: GateId },
    #[error("expected a {expected:?} gate answer, got {actual:?}")]
    InvalidGateAnswer { expected: GateKind, actual: GateKind },
    #[error("the plan is not fog-clear: {reasons:?}")]
    PlanNotFogClear { reasons: Vec<String> },
    #[error("step {step_id:?} has no owning module, entry point, or package root")]
    MissingOwningUnit { step_id: StepId },
    #[error("the rationale is empty")]
    RationaleEmpty,
    #[error("the rationale must be authored by the human principal")]
    RationaleAuthoredByAgent,
    #[error("a blocking decision remains: {key}")]
    BlockingDecisionRemains { key: String },
    #[error("the protected path {path:?} has no decision")]
    UnresolvedProtectedPath { path: PathBuf },
    #[error("the attestation binding does not match: {field}")]
    AttestationBindingMismatch { field: &'static str },
    #[error("the task {task_id:?} is not attested")]
    NotAttested { task_id: TaskId },
    #[error("the override requires the typed task id")]
    OverrideRequiresTypedTaskId,
    #[error("the operation violates invariant {invariant}: {detail}")]
    InvariantViolation { invariant: &'static str, detail: String },
}
```

### 8.2 Port errors

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session {task_id:?} was not found")]
    NotFound { task_id: TaskId },
    #[error("task {task_id:?} is already locked")]
    LockHeld { task_id: TaskId, pid: Option<u32>, host: Option<String> },
    #[error("the lock for {task_id:?} is stale")]
    StaleLock { task_id: TaskId },
    #[error("event sequence mismatch: expected {expected:?}, found {actual:?}")]
    SequenceMismatch { expected: EventSequence, actual: EventSequence },
    #[error("the session snapshot is corrupt: {reason}")]
    SnapshotCorrupt { reason: String },
    #[error("the event log is corrupt: {reason}")]
    EventLogCorrupt { reason: String },
    #[error("schema version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("failed to write {path:?}: {message}")]
    WriteFailed { path: PathBuf, message: String },
    #[error("the artifact is too large: {actual} > {limit}")]
    ArtifactTooLarge { actual: u64, limit: u64 },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no .forge/config was found")]
    NotFound,
    #[error("failed to parse {path:?}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("a required field is missing: {field}")]
    MissingField { field: &'static str },
    #[error("schema version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("failed to read {path:?}: {message}")]
    ReadFailed { path: PathBuf, message: String },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("no .forge/policy was found")]
    NotFound,
    #[error("failed to parse the decision policy: {message}")]
    Parse { message: String },
    #[error("schema version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("unknown policy trigger {name}")]
    UnknownTrigger { name: String },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("record {record_id} was not found")]
    RecordNotFound { record_id: String },
    #[error("duplicate record id {record_id}")]
    DuplicateId { record_id: String },
    #[error("a required field is missing: {field}")]
    MissingField { field: &'static str },
    #[error("invalid {kind} reference: {value}")]
    InvalidReference { kind: RefKind, value: String },
    #[error("a secret was detected at {location}")]
    SecretDetected { location: String },
    #[error("schema version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("ledger verification failed: {reasons:?}")]
    VerificationFailed { reasons: Vec<String> },
    #[error("failed to write {path:?}: {message}")]
    WriteFailed { path: PathBuf, message: String },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("not a Git repository: {path:?}")]
    NotARepository { path: PathBuf },
    #[error("the Git CLI is unavailable")]
    GitMissing,
    #[error("Git {found} is older than the required {required}")]
    VersionTooOld { found: String, required: String },
    #[error("git {command} failed: {stderr}")]
    CommandFailed { command: String, stderr: String },
    #[error("the main worktree is dirty: {path:?}")]
    DirtyWorktree { path: PathBuf },
    #[error("the branch does not match: expected {expected:?}, found {actual:?}")]
    BranchMismatch { expected: BranchName, actual: BranchName },
    #[error("the merge conflicts")]
    MergeConflict,
    #[error("the commit signature is invalid: {reason}")]
    SignatureInvalid { reason: String },
    #[error("the commit is not signed")]
    SignatureMissing,
    #[error("the commit does not match the expected binding: {field}")]
    CommitMismatch { field: &'static str },
    #[error("the worktree is missing: {path:?}")]
    WorktreeMissing { path: PathBuf },
    #[error("the hook {hook:?} is conflicting")]
    HookConflict { hook: HookKind },
    #[error("failed to run Git: {message}")]
    Io { message: String },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("the provider is unavailable: {status} {message}")]
    ProviderUnavailable { status: u16, message: String },
    #[error("the model request timed out")]
    Timeout,
    #[error("the provider rate-limited the request")]
    RateLimited { retry_after: Option<Duration> },
    #[error("the tool call is malformed: {detail}")]
    MalformedToolCall { detail: String },
    #[error("the stream is malformed: {detail}")]
    MalformedStream { detail: String },
    #[error("the usage is unknown")]
    UnknownUsage,
    #[error("the profile {profile:?} is not configured")]
    ProfileNotFound { profile: ProfileName },
    #[error("the key for {env_var} is missing")]
    KeyMissing { env_var: String },
    #[error("the model call was cancelled")]
    Cancelled,
    #[error("the model call failed: {message}")]
    Io { message: String },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("the host is unsupported: {reason}")]
    UnsupportedHost { reason: String },
    #[error("no execution policy was accepted")]
    PolicyMissing,
    #[error("the execution policy is invalid: {reason}")]
    PolicyInvalid { reason: String },
    #[error("the canary {canary:?} failed: {detail}")]
    CanaryFailed { canary: CanaryKind, detail: String },
    #[error("the action was denied: {action}")]
    Denied { action: String, reason: String },
    #[error("a permission request is required")]
    PermissionRequired { request: PermissionRequest },
    #[error("the permission request was denied")]
    PermissionDenied,
    #[error("the worker crashed with exit code {exit_code:?}")]
    WorkerCrashed { exit_code: Option<i32> },
    #[error("the IPC message exceeded {limit} bytes")]
    IpcLimitExceeded { limit: usize },
    #[error("the tool timed out")]
    Timeout,
    #[error("the trust declaration does not permit this mode: {reason}")]
    TrustViolation { reason: String },
    #[error("the operation was cancelled")]
    Cancelled,
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AttestationError {
    #[error("the attestation question could not be generated")]
    QuestionGenerationFailed,
    #[error("the judge is unavailable")]
    JudgeUnavailable,
    #[error("the rubric is missing for question {question_id}")]
    RubricMissing { question_id: String },
    #[error("the attestation signature is invalid: {reason}")]
    SignatureInvalid { reason: String },
    #[error("the attestation hash does not match: {field}")]
    HashMismatch { field: &'static str },
    #[error("attestation {attestation_id:?} already exists")]
    RecordExists { attestation_id: AttestationId },
    #[error("schema version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("the attestation is missing for {task_id:?}")]
    AttestationMissing { task_id: TaskId },
    #[error("the attestation is invalid: {reason}")]
    Invalid { reason: String },
    #[error("failed to write the attestation: {message}")]
    Io { message: String },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("the audit report is unavailable: {reason}")]
    ReportUnavailable { reason: String },
    #[error("failed to write the audit entry: {message}")]
    WriteFailed { message: String },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum IdSourceError {
    #[error("the id source is exhausted")]
    Exhausted,
    #[error("the id source failed: {message}")]
    Storage { message: String },
}
```

### 8.3 `ApplicationError`

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Policy(#[from] PolicyError),
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    #[error(transparent)]
    Git(#[from] GitError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Sandbox(#[from] SandboxError),
    #[error(transparent)]
    Attestation(#[from] AttestationError),
    #[error(transparent)]
    Audit(#[from] AuditError),
    #[error(transparent)]
    IdSource(#[from] IdSourceError),
    #[error("task {task_id:?} was not found")]
    TaskNotFound { task_id: TaskId },
    #[error("gate {gate_id:?} was already answered")]
    GateAlreadyAnswered { gate_id: GateId },
    #[error("expected a {expected:?} gate answer, got {actual:?}")]
    InvalidGateAnswer { expected: GateKind, actual: GateKind },
    #[error("precondition failed: {requirement}")]
    PreconditionFailed { requirement: String },
    #[error("the task {task_id:?} was modified concurrently")]
    ConcurrentModification { task_id: TaskId },
    #[error("the task {task_id:?} is not resumable from {state:?}")]
    NotResumable { task_id: TaskId, state: TaskState },
    #[error("the task {task_id:?} is not reopenable from {state:?}")]
    NotReopenable { task_id: TaskId, state: TaskState },
    #[error("the task {task_id:?} is not mergeable: {reason}")]
    NotMergeable { task_id: TaskId, reason: String },
    #[error("the task {task_id:?} requires a budget increase")]
    BudgetIncreaseRequired { task_id: TaskId },
    #[error("unsupported operation: {operation}")]
    Unsupported { operation: &'static str },
}
```

`ApplicationError` is the only error type returned by an inbound capability. It carries the port error unchanged so the CLI can render a stable category without parsing strings.

`TerminalError` is **not** a core error. The TUI lives entirely in `infra_local`; terminal rendering failures never cross a port. This corrects the error list in `arch_docs/architecture.md` §9.2.

---

## 9. Application layer and assembly

### 9.1 Use cases

Each inbound capability is implemented by exactly one private use case. A use case may depend on several ports; it never depends on a concrete adapter.

| Use case | Implements | Ports |
|---|---|---|
| `StartTaskUseCase` | `StartTask` | `ClockPort`, `IdSourcePort`, `ConfigPort`, `PolicyPort`, `SessionStorePort`, `SandboxPort`, `GitPort`, `AuditPort` |
| `ResumeTaskUseCase` | `ResumeTask` | `SessionStorePort`, `ConfigPort`, `PolicyPort`, `SandboxPort`, `GitPort` |
| `RetryTaskUseCase` | `RetryTask` | `SessionStorePort`, `GitPort` |
| `PauseTaskUseCase` | `PauseTask` | `SessionStorePort` |
| `CancelTaskUseCase` | `CancelTask` | `SessionStorePort`, `GitPort` |
| `ReopenTaskUseCase` | `ReopenTask` | `SessionStorePort`, `GitPort`, `AttestationPort` |
| `MergeTaskUseCase` | `MergeTask` | `SessionStorePort`, `GitPort`, `AttestationPort`, `LedgerPort`, `AuditPort` |
| `OverrideMergeUseCase` | `OverrideMerge` | `SessionStorePort`, `GitPort`, `AttestationPort`, `AuditPort` |
| `AnswerGateUseCase` | `AnswerGate` | `SessionStorePort`, `LedgerPort`, `GitPort`, `SandboxPort` |
| `OpenReviewUseCase` | `OpenReview` | `SessionStorePort`, `GitPort`, `LedgerPort`, `AuditPort` |
| `AttestTaskUseCase` | `AttestTask` | `SessionStorePort`, `AttestationPort`, `GitPort`, `LedgerPort`, `ModelPort`, `AuditPort` |
| `SwitchModelUseCase` | `SwitchModel` | `SessionStorePort`, `ConfigPort`, `AuditPort` |
| `IncreaseBudgetUseCase` | `IncreaseBudget` | `SessionStorePort`, `AuditPort` |
| `QueryStatusUseCase` | `QueryStatus` | `SessionStorePort`, `ConfigPort`, `SandboxPort`, `AuditPort` |
| `QueryTranscriptUseCase` | `QueryTranscript` | `SessionStorePort` |
| `QueryAuditUseCase` | `QueryAudit` | `AuditPort`, `SessionStorePort`, `GitPort`, `LedgerPort` |

Each use case is `pub(crate)`, implements exactly one inbound trait, and is never re-exported.

### 9.2 Mapping

`application::mapping` is the only module that converts between private domain types and public port records. It provides:

- `TaskSnapshot -> Task` and `Task -> TaskSnapshot`
- `SessionSnapshot -> (Task, Option<Plan>, Vec<Gate>, Vec<Assumption>)` and the inverse
- `PlanSnapshot -> Plan` and `Plan -> PlanSnapshot`
- `DecisionRecord -> Decision` and `Decision -> DecisionRecord`
- `AssumptionRecord -> Assumption` and `Assumption -> AssumptionRecord`
- `AttestationRecord -> Attestation` and `Attestation -> AttestationRecord`
- `OverrideRecord -> Override` and `Override -> OverrideRecord`
- `DecisionPolicySnapshot -> DecisionPolicy` and `ExecutionPolicySnapshot -> ExecutionPolicy`
- `ConfigSnapshot -> RepoMap`, `CheckDefinition`, `VerificationChoice`

Mapping rules:

- A missing required field is an error, never a default.
- A newer `schema_version` is `ApplicationError::PreconditionFailed` with the found and supported versions.
- Unknown enum variants from storage are mapped to a typed error, never silently dropped.
- No mapping performs I/O or reads the clock.
- Mapping is covered by round-trip tests for every record that has a public and private form.

### 9.3 `ApplicationPorts`, `build_application`, and `Application`

```rust
// assembly.rs
pub struct ApplicationPorts {
    pub clock: Arc<dyn ClockPort + Send + Sync>,
    pub id_source: Arc<dyn IdSourcePort + Send + Sync>,
    pub config: Arc<dyn ConfigPort + Send + Sync>,
    pub session_store: Arc<dyn SessionStorePort + Send + Sync>,
    pub policy: Arc<dyn PolicyPort + Send + Sync>,
    pub ledger: Arc<dyn LedgerPort + Send + Sync>,
    pub git: Arc<dyn GitPort + Send + Sync>,
    pub model: Arc<dyn ModelPort + Send + Sync>,
    pub sandbox: Arc<dyn SandboxPort + Send + Sync>,
    pub attestation: Arc<dyn AttestationPort + Send + Sync>,
    pub audit: Arc<dyn AuditPort + Send + Sync>,
}

// ports/application.rs
pub struct Application {
    pub(crate) start_task: Arc<dyn StartTask + Send + Sync>,
    pub(crate) resume_task: Arc<dyn ResumeTask + Send + Sync>,
    pub(crate) retry_task: Arc<dyn RetryTask + Send + Sync>,
    pub(crate) pause_task: Arc<dyn PauseTask + Send + Sync>,
    pub(crate) cancel_task: Arc<dyn CancelTask + Send + Sync>,
    pub(crate) reopen_task: Arc<dyn ReopenTask + Send + Sync>,
    pub(crate) merge_task: Arc<dyn MergeTask + Send + Sync>,
    pub(crate) override_merge: Arc<dyn OverrideMerge + Send + Sync>,
    pub(crate) answer_gate: Arc<dyn AnswerGate + Send + Sync>,
    pub(crate) open_review: Arc<dyn OpenReview + Send + Sync>,
    pub(crate) attest_task: Arc<dyn AttestTask + Send + Sync>,
    pub(crate) switch_model: Arc<dyn SwitchModel + Send + Sync>,
    pub(crate) increase_budget: Arc<dyn IncreaseBudget + Send + Sync>,
    pub(crate) query_status: Arc<dyn QueryStatus + Send + Sync>,
    pub(crate) query_transcript: Arc<dyn QueryTranscript + Send + Sync>,
    pub(crate) query_audit: Arc<dyn QueryAudit + Send + Sync>,
}

impl Application {
    pub fn start_task(&self) -> Arc<dyn StartTask + Send + Sync>;
    pub fn resume_task(&self) -> Arc<dyn ResumeTask + Send + Sync>;
    pub fn retry_task(&self) -> Arc<dyn RetryTask + Send + Sync>;
    pub fn pause_task(&self) -> Arc<dyn PauseTask + Send + Sync>;
    pub fn cancel_task(&self) -> Arc<dyn CancelTask + Send + Sync>;
    pub fn reopen_task(&self) -> Arc<dyn ReopenTask + Send + Sync>;
    pub fn merge_task(&self) -> Arc<dyn MergeTask + Send + Sync>;
    pub fn override_merge(&self) -> Arc<dyn OverrideMerge + Send + Sync>;
    pub fn answer_gate(&self) -> Arc<dyn AnswerGate + Send + Sync>;
    pub fn open_review(&self) -> Arc<dyn OpenReview + Send + Sync>;
    pub fn attest_task(&self) -> Arc<dyn AttestTask + Send + Sync>;
    pub fn switch_model(&self) -> Arc<dyn SwitchModel + Send + Sync>;
    pub fn increase_budget(&self) -> Arc<dyn IncreaseBudget + Send + Sync>;
    pub fn query_status(&self) -> Arc<dyn QueryStatus + Send + Sync>;
    pub fn query_transcript(&self) -> Arc<dyn QueryTranscript + Send + Sync>;
    pub fn query_audit(&self) -> Arc<dyn QueryAudit + Send + Sync>;
}

pub fn build_application(ports: ApplicationPorts) -> Application;
```

`build_application` is pure wiring: it clones `Arc`s, constructs each use case, and returns `Application`. It performs no I/O, reads no environment, and cannot fail. All ports are mandatory. There is no service locator, no optional port, and no adapter construction.

`Application` is defined in `ports::application` and re-exported as `app_core::ports::Application`. It is the only public object that holds capabilities. It is `Send + Sync` because every field is `Arc<dyn Trait + Send + Sync>`. `ports.rs` re-exports only `ApplicationPorts` and `build_application` from `assembly`, preserving DEP-11. `Application`'s fields are `pub(crate)` so `assembly` can construct it; the public surface is the accessors.

### 9.4 Gate lifecycle in the core

1. A use case constructs a `Gate` with a new `GateId`, stores it in `SessionSnapshot.gates`, and appends `SessionEventKind::GateOpened`.
2. `AnswerGate` loads the snapshot, finds the gate, and checks `GateAnswer` against `GateKind`.
3. On a decision answer, the use case builds a `DecisionRecord` from the human’s verbatim rationale, writes it through `LedgerPort`, then removes the gate and appends `DecisionRecorded`.
4. On a permission answer, the use case builds a `Grant`, calls `SandboxPort::apply_grant`, appends `PermissionDecided`, and removes the gate.
5. On `Defer`, the gate stays in `SessionSnapshot.gates` with `GateStatus::Deferred`, and the task transitions to `Paused`.
6. On `Reject`, the task transitions to `Planning`; no decision record is written.
7. A gate is never removed by timeout, process exit, or resume. Only an answer removes it.

### 9.5 Policy, review, and attestation seams

- `StartTaskUseCase` loads `ConfigSnapshot`, `DecisionPolicySnapshot`, and `ExecutionPolicySnapshot`, pins their hashes into `TaskSnapshot`, and calls `SandboxPort::probe`. It refuses `Guarded` for an untrusted repo and refuses `Sandboxed` if any canary fails.
- `PolicyEvaluator` runs at plan time, before every mutating action, and on replan. The model may add a `PolicyTrigger` with `TriggerSource::Model`; it cannot remove a `TriggerSource::Policy` trigger.
- `OpenReviewUseCase` reads the diff through `GitPort`, the ledger through `LedgerPort`, and the session through `SessionStorePort`, then assembles `ReviewPacket`, stores a `ReviewGate` in `SessionSnapshot.gates`, and appends `ReviewOpened`. It never reads a TUI type. `AttestTaskUseCase` removes the review gate only after the human attests.
- `AttestTaskUseCase` generates three deterministic anchors from real artifacts (or asks `ModelPort` to phrase them), builds a host rubric, judges each free-text answer, and creates `AttestationRecord` only after every required question passes and the human types `attest`.
- `MergeTaskUseCase` verifies the task is `Done`, the attestation commit is signed, `HEAD == C2`, `C2^ == C`, the subject tree and diff hashes match, and the merge is clean. It writes ledger backlinks into the merge commit, never into `C2`.
- `OverrideMergeUseCase` is the only bypass. It requires a typed task ID and a reason, writes `OverrideRecord`, merges with the override trailers, and leaves the task `Stopped`.

TDD-0004, TDD-0008, and TDD-0010 own the exact rules behind these steps. This document fixes the capability boundary and the records.

---

## 10. Fakes and tests

### 10.1 Fakes

The following test doubles live in `#[cfg(test)]` modules inside `app_core` or in `infra_local/tests/support`. They are never public and never ship in the production binary.

| Fake | Behavior |
|---|---|
| `FakeClock` | Fixed timestamp plus explicit `advance`; no wall-clock read. |
| `FakeIdSource` | Queued `TaskId`, `GateId`, `DecisionId`, and `AssumptionId`; can fail with `IdSourceError`. |
| `FakeConfigPort` | In-memory `ConfigSnapshot`; can return `ConfigError`. |
| `FakeSessionStore` | In-memory snapshots, plans, artifacts, events, and one lock; enforces sequence and stale-lock rules. |
| `FakePolicyPort` | In-memory `DecisionPolicySnapshot`; can return `PolicyError`. |
| `FakeLedgerPort` | In-memory records and index; enforces duplicate IDs and supports `verify`. |
| `FakeGitPort` | In-memory refs, worktrees, commits, merges, hooks, and signatures; can return `GitError`. |
| `FakeModelPort` | Scripted text, reasoning, tool calls, usage, errors, and cancellation. |
| `FakeSandboxPort` | Scripted mode, canaries, permission requests, grants, tool results, and cancellation. |
| `FakeAttestationPort` | In-memory attestations and overrides; verifies recorded hashes and signatures. |
| `FakeAuditPort` | In-memory audit entries and report. |

`FakeSessionStore` enforces `expected_sequence`; a stale save returns `SessionError::SequenceMismatch`. `FakeGitPort` never performs filesystem I/O; it records the exact `CommitRequest`, `AttestationCommitRequest`, and `MergeRequest` it receives so tests can assert trailers and bindings. `FakeSandboxPort` records every `Grant` and every `ToolRequest`.

### 10.2 Test layers

- **Domain unit tests** — pure, no fakes, no async. They cover value-object grammars, state transitions, plan validation, decision coverage, gate scheduling, and invariants.
- **Application use-case tests** — `pollster::block_on` with fakes. They cover plan, gates, resume, review, attestation, merge, reopen, override, budget, and model switching.
- **Mapping tests** — round-trip every public/private record pair; missing fields and newer schemas fail.
- **Port contract tests** — in `infra_local`, each adapter implements the public trait and is tested against the contract. The core tests use fakes; the adapter tests use the real adapter.
- **Architecture tests** — in `infra_local/tests/architecture.rs`, as required by `arch_docs/dependency-rules.md`. They are not owned by this TDD but depend on its module layout.

### 10.3 Required scenarios

The following scenarios must exist before TDD-0002 is considered implemented:

- `given_full_risk_plan_when_plan_gate_opens_then_execution_is_blocked`
- `given_light_risk_plan_when_no_blocking_decision_then_execution_starts`
- `given_pending_decision_when_deferred_then_task_pauses_and_gate_stays`
- `given_pending_decision_when_answered_then_ledger_record_has_verbatim_rationale`
- `given_agent_provenance_when_decision_answered_then_rejected`
- `given_permission_request_when_denied_then_task_stays_in_permission`
- `given_permission_request_when_allowed_once_then_grant_is_task_scoped`
- `given_untrusted_repo_when_guarded_requested_then_refused`
- `given_failed_canary_when_starting_then_mode_is_guarded`
- `given_crash_mid_action_when_resuming_then_intent_is_interrupted`
- `given_sequence_mismatch_when_resuming_then_events_are_trusted`
- `given_review_packet_when_opened_then_every_section_is_present_or_na`
- `given_failed_viva_attempt_when_two_attempts_exhausted_then_explain_or_stop`
- `given_all_viva_questions_passed_when_attest_typed_then_attestation_created`
- `given_commit_after_c2_when_merging_then_refused`
- `given_valid_attestation_when_merging_then_merge_is_no_ff_and_trailers_present`
- `given_unattested_branch_when_override_then_task_stays_stopped`
- `given_done_task_when_reopen_then_worktree_restored_and_attestation_superseded`
- `given_budget_paused_task_when_budget_increased_then_resumable`
- `given_model_switch_when_audited_then_session_records_it`

---

## 11. Requirement-to-test mapping

The mapping below is the acceptance evidence for this TDD. Every test name follows `arch_docs/naming-conventions.md` §12.2.

| Requirement or invariant | Test | Layer |
|---|---|---|
| NFR-MAINT-001 S1 — two crates, ports-only core | `architecture::given_workspace_when_inspected_then_two_members_and_ports_only` | architecture |
| NFR-MAINT-001 S2 — fakes for hard behavior | `application::given_plan_gate_and_merge_when_tested_then_fakes_cover_plan_gates_resume_review_attestation_merge_reopen_override` | application |
| NFR-MAINT-001 S3 — hidden worker | `architecture::given_binary_when_help_rendered_then_worker_is_absent` | architecture |
| DEP-01..DEP-03 — workspace and core dependency | `architecture::given_core_manifest_when_inspected_then_dependencies_are_exactly_async_trait_and_thiserror` | architecture |
| DEP-04..DEP-07 — package and lockstep version | `architecture::given_packages_when_inspected_then_harness_depends_on_core_with_exact_version` | architecture |
| DEP-08..DEP-09 — no forbidden imports or I/O | `architecture::given_core_sources_when_scanned_then_no_infrastructure_or_io_imports` | architecture |
| DEP-10..DEP-11 — ports/domain separation | `architecture::given_core_modules_when_scanned_then_ports_do_not_import_domain_or_application` | architecture |
| DEP-12 — infra uses only ports | `architecture::given_infra_sources_when_scanned_then_only_app_core_ports_is_imported` | architecture |
| DEP-13 — adapter isolation | `architecture::given_adapters_when_scanned_then_no_adapter_imports_another` | architecture |
| DEP-14 — worker isolation | `architecture::given_worker_when_scanned_then_no_host_adapter_is_imported` | architecture |
| DEP-19 — no type-erased errors | `architecture::given_ports_when_scanned_then_no_anyhow_or_boxed_error` | architecture |
| DEP-20 — no lock across await | `application::given_concurrent_gate_answers_when_racing_then_one_wins_and_no_lock_is_held` | application |
| INV-01 — one active task | `application::given_active_task_when_starting_second_then_lock_refused` | application |
| INV-02 — one open branch | `application::given_open_task_branch_when_starting_task_then_refused` | application |
| INV-03 — pending requests not in ledger | `application::given_pending_decision_when_recorded_then_ledger_untouched` | application |
| INV-04 — verbatim human rationale | `application::given_agent_authored_rationale_when_submitted_then_rejected` | application |
| INV-05 — host-owned writes | `application::given_worker_capability_when_writing_control_path_then_denied` | application |
| INV-06 — human attestation | `application::given_agent_attempt_when_attesting_then_rejected` | application |
| INV-07 — `C2^ == C` | `application::given_attestation_commit_when_parent_is_not_subject_then_merge_refused` | application |
| INV-08 — merge only through Forge | `application::given_unattested_branch_when_merging_then_refused` | application |
| INV-09 — no gate timeout | `application::given_unanswered_gate_when_clock_advances_then_no_default_answer` | application |
| INV-10 — canaries before `Sandboxed` | `application::given_failed_canary_when_starting_then_mode_is_guarded` | application |
| INV-11 — untrusted requires sandbox | `application::given_untrusted_repo_when_guarded_requested_then_refused` | application |
| INV-12 — `Guarded` label | `domain::given_guarded_mode_when_rendered_then_label_says_not_sandboxed` | domain |
| INV-13 — instruction sources | `application::given_repository_content_when_request_built_then_every_block_is_enveloped` | application |
| INV-14 — protected path | `application::given_protected_path_change_when_committing_then_refused` | application |
| INV-15 — immutable rationale | `application::given_superseded_decision_when_read_then_rationale_unchanged` | application |
| INV-16 — newer schema read-only | `application::given_newer_schema_when_loaded_then_read_only_error` | application |
| INV-17 — one repo per session | `application::given_two_repos_when_sessions_run_then_state_is_isolated` | application |
| INV-18 — evidence required | `application::given_no_evidence_when_declaring_verified_then_rejected` | application |
| INV-19 — unknown usage | `application::given_unknown_usage_when_costed_then_usd_is_unknown_not_zero` | application |
| INV-20 — no secrets or reasoning in commits | `application::given_reasoning_or_secret_when_attesting_then_record_rejected` | application |
| FR-PLAN-004 — risk classification | `domain::given_policy_trigger_when_classified_then_full_risk` | domain |
| FR-PLAN-005 — decision map | `domain::given_blocking_decision_when_execution_requested_then_refused` | domain |
| FR-PLAN-006 — unknown buckets | `domain::given_unbucketed_unknown_when_plan_validated_then_error` | domain |
| FR-PLAN-007 — plan revisions | `domain::given_material_change_when_replanned_then_new_version_and_review` | domain |
| FR-GATE-001 — decision content | `application::given_policy_trigger_when_gate_opens_then_options_recommendation_and_no_default` | application |
| FR-GATE-002 — answer forms | `application::given_decision_answer_when_defer_then_task_pauses` | application |
| FR-GATE-003 — permission content | `application::given_permission_denied_when_no_alternative_then_task_stops` | application |
| FR-GATE-004 — grant scopes | `application::given_credential_read_when_granting_then_once_only` | application |
| FR-GATE-005 — durable gate | `application::given_pending_gate_when_process_restarts_then_gate_reprinted` | application |
| FR-EXEC-003 — execution modes | `domain::given_each_execution_mode_when_rendered_then_exact_label` | domain |
| FR-EXEC-008 — six canaries | `application::given_six_canaries_when_probed_then_sandboxed_only_when_all_pass` | application |
| FR-EXEC-010 — untrusted repo | `application::given_untrusted_repo_when_host_cannot_sandbox_then_run_refused` | application |
| FR-EXEC-011 — policy pinning | `application::given_policy_change_mid_task_when_resumed_then_paused` | application |
| FR-EXEC-013 — intent/result | `application::given_intent_without_result_when_resumed_then_interrupted` | application |
| FR-LEDGER-003 — rationale | `application::given_blank_rationale_when_decision_answered_then_rejected` | application |
| FR-LEDGER-005 — append-only | `application::given_supersession_when_written_then_old_record_unchanged` | application |
| FR-LEDGER-010 — pending vs resolved | `application::given_resolved_decision_when_written_then_only_record_enters_ledger` | application |
| FR-VER-005 — evidence | `application::given_check_when_completed_then_artifact_has_command_exit_time_tree_hash` | application |
| FR-VER-007 — verified checklist | `application::given_missing_criterion_when_verifying_then_not_review` | application |
| FR-REV-001 — review packet | `application::given_review_when_opened_then_every_section_present_or_na` | application |
| FR-REV-006 — three anchors | `application::given_attestation_when_started_then_three_artifact_backed_questions` | application |
| FR-REV-008 — viva | `application::given_two_failed_attempts_when_viva_continues_then_explain_or_stop` | application |
| FR-REV-010 — attestation record | `application::given_passed_viva_when_attested_then_record_binds_subject_tree_diff_plan_policy_ledger_evidence` | application |
| FR-REV-011 — signed C2 | `application::given_unsigned_c2_when_merging_then_refused` | application |
| FR-MERGE-001 — merge preconditions | `application::given_attested_done_task_when_merging_then_no_ff_with_trailers` | application |
| FR-MERGE-003 — base and clean tree | `application::given_dirty_main_worktree_when_merging_then_refused` | application |
| FR-MERGE-004 — binding | `application::given_ledger_hash_mismatch_when_merging_then_refused` | application |
| FR-MERGE-005 — reopen | `application::given_done_task_when_reopened_then_worktree_restored_and_v2_required` | application |
| FR-MERGE-006 — override | `application::given_unattested_branch_when_override_then_signed_record_and_stopped` | application |
| FR-MERGE-010 — backlinks | `application::given_merge_when_committed_then_backlinks_are_in_merge_commit_not_c2` | application |
| FR-REC-002 — atomic state | `application::given_crash_mid_save_when_loading_then_old_snapshot_or_event_rebuild` | application |
| FR-REC-003 — resume | `application::given_paused_task_when_resumed_then_header_last_50_and_gate` | application |
| FR-REC-005 — crash recovery | `application::given_partial_file_write_when_resumed_then_hash_detects_it` | application |
| FR-REC-009 — state machine | `domain::given_every_transition_when_checked_then_only_the_table_is_allowed` | domain |
| NFR-REL-001 — crash safety | `application::given_truncated_event_tail_when_loaded_then_ignored` | application |
| NFR-REL-002 — durable gate | `application::given_gate_when_process_exits_then_state_persists` | application |
| NFR-COST-001 — budget | `application::given_hard_budget_when_reached_then_paused_and_resumable_with_increase` | application |
| NFR-PRIV-001 — local-only | `application::given_session_and_reasoning_when_scanned_then_only_local_paths` | application |
| NFR-OBS-001 — status/audit | `application::given_degraded_hooks_when_status_then_degraded_is_explicit` | application |
| NFR-SCALE-001 — repo isolation | `application::given_two_repos_when_sessions_run_then_no_cross_repo_state` | application |
| NFR-SCALE-002 — repo-level state | `application::given_monorepo_when_state_created_then_one_ledger_policy_and_lock` | application |

---

## 12. Seams with later TDDs

This document fixes the Rust contract. The following TDDs own the encodings and mechanics behind it. They may add fields or variants only by amending this document; they may not change the meaning of an existing field.

| TDD | Owns | May add without amendment |
|---|---|---|
| TDD-0001 | `.forge/config` TOML schema, init flow, repo-map confirmation, CLI | New `ConfigSnapshot` fields if they are optional and mapped to a default; new `CheckKind` variants |
| TDD-0003 | `state.json`, `events.jsonl`, `plans/`, `artifacts/`, resume algorithm, lock file | New `SessionEventKind` variants; new `SessionSnapshot` persistence metadata |
| TDD-0004 | Plan materiality, policy trigger rules, gate content, state-machine guards, failure policy | New `PolicyTriggerKind` variants only with an amendment; new domain-service internals |
| TDD-0005 | Decision and assumption Markdown/YAML, validation, index, trailer parsing | New `DecisionRecord`/`AssumptionRecord` fields only with an amendment |
| TDD-0006 | MXC policy JSON, worker IPC framing, permission store, canaries | New `SandboxProbeCheck` detail; new `SandboxError` variants |
| TDD-0007 | Provider profiles, wire mappings, price table, budget arithmetic | New `ModelInfo` fields; new `ModelError` variants |
| TDD-0008 | Git adapter mechanics, attestation JSON, signing, hooks, merge, reopen | New `AttestationRecord`/`OverrideRecord` fields only with an amendment |
| TDD-0009 | TUI layout, ANSI, plain mode, `TerminalError` | Nothing in the core |
| TDD-0010 | Sanitizer, caps, secret scan, steering patterns, egress, review flags | New `ReviewFlag` and `SteeringClass` values |
| TDD-0011 | Dogfood diary, metrics, report | New `SessionEventKind` metric fields |

### 12.1 Open items

- TDD-0007 must confirm that `ModelRequest`/`ModelResponse` are sufficient for both Anthropic Messages and OpenAI Chat Completions; if not, it amends this document rather than leaking a provider type.
- TDD-0008 must confirm that `AttestationRecord` carries every binding in FR-REV-010 and FR-MERGE-004; if not, it amends this document.
- TDD-0004 must confirm that the `PolicyEvaluator` boundary is sufficient for plan-time, pre-action, and replan checks; if not, it amends this document.
- `arch_docs/architecture.md` §9.2 is corrected by this TDD to remove `TerminalError` from the core error list. The architecture doc remains authoritative for the boundary; the correction is recorded in the same change.

---

## 13. Traceability

| Source | Covered by |
|---|---|
| PRD §5 domain model and ubiquitous language | §5; §6.2–6.8 |
| PRD §6.2 host/worker split | §7.2 `SandboxPort`; §9.5 |
| PRD §6.3 authoritative state | §6.3 `SessionSnapshot`; §7.2 `SessionStorePort` |
| PRD §6.5 workspace and module boundaries | §3; §4; §9.3 |
| PRD §8 interfaces | §6; §7 |
| PRD §9 data model | §6.3–6.9; §9.2 |
| PRD §10.2 state machine | §5.5; §11 `FR-REC-009` |
| D-08 ledger | §6.8; §9.2 |
| D-09 state and resume | §5.5; §6.11; §9.2 |
| D-13 scope and stack | §3; §4 |
| D-16 providers and onboarding | §6.9; §9.5 |
| D-20 monorepo | §6.9; §9.5 |
| NFR-MAINT-001 | §3; §4; §10; §11 |
| NFR-REL-001, NFR-REL-002 | §7.4; §11 |
| NFR-COST-001 | §6.3; §6.10; §11 |
| NFR-PRIV-001 | §5.7 INV-20; §6.8; §11 |
| NFR-SCALE-001, NFR-SCALE-002 | §5.7 INV-17; §6.9; §11 |
| NFR-OBS-001 | §6.11; §11 |
| ADR-0001 | §3; §4; §9.3 |
| ADR-0002 | §3; §7.4 |
| ADR-0003 | §3; §4 |
| ADR-0004 | §6.10; §7.2 `ModelPort`; §7.4 |
| ADR-0005 | §5.2; §6.2; §7.2 `IdSourcePort` |
| Ticket #21 | All sections |
| Ticket #20 resolution | §3; §4; §6.1; §9.3 |
