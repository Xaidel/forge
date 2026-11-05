---
status: accepted
date: 2026-11-05
---

# No retry after the first streamed delta

## Context

FR-MODEL-006 and FR-EXEC-012 give a model or API error three attempts with backoff, then a pause with the `/model` path. Forge streams text, reasoning, and tool-call deltas to the transcript as they arrive (FR-TUI-004, ADR-0004). A retry after a partial stream would either duplicate visible text or require Forge to hide output until the response is complete.

Buffering the whole response before display would make retries safe but would remove the streaming behavior the PRD requires. Restarting after partial output would duplicate text and could replay tool calls, which is worse than a pause: a tool call is a side effect the human may have already seen and the agent may have already acted on.

## Decision

A model request is retried only while **no text delta and no tool-call delta** has been emitted.

- The attempt counter starts at one. A retryable failure before any content allows the next attempt, up to three attempts total.
- Once any text, reasoning, or tool-call delta reaches the sink, a retryable failure is not retried. The task pauses with the partial message marked `[interrupted]`, the failure, and the `/model` path.
- The adapter still returns `ModelError::StreamInterrupted`; the application owns the pause state and the transcript marker.
- No retry switches profile or model. A switch is an explicit, audited `/model` action.
- Usage from a failed partial request is recorded if the provider reported it; otherwise it is labeled unknown or estimated. It is never silently zero.

## Consequences

- A transient mid-stream failure pauses the task instead of silently duplicating output.
- A retry can still recover a request that failed during connection, headers, or before the first event.
- Tool calls are never replayed by a transparent retry.
- The retry loop must wrap the sink to record whether content was emitted; the adapter alone cannot decide.
- The developer sees a partial response and a clear pause rather than a response that quietly changed identity.
- A future resume could continue a partial stream only with an explicit protocol contract; that is not v1.

## Considered options

- **Buffer the full response, then render.** Rejected: removes live streaming, delays gates, and contradicts FR-TUI-004.
- **Retry and discard the partial output.** Rejected: visible text cannot be un-rendered from native scrollback; tool-call side effects may already have happened.
- **Retry and mark the first attempt as interrupted, then append the second.** Rejected: duplicate text and ambiguous tool-call identity.
- **No retry at all.** Rejected: FR-EXEC-012 requires three attempts for transient errors.
- **Retry only before the first delta; pause after.** Chosen: preserves streaming, prevents duplicate output, and keeps retries honest.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the model adapters, provider profiles, and cost contract (TDD-0007)](https://github.com/Xaidel/forge/issues/27)
- `docs/PRD.md` §7.9 FR-MODEL-006; §7.4 FR-EXEC-012; §7.10 FR-TUI-004
- [ADR-0004](0004-callback-model-streaming-cancellation.md)
- [TDD-0007](../tdd/0007-model-adapters-providers-cost.md) §8.14
