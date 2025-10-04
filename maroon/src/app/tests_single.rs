use crate::app::Params;
use crate::app::interface::CurrentOffsets;
use crate::network::*;
use crate::test_helpers::{new_test_instance, new_test_instance_with_params, reaches_state, test_tx};
use common::duplex_channel::create_a_b_duplex_pair;
use common::invoker_handler::create_invoker_handler_pair;
use common::logical_time::LogicalTimeAbsoluteMs;
use common::range_key::{KeyOffset, KeyRange, U64BlobIdClosedInterval, UniqueU64BlobId};
use epoch_coordinator::epoch::Epoch;
use epoch_coordinator::interface::{EpochRequest, EpochUpdates};
use generated::maroon_assembler::Value;
use libp2p::PeerId;
use protocol::node2gw::TxUpdate;
use protocol::transaction::{Meta, TxStatus};
use runtime::ir::FiberType;
use runtime::runtime::{Input as RuntimeInput, Output as RuntimeOutput, TaskBlueprint};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::oneshot;

///
/// In this file we're testing app as a black box by accessing only publicly available interface module
/// not really integration tests, but not unit either
///

#[tokio::test(flavor = "multi_thread")]
async fn app_calculates_consensus_offset() {
  let (a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
  let (a2b_epoch, _b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();
  let (a2b_runtime, _b2a_runtime) = create_a_b_duplex_pair::<RuntimeInput, RuntimeOutput>();

  let (state_invoker, handler) = create_invoker_handler_pair();
  let mut app = new_test_instance(b2a_endpoint, handler, a2b_epoch, a2b_runtime);
  let (_shutdown_tx, shutdown_rx) = oneshot::channel();

  let n1_peer_id = PeerId::random();
  let n2_peer_id = PeerId::random();

  tokio::spawn(async move {
    app.loop_until_shutdown(shutdown_rx).await;
  });

  a2b_endpoint
    .sender
    .send(Inbox::State((
      n1_peer_id,
      NodeState {
        offsets: HashMap::from([(KeyRange(1), KeyOffset(3)), (KeyRange(2), KeyOffset(7)), (KeyRange(4), KeyOffset(1))]),
      },
    )))
    .unwrap();
  a2b_endpoint
    .sender
    .send(Inbox::State((
      n2_peer_id,
      NodeState { offsets: HashMap::from([(KeyRange(1), KeyOffset(2)), (KeyRange(2), KeyOffset(9))]) },
    )))
    .unwrap();

  assert!(
    reaches_state(
      3,
      Duration::from_millis(5),
      &state_invoker,
      CurrentOffsets {
        self_offsets: HashMap::new(),
        consensus_offset: HashMap::from([(KeyRange(1), KeyOffset(2)), (KeyRange(2), KeyOffset(7))]),
      }
    )
    .await
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn app_gets_missing_transaction() {
  let (a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
  let (a2b_epoch, _b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();
  let (a2b_runtime, _b2a_runtime) = create_a_b_duplex_pair::<RuntimeInput, RuntimeOutput>();
  let (state_invoker, handler) = create_invoker_handler_pair();
  let mut app = new_test_instance(b2a_endpoint, handler, a2b_epoch, a2b_runtime);
  let (_shutdown_tx, shutdown_rx) = oneshot::channel();

  tokio::spawn(async move {
    app.loop_until_shutdown(shutdown_rx).await;
  });

  // app gets some transaction from the future
  a2b_endpoint.send(Inbox::NewTransaction(test_tx(5)));
  a2b_endpoint.send(Inbox::NewTransaction(test_tx(0)));

  assert!(
    reaches_state(
      3,
      Duration::from_millis(5),
      &state_invoker,
      CurrentOffsets { self_offsets: HashMap::from([(KeyRange(0), KeyOffset(0))]), consensus_offset: HashMap::new() }
    )
    .await
  );

  // and now app gets missing transaction
  a2b_endpoint.send(Inbox::MissingTx(vec![test_tx(3), test_tx(4), test_tx(2), test_tx(0), test_tx(1)]));

  assert!(
    reaches_state(
      3,
      Duration::from_millis(5),
      &state_invoker,
      CurrentOffsets { self_offsets: HashMap::from([(KeyRange(0), KeyOffset(5))]), consensus_offset: HashMap::new() }
    )
    .await
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn app_gets_missing_transactions_that_smbd_else_requested() {
  let (mut a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
  let (a2b_epoch, _b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();
  let (a2b_runtime, _b2a_runtime) = create_a_b_duplex_pair::<RuntimeInput, RuntimeOutput>();
  let (state_invoker, handler) = create_invoker_handler_pair();
  let mut app = new_test_instance(b2a_endpoint, handler, a2b_epoch, a2b_runtime);
  let (_shutdown_tx, shutdown_rx) = oneshot::channel();

  tokio::spawn(async move {
    app.loop_until_shutdown(shutdown_rx).await;
  });

  a2b_endpoint.send(Inbox::NewTransaction(test_tx(2)));
  a2b_endpoint.send(Inbox::NewTransaction(test_tx(3)));
  a2b_endpoint.send(Inbox::NewTransaction(test_tx(1)));
  a2b_endpoint.send(Inbox::NewTransaction(test_tx(0)));
  a2b_endpoint.send(Inbox::NewTransaction(test_tx(4)));

  assert!(
    reaches_state(
      3,
      Duration::from_millis(5),
      &state_invoker,
      CurrentOffsets { self_offsets: HashMap::from([(KeyRange(0), KeyOffset(4))]), consensus_offset: HashMap::new() }
    )
    .await
  );

  let rnd_peer = PeerId::random();
  a2b_endpoint
    .sender
    .send(Inbox::RequestMissingTxs((rnd_peer, vec![U64BlobIdClosedInterval::new(1, 3)])))
    .expect("channel shouldnt be dropped");

  while let Some(outbox) = a2b_endpoint.receiver.recv().await {
    let Outbox::RequestedTxsForPeer((peer, requested_txs)) = outbox else {
      continue;
    };
    assert_eq!(rnd_peer, peer);
    assert_eq!(requested_txs, vec![test_tx(1), test_tx(2), test_tx(3)]);

    break;
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn app_detects_that_its_behind_and_makes_request() {
  let (mut a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
  let (a2b_epoch, _b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();
  let (a2b_runtime, _b2a_runtime) = create_a_b_duplex_pair::<RuntimeInput, RuntimeOutput>();
  let (state_invoker, handler) = create_invoker_handler_pair();
  let mut app = new_test_instance(b2a_endpoint, handler, a2b_epoch, a2b_runtime);
  let (_shutdown_tx, shutdown_rx) = oneshot::channel();

  tokio::spawn(async move {
    app.loop_until_shutdown(shutdown_rx).await;
  });

  a2b_endpoint.send(Inbox::NewTransaction(test_tx(0)));
  a2b_endpoint.send(Inbox::NewTransaction(test_tx(4)));

  assert!(
    reaches_state(
      3,
      Duration::from_millis(5),
      &state_invoker,
      CurrentOffsets { self_offsets: HashMap::from([(KeyRange(0), KeyOffset(0))]), consensus_offset: HashMap::new() }
    )
    .await
  );

  let rnd_peer = PeerId::random();
  a2b_endpoint
    .sender
    .send(Inbox::State((rnd_peer, NodeState { offsets: HashMap::from([(KeyRange(0), KeyOffset(8))]) })))
    .expect("dont drop");

  while let Some(outbox) = a2b_endpoint.receiver.recv().await {
    let Outbox::RequestMissingTxs((peer, requested_intervals)) = outbox else {
      continue;
    };
    assert_eq!(rnd_peer, peer);
    assert_eq!(requested_intervals, vec![U64BlobIdClosedInterval::new(1, 3), U64BlobIdClosedInterval::new(5, 8),]);

    break;
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn app_sends_epochs_to_epoch_coordinator() {
  let (a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
  let (a2b_epoch, mut b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();
  let (a2b_runtime, _b2a_runtime) = create_a_b_duplex_pair::<RuntimeInput, RuntimeOutput>();
  let (_state_invoker, handler) = create_invoker_handler_pair();
  let mut app = new_test_instance_with_params(
    b2a_endpoint,
    handler,
    a2b_epoch,
    a2b_runtime,
    Params::default()
      .set_consensus_nodes(NonZeroUsize::new(1).unwrap())
      .set_epoch_period(LogicalTimeAbsoluteMs::from_millis(200))
      .set_advertise_period(Duration::from_millis(100)),
  );
  let (_shutdown_tx, shutdown_rx) = oneshot::channel();

  tokio::spawn(async move {
    app.loop_until_shutdown(shutdown_rx).await;
  });

  let increments = Arc::new(Mutex::new(Vec::<Vec<U64BlobIdClosedInterval>>::new()));
  let incs_spawn = increments.clone();

  tokio::spawn(async move {
    while let Some(v) = b2a_epoch.receiver.recv().await {
      let mut guard = incs_spawn.lock().await;
      guard.push(v.epoch.increments.clone());

      // get and immediately "accept" a new epoch
      b2a_epoch.send(EpochUpdates::New(v.epoch));
    }
  });

  a2b_endpoint.sender.send(Inbox::NewTransaction(test_tx(0))).unwrap();
  // need this sleep in order to send tx in two different epochs
  // epoch period is much lower now(200ms) than this sleep, so probably it will happen in two different epochs
  // TODO: use tokio time manipulation techniques for making this test more reliable
  tokio::time::sleep(Duration::from_millis(1000)).await;
  a2b_endpoint.sender.send(Inbox::NewTransaction(test_tx(1))).unwrap();

  let mut has_expected_tx = false;

  for _ in 0..3 {
    let guard = increments.lock().await;
    if guard.contains(&vec![U64BlobIdClosedInterval::new(0, 0)])
      && guard.contains(&vec![U64BlobIdClosedInterval::new(1, 1)])
    {
      has_expected_tx = true;
      break;
    }
    drop(guard);

    tokio::time::sleep(Duration::from_millis(500)).await;
  }

  let guard_collected = increments.lock().await;
  assert!(has_expected_tx, "{:?}", guard_collected);
}

#[tokio::test(flavor = "multi_thread")]
async fn app_executes_after_epoch_confirmed() {
  // tests cycle:
  // (commited epoch) -> app -> runtime(with confirmed txs) -> (runtime exec result) -> app -> p2p network layer
  let (mut a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
  let (a2b_epoch, b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();
  let (a2b_runtime, mut b2a_runtime) = create_a_b_duplex_pair::<RuntimeInput, RuntimeOutput>();
  let (_state_invoker, handler) = create_invoker_handler_pair();
  let mut app = new_test_instance_with_params(
    b2a_endpoint,
    handler,
    a2b_epoch,
    a2b_runtime,
    Params::default()
      .set_consensus_nodes(NonZeroUsize::new(1).unwrap())
      .set_epoch_period(LogicalTimeAbsoluteMs::from_millis(200))
      .set_advertise_period(Duration::from_millis(100)),
  );
  let (_shutdown_tx, shutdown_rx) = oneshot::channel();

  a2b_endpoint.sender.send(Inbox::NewTransaction(test_tx(0))).unwrap();
  a2b_endpoint.sender.send(Inbox::NewTransaction(test_tx(1))).unwrap();

  tokio::spawn(async move {
    app.loop_until_shutdown(shutdown_rx).await;
  });

  tokio::time::sleep(Duration::from_millis(500)).await;

  let rnd_peer = PeerId::random();
  // imitate new epoch came
  b2a_epoch.send(EpochUpdates::New(Epoch::next(
    rnd_peer,
    vec![U64BlobIdClosedInterval::new(0, 1)],
    None,
    LogicalTimeAbsoluteMs(0),
  )));

  // wait until app processes epoch and sends what's needed
  tokio::time::sleep(Duration::from_millis(500)).await;

  // check that app sent correct TXs to runtime
  let increment = b2a_runtime.receiver.recv().await;
  assert_eq!(
    Some((
      LogicalTimeAbsoluteMs(0),
      vec![
        TaskBlueprint {
          global_id: UniqueU64BlobId(0),
          fiber_type: FiberType::new("application"),
          function_key: "async_foo".to_string(),
          init_values: vec![Value::U64(1), Value::U64(1)]
        },
        TaskBlueprint {
          global_id: UniqueU64BlobId(1),
          fiber_type: FiberType::new("application"),
          function_key: "async_foo".to_string(),
          init_values: vec![Value::U64(1), Value::U64(1)]
        }
      ]
    )),
    increment
  );

  // imitate result from runtime
  b2a_runtime.send((UniqueU64BlobId(0), Value::U64(2)));
  b2a_runtime.send((UniqueU64BlobId(1), Value::U64(2)));

  let mut checked = false;

  while let Some(msg) = a2b_endpoint.receiver.recv().await {
    let Outbox::NotifyGWs(updated_txs) = msg else {
      continue;
    };

    assert_eq!(
      vec![
        TxUpdate { meta: Meta { id: UniqueU64BlobId(0), status: TxStatus::Finished }, result: Some(Value::U64(2)) },
        TxUpdate { meta: Meta { id: UniqueU64BlobId(1), status: TxStatus::Finished }, result: Some(Value::U64(2)) },
      ],
      updated_txs,
    );
    checked = true;
    break;
  }
  assert!(checked);
}
