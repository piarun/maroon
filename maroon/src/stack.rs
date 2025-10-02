use crate::app::{App, Params, Request, Response};
use crate::linearizer::LogLineriazer;
use crate::network::{Inbox, Outbox, P2P};
use common::duplex_channel::create_a_b_duplex_pair;
use common::invoker_handler::{InvokerInterface, create_invoker_handler_pair};
use common::logical_clock::MonotonicTimer;
use common::logical_time::LogicalTimeAbsoluteMs;
use common::range_key::UniqueU64BlobId;
use epoch_coordinator::etcd::EtcdEpochCoordinator;
use epoch_coordinator::interface::{EpochRequest, EpochUpdates};
use libp2p::PeerId;
use log::{debug, info};
use runtime::generated::Value;
use runtime::ir_spec::sample_ir;
use runtime::runtime::{Runtime, TaskBlueprint};
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub struct MaroonStack {
  pub id: PeerId,
  p2p: P2P,
  epoch_coordinator: EtcdEpochCoordinator,
  app: App<LogLineriazer>,
  runtime: Runtime<MonotonicTimer>,
}

/// contains signals/interfaces to control/communicate with maroon stack
/// not sure if it's a good abstraction, maybe it should gone at some point
pub struct StackRemoteControl {
  /// allows to communicate with the instance of app::App
  /// ex: getting current state
  pub state_invoker: InvokerInterface<Request, Response>,
}

impl MaroonStack {
  pub fn new(
    node_urls: Vec<String>,
    etcd_urls: Vec<String>,
    self_url: String,
    params: Params,
  ) -> Result<(MaroonStack, StackRemoteControl), Box<dyn std::error::Error>> {
    let (a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
    let (a2b_epoch, b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();
    let (a2b_runtime, b2a_runtime) =
      create_a_b_duplex_pair::<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>();

    let epoch_coordinator = EtcdEpochCoordinator::new(&etcd_urls, b2a_epoch);

    let p2p = P2P::new(node_urls, self_url, a2b_endpoint)?;
    let id = p2p.peer_id;

    let (state_invoker, state_handler) = create_invoker_handler_pair();
    let app = App::<LogLineriazer>::new(id, b2a_endpoint, a2b_runtime, state_handler, a2b_epoch, params)?;

    // TODO: copy it to other components as well
    let timer = MonotonicTimer::new();

    let runtime = Runtime::new(timer.clone(), sample_ir(), b2a_runtime);

    Ok((MaroonStack { id, p2p, epoch_coordinator, app: app, runtime }, StackRemoteControl { state_invoker }))
  }

  /// starts listening and network operations in a separate tokio threads
  /// returns shutdown function
  pub fn start(self) -> impl FnOnce() {
    let (shutdown_tx, app_shutdown_rx) = oneshot::channel();

    let MaroonStack { id, mut p2p, epoch_coordinator, mut app, mut runtime } = self;
    _ = id;

    p2p.prepare().expect("if error occured - it won't work");

    tokio::spawn(async move {
      // TODO(akantsevoi): add shutdown signal here
      p2p.start_event_loop().await;
    });
    tokio::spawn(async move {
      // TODO(akantsevoi): add shutdown signal here
      if let Err(e) = epoch_coordinator.start().await {
        // TODO(akantsevoi): some errors are ok(ex: empty etcd nodes in a single node mode), but some are not ok
        // I need to differentiate these errors. Log some of them and panic on others
        debug!("epoch_coordinator_start: {e:?}");
      }
    });
    tokio::spawn(async move {
      app.loop_until_shutdown(app_shutdown_rx).await;
    });

    tokio::spawn(async move {
      runtime.run().await;
    });

    move || {
      if let Err(e) = shutdown_tx.send(()) {
        info!("app shutdown signal err: {e:?}");
      }
    }
  }
}
