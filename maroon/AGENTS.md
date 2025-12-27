# Maroon Crate — Essentials

Compact guide for coding agents: core flow, interfaces, edit points, and constraints.

## Purpose
- Core node that tracks offsets, exchanges state/txs over libp2p, proposes/accepts epochs, executes confirmed tasks, and notifies gateways.

## Core Flow
1. `main.rs` builds and starts `MaroonStack` (p2p + app + epoch coordinator + runtime + metrics).
2. `P2P` forwards libp2p events to `App` as `Inbox` and publishes `Outbox` from `App`.
3. `App` loop:
   - Advertise `NodeState`, detect gaps, request/provide missing txs.
   - On tick, compute epoch increments (consensus vs committed) and request commit.
   - On commit, linearize and send tasks to `runtime`; on results, `NotifyGWs`.
   - Batch runtime results via `receiver.recv_many` before emitting `NotifyGWs`.

## Key Interfaces
- App state: `app::interface::{Request::GetState, Response::State(CurrentOffsets)}`.
- Node wire: `network::interface::{Outbox, Inbox, NodeState}`.
- Epochs: `epoch_coordinator::interface::{EpochRequest, EpochUpdates}`.
- Runtime: `runtime::runtime::{TaskBlueprint, Input, Output}`.
- Runtime endpoint: `Endpoint<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>`.

## Edit Points
- App logic: `src/app/app.rs` (offsets, consensus, epochs, inbox handling, runtime I/O).
- P2P wire: `src/network/interface.rs` (types), `src/network/p2p.rs` (send/recv paths, gossip topics).
- Epoch cadence: `src/epoch_decision_engine.rs` (ordering and `should_send`).
- Linearization: `src/linearizer.rs` (implement/plug alternative policies).
- Composition: `src/stack.rs` (channels/wiring), `src/main.rs` (env, metrics).

## Parameters & Env
- App params: `advertise_period`, `consensus_nodes`, `epoch_period` (`src/app/params.rs`).
- Env vars (binary): `NODE_URLS`, `ETCD_URLS`, `SELF_URL`, `CONSENSUS_NODES`, `OTEL_EXPORTER_OTLP_GRPC_ENDPOINT`.

## Constraints & Gotchas
- Offsets advance only across contiguous txs; gaps stall progress (`update_self_offsets`).
- Beware range boundary overflow TODO when iterating `UniqueU64BlobId`.
- Advertise and epoch timers are independent; epochs may not align with every advertise.
- Gateway notifications via gossip are best-effort; missing updates must be re-queried by gateways.
