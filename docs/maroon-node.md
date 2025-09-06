# Maroon node

Core piece of the maroon engine.

## Epoch publishing

There is no permanent leader. Nodes coordinate via a ring-based time-slice scheduler and commit epochs to etcd with CAS. See [epoch publisher scheduling](./leader-election.md).

MN responsibilities

- Receive transactions from gateways, deduplicate by `UniqueU64BlobId`, and gossip them via P2P.
- Periodically advertise local per-range offsets to peers.
- Decide when to attempt publishing an epoch; assemble increments from quorum offsets vs committed offsets; attempt etcd commit.
- Watch etcd for new epochs and apply them deterministically.
- Execute transactions(TBA)