---
status: accepted
date: 2026-11-03
---

# Callback-based model streaming with drop and explicit cancellation

## Context

`app_core` is executor-agnostic and I/O-free. It may depend on `async-trait` and `thiserror`; `futures-core` is optional and feature-gated only if the streaming model port genuinely needs a `Stream` return. The model adapter must stream text deltas, reasoning deltas, tool-call deltas, usage, and a finish event from an HTTP/SSE provider to the core without leaking a provider type or a runtime type.

A `Stream` return would force `futures-core` into the core, make the port harder to objectify, and leave cancellation to the consumer’s poll/drop semantics. A channel would force the core to own a channel type or a runtime dependency. A callback sink keeps the port object-safe and lets the adapter drive the stream while the caller decides what to render.

Long-running out-of-process work — a model request and a worker tool — also needs a way to stop remote work when the user pauses, cancels, or the renderer shuts down. Dropping a future stops the local poll, but it cannot by itself abort an HTTP request or a worker process.

## Decision

The model port streams through a synchronous callback sink:

```rust
pub trait ModelEventSink: Send {
    fn on_text_delta(&mut self, delta: &str);
    fn on_reasoning_delta(&mut self, delta: &str);
    fn on_tool_call_delta(&mut self, delta: &ToolCallDelta);
    fn on_usage(&mut self, usage: &Usage);
    fn on_finish(&mut self, stop_reason: &StopReason);
}

#[async_trait::async_trait]
pub trait ModelPort: Send + Sync {
    async fn stream(
        &self,
        request: ModelRequest,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelError>;
    async fn cancel(&self, call_id: &ModelCallId) -> Result<(), ModelError>;
}
```

The same pattern applies to `SandboxPort::run_tool` with `ToolEventSink`, `OperationId`, and `SandboxPort::cancel`.

Cancellation rules:

- Dropping an in-process future is the cancellation signal. Every mutating port is atomic or exposes an idempotent recovery path.
- The adapter must stop provider work or the worker process when `cancel` is called; it returns `ModelError::Cancelled` or `SandboxError::Cancelled`.
- `ModelCallId` and `OperationId` are opaque strings owned by the adapter or application, not IDs allocated by `IdSourcePort`.
- No lock is held across an `.await`; no request-scoped state is stored in the adapter.

## Consequences

- `app_core` does not depend on `futures-core` or any runtime. The optional `streaming` feature is absent.
- The model and sandbox ports stay object-safe and `Send + Sync`.
- The adapter owns HTTP/SSE parsing and process supervision; the core owns the normalized events and the decision to cancel.
- A sink callback is infallible. A rendering failure is handled by the caller before or after the stream, not by the core port.
- A future adapter that needs a true `Stream` must amend this ADR and the port, not smuggle `futures-core` into the core.
- The TUI receives the same normalized events in both interactive and plain mode.

## Considered options

- **Return `impl Stream` from `ModelPort`.** Natural for async Rust, but forces `futures-core` into the public core, makes the trait harder to objectify, and leaves cancellation implicit.
- **Return a channel receiver.** Works with any runtime, but the channel type becomes a core dependency or a hand-rolled abstraction, and the core starts owning buffering policy.
- **Use a callback sink.** Chosen: no runtime, no extra dependency, object-safe, and the adapter can stop on drop or explicit cancel.
- **Use Tokio’s `CancellationToken`.** Rejected because Tokio is forbidden in the core and the token type would leak a runtime choice.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the domain model, ports, and application assembly (TDD-0002)](https://github.com/Xaidel/forge/issues/21)
- `docs/PRD.md` §6.5, §7.9, §8.4, NFR-MAINT-001, D-13
- `arch_docs/architecture.md` §10, `arch_docs/dependency-rules.md` DEP-05, DEP-06, DEP-20
- [TDD-0002](../tdd/0002-domain-model-ports-assembly.md) §6.10, §7.2, §7.4
