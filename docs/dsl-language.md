# Maroon DSL — Goals, Non‑Goals, and Core Spec (Draft)

This doc describes a small, purpose-built language for Maroon. Code in this DSL compiles to Maroon IR (our “assembler”) and runs on the runtime. The goal is simple: you write business logic; the platform guarantees durable compute by determenistic execution on several nodes.

## Why a DSL
- Deterministic by default: no hidden time/RNG(random number generator)/float surprises, stable iteration order
- Fits Maroon’s model: fibers, queues, futures, and timers are native concepts with deterministic scheduling.
- Safe to replay: re-running the same history yields the same state and outputs.
- Easy to check: we can warn/error on unbounded loops, impure code in `pure` functions, and risky waits.
- Smooth upgrades: state has versions and migrations, so rolling upgrades don’t corrupt data.
- Concurrency model: v1 runs under a single logical total order for simplicity and replayability; future versions may relax this with explicit primitives (compare-and-swap, CRDTs) where commutativity makes it safe—without sacrificing deterministic outcomes. 

## Out of Scope (on purpose)
- No arbitrary threads/syscalls/FFI(foreign function interface).
- No host-specific behavior (wall-clock reads, RNG, hash-map iteration order, IEEE float edge cases).
- No DIY persistence: only schema-defined types go to storage; the runtime handles format and migrations.
- No implicit I/O: external calls must be declared with types, timeouts, and retry policies.
- No relaxed consistency primitives (CAS/CRDTs) in v1: all effects are sequenced by the single total order.

## How Code Runs
- Unit of work: a lightweight [fiber](./fiber.md).
- Communication: named FIFO queues with directional capability types (`RecvQueue<T>`, `SendQueue<T>`, optional `DuplexQueue<T>` for both directions).
- Time: logical monotonic ms via timers (`after(ms)`), not wall-clock.

### Source Organization (design note)
- Declaration order and file/dir layout are not part of semantics; the compiler builds a single module graph from all inputs.
- Interfaces (ingress/egress, queues, external gateways) are declared in the DSL and compiled together with the code that uses them.

### Concurrency and Ordering (design note)
- v1 uses a single logical total order of events/effects, which makes execution, replay, and debugging straightforward.
- Over time, we may introduce opt-in, scoped primitives that allow concurrency without global coordination:
  - Compare-and-swap (CAS) on targeted state fields.
  - CRDTs (conflict-free replicated data types) for commutative/associative updates (e.g., counters, OR-sets) with deterministic merge semantics.
- Any relaxation will remain compatible with deterministic replay by using canonical encodings, explicit merge rules, and well-defined failure/retry behavior.

## External Effects
- Three kinds: `pure` (no effects), `timer` (logical), `external(service)` (declared capability).
- External call must declare: request/response types, idempotency key, timeout, retry/backoff; optional compensation.
- Observability (logs/metrics/traces) does not change behavior/order.

## Numbers and Data
- Integers: `I64`, `U64` (optionally `I128` later). Overflow behavior is explicit: checked (default), saturating, or wrapping.
- Decimals: fixed-point `Decimal{scale}` (no floats/NaNs/inf). Rounding mode is explicit and stable.
- Text/bytes: `String` (UTF‑8) and `Bytes`.
- Collections: `Vec<T>` (stable order), `Map<K,V>` and `Set<T>` with keys ordered by their byte encoding. A hash‑map (`HashMap<K,V>`) may be provided for performance, but all observable operations are deterministic (e.g., iteration defined as canonical key order); non‑deterministic iteration is not exposed.
- Types: `struct`, `enum`, and `type` aliases with explicit field/variant order.

## Canonical Encoding (why ordering is stable)
- Every value has one byte representation (platform-independent, invertible). We sort map/set keys by these bytes.
- Examples: big-endian integers; `Decimal{2}(1)` and `1.00` encode the same; strings use UTF‑8 NFC normalization; no “-0”.

## Fiber State and Persistence
- No global app state. Each fiber owns its own persistent state, defined inside that fiber.
- Define per‑fiber state in the DSL: within a `fiber` block, declare `state current { ... }`. Optionally, during upgrades, also declare `state next { ... }`. Only these schema types are persisted for that fiber.
- Creation initializes `current` deterministically. Upgrades use a two‑version migration: `migrate current -> next { ... }` with at most two states present at any time.
- All writes happen within the owning fiber, driven by messages/timers. Other fibers cannot mutate this state; they must send messages.
- Reads see the fiber’s deterministic view for the current step. The runtime snapshots each fiber’s state in canonical form and replays that fiber’s message stream to recover.

### State access inside a fiber
- Access persistent fields with `self.<field>` for both reads and writes (e.g., `let n = self.count + 1; self.count = n`).
- Local variables use bare identifiers (e.g., `let n = 0;`). Shadowing state field names is not allowed.
- Initialize state via field defaults, a pure initializer `fn init { ... }`, or during `migrate` steps; avoid relying on implicit, unspecified defaults.

