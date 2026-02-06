# Maroon DSL — White Paper & Core Spec (v0.1 sketch)

> A small language for durable/stateful computing via deterministic replay.

## 0. Status

This document is a **v0.1 sketch**: it defines the intent, the execution model, and the minimum normative rules required for deterministic, replayable stateful programs. Anything not explicitly specified is **undefined** and may change.

**Ordering note (v0.1 assumption):** v0.1 assumes the runtime presents **each fiber** with a replayable **total order** of events relevant to that fiber (message deliveries, timer firings, external request/response events). This makes constructs like `select` unambiguous.

The language spec does **not** require a single global total order that is directly observable to user code. The runtime may maintain a global ordering internally (e.g., for effect completion routing), but the only language-level requirement is that each fiber’s event history is a deterministic, replay-stable sequence.

### 0.1 Portability and conformance levels

Small differences in event ordering are where workflow engines often hide determinism bugs. Maroon therefore distinguishes **determinism** from **portability**.

**Core conformance (v0.1):** a runtime is compliant if it delivers a replay-stable per-fiber total order. Under Core conformance, the same program MAY behave differently across runtimes, even though each runtime is internally deterministic.

**Portable profile (future):** a stricter profile MAY standardize enough of the ordering (and other replay-relevant details) that the same program behaves the same across all Portable-profile runtimes. v0.1 does not define this profile.

If portability across independent runtimes becomes a goal, the platform contract MUST either (a) standardize ordering rules or (b) attach an explicit **runtime profile identifier** to deployments so users can reason about behavioral compatibility.

A future version may relax or eliminate the TotalOrder assumption by introducing explicit concurrency/merge primitives (e.g., compare-and-swap, transactions, CRDT-style data types) with well-defined conflict semantics.

---

## 1. Motivation

Most stateful systems re-implement the same hard problems: durability, recovery, retries, ordering, upgrades, and “exactly-once-ish” effect handling.

**Maroon DSL** aims to make stateful logic boring:

* You write deterministic business logic inside durable units of computation.
* The platform guarantees recovery by replaying the same event history. 
* Upgrades are safe via explicit, replay-safe schema migrations.

---

## 2. What Maroon is

Maroon is:

* a **Rust-embedded DSL**: Maroon programs are **valid Rust** that compile with the standard Rust toolchain.
* a **small semantic subset** of Rust with additional restrictions required for deterministic replay.
* compiled by a **Maroon compiler** (in addition to `rustc`) to **Maroon IR** and then to an executable form for the durable runtime.

Maroon is executed in two practical modes:

1. **Native Rust mode (local / non-durable):** the same source compiles with `rustc` and can run as ordinary single-node code. This mode is intended for fast iteration, unit testing, and IDE/tooling compatibility; it does not provide durability or distributed execution guarantees.
2. **Durable mode:** the Maroon compiler lowers the same source into Maroon IR / assembler, and the runtime executes it as durable fibers with deterministic replay.

The **language semantics** in this document describe **Durable mode**. Native Rust mode is a convenience execution mode; any behavioral differences that arise from missing durability features (e.g., replay, durable queues) are considered outside the language guarantees.

Given the same **event history** (message deliveries, logical timer firings, and recorded external-effect results), replay MUST produce identical state transitions and outputs. You can think about it in a way that a Maroon fiber behaves as if it were a pure function from an append-only event log to durable state and emitted messages.

---

## 2.1 Rust subset and surface syntax

Maroon intentionally reuses Rust syntax and tooling.

**Surface mapping (v0.1):**

* Fibers, state blocks, queues, and effects are expressed using Rust items plus a small set of Maroon-provided procedural macros / attributes and library types.
* The exact macro names are not specified in this whitepaper; they are part of the standard library / SDK surface.

**Subset rule:** only a restricted subset of Rust is permitted inside durable fibers (e.g., no ambient I/O, no threads, no access to nondeterministic OS APIs). These restrictions are specified throughout this document (notably §6 Determinism contract).

