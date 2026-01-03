# Maroon DSL — Echo and Migration Examples (Draft)

These examples illustrate how the DSL in `docs/dsl-language.md` maps to fibers, queues, `select`, and schema migration using a minimal Echo fiber. Syntax here follows the conceptual spec (fibers, `state current/next`, `send`/`recv`, `after(ms)`, `await`, `select`, `external`) rather than the current parser status.

## Example 1 — Minimal Echo Fiber

Purpose: show `recv`/`send` and per-fiber state. Deterministic: no wall clock, pure counter.

```dsl
// Outbound payload for echoes
struct EchoOut {
  echo_of: String,
  count: U64,
  fiber: String,
}

// Explicit constructor parameters: fiber identity + typed directional queues
fiber Echo(name: String, in_queue_first: RecvQueue<String>,  in_queue_second: RecvQueue<String>, out_queue: SendQueue<EchoOut>) {
  state current {
    seen: U64,
  }

  // Pure constructor: runs once on fiber creation
  fn init {
    self.seen = 0;
  }

  // Main loop: illustrate select across multiple inbound queues
  fn main() {
    loop {
      select {
        // Arm 1: receive from the first queue (RecvQueue)
        let msg: String = self.in_queue_first.await => {
          let n = self.seen + 1;
          self.seen = n;
          self.out_queue.send(EchoOut { echo_of: msg + " first", count: n, fiber: self.name });
        }

        // Arm 2: receive from the second queue (RecvQueue)
        let msg: String = self.in_queue_second.await => {
          let n = self.seen + 1;
          self.seen = n;
          self.out_queue.send(EchoOut { echo_of: msg + " second", count: n, fiber: self.name });
        }
      }
    }
  }
}
```

Ingress/egress queues (illustrative bindings):
- In 1: `queue("echo.in.first.<name>")` -> `in_queue_first: RecvQueue<String>`
- In 2: `queue("echo.in.second.<name>")` -> `in_queue_second: RecvQueue<String>`
- Out:  `queue("echo.out.<name>")` -> `out_queue: SendQueue<EchoOut>`

Note: The runtime’s underlying channel is duplex, but the DSL encourages directional capability types for clarity and static safety. Use `DuplexQueue<T>` only when both directions are truly needed (e.g., brokers/tests), or split a duplex handle into `(RecvQueue<T>, SendQueue<T>)` for explicit usage.

## Example 2 — Two-Version Migration: current -> next

Goal: demonstrate an explicit state migration that both renames a field and adds a new one.

Current state (from the example above):

```dsl
state current {
  seen: U64,
}
```

Desired next state (phased rename):
- Add `last_input: Option<String>` with an explicit default
- Introduce `count: U64` alongside existing `seen` to allow code to switch safely

```dsl
migrate current -> next {
  // `from.<field>` is the old snapshot; `self.<field>` is the new state
  self.last_input = None;   // explicit default
  self.count = from.seen;   // first step of renaming field
}

state next {
  seen: U64,
  count: U64,
  last_input: Option<String>,
}
```

- Deploy with `state next` present: migration runs to completion in the background. Before migration finishes, code continues to use the `current` shape for reads/writes.
- After migration, update the main logic to use `self.count` instead of `self.seen`, and optionally track the last input when receiving a message:

```dsl
// inside the first select arm
let n = self.count + 1;
self.count = n;
self.last_input = Some(msg);
self.out_queue.send(EchoOut { echo_of: msg + " first", count: n, fiber: self.name });
```

Finalize/promotion: after rolling out the code that uses `self.count`, promote `next` -> `current` by removing the old `current` and the `migrate` block, leaving only the new `current`:

```dsl
state next {
  count: U64,
  last_input: Option<String>,
}
```

After promotion, any reference to the removed `seen` field is a compile error.

Notes on migration:
- Migrations are pure and must not perform I/O or waits; no `await`/`select`/`external` inside `migrate`.
- Initialize every new-state field explicitly. Uninitialized fields are a compile error.
- Reading old fields is done via `from.<field>`; writing new fields via `self.<field>`.
- If a key type changes inside `Map`/`Set`, ensure the transformation preserves canonical ordering.
