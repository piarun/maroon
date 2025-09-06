# Epoch Publisher Scheduling (aka “leader election”)

The mechanism described below is applied only to [global-queue](./global-queue.md) functioning.

There is no permanent single leader. Any node can publish the next epoch during its time slice in a ring schedule. Concurrent attempts are resolved by etcd using CAS (compare and swap).

- Scheduling: Nodes sort all known `PeerId`s and form a ring. Given the last committer and the current node’s position, a node’s earliest publish time is `(position+1) * epoch_period` after the last commit.
- Decision: Each node runs a local decider that checks if its time slice has arrived (based on the last epoch’s timestamp and committer) and prepares an epoch from current quorum offsets.
- Commit: The node attempts to commit the next epoch to etcd with a transaction that puts both `/maroon/latest` and `/maroon/history/<seq>` only if the history key does not yet exist. Only one attempt wins per sequence number.
    - Etcd as arbiter: We use etcd only to CAS-commit epochs, batching many transactions per epoch—
no per-tick Raft confirmations
- Distribution: All nodes watch `/maroon/latest` to learn about new epochs. On startup or reconnection they may also scan `/maroon/history` to backfill missed epochs.

Why this is “leader-like”

The time-slice owner acts like a temporary leader for one epoch. If they fail, the next node’s slice arrives and it can publish instead. This provides liveness without a dedicated leader election protocol. etcd’s CAS provides the final arbitration.