**Why Rust (informative):** using Rust as the concrete syntax is a deliberate ergonomics choice to reuse existing IDE support, linters, formatters, test frameworks, and the broader ecosystem. The Maroon compiler enforces the additional semantic rules required for durable execution.

---

## 3. Non-goals (v0.1)

Maroon does **not** aim to provide:

* general-purpose OS access (threads, syscalls)
* arbitrary FFI / native extensions that can violate determinism, replay-safety, or upgrade invariants
* implicit persistence outside declared fiber state
* ambient I/O or hidden side effects
* best-effort “eventual determinism”

---

## 4. Core concepts

### 4.1 Fiber

A **fiber** is the unit of durable computation.

A fiber has:

* **constructor parameters** (identity, queue handles, config)
* **private durable state** (schema-defined)
* **functions** that implement behavior (often `init` + a main loop or handlers)

**Ownership rule (normative):** only the owning fiber may mutate its durable state.

**No global state (normative):** Maroon has no global mutable variables. All durable mutable state exists **inside fibers**. Concurrency happens via message passing between fibers, not via shared memory.

### 4.2 Queues

Fibers communicate by sending typed messages over durable queues.

Queue handles are capability-typed:

* `RecvQueue<T>` receive-only
* `SendQueue<T>` send-only
* `DuplexQueue<T>` (discouraged unless required)

Queues are FIFO.

**Delivery semantics (normative, v0.1):**

* Queue communication is **internal** to the durable runtime.
* For a given sender `send` and receiving fiber, the corresponding delivery event appears **exactly once** in the receiving fiber’s event history.
* Queue operations do **not** expose acknowledgements, retries, or transport-level concerns at the language level.

Message delivery order is deterministic and replay-stable.

### 4.3 Futures and `await`


Many operations produce a `Future<T>`.

`await` blocks the fiber until the future resolves:

```dsl
let x = await f;
```

### 4.4 `select`

`select` waits on multiple futures and executes exactly one arm.

**Normative (v0.1):**

* Exactly one arm MUST execute.
* The chosen arm is the earliest-ready future in the runtime’s deterministic event order.
* Non-chosen futures are **dropped**. Dropping a future MAY request cancellation; whether cancellation occurs is future-kind specific, but the behavior MUST be replay-stable (i.e., replay MUST observe the same resolution/cancellation outcomes).

### 4.4.1 `select` details that MUST be defined for v0.1

This section clarifies the minimum required semantics for deterministic replay within a given runtime profile.

**Readiness (normative):** a future is *ready* when it has a completion outcome available to the fiber at the time of the `select` decision (typically a success value, or an explicit error result if the future’s type includes errors, e.g. `Future<Result<T,E>>`).

**Tie-breaking (normative):**

* If multiple futures are ready at the decision point, the runtime MUST choose the winner by a deterministic total order.
* The total order MUST be stable across replay and across machines.

(v0.1 intentionally does not mandate *which* total order is used; it mandates only that the runtime has one and that it is stable.)

**Drop vs cancel (normative):** dropping a non-chosen future MUST:

* prevent its value/error from being observed by the current `select`, and
* release any fiber-local resources associated with waiting on it.

Dropping MAY request cancellation. Cancellation is not guaranteed unless the future kind promises it.

**Future kind expectations (v0.1):**

* **Queue receive futures:** dropping MUST unregister the wait; it MUST NOT consume a message.
* **Timer futures:** dropping SHOULD cancel the timer; if cancellation races with firing, the observable outcome MUST be replay-stable.
* **External-call futures:** dropping MUST cancel only the *waiting*; the underlying external request MAY continue. If the request completes, the runtime MUST handle the completion in a replay-safe way (e.g., record-and-ignore, or mark as cancelled) so replay observes identical outcomes.

