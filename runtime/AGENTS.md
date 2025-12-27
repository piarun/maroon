# Runtime Crate — Quick Guide

Compact overview for contributors. Focus on the essentials: purpose, model, APIs, edit points, and a few gotchas.

## Purpose
- Execute generated Maroon assembler as lightweight fibers.
- Bridge external committed tasks into internal queues and return results via public futures.
- Provide timers via scheduled futures.

## Key Ideas
- **Fiber:** State machine stepped by `Fiber::run`, producing `RunResult` (e.g., `Select`, `Create`, `SetValues`, `Done`).
- **Primitives:** Queues (FIFO), Futures (one-shot), Schedule(ms) to create timer futures.
- **Generated Types:** Minimal surface you’ll touch: `Value`, `CreatePrimitiveValue`, `SetPrimitiveValue`, `SelectArm`, `State`.

## Execution Model (loop)
1) Resolve due timers (produce completed future with `Unit`).
2) Step active fibers.
3) Resolve one completed future (wake a waiter or requeue if none).
4) Deliver one message from the next non-empty queue to a waiting fiber.
5) Ingest external tasks (time-gated) and enqueue their values.

## Public API
- `Runtime<T: Timer>::new(timer, Endpoint<Output, Input>) -> Runtime<T>`
- `async fn run(&mut self, root_type: String)`
- `fn debug_handle(&self) -> Arc<Mutex<String>>`
- Types:
  - `Input = (LogicalTimeAbsoluteMs, Vec<TaskBlueprint>)`
  - `Output = (UniqueU64BlobId, Value)`
  - `TaskBlueprint { global_id, q_name, value }`

## Edit Here
- `src/runtime.rs` — scheduler, queues/futures, external I/O, timers.
- `src/fiber.rs` — interpreter bridge, `RunResult` handling.
- `src/wait_registry.rs` — select registration, wake/cancel, FIFO.
- `src/ir_spec.rs` and `build.rs` — IR sample and codegen.

## Gotchas
- Sending to a non-existing queue panics (temporary design).
- A future resolved before anyone awaits is re-queued; if never awaited, it may leak.
- Queue/future wakeups are FIFO per key.

## Minimal Usage
- Create endpoints: `let (a2b, b2a) = create_a_b_duplex_pair::<Input, Output>();`
- Start runtime: `let mut rt = Runtime::new(MonotonicTimer::new(), b2a); tokio::spawn(async move { rt.run("<Root>").await; });`
- Send tasks: `a2b.send((ts, blueprints))`; read results: `(UniqueU64BlobId, Value)` from `a2b.receiver`.

## Tests
- `make test` for quick run.
- Unit tests cover the wait registry and generated stepping; `debug_handle()` is used for assertions.
