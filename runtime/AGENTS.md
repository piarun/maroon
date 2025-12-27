# Runtime Crate — Essentials

Compact guide for coding agents: purpose, flow, APIs, edit points, invariants, and testing for the Maroon fiber runtime.

## Purpose
- Execute Maroon assembler produced by `dsl` into `generated::maroon_assembler`.
- Model execution as lightweight fibers (state machines) with cooperative scheduling, async waiting on queues/futures, and simple timers.
- Bridge external committed tasks into internal queues and return results back to callers via public futures.

## Core Concepts
- **Fiber:** Lightweight state machine with a stack and heap. Created via `Fiber::new(FiberType, id, init_vars)` and stepped by `Fiber::run`. Returns `RunResult` to drive the runtime.
- **IR & Codegen:** `runtime/build.rs` generates `generated::maroon_assembler` from `runtime/src/ir_spec.rs` using `dsl`. Do not edit `generated` by hand.
- **Primitives:**
  - **Queues:** Named FIFO channels used to pass `Value` messages between fibers or from outside.
  - **Futures:** One-shot values that wake awaiting fibers; can be “public” (mapped to external request ids) or internal.
  - **Schedule:** Time-based future resolution (resolves to `Unit(())` after N ms of logical runtime time).

## Execution Model
- **Main Loop Priority:**
  1) Resolve due timers (scheduled futures). 2) Step all active fibers. 3) Deliver one message from next non-empty queue to a waiting fiber. 4) Ingest external tasks (time-gated) and enqueue their values.
- **Select/Await:** Fibers yield `RunResult::Select(Vec<SelectArm>)`. The runtime registers each arm in `WaitRegistry` under a key `Queue(name)` or `Future(id)`. Wakeups are FIFO per key; when one arm wins, sibling arms are canceled as a group.
- **Messages:** Internal map `queue_messages: HashMap<String, VecDeque<Value>>` holds values. `non_empty_queues` rotates fairness across queues. Sending to a non-existent queue is a hard error (panic) by design.
- **Futures:**
  - Internal completion pushes `(FutureId, Value)` into `resolved_futures`, which wakes the first awaiting fiber for that future.
  - Public completion maps a generated string id to an external `UniqueU64BlobId` (`public_futures`) and emits `Output` through the runtime endpoint.
  - If a future resolves before anyone awaits it, it is re-queued (potential leak noted, see Gotchas).
- **Timers:** `ScheduledBlob` entries live in a min-heap (via reverse `Ord`). When due, a `Unit(())` result is inserted into `resolved_futures` under the scheduled `FutureId`.
- **External I/O:**
  - Input: `(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>)` read from `interface.receiver` (non-blocking poll + small sleeps).
  - Output: `(UniqueU64BlobId, Value)` sent via `interface.send` when a public future is set.
- **Debugging:** Fibers append human-readable logs (debug/vars/phase) into an `Arc<Mutex<String>>` sink available via `Runtime::debug_handle()` for tests.

## Public API
- `Runtime<T: Timer>::new(timer: T, interface: Endpoint<Output, Input>) -> Runtime<T>`
  - `Timer` from `common::logical_clock` (e.g., `MonotonicTimer`).
  - `Endpoint` from `common::duplex_channel` (see `create_a_b_duplex_pair`).
- `Runtime::run(root_type: String) -> impl Future<()>`
  - Spawns root fiber `FiberType(root_type)` with id `0` and no init vars; then runs the main loop forever.
- `Runtime::debug_handle() -> Arc<Mutex<String>>`
  - Read-only handle to the debug buffer for tests or diagnostics.
- `TaskBlueprint { global_id, q_name, value }`
  - `global_id: UniqueU64BlobId` is the external correlation id (public future mapping).
  - `q_name: String` must match an existing public queue inside the model.
  - `value: generated::maroon_assembler::Value` (public payload); internally converted via `pub_to_private`.
- Type aliases:
  - `Input = (LogicalTimeAbsoluteMs, Vec<TaskBlueprint>)`
  - `Output = (UniqueU64BlobId, Value)`
  - `A2BEndpoint = Endpoint<Input, Output>`, `B2AEndpoint = Endpoint<Output, Input>`

## Generated Types (from `generated::maroon_assembler`)
- **Value:** Sum type covering all IR-defined structs, primitives, arrays, options, futures, strings, etc.
- **SetPrimitiveValue:** `QueueMessage { queue_name, value }` | `Future { id, value }` emitted by fibers.
- **CreatePrimitiveValue:** `Queue { name, public }` | `Future` | `Schedule { ms }` requested atomically by fibers.
- **SelectArm / State / StackEntry / StepResult:** Machine-level artifacts used by `Fiber::run`.

