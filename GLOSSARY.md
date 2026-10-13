# Forge

Forge is a Human-in-the-Loop coding harness that prevents cognitive debt in AI-assisted development: AI executes routine engineering work, while consequential engineering decisions stay developer-owned.

## Language

**Forge**:
The product this repo builds: a decision-governance harness around an AI coding agent.
_Avoid_: agent, assistant, bot.

**Pi**:
The extensible AI agent harness (`earendil-works/pi`, formerly `badlogic/pi-mono`) that Forge orchestrates as the underlying coding agent.
_Avoid_: pi-mono, Pi CLI, the model.

**Consequential engineering decision**:
A decision that materially changes or constrains system behavior, invariants, interfaces, data ownership, persistence semantics, boundaries, security, concurrency, external interactions, significant performance characteristics, or future implementation constraints (PRD §3.6).
_Avoid_: architectural decision (too narrow), design choice.

**Developer decision**:
A consequential engineering decision owned by the developer, as opposed to an AI recommendation, which can inform but never replace it.
_Avoid_: approval, sign-off.

**Agent authority**:
The boundary between work the AI agent may execute autonomously and the decisions that must stay developer-owned.
_Avoid_: permissions, autopilot.

**Gate extension**:
Forge's thin component inside the governed Pi process that is the sole enforcement actuator: it blocks tool calls before execution and holds each pause until an answer returns from Forge's core, carrying no policy of its own.
_Avoid_: plugin, sandbox, permission system.

**Agent loop**:
Pi's conversation engine inside the governed process: model turns, tool dispatch, results, retries.
_Avoid_: governance loop, orchestration.

**Governance loop**:
Forge's supervision cycle around the agent loop: task entry, authority verdicts, developer decisions, specification, checkpoints, provenance.
_Avoid_: agent loop, control loop.

**TDD (Technical Design Document)**:
The repo-owned artifact recording what the system must do and the design behind a developer decision, persisted under `/spec/tdd/`.
_Avoid_: test-driven development, tests, spec (ambiguous).

**ADR (Architecture Decision Record)**:
The repo-owned artifact recording why a structural or architectural approach was chosen, persisted under `/spec/adr/`.
_Avoid_: design doc.

**Specification conflict**:
A contradiction between an accepted specification and the implementation; Forge surfaces it for developer resolution rather than choosing silently.
_Avoid_: bug, drift.

**Cognitive checkpoint**:
A risk-triggered verification that the developer can reason about a consequential decision (causes, trade-offs, failure modes, boundaries), not merely acknowledge it.
_Avoid_: quiz, confirmation.

**Decision provenance**:
The traceable chain decision → specification → implementation → tests, queryable in both directions.
_Avoid_: traceability (generic), audit log.