**Error outcomes (normative):** a `select` arm may win with an error. The error MUST be delivered deterministically as part of the same ordering rule.

**No fairness guarantee (non-normative, v0.1):** because selection is defined by deterministic order, `select` does not guarantee fairness among always-ready futures(ex. users should not assume round-robin behavior)

**Side effects (normative):** only the winning arm’s body executes. The non-chosen arms’ bodies MUST NOT execute, and any side effects that would have occurred inside those bodies MUST NOT occur.


---

## 5. Time model

Maroon code MUST NOT read wall-clock time directly.

Maroon supports two time domains:

1. **Logical monotonic time (timers)**

* Use logical monotonic timers, e.g. `after(ms) -> Future<Unit>`.
* Logical time progression is part of the event history and MUST replay deterministically.

2. **Wall-clock / calendar time (external schedule events)**

* Calendar notions like “Thursday at 15:00 in Atlantic/Madeira” are inherently tied to time zones and DST and therefore MUST be modeled as an external capability (e.g., `calendar` / `scheduler`).
* The scheduler emits discrete schedule-fired events (or returns recorded `now()` samples) into the fiber’s event history.
* Replay re-delivers those same schedule events/results; user code does not observe the live wall clock.

---

## 6. Determinism contract (normative)

A Maroon program MUST be deterministic with respect to its event history.

Therefore, user code MUST NOT:

* read wall-clock / OS time
* read ambient environment (hostname, process id, env vars, filesystem state, etc.)
* use nondeterministic iteration order
* use host-dependent floating point for persisted or branching-critical decisions

  * use fixed-point `Decimal{scale}` instead

### 6.1 Observability and the read/write split

Observability (logs/metrics/traces) is **non-semantic**: it MUST NOT affect ordering decisions, control flow, durable state, or determinism.

Normative expectations:

* Emitting telemetry MUST be fire-and-forget from the perspective of program semantics.
* Telemetry backpressure, sampling, delays, or drops MUST NOT change program behavior.
* Telemetry is write-only for fibers: it MUST NOT be used as an input source.

**Separated read path:** read queries MUST NOT participate in fiber scheduling/order and MUST NOT mutate durable state. Read views may lag behind the write path by policy.

---

## 7. Type system (persistable values)

### 7.1 Primitives

* `Unit`, `Bool`
* `I64`, `U64`
* `Decimal{scale}` (fixed-point)
* `Bytes`, `String`

### 7.2 Composite types

* `Option<T>`
* `Vec<T>`
* `MaroonMap<K,V>`
* `MaroonSet<T>`
* `struct` and `enum`
* `type` aliases

### 7.3 Deterministic iteration rules (normative)

* `Vec<T>` iterates in insertion order.
* `MaroonMap<K,V>` and `MaroonSet<T>` iterate in a stable total order over keys.

  * The exact ordering function is defined by the platform/runtime (and MUST be identical across machines and versions that interoperate).

---

## 8. State and persistence model

Only **schema-defined fiber state** is durable.

The runtime persists fiber state using a stable, deterministic serialization format and can recover by replaying:

* fiber creation
* message deliveries
* timer firings
* external effect results (when applicable)

**Rule:** durable state MUST be serializable and replayable in a way that is identical across platforms and versions that interoperate.

**Cross-version persistence note (informative):** guaranteeing that persisted fiber state and event histories remain readable and replayable across runtime and language versions requires a well-defined storage and serialization compatibility contract (e.g., versioned encodings, backward/forward compatibility rules, canonical representations). This contract is intentionally **out of scope** for the language specification and is defined in a separate runtime/storage specification.

(Exact byte-level encoding is an implementation detail and is defined in a separate runtime/storage specification, not in this white paper.)

---

## 9. Failure and error model (v0.1)

This section defines how *failures* are represented to user code and how they participate in deterministic replay.

### 9.1 Principles

