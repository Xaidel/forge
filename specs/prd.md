
# Product Requirement Document: Forge

**Core Thesis:** Forge keeps developers cognitively engaged in consequential engineering decisions while allowing AI to autonomously handle routine implementation work.

**Guiding Principle:** **Developers drive AI, not the other way around.**

**Supporting Principle:** **Automate execution, not understanding.**

---

# 1. Executive Summary

Forge is a Human-in-the-Loop (HITL) coding harness designed to prevent cognitive debt in AI-assisted software development.

Modern coding agents can independently plan, reason, implement, test, and modify large portions of a codebase. This increases development velocity, but it creates a new failure mode: developers become increasingly detached from the reasoning behind the systems they maintain.

Forge addresses this by establishing a strict boundary between AI-owned execution and developer-owned consequential engineering decisions.

AI agents autonomously execute routine engineering work. However, when Forge identifies a consequential engineering decision, it pauses autonomous execution and brings the developer into the loop **before proceeding past that decision**.

The decision may be discovered during planning or during implementation. Forge does not require the AI to predict every consequential decision before execution begins. It requires the AI to stop when it encounters one.

Consequential decisions are persisted as repository-owned specifications under:

```text
/spec/tdd/
/spec/adr/
```

Forge maintains the relationship between:

```text
Decision
    ↓
Specification
    ↓
Implementation
    ↓
Tests
```

This provides durable decision provenance, specification integrity, conflict detection, and a basis for risk-based cognitive checkpoints.

Forge does not attempt to maximize human involvement. It aims for **minimum necessary intervention while preserving human ownership of consequential engineering decisions.**

The fundamental relationship is:

```text
Developer intent
      ↓
Developer decisions
      ↓
Forge governs execution boundaries
      ↓
AI executes
```

AI exists to amplify developer intent, not determine developer direction.

---

# 2. Problem

## 2.1 Cognitive Debt

Cognitive debt is the gap between the complexity of a software system and a developer's ability to understand its architecture, decisions, constraints, and behavior.

* **Technical debt** concerns the future cost of maintaining poorly structured or suboptimal code.
* **Knowledge debt** concerns critical system knowledge existing only in individual memory or unrecorded discussions.
* **Cognitive debt** concerns the developer losing the ability to reason about the system because critical reasoning was delegated to AI.
* **Specification debt** concerns specifications no longer accurately representing the system's actual behavior or constraints.

A developer experiencing cognitive debt cannot confidently answer:

* Why does this component exist?
* What constraints does this implementation depend on?
* What behavior is guaranteed?
* What decision caused this architecture?
* Which existing decisions does a proposed change invalidate?
* What happens when an important assumption fails?

The fundamental danger is not that AI writes bad code.

The danger is that **AI can write code that works while the developer gradually loses the ability to reason about why it works.**

---

## 2.2 How AI Creates Cognitive Debt

AI-assisted development creates cognitive debt when:

* AI performs consequential architectural reasoning without exposing the decision.
* Important decisions exist only inside transient AI chat sessions.
* Developers approve generated implementations without understanding their trade-offs.
* AI modifies existing behavior without identifying affected specifications.
* Multiple AI sessions make independent and conflicting assumptions.
* AI silently resolves conflicts between existing specifications and implementation.
* Developers repeatedly accept AI recommendations without retaining the reasoning behind them.
* Developers begin adapting their engineering decisions to the behavior of the AI rather than directing the AI toward their intended solution.

The problem is not AI autonomy itself.

The problem occurs when:

> **AI autonomy crosses the boundary from execution into consequential engineering decisions without developer ownership.**

---

## 2.3 Debt Comparison

| Debt Type              | Definition                                                                                                 |
| :--------------------- | :--------------------------------------------------------------------------------------------------------- |
| **Technical Debt**     | Implementation shortcuts or structural problems that increase future maintenance cost.                     |
| **Knowledge Debt**     | Critical system knowledge exists only in individual memory or unrecorded discussions.                      |
| **Cognitive Debt**     | The developer loses the ability to reason about the system because critical reasoning was delegated to AI. |
| **Specification Debt** | Specifications no longer accurately represent the system's actual behavior or constraints.                 |

