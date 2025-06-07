use crate::app::{App, Params, Request, Response};
use crate::epoch_coordinator::etcd::EtcdEpochCoordinator;
use crate::epoch_coordinator::interface::{EpochRequest, EpochUpdates};
use crate::linearizer::LogLineriazer;
use crate::network::{Inbox, Outbox, P2P};
use common::duplex_channel::create_a_b_duplex_pair;
use common::invoker_handler::{InvokerInterface, create_invoker_handler_pair};

pub fn create_stack(
  node_urls: Vec<String>, self_url: String, params: Params,
) -> Result<(App<LogLineriazer>, InvokerInterface<Request, Response>), Box<dyn std::error::Error>> {
  let (a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Inbox, Outbox>();
  let (a2b_epoch, b2a_epoch) = create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();

  let epoch_coordinator = EtcdEpochCoordinator::new(&vec![], b2a_epoch);

  let mut p2p = P2P::new(node_urls, self_url, a2b_endpoint)?;
  let my_id = p2p.peer_id;

  _ = p2p.prepare()?;

  tokio::spawn(async move {
    p2p.start_event_loop().await;
  });
  tokio::spawn(async move {
    _ = epoch_coordinator.start().await;
  });

  let (state_invoker, state_handler) = create_invoker_handler_pair();

  Ok((App::<LogLineriazer>::new(my_id, b2a_endpoint, state_handler, a2b_epoch, params)?, state_invoker))
}