### Initialization
- Purpose: set up newly created fibers deterministically before handling any messages.
- Syntax: place a `fn init { /* pure */ }` inside the `fiber` block.
- Semantics:
  - Runs exactly once, only on creation of a new fiber instance, after constructor params are bound and before the first activation of `main`/handlers.
  - Pure only: no `await`, `select`, `send`, or `external` inside `init`.
  - Can read constructor params (e.g., `self.name`) and assign to state fields.
  - Must leave the base state version fully initialized, either via field defaults or explicit assignments.
- Migrations and init:
  - Existing fibers never run `init` during upgrades; they transition via `migrate current -> next` only.
  - Creation when `state next` exists:
    - Construct `state current` using its field defaults and `init`.
    - Apply the migration `current -> next`.
    - Only after migration completes is the fiber considered created; `main`/handlers may run thereafter.
    - `init` executes against the `current` shape; fields introduced in `next` must be initialized in `migrate current -> next`.
  - For effectful bootstrapping at creation, use a bootstrap message pattern; keep `init` pure.

### Constructor parameters
- Fiber constructor parameters (identity, handles like queues, config) are read-only fields of the fiber instance.
- Access them as `self.<param>` inside the fiber (e.g., `self.name`, `self.inbox_queue`).
- Parameters cannot be reassigned; locals remain bare identifiers. Shadowing parameter names is not allowed.
- In `select`, the sugar `self.queue.await` is valid when `self.queue` is a `RecvQueue<_>` (or `DuplexQueue<_>`, though using `RecvQueue` is preferred) and desugars to `await recv(self.queue)`.

### State migrations (two‑version model)
- Syntax: `migrate current -> next { /* transforms */ }` placed inside the `fiber` block before `state next`.
- Scope:
  - `from.<field>`: read-only view of the previous `current` state snapshot.
  - `self.<field>`: the `next` state you must initialize.
- Rules:
  - Explicit init: every newly introduced or type-changed field in `state next` must be assigned; unchanged fields carry forward implicitly.
  - Determinism: transformations must be deterministic and terminate.
  - Type changes: allowed if you provide an explicit transform; otherwise keep the same type.
  - Collections: when changing key types in `Map`/`Set`, ensure canonical encoding order is preserved by re-encoding keys.

Design note (v1 scope): migrations are defined per‑fiber using a two‑version model (`current` and `next`). Alternative models (e.g., per‑type/era migrations applied across instances) are under consideration and may be adopted if they provide better ergonomics without sacrificing determinism.

Example (direct rename + add default):
  ```dsl
  migrate current -> next {
    self.count = from.seen;
    self.last_input = None;
  }

  state next { count: U64, last_input: Option<String> }
  ```

#### Finalize/Promotion
- After migration completes and code uses the `next` fields, promote `next` -> `current` by removing the old `current` and the `migrate` block, leaving only `state current { ... }`.
- At any time, there must be at most two states present (`current` and optionally `next`).

### Interface Schema Versioning (queues/gateways)
- Priority: define versioning for fiber interfaces (cross‑fiber queues and external gateway APIs) so producers/consumers can upgrade safely.
- Compatibility: interface types evolve via eras/versions with explicit upgrade rules; mixed‑version communication must be either rejected or mediated via canonical transforms.
- Storage vs. interface: internal fiber state shape is important, but interface compatibility governs safe rolling deploys and should be specified first.

## Transactions and IDs
- Each transaction has a unique `idempotency ID` (assigned by gateways layer).
- Idempotency by design: re-applying a transaction yields the same result.

## Resource Limits
- We track CPU/memory/I/O “cost”.
- Per-transaction and per-fiber limits apply with prioritization classes; the system avoids starving main business logic. Hitting a limit triggers backpressure on lower‑priority work first.

## Build Pipeline
- DSL -> typed AST -> Maroon IR -> generated code that the runtime executes.
- You’ll see types like `Value`, `CreatePrimitiveValue`, `SetPrimitiveValue`, `SelectArm`, `State` in the generated layer.
- Runtime input/output: `Input = (LogicalTimeAbsoluteMs, Vec<TaskBlueprint>)`, results are `(UniqueU64BlobId, Value)`.

## Static Checks (examples)
- `pure` functions can’t call effects or timers.
- `Map`/`Set` keys must be orderable (canonical encoding available).
- `select` cases must be cancellable or time-bounded; if a waitable is non‑cancellable it must be a single‑step arm.
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
### Rust Interop (design note)
- To remain Rust‑friendly, we expose `await`/`select` as macros (e.g., `await!`, `maroon_await!`) when embedding; the core primitive is `select` and `await` is its single‑arm form.

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