Forge primarily targets **cognitive debt**.

TDDs, ADRs, provenance tracking, decision enforcement, and cognitive validation are mechanisms Forge uses to prevent cognitive debt from compounding into knowledge or specification debt.

---

# 3. Product Thesis & Decision Boundary

## 3.1 Developers Drive AI

Forge establishes a fundamental relationship between developers and AI:

> **Developers drive AI, not the other way around.**

The developer establishes:

* intent;
* requirements;
* consequential engineering decisions;
* constraints;
* acceptable trade-offs;
* scope.

AI uses that direction to perform autonomous execution.

The AI may propose alternatives, identify risks, and recommend approaches, but those capabilities exist to support developer decision-making rather than replace it.

The intended relationship is:

```text
Developer
    ↓
Intent
    ↓
Decisions
    ↓
Constraints
    ↓
AI execution
    ↓
Implementation
```

Not:

```text
AI
    ↓
Generated implementation
    ↓
Developer adapts to AI's decisions
    ↓
System direction
```

Forge therefore treats developer intent as the source of direction and AI execution as the mechanism for carrying that direction out.

---

## 3.2 Automate Execution, Not Understanding

Forge allows AI agents to autonomously perform routine implementation work while requiring developers to participate in consequential engineering decisions.

The central product boundary is:

```text
Developer
    │
    │ owns
    ▼
Consequential Decisions
    │
    │ persisted as
    ▼
TDD / ADR
    │
    │ executed by
    ▼
AI Agent
    │
    ▼
Routine Implementation
```

Forge does not require developers to approve every code change.

Instead:

> **The developer owns consequential decisions. The AI executes decisions that have already been made.**

---

## 3.3 Product Boundary

Forge is a **decision-governance harness around AI coding agents**.

Forge is responsible for:

* enforcing the decision boundary;
* determining whether the agent is authorized to proceed;
* managing developer-owned consequential decisions;
* persisting consequential decisions as specifications;
* detecting specification conflicts;
* maintaining decision provenance;
* triggering risk-based cognitive checkpoints.

The underlying AI coding agent is responsible for:

* repository exploration;
* routine implementation;
* routine refactoring;
* test creation from established requirements;
* test execution;
* validation;
* localized defect correction within established requirements.

Forge does not need to replace the underlying coding intelligence.

Its primary responsibility is to govern **when that intelligence may act autonomously and when it must defer to the developer.**

---

## 3.4 Decision Boundary

Forge establishes a boundary between AI-owned execution and developer-owned consequential decisions.

AI may autonomously:

* analyze tasks;
* inspect the repository;
* inspect existing specifications;
* implement routine changes;
* write tests from existing specifications;
* execute validation;
* perform mechanical refactoring;
* fix localized defects without changing established requirements.

AI must stop and involve the developer when it encounters a consequential engineering decision.

The decision boundary may be encountered during:

* initial planning;
* repository exploration;
* implementation;
* testing;
* debugging;
* integration.

Therefore:

> **The AI is not required to know every consequential decision before execution begins. It is required to stop when it encounters one.**

---

## 3.5 Decision Ownership

Forge distinguishes between an AI recommendation and a developer decision.

The AI may:

* identify a decision;
* explain alternatives;
* identify trade-offs;
* recommend an approach;
* identify potential consequences;
* identify risks;
* challenge an existing assumption.

The AI may not silently establish the final consequential decision.

The developer must explicitly own the decision before Forge allows dependent implementation to proceed.

Therefore:

```text
AI Recommendation ≠ Developer Decision
```

A recommendation may inform the developer's decision, but does not replace it.

---

## 3.6 Consequential Engineering Decision

A consequential engineering decision is a decision that materially changes or constrains one or more of:

* system behavior;
* domain invariants;
* public interfaces;
* data ownership;
* persistence semantics;
* architectural boundaries;
* security boundaries;
* concurrency semantics;
* external system interactions;
* significant performance characteristics;
* future implementation constraints.

Examples include:

* choosing a persistence strategy;
* changing a transaction boundary;
* introducing a new architectural layer;
* changing ownership of domain data;
* introducing a new external dependency with architectural implications;
* changing authentication or authorization architecture;
* changing concurrency or consistency semantics;
* changing a public API contract;
* changing a domain invariant.

Routine implementation choices that do not materially affect these concerns remain within the AI agent's authority.

---

## 3.7 Decision Detection

A potential consequential decision may be identified by:

1. the AI agent during planning or implementation; or
2. Forge's decision-boundary policies and validation mechanisms.

The AI may raise a decision candidate.

However, the AI cannot determine that it is authorized to proceed solely because it does not consider the decision consequential.

Forge remains responsible for enforcing the authority boundary.

This prevents the system from relying entirely on the AI to decide when the AI itself requires supervision.

---

# 4. Agent Authority Model

Forge explicitly defines what the AI agent may and may not do autonomously.

| Action                                         |  AI | Developer |
| :--------------------------------------------- | :-: | :-------: |
| Read repository                                | Yes |    Yes    |
| Read specifications                            | Yes |    Yes    |
| Analyze a task                                 | Yes |    Yes    |
| Implement an existing TDD                      | Yes |    Yes    |
| Write tests from established requirements      | Yes |    Yes    |
| Mechanical refactoring                         | Yes |    Yes    |
| Run validation                                 | Yes |    Yes    |
| Fix localized implementation defects           | Yes |    Yes    |
| Propose an architectural approach              | Yes |    Yes    |
| Recommend a consequential decision             | Yes |    Yes    |
| Choose a consequential architecture            |  No |    Yes    |
| Establish a new consequential decision         |  No |    Yes    |
| Create a developer-owned consequential ADR/TDD |  No |    Yes    |
| Change active TDD semantics                    |  No |    Yes    |
| Resolve a consequential specification conflict |  No |    Yes    |
| Expand consequential scope                     |  No |    Yes    |
| Silently change behavioral requirements        |  No |    Yes    |

The fundamental authority rule is:

> **AI may execute a developer-owned decision, but may not silently become the owner of a consequential engineering decision.**

---

# 5. Decision Lifecycle

A consequential decision follows a defined lifecycle.

```text
Task
  ↓
Analyze
  ↓
Execute routine work
  ↓
Decision candidate discovered
  ↓
Is developer ownership required?
  ├── No
  │    ↓
  │  Continue
  │
  └── Yes
       ↓
     Pause
       ↓
Present decision + alternatives + trade-offs
       ↓
Developer decides
       ↓
Persist decision as TDD / ADR
       ↓
Validate specification
       ↓
Continue implementation
       ↓
Run validation
       ↓
Cognitive checkpoint if required
       ↓
Complete
```

A decision may therefore be discovered **after implementation has already started**.

Forge must pause before the agent proceeds beyond that decision.

---

# 6. Specification System

## 6.1 Repository-Owned Specifications

Consequential decisions must have a durable representation inside the repository.

Forge uses:

```text
/spec/tdd/
/spec/adr/
```

The repository, rather than the AI conversation, is the durable source of consequential engineering decisions.

AI conversation history may provide context, but it is not considered authoritative project knowledge.

---

## 6.2 TDD

A TDD defines:

> **What the system must do.**

A TDD may describe:

* required behavior;
* inputs and outputs;
* domain invariants;
* acceptance criteria;
* failure behavior;
* validation requirements;
* test expectations.

TDDs should be used when the consequential decision primarily establishes or changes system behavior.

---

## 6.3 ADR

An ADR defines:

> **Why a structural or architectural approach was chosen.**

An ADR may describe:

* context;
* problem;
* considered alternatives;
* decision;
* trade-offs;
* consequences;
* constraints.