**Errors are values (normative, v0.1):** operations that can fail MUST expose failure explicitly in their return types (e.g., `Future<Result<T,E>>`). The runtime MUST NOT inject hidden exceptions into `Future<T>`.

**Determinism (normative):** given the same event history, replay MUST deliver the same success/error outcomes at the same logical points.

**No implicit recovery (non-normative):** Maroon does not assume a single global failure policy. Recovery is expressed explicitly (retry loops, supervision fibers, compensations).

### 9.2 Fiber failure states

A fiber can be in one of these runtime-visible states:

* **Running** — normal execution
* **Failed** — execution terminated at a specific logical point; the fiber instance remains terminated (v0.1 does not define resuming a failed instance)


**Failed (clarifying):**

* **Failed** is a *program failure*: e.g., a bug or an unhandled fatal error. In v0.1, failure is **terminal for that fiber instance**.

  *Recovery model (v0.1):* recovery is expressed by **supervision**: a supervision fiber may react to `FiberFailed` by creating a *new* fiber instance (optionally of the same type) and/or rerouting work.

### 9.3 What constitutes a fiber failure

A fiber enters **Failed** when execution terminates due to a *fatal* condition.

**Initiation:**

* A fatal condition MAY be triggered by **user code** (e.g., `panic`, `fail!`, or an explicit `return Err` that is not handled and is defined by the runtime to be fatal for that entrypoint).
* A fatal condition MAY be triggered by the **runtime/system** (e.g., state decode failure, invariant violation, determinism contract breach).

**Normative:**

* Regardless of who triggers it, a **TerminalEvent** with `class = Failed` is emitted by the **runtime** to record that the fiber instance became terminal at a specific logical point.
* If a fiber fails, the failure MUST be represented as a deterministic event in the fiber’s history (a **TerminalEvent** with `class = Failed`) so replay reproduces the same termination.

### 9.4 How failures appear in history

The fiber’s event history MAY contain the following failure-related entries:

* **FailureEvent { kind, detail }** — records that the fiber instance became terminal at a specific logical point.

**Normative:**

* On replay, the runtime MUST re-deliver the same failure events at the same logical position.
* After a FailureEvent, user code MUST NOT continue executing past that point during replay (the fiber instance remains terminated).

### 9.4.1 System fiber event stream (supervision)

To support *supervision fibers* (user-defined recovery policy), the runtime exposes a **system event stream** about fiber lifecycle.

This stream is the *delivery mechanism* for failure events described above; it does not introduce new semantics beyond making those events observable to user code.

**Normative (v0.1):**

* The runtime MUST provide a built-in, receive-only queue of system events with a stable identifier in a reserved namespace.
* User code MUST NOT be able to send into this queue.
* Delivery of system events to a supervision fiber MUST be replay-stable (i.e., they appear in that fiber’s event history like any other message deliveries).

**Naming:** the exact string name is an implementation detail as long as it is stable and reserved. Recommended convention is a reserved prefix such as `mrn.system.*` (e.g., `mrn.system.fibers`).

**Event type (v0.1 minimum):**

```dsl
enum SystemFiberEvent {
  FiberFailed {
    fiber_id: String,
    fiber_type: String,
    kind: String,
    detail: String,
  },
}
```


### 9.5 Retries

Retries are modeled explicitly in user code (or via explicit helper libraries), not as implicit runtime behavior.

Two common patterns:

1. **Retry as control flow:**

   * An external call returns `Result<T,E>`.
   * The fiber decides if/when to retry.

2. **Retry as policy wrapper (still explicit):**

   * A library provides `retry(policy, || external_call())`.
   * The wrapper is deterministic because retry attempts, delays, and final outcomes are driven by recorded events (timers + external results).

**Normative:** any retry decision that depends on nondeterministic facts MUST be based on recorded events (e.g., a timer firing or an external error result).



### 9.7 Cancellation and timeouts

Cancellation is not a generic hidden error. If a future’s API exposes cancellation or timeouts, it MUST do so in the type (e.g., `Result<T, TimeoutOr<E>>`).