## Edit Points
- **Scheduling & I/O:** `runtime/src/runtime.rs` (priority, queues, futures, external ingestion, timer handling).
- **Interpreter:** `runtime/src/fiber.rs` (stack discipline, return binding, `RunResult` mapping from `StepResult`).
- **Wait Registry:** `runtime/src/wait_registry.rs` (per-key FIFO fairness, registration/cancel/wake logic).
- **IR Sample & Tests:** `runtime/src/ir_spec.rs` (sample IR used by tests/codegen), `runtime/src/generated_test.rs` (behavioral tests), `runtime/src/ir_test.rs` (IR validity).
- **Codegen:** `runtime/build.rs` rewrites `generated/src/maroon_assembler.rs` from IR; avoid manual edits to `generated/`.

## Invariants & Gotchas
- **Data Consistency:** If a fiber is in `wait_index`, it must also be in `awaiting_fibers`, and vice versa on removal.
- **Select Semantics:** Waking any arm cancels sibling arms for that fiber; wake order is FIFO per key.
- **Queue Send Panics:** Sending to a non-existing queue panics (by temporary design, later will be changed); ensure queues are created before use.
- **Atomic Create:** `RunResult::Create` validates all primitives first; on any error, binds `Option<String>` errors and takes the fail branch. On success, binds all ids and takes the success branch.
- **Future Leak Risk:** If a fiber never awaits a created future that later resolves, it may remain in `resolved_futures` (TODO). Consider IR/DSL-level ownership/lifetimes to address this.
- **Monotonic IDs:** `next_fiber_id` and `next_created_future_id` are monotonically increasing; string future ids are used as keys.
- **Timing:** External inputs are time-gated; if timestamp > now, the loop sleeps until due. Timer resolution uses `tokio::time::sleep`.
- **Args Count Check:** Interpreter panics if a step requires more arguments than currently on the stack (IR/codegen bug or misuse).

## Usage Pattern (high level)
- **Wiring:**
  - Create a duplex pair: `let (a2b, b2a) = create_a_b_duplex_pair::<Input, Output>();`
  - Construct runtime: `let mut rt = Runtime::new(MonotonicTimer::new(), b2a);`
  - Spawn loop: `tokio::spawn(async move { rt.run("<RootFiberType>".to_string()).await; });`
- **Ingest Tasks:** Send `(ts, Vec<TaskBlueprint>)` through `a2b.send`. Each blueprint value is converted to private form and enqueued to `q_name`.
- **Observe Results:** Receive `(UniqueU64BlobId, Value)` from `a2b.receiver` when a public future is set by a fiber.
- **Debug:** Read `rt.debug_handle()` during tests to assert execution traces/logs.

## Testing
- **Unit Tests:**
  - `wait_registry.rs`: FIFO ordering, cancellation by select id, mixed keys fairness.
  - `generated_test.rs`: end-to-end fiber stepping (select, set values, function calls, debug/vars).
  - `ir_test.rs`: IR validity check on `sample_ir()`.
- **Run Locally:**
  - `make test` (quick verification).
  - Before pushing: `make test && make fmt && make integtest && make integtest-dockerized`.
- **Patterns:** Use `debug_handle()` to assert human-readable logs; use sample IR to generate minimal runnable fibers for tests.

## Extending
- **New Primitive:** Add variant to DSL/IR and codegen so `StepResult` emits appropriate control; extend `Fiber::run` and `Runtime` to handle new `RunResult`/side-effects.
- **Scheduling Policy:** Adjust priority or fairness in `runtime.rs` (`active_fibers`, `non_empty_queues`, timer cadence).
- **Public Types:** Add to DSL `Type` universe; codegen updates `Value` automatically.
- **Error Strategy:** Replace panics with error propagation if/when the runtime should be resilient to malformed inputs.

## Cross-Crate Interfaces
- **From App/Maroon:** `runtime::runtime::{TaskBlueprint, Input, Output}`, and duplex endpoints from `common::duplex_channel`.
- **With DSL/Generated:** `generated::maroon_assembler::{Value, SetPrimitiveValue, CreatePrimitiveValue, SelectArm, State, StackEntry, StepResult}`; `dsl::ir::FiberType`.
- **Clock:** `common::logical_clock::{Timer, MonotonicTimer}`; logical times are `common::logical_time::LogicalTimeAbsoluteMs`.

## File Map
- `src/runtime.rs`: Main scheduler, message/future queues, external I/O, timers.
- `src/fiber.rs`: Fiber struct, interpreter bridge, `RunResult` mapping.
- `src/wait_registry.rs`: Select registration, wake/cancel, FIFO logic.
- `src/trace.rs`: TraceEvent for per-state execution tracing.
- `src/ir_spec.rs`: Sample IR used for codegen/tests.
- `build.rs`: Codegen into `generated/src/maroon_assembler.rs`.