ADRs should be used when the consequential decision primarily establishes or changes architecture, boundaries, infrastructure, integration strategy, or other structural concerns.

---

## 6.4 TDD and ADR Relationship

A consequential architectural decision may produce both an ADR and one or more TDDs.

For example:

```text
ADR
 ↓
Architectural decision
 ↓
TDD
 ↓
Required behavior
 ↓
Implementation
 ↓
Tests
```

The distinction is:

```text
ADR = Why / structural decision

TDD = What / required behavior
```

---

# 7. Specification Lifecycle

Specifications are not immutable.

A specification follows:

```text
Draft
  ↓
Accepted
  ↓
Active
  ↓
Superseded
  ↓
Archived
```

An active specification represents the current accepted decision.

When a consequential decision changes, Forge should preserve the historical decision rather than silently rewriting it.

Example:

```text
ADR-003
Authentication uses Keycloak.
        ↓
      Active
        ↓
ADR-017
Authentication migrated to Auth0.
        ↓
ADR-003 → Superseded
```

Historical decisions remain traceable.

A new decision should supersede an existing decision when the new decision changes the assumptions, constraints, or architectural consequences of the previous decision.

---

# 8. Specification Integrity

## 8.1 Specification Conflicts

Forge must detect and surface consequential conflicts between specifications and implementation.

Example:

```text
TDD:
User cannot activate before verification.

Implementation:
User activates immediately.

        ↓

SPECIFICATION CONFLICT
        ↓
      PAUSE
        ↓
Developer resolves
```

The developer must determine whether:

1. the implementation is incorrect; or
2. the specification is outdated.

Forge must not silently choose between these outcomes.

---

## 8.2 AI Cannot Rewrite Specifications to Resolve Its Own Conflicts

AI must not modify an active TDD or ADR merely to make its implementation conform to its own decision.

The correct flow is:

```text
Conflict
  ↓
Pause
  ↓
Developer evaluates
  ├── Implementation wrong
  │      ↓
  │   Fix implementation
  │
  └── Specification outdated
         ↓
      New developer decision
         ↓
      Update / supersede specification
```

This protects specification integrity and decision ownership.

---

# 9. Developer Decision Validation

Forge recognizes that developer ownership does not imply that every developer decision is automatically correct.

Forge may challenge a developer decision without silently replacing it.

## 9.1 Recommendation

Forge presents:

* alternatives;
* trade-offs;
* risks;
* expected consequences.

The developer may select a valid alternative.

---

## 9.2 Warning

When Forge identifies a significant concern, it may require explicit developer acknowledgment.

Examples:

* conflict with an existing specification;
* significant operational cost;
* compatibility concern;
* major performance trade-off;
* increased system complexity.

---

## 9.3 Hard Block

Forge may prevent execution when a proposed decision violates a mandatory system or Forge constraint.

Examples:

* modifying protected specifications without authority;
* bypassing mandatory security constraints;
* performing destructive operations outside granted scope;
* violating explicit repository policies.

Hard blocks must be reserved for violations of established constraints.

Forge should not use hard blocks merely because the AI disagrees with the developer's engineering judgment.

---

# 10. Cognitive Checkpoints

Forge may use cognitive checkpoints to verify that the developer retains sufficient understanding of consequential decisions.

A cognitive checkpoint should test the developer's ability to reason about the decision rather than merely acknowledge it.

A checkpoint should test:

* causal reasoning;
* trade-offs;
* failure modes;
* consequences;
* architectural boundaries.

Examples:

> Why is this validation implemented at the domain boundary rather than the HTTP handler?

> What happens if the second repository operation fails under this transaction strategy?

> What behavior changes if this invariant is removed?

> Why was this responsibility placed in this component?

The developer should attempt the checkpoint before receiving AI assistance.

Forge may then provide:

* explanation;
* relevant specification context;
* trade-off analysis;
* corrective feedback.

The developer may retry after receiving assistance.

A simple confirmation such as "I understand" is not considered sufficient evidence of understanding.

---