Timeouts SHOULD be implemented via logical timers + `select` in user code (or via explicit library helpers), preserving replay determinism.

---

## 10. External effects (declared capabilities)

Maroon has no ambient I/O. Any interaction with the outside world MUST be declared.

Effect classes:

* `pure`: no effects
* `timer`: logical time only
* `external(service)`: typed request/response to an external service

Each external call MAY declare:

* timeout policy
* retry/backoff policy
* idempotency strategy (e.g., idempotency key)

### 10.1 External calls as two history events (normative, v0.1)

External calls are executed by **gateways/adapters**, not directly by fibers.

A fiber-level external call is represented by **durable effect-log entries** that produce **two** events in the fiber’s event history:

1. **ExternalRequestSent** — records the request parameters, a stable `call_id`, and the declared execution policy (timeouts, retry/backoff, idempotency key, etc.).
2. **ExternalResponseReady** — records the final outcome associated with that `call_id`, either a success value or a failure value.

Together, these entries form the **effect log contract** for external calls.

**Gateway-executed retries (normative, v0.1):** gateways MAY perform multiple HTTP attempt(s) according to the policy recorded in `ExternalRequestSent`. Intermediate attempts are not observable to user code.

**Replay contract (normative, v0.1):** during replay, the runtime MUST re-deliver the same effect-log entries (`ExternalRequestSent` and `ExternalResponseReady`) at the same logical positions and MUST NOT re-execute the external side effect.

**Routing / ordering note (informative):** the runtime may use an internal global ordering mechanism to route effect outcomes back to fibers, but this is not a language-level primitive. The language only relies on the resulting per-fiber event history being deterministic.

(Exact gateway implementation mechanisms are runtime-defined in v0.1, but MUST satisfy these semantics.)

---

## 11. Schema evolution

Maroon has two broad classes of schema evolution:

1. **Internal state evolution** (fiber-owned durable state)
2. **Interface evolution** (queue message schemas and gateway-exposed endpoints)

Internal state can be migrated in place per fiber instance. Interface schemas are shared contracts across producers/consumers and typically require versioning and compatibility rules.

Fibers may define versioned state:

* `state current { ... }`
* `state next { ... }`
* `migrate current -> next { ... }`

### 11.1 Migration rules (normative)

A migration is a deterministic transformation that transitions a fiber instance from `current` state to `next` state.

**Allowed operations (v0.1):**

* `migrate` MUST be **pure**: it MUST NOT use `await`, `select`, queue send/receive, timers, or external calls.

**Determinism / replay-safety:**

* `migrate` MUST be deterministic and terminating.

**State access:**

* Migration reads from `from.<field>` (old state snapshot) and writes to `self.<field>` (new state).
* New fields MUST be explicitly initialized.

**Practical note (non-normative):**

If you need effectful backfills or external coordination during an upgrade, do it in normal fiber code (e.g., via a bridge fiber or a phased rollout), not inside `migrate`.

### 11.2 Promotion

After rollout, the fiber schema is “promoted” by removing the old state and migration, leaving only the new schema as `current`.

### 11.3 Interface evolution (queues and gateways)

Queue message schemas are **interfaces**: they couple multiple deployable components (fibers and/or external clients via gateways). This makes them more sensitive than internal fiber state.

**Recommended rule (v0.1): treat queue schemas as immutable contracts**

* Breaking changes to a queue schema SHOULD NOT be performed in place.
* Introduce a new queue (e.g., `orders.v2`) with a new message type.

**Migration patterns (common):**

1. Parallel queues + dual write
2. Bridge fiber (transform old -> new)
3. Dual readers
4. Gateway version routing (for public queues)

### 11.4 Code evolution and in-flight upgrades (normative, v0.1)

Schema migrations change the **structure of stored data** (fields, types, layout). By themselves, they do **not** make changes to program **behavior or execution order** safe for replay.

