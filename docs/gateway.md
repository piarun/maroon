# Gateway (GW)

## Synonyms
- sidecar
- client library

## Init

Before GW can start working it should request a `KeyRange` lease for unique keys from the [MN/maroon node](./maroon-node.md).
The lease is persisted (etcd, planned) to avoid overlapping allocations and to enable recovery.

## Work
When GW gets a new request:
- It forms a transaction and assigns a `UniqueU64BlobId` from its current [key range](./keys-range.md).
  - If the range is exhausted, it requests the next `KeyRange` and continues.
- It sends the transaction to at least one MN (can send to more for lower latency).
  - MNs gossip transactions via P2P; deduplication is by `UniqueU64BlobId`.
- Retries: GW retries on delivery errors with backoff. It is safe to retry because IDs are unique and idempotent at MN.
  - Policy/TODO: cap retry horizon and provide backpressure signals.
- Keeps connection with the requester and returns response when MN report finishing.

## Control plane
GW should know MN topology or at least one reachable MN address (`NODE_URLS`).
There is no single long-lived leader; see [epoch publisher scheduling](./leader-election.md) for how epochs are produced.