# 11. Cognitive Checkpoint Trigger Policy

Cognitive checkpoints must be risk-based.

Forge should not interrupt developers for routine implementation.

A conceptual risk model is:

```text
Routine CRUD
  ↓
No checkpoint

Small behavioral change
  ↓
Possible checkpoint

New transaction strategy
  ↓
Checkpoint

Architectural change
  ↓
Checkpoint

Security boundary change
  ↓
Checkpoint
```

The principle is:

> **The greater the cognitive consequence of a decision, the greater the required developer engagement.**

The objective is not to maximize checkpoints.

The objective is to preserve understanding while minimizing unnecessary interruption.

---

# 12. Decision Provenance

Forge maintains a traceable relationship between decisions and implementation.

The intended provenance chain is:

```text
Developer Decision
       ↓
    TDD / ADR
       ↓
 Implementation
       ↓
     Tests
```

Forge should be able to answer:

> Why does this implementation exist?

For example:

```text
Implementation
  ↓
TDD-014
  ↓
ADR-006
  ↓
Developer decision
```

Forge should also be able to answer:

> Where is this decision implemented?

For example:

```text
ADR-006
  ↓
TDD-014
  ↓
Implementation
  ↓
Tests
```

Specifications therefore function as active engineering artifacts rather than documentation generated for its own sake.

---

# 13. Minimum Necessary Intervention

Forge must optimize for **minimum necessary intervention**.

Too little intervention creates cognitive debt.

Too much intervention destroys the productivity benefit of AI autonomy.

Therefore Forge should distinguish:

```text
Routine execution
        ↓
AI autonomy
```

from:

```text
Consequential decision
        ↓
Developer ownership
```

The objective is:

> **Maximize autonomous execution subject to preserving developer ownership and understanding of consequential engineering decisions.**

---

# 14. Failure Modes Forge Must Prevent

Forge should explicitly guard against the following failure modes.

## 14.1 Silent Architectural Decisions

AI introduces consequential architecture without developer ownership.

## 14.2 Specification Drift

Implementation changes while the specification remains inaccurate.

## 14.3 Specification Contradiction

New implementation conflicts with an existing accepted decision.

## 14.4 Decision Evaporation

A consequential decision exists only in an AI conversation and is lost afterward.

## 14.5 Blind Acceptance

The developer repeatedly accepts AI recommendations without understanding their consequences.

## 14.6 Scope Expansion

AI turns a routine task into a consequential architectural change without developer involvement.

## 14.7 Documentation Theater

Forge generates specifications that do not participate in implementation, testing, or decision provenance.

## 14.8 Excessive Intervention

Forge interrupts developers for routine implementation choices that do not materially affect the system.

## 14.9 AI-Driven Direction

The developer begins adapting their goals, architecture, or engineering decisions to the AI's generated implementation rather than using AI to execute developer-defined intent.

Forge should treat this as a fundamental failure of the developer-AI relationship.

---

# 15. Success Metrics

Forge should measure whether it actually reduces cognitive debt rather than merely increasing developer interaction.

## 15.1 Decision Provenance Coverage

Percentage of consequential implementation changes associated with an accepted TDD or ADR.

## 15.2 Decision Retention

Percentage of relevant consequential decisions that the developer can correctly explain after implementation.

## 15.3 Cognitive Checkpoint Performance

Percentage of cognitive checkpoints correctly answered without AI assistance.

## 15.4 Intervention Rate

Frequency with which Forge interrupts autonomous execution.

## 15.5 Intervention Precision

Percentage of Forge interventions judged to be genuinely consequential.

## 15.6 Intervention Efficiency

```text
Valuable consequential decisions surfaced
------------------------------------------
Developer interruptions
```

The goal is not to maximize intervention count.

A successful Forge should surface important decisions while minimizing unnecessary interruptions.

## 15.7 Developer Direction Preservation

Measure whether consequential engineering decisions originate from explicit developer intent rather than being implicitly determined by AI-generated implementation.

The goal is:

> **AI execution should follow developer direction, not establish it.**

---

# 16. MVP Scope

The first version of Forge should focus on proving the central product thesis rather than implementing every possible governance capability.

The MVP should demonstrate the following loop:

```text
Developer gives task
        ↓
AI agent executes routine work
        ↓
Consequential decision encountered
        ↓
Forge pauses execution
        ↓
Developer chooses the decision
        ↓
Decision becomes TDD / ADR
        ↓
AI continues implementation
        ↓
Forge validates against specification
        ↓
Risk-based cognitive checkpoint
        ↓
Task completes
```

The MVP should therefore prioritize:

1. repository initialization;
2. task execution;
3. agent authority enforcement;
4. consequential decision detection;
5. developer decision interaction;
6. TDD / ADR persistence;
7. specification conflict detection;
8. basic decision provenance;
9. risk-based cognitive checkpoints.

The MVP does **not** need to solve complete codebase understanding, fully automated architecture evaluation, or universal cognitive-debt measurement.

---

# 17. Non-Goals

Forge is not intended to:

* replace developer judgment;
* guarantee whole-codebase understanding;
* prevent all AI defects;
* automatically make consequential architecture decisions;
* require human approval for every code change;
* document every implementation detail;
* replace tests;
* replace static analysis;
* replace code review;
* guarantee architecture correctness;
* act as a project-management system;
* act as a generic AI documentation generator;
* maximize human interactions;
* eliminate the need for experienced engineers.

Forge is specifically concerned with:

> **Maintaining human ownership and understanding of consequential engineering decisions while allowing AI to autonomously execute routine engineering work.**

---

# 18. Product Invariants

Forge must preserve the following invariants.

## 18.1 Developer Direction

Developer intent is the source of direction.

AI execution must remain subordinate to developer-owned requirements, decisions, and constraints.

## 18.2 Decision Ownership

AI may execute a developer-owned decision, but may not silently become the owner of a consequential engineering decision.

## 18.3 Decision Persistence

A consequential decision must have a durable repository representation before dependent implementation is finalized.

## 18.4 Specification Integrity

AI must not silently modify active specifications to make its implementation conform to its own decisions.

## 18.5 Conflict Visibility

Forge must surface consequential conflicts rather than silently selecting a resolution.

## 18.6 Proportional Intervention

Forge should intervene only when the expected cognitive or engineering risk justifies developer interruption.

## 18.7 Repository Ownership

The repository is the durable source of consequential engineering decisions.

AI conversation history is not authoritative project knowledge.

## 18.8 Historical Integrity

Superseded decisions remain traceable and must not be silently rewritten as though they were never made.

---

# 19. Product Model

Forge can be summarized as:

```text
                 ┌─────────────────────────┐
                 │          Forge          │
                 │                         │
Developer ──────►│ Developer Direction     │
                 │ Decision Boundary       │
                 │ Agent Authority         │
                 │ Specification           │
                 │ Conflict Detection      │
                 │ Provenance              │
                 │ Cognitive Checkpoints   │
                 └───────────┬─────────────┘
                             │
                             ▼
                      AI Coding Agent
                             │
                             ▼
                        Repository
```

The central relationship is:

```text
Developer
    ↓
Defines intent
    ↓
Owns consequential decisions
    ↓
Forge
    ↓
Enforces, persists, tracks, and validates those decisions
    ↓
AI
    ↓
Executes routine implementation
```

The central distinction is therefore not:

```text
Human vs AI
```

It is:

```text
Decision vs Execution
```

And the governing relationship is:

```text
Developer → AI
```

not:

```text
AI → Developer
```

---

# 20. Core Product Principle

Forge exists to preserve the boundary between **AI-executable implementation** and **human-owned engineering decisions**.

Its purpose is not to make developers approve more AI output.

Its purpose is to ensure that as AI becomes capable of executing more of the software development lifecycle, developers remain the ones directing what the software should become.

> **Developers drive AI, not the other way around.**

> **Automate execution, not understanding.**