**Replay compatibility (normative):** a fiber instance MUST be executed by code that can deterministically consume that instance’s existing event history.

**In-flight upgrade policy (v0.1 default): instance pinning.**

* Each fiber instance has an associated **CodeVersionID** established at creation.
* For its lifetime, a fiber instance MUST be executed only by code that is replay-compatible with that CodeVersionID.
* New deployments MAY introduce new CodeVersionIDs; these apply only to newly created fiber instances.

Under this policy, a fiber instance is **not implicitly upgraded in place** to new control-flow logic.

**Instance replacement (recommended pattern):** when control-flow changes are not replay-compatible, upgrade by creating a new fiber instance and handing off work.

A simple, robust replacement upgrade sequence (informative):

1. Deploy the new code version (e.g., `A@v2`).
2. Send an explicit control message (e.g., `Shutdown { reason }`) to the old fiber instance `f1 : A@v1`.
3. `f1 : A@v1` reaches a normal receive point, enters **draining mode**, finishes in-flight work, and stops accepting new work.
4. `f1 : A@v1` emits a **handoff snapshot** (a schema-defined, deterministic snapshot of its durable state) and then terminates.
5. The runtime creates a new fiber instance `f1 : A@v2`, reusing the same **logical identity** but a new runtime instance.
6. `f1 : A@v2` starts from `init` / `main`, consumes the handoff snapshot (or a pointer to it), reconstructs its durable state, and continues execution.

**Execution boundary (normative):** the runtime MUST NOT preempt a fiber in the middle of a step. A fiber may be stopped or terminated only at yield boundaries (blocking `await` / `select`, or function return).

**Future (non-normative):** a later version of Maroon MAY add an explicit version-gating primitive that records control-flow decisions into the event history, enabling a single binary to replay old histories while using new logic for new instances.

---

## 12. Minimal example (echo fiber)

This example demonstrates:

* durable per-fiber state
* multi-queue receive with `select`
* deterministic state updates
* typed message sending

**Note (informative):** this example intentionally shows a *stable, long-lived control loop*. In v0.1, incompatible changes to this control flow (for example, removing or reordering queue receives) require **instance replacement** as described in §11.4; they MUST NOT be applied as in-place upgrades to running instances.

```dsl
struct EchoOut {
  echo_of: String,
  count: U64,
  fiber: String,
}

enum Control {
  Shutdown { reason: String },
}

fiber Echo(
  name: String,
  in_first: RecvQueue<String>,
  in_second: RecvQueue<String>,
  control: RecvQueue<Control>,
  out: SendQueue<EchoOut>,
) {
  state current {
    seen: U64,
  }

  fn init {
    self.seen = 0;
  }

  fn main() {
    loop {
      select {
        let _ = self.control.await() => {
          // enter drain-and-stop mode
          return;
        }

        let msg: String = self.in_first.await() => {
          let n = self.seen + 1;
          self.seen = n;
          self.out.send(EchoOut { echo_of: msg + " first", count: n, fiber: self.name });
        }

        let msg: String = self.in_second.await() => {
          let n = self.seen + 1;
          self.seen = n;
          self.out.send(EchoOut { echo_of: msg + " second", count: n, fiber: self.name });
        }
      }
    }
  }
}
    }
  }
}
```

---

## 13. Glossary

* **Event history:** the deterministic sequence of inputs the runtime provides to fibers (messages, timers, external results).
* **Replay:** re-executing fiber code from a checkpoint or from genesis using the same history to reconstruct state.
* **Durable state:** schema-defined fiber-owned data persisted by the runtime in a replay-safe format.
* **Effect:** any interaction beyond pure computation (timers and externals), always explicit.
* **TotalOrder (v0.1 assumption):** the runtime-defined, replayable total ordering of events delivered to fibers that disambiguates scheduling decisions (e.g., which `select` arm wins).
