# Maroon DSL — Goals, Non‑Goals, and Core Spec (Draft)

This doc describes a small, purpose-built language for Maroon. Code in this DSL compiles to Maroon IR (our “assembler”) and runs on the runtime. The goal is simple: you write business logic; the platform guarantees durable compute by determenistic execution on several nodes.

## Why a DSL
- Deterministic by default: no hidden time/RNG(random number generator)/float surprises, stable iteration order, one standard way to encode data.
- Fits Maroon’s model: fibers, queues, futures, and timers are native concepts with deterministic scheduling.
- Safe to replay: re-running the same history yields the same state and outputs.
- Easy to check: we can warn/error on unbounded loops, impure code in `pure` functions, and risky waits.
- Smooth upgrades: state has versions and migrations, so rolling upgrades don’t corrupt data.

## Out of Scope (on purpose)
- No arbitrary threads/syscalls/FFI(foreign function interface).
- No host-specific behavior (wall-clock reads, RNG, hash-map iteration order, IEEE float edge cases).
- No DIY persistence: only schema-defined types go to storage; the runtime handles format and migrations.
- No implicit I/O: external calls must be declared with types, timeouts, and retry policies.

## How Code Runs
- Unit of work: a lightweight [fiber](./fiber.md).
- Communication: named FIFO queues with directional capability types (`RecvQueue<T>`, `SendQueue<T>`, optional `DuplexQueue<T>` for both directions).
- Time: logical monotonic ms via timers (`after(ms)`), not wall-clock.

## External Effects
- Three kinds: `pure` (no effects), `timer` (logical), `external(service)` (declared capability).
- External call must declare: request/response types, idempotency key, timeout, retry/backoff; optional compensation.
- Observability (logs/metrics/traces) does not change behavior/order.

## Numbers and Data
- Integers: `I64`, `U64` (optionally `I128` later). Overflow behavior is explicit: checked (default), saturating, or wrapping.
- Decimals: fixed-point `Decimal{scale}` (no floats/NaNs/inf). Rounding mode is explicit and stable.
- Text/bytes: `String` (UTF‑8) and `Bytes`.
- Collections: `Vec<T>` (stable order), `Map<K,V>` and `Set<T>` with keys ordered by their byte encoding. No hash maps/sets.
- Types: `struct`, `enum`, and `type` aliases with explicit field/variant order.

## Canonical Encoding (why ordering is stable)
- Every value has one byte representation (platform-independent, invertible). We sort map/set keys by these bytes.
- Examples: big-endian integers; `Decimal{2}(1)` and `1.00` encode the same; strings use UTF‑8 NFC normalization; no “-0”.

## Fiber State and Persistence
- No global app state. Each fiber owns its own persistent state, defined inside that fiber.
- Define per‑fiber state in the DSL: within a `fiber` block, declare `state vN { ... }`. Only these schema types are persisted for that fiber.
- First activation initializes state deterministically. Upgrades use per‑fiber migrations: `migrate vN -> vN+1 { ... }`.
- All writes happen within the owning fiber, driven by messages/timers. Other fibers cannot mutate this state; they must send messages.
- Reads see the fiber’s deterministic view for the current step. The runtime snapshots each fiber’s state in canonical form and replays that fiber’s message stream to recover.

### State access inside a fiber
- Access persistent fields with `self.<field>` for both reads and writes (e.g., `let n = self.count + 1; self.count = n`).
- Local variables use bare identifiers (e.g., `let n = 0;`). Shadowing state field names is not allowed.
- Initialize state explicitly in `on start { ... }` or during `migrate` steps; avoid relying on implicit defaults.

### Constructor parameters
- Fiber constructor parameters (identity, handles like queues, config) are read-only fields of the fiber instance.
- Access them as `self.<param>` inside the fiber (e.g., `self.name`, `self.inbox_queue`).
- Parameters cannot be reassigned; locals remain bare identifiers. Shadowing parameter names is not allowed.
- In `select`, the sugar `self.queue.await` is valid when `self.queue` is a `RecvQueue<_>` (or `DuplexQueue<_>`, though using `RecvQueue` is preferred) and desugars to `await recv(self.queue)`.

### State migrations
- Syntax: `migrate vN -> vN+1 { /* transforms */ }` placed inside the `fiber` block before the next `state vN+1`.
- Scope:
  - `from.<field>`: read-only view of the previous state snapshot (version N).
  - `self.<field>`: the new state (version N+1) you must initialize.
