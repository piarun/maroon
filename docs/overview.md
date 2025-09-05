# Overview

It's a maroon framework. The idea is that you as a developer write only business logic, while the framework provides ordering, replication, and deterministic execution.

- You don't need to design persistence, conditional writes, or total order logic — the framework handles it.
- Transactions are globally ordered via epochs and applied deterministically across nodes.

Guarantees (current design)

- Total order: All nodes observe the same append-only sequence of transactions.
- Deterministic application: Same inputs produce the same state, no extra coordination needed.
- Idempotency: Transactions are uniquely identified by `UniqueU64BlobId`.
- Eventual consistency: Nodes that fall behind catch up by replaying epochs and fetching missing transactions.

