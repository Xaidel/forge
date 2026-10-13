# ADR: Attach to Pi over RPC with a thin gate extension

Type: project
Status: accepted
Classification: architecture

## Problem

Forge must enforce an authority boundary: the governed agent executes routine engineering work, while consequential engineering decisions stay developer-owned (PRD §4–§5, [spec/prd.md](../../../prd.md)).
Enforcement needs a mechanism that can stop a model-issued tool call before it executes and hold a pending decision until the developer answers.
Pi offers no such mechanism over RPC alone: the wire has no pre-execution block, and the earliest signal that a tool call exists is emitted when execution has already begun (ticket [Research: Which Pi surfaces can host Forge's governance?](https://github.com/Xaidel/forge/issues/2)).
The only pre-execution gate is an in-process TypeScript extension's `tool_call` handler, which can mutate input, block, and fail closed.
The surrounding constraints: Forge is a Rust + Ratatui TUI, Pi runs under Forge and does not own the UI, and pure Rust is preferred where the platform allows.
The decision question: how does Forge attach to Pi, and where does enforcement live?

## Decision

Forge is a Rust core supervising a Pi child over RPC plus a thin TypeScript gate extension inside Pi as the sole enforcement actuator; the core owns all policy and the extension owns none.

- Every model-issued tool call round-trips to the core: the extension appends the pending call (structured, via `pi.appendEntry`, excluded from model context) and raises a minimal forwarded dialog; the core answers via `extension_ui_response` — auto-allow for routine calls, developer-facing for candidates — and a denial reaches the model as the tool result's reason.
- Authority dialogs fail closed: every dialog carries an explicit timeout resolving to denied; developer-facing decisions get a generous, human-sized timeout.
- Decisions serialize: parallel tool calls queue at the gate, one developer decision at a time, siblings waiting behind it within the timeout bound.
- Pi owns the agent loop; the core owns the governance loop — task entry, authority verdicts, developer decisions, specification, checkpoints, provenance; the extension owns neither and is only the hook point where the boundary attaches.
- One Forge process owns the child and its whole stream: stdout JSONL (LF framing, CR stripped) pumps typed events — session events, gate dialogs, dialog responses — into the Ratatui loop, commands flow back, stderr is diagnostics, and Pi runs headless under Forge with no TTY.
- Launch configures the perimeter: explicit tool selection, explicit extension allowlist, auto-discovery off; the active tool set is a launch-time property that RPC cannot change later.
- Attachment is verified before trust: the extension registers a namespaced `forge` command (visible via `get_commands`) and appends a `session_start` handshake carrying its schema version; a missing gate or a version mismatch refuses the session.
- The extension is written against a pinned Pi revision, and Pi upgrades must re-verify the gate contracts before adoption.
- The gate is a policy gate within Pi's trust domain, not a sandbox; documentation must not claim otherwise, and hard containment is out of MVP scope.

## Alternatives considered

**Pure RPC supervisor with post-hoc control**: reject nothing; observe tool events, `abort` after the fact, and steer the next turn. Rejected: it cannot meet MVP priority 3 or PRD §5's pause-before-proceeding, because a side-effecting call may already have run before the supervisor reacts, and abort cannot hold a pending decision.

**In-process SDK host**: embed Pi in a Node/Bun process with direct TypeScript access and inline extensions, gaining the complete control surface. Rejected: it makes Forge a TypeScript application, discarding the Rust + Ratatui posture the map fixes, to buy control the hybrid already achieves at the tool boundary.

**pi-durable harness**: adopt the experimental durable agent harness for commit-before-run state and `resume()` across process death. Rejected for the MVP: it is a separate TypeScript harness (Node ≥ 22.19) that forgoes the coding-agent CLI/RPC surface, and restart-durable pause is not an MVP requirement. It stays a candidate for that later requirement.

**Split policy with an extension pre-screen**: let the extension statically allowlist routine calls and escalate only candidates, saving wire round trips. Rejected: it puts part of the authority decision in TypeScript, creating two policy layers that can drift, and the round trip it saves is a single JSONL line each way.

## Consequences

- The Rust core remains the single, testable authority point over the full tool boundary; the TypeScript surface stays minimal and replaceable.
- A TypeScript component is a permanent second language and runtime in the product, bound to a pinned Pi revision; upgrades are deliberate, re-verified acts.
- Every tool call pays one pipe round trip, buying full tool-boundary visibility in Rust at negligible cost.
- The forwarded-dialog protocol is the most brittle interface in the system because it binds early to Pi's extension-UI contracts; the startup canary turns a missing or mismatched gate into a refused session instead of a silent ungated one.
- Parallel tool calls serialize at the gate, so one developer decision can delay sibling calls, bounded by the fail-closed timeout.
- This form does not provide pause/resume across process restarts — the coding agent resumes from session files around the CLI, not mid-turn; that requirement remains open on the map.
- The gate is policy, not isolation: a misbehaving model that bypasses Pi, or a malicious co-loaded extension, is outside what the gate can stop; hard containment (container/VM) is deferred beyond the MVP.
- Confirmation: the environment stand-up ticket ([Task: Stand up Pi, a provider, and the sample repo](https://github.com/Xaidel/forge/issues/14)) exercises spawn, RPC, and extension load; the canary makes an ungated session observable; gate conformance is tested against the pinned revision.
- Related: the platform posture is recorded in [Target native Windows first for the Forge MVP](2026-10-13-target-native-windows-first-for-the-forge-mvp.md).
