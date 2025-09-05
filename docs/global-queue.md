Global queue defines a deterministic total order of transactions across all `KeyRange`s using quorum offsets and epochs.

- Inputs: Every node periodically advertises, per `KeyRange r`, the highest contiguous `KeyOffset` it has locally, `o[i,r]`.
- Quorum: Given `CONSENSUS_NODES = k`, the quorum offset for range `r` is `O[r] = kth_largest({ o[i,r] })`. If fewer than `k` nodes reported for `r`, `O[r]` is undefined for this round.
- Increments: Let `C[r]` be the last committed offset for `r` (derived from previous epochs). If `O[r] > C[r]`, the next epoch includes a closed interval `[C[r]+1, O[r]]` for `r`.
- Epoch: An epoch is a set of disjoint or overlapping intervals across ranges. It is committed via CAS to etcd history and exposed as `/maroon/latest` for watchers.
- Order within epoch: Intervals are sorted by their start `UniqueU64BlobId` (then by end), then expanded in increasing id order. The expanded sequence is appended to the global log.

Example

Assume three nodes report per-range offsets:
```
N1<(1,3),(2,1),(3,3)>
N2<(1,2),(2,1),(3,3)>
N3<(1,4),(2,1),(3,2)>
```
With `k=2` (two-node quorum), the quorum offsets are: `(1,3),(2,1),(3,3)`.
If all `C[r]=0`, the first epoch’s increments are:
```
e1: [(1,[1..3]), (2,[1..1]), (3,[1..3])]
```
Expanded and ordered, this yields:
```
(1,1)(1,2)(1,3)(2,1)(3,1)(3,2)(3,3)
```
If later the quorum for range 2 advances to `4` while others don’t, the next epoch adds:
```
e2: [(2,[2..4])]  ->  (2,2)(2,3)(2,4)
```
Total order is the concatenation of epoch expansions.

Notes

- Safety model: This quorum is crash-fault tolerant; Byzantine behavior is out of scope.
- Liveness: The ring-based scheduler (see leader-election) staggers commit attempts; etcd CAS resolves races if multiple nodes attempt the same sequence number.
- Determinism: All nodes observe the same epochs and expand intervals identically, yielding the same total order. See [aligned execution](./aligned-execution.md).