- Rules:
  - Pure only: no `await`, `select`, `send`, or `external` calls inside `migrate`.
    - that's questionable. Of course pure migrations are easier, but I can easily imagine migration where I need to make an http call for some data
    - maybe in the first version if external data is needed, use a two-phase pattern: add new fields, deploy code to backfill via normal runtime flows, then finalize with a follow-up migration
  - Explicit init: every newly introduced or type-changed field in `state vN+1` must be assigned; unchanged fields carry forward implicitly.
  - Determinism: transformations must be deterministic and terminate.
  - Type changes: allowed if you provide an explicit transform; otherwise keep the same type.
  - Collections: when changing key types in `Map`/`Set`, ensure canonical encoding order is preserved by re-encoding keys.
- Example (direct rename + add default):
  ```dsl
  migrate v1 -> v2 {
    self.count = from.seen;
    self.last_input = None;
  }

  state v2 { count: U64, last_input: Option<String> }
  ```

## Transactions and IDs
- Each transaction has a unique `UniqueU64BlobId` (from the gateway’s key range).
- Idempotency by design: re-applying a transaction yields the same result.

## Resource Limits
- We track CPU/memory/I/O “cost”.
- Per-transaction and per-fiber limits apply; hit a limit -> get a backpressure.

## Build Pipeline
- DSL -> typed AST -> Maroon IR -> generated code that the runtime executes.
- You’ll see types like `Value`, `CreatePrimitiveValue`, `SetPrimitiveValue`, `SelectArm`, `State` in the generated layer.
- Runtime input/output: `Input = (LogicalTimeAbsoluteMs, Vec<TaskBlueprint>)`, results are `(UniqueU64BlobId, Value)`.

## Static Checks (examples)
- `pure` functions can’t call effects or timers.
- `Map`/`Set` keys must be orderable (canonical encoding available).
- `select` cases must be cancellable or time-bounded.
- Reads/writes are validated against the active schema version during upgrades.

## Core Building Blocks (maps 1:1 to runtime)
- Values: `Unit | Bool | I64 | U64 | Decimal{s} | Bytes | String | Vec<T> | Map<K,V> | Set<T> | struct | enum`.
- Queues: named FIFO channels of `Value`, with directional capabilities (`RecvQueue<T>`, `SendQueue<T>`, and `DuplexQueue<T>` when both are required).
- Futures/Timers: one-shot futures; `after(ms)` creates a timer; `await` resolves.
 - Fibers: define a fiber type with parameters (identity) and its private `state` and handlers.

### Queue Capability Types and API
- Types:
  - `RecvQueue<T>`: receive-only capability for a named channel of `T`.
  - `SendQueue<T>`: send-only capability for a named channel of `T`.
  - `DuplexQueue<T>`: full capability (both send and receive). Prefer directional types for clarity; reserve `DuplexQueue` for cases that truly need both directions.
- API:
  - Send: `queue.send(value)` where `queue: SendQueue<T> | DuplexQueue<T>` and `value: T`.
  - Receive (canonical): `recv(queue) -> Future<T>` where `queue: RecvQueue<T> | DuplexQueue<T>`.
  - Await (outside select): `let v: T = await recv(queue)`.
  - Await sugar (inside select): `queue.await` is shorthand for `await recv(queue)` and requires `queue: RecvQueue<T> | DuplexQueue<T>`.
- Conversions/helpers (conceptual):
  - `split(q: DuplexQueue<T>) -> (RecvQueue<T>, SendQueue<T>)`.
  - `join(rx: RecvQueue<T>, tx: SendQueue<T>) -> DuplexQueue<T>` if they reference the same named channel.
- Alias (optional for ergonomics/back-compat in docs): `type Queue<T> = DuplexQueue<T>`.

### Select Syntax and Waitables
- Waitables: any `Future<T>` can be selected: `recv(queue)`, `after(ms)`, `external(...)`, etc.
- Canonical arms:
  - `case await recv(queue) as v: T => { ... }`
  - `case await after(ms) => { ... }  // T = Unit`
- Concise let-binding arms (sugar, only in `select`):
  - `let v: T = queue.await => { ... }        // requires RecvQueue<_> (or DuplexQueue<_>); desugars to case await recv(queue)`
  - `let _: Unit = after(ms).await => { ... } // desugars to case await after(ms)`
- Semantics: first-ready arm runs; other pending waitables are cancelled deterministically.

## Open Questions
- Exact syntax vs. minimal IR friendliness.
- Standard library scope (safe math, codecs, collection helpers).
- Effect capabilities and per-call metering.
- Formalization scope (how much of the core we specify/prove).

## Next Steps
- Compile a small example to today’s IR and run it on the runtime.
- Finalize byte encodings for all base types and composite keys.
- Implement basic static checks (bounded loops, determinism guards, purity) in the DSL frontend.

## Examples
- See `docs/dsl-examples-echo.md` for small, focused examples (fibers, queues, select sugar, and schema migration).
