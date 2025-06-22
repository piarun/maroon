use crate::app::{App, CurrentOffsets, Params, Request as AppStateRequest, Response as AppStateResponse};
use crate::linearizer::LogLineriazer;
use crate::network::{Inbox, Outbox};
use common::invoker_handler::HandlerInterface;
use common::invoker_handler::InvokerInterface;
use common::{
  duplex_channel::Endpoint,
  range_key::UniqueU64BlobId,
  transaction::{Transaction, TxStatus},
};
#[cfg(test)]
use epoch_coordinator::interface::A2BEndpoint;
use libp2p::PeerId;
use std::time::Duration;

#[cfg(test)]
pub fn new_test_instance(
  p2p_interface: Endpoint<Outbox, Inbox>,
  state_interface: HandlerInterface<AppStateRequest, AppStateResponse>,
  epoch_coordinator: A2BEndpoint,
) -> App<LogLineriazer> {
  App::<LogLineriazer>::new(PeerId::random(), p2p_interface, state_interface, epoch_coordinator, Params::default())
    .expect("failed to create test App instance")
}

pub fn new_test_instance_with_params(
  p2p_interface: Endpoint<Outbox, Inbox>,
  state_interface: HandlerInterface<AppStateRequest, AppStateResponse>,
  epoch_coordinator: A2BEndpoint,
  params: Params,
) -> App<LogLineriazer> {
  App::<LogLineriazer>::new(PeerId::random(), p2p_interface, state_interface, epoch_coordinator, params)
    .expect("failed to create test App instance")
}

#[cfg(test)]
pub fn test_tx(id: u64) -> Transaction {
  Transaction { id: UniqueU64BlobId(id), status: TxStatus::Pending }
}

#[cfg(test)]
pub async fn reaches_state(
  attempts: u32,
  tick: Duration,
  state_invoker: &InvokerInterface<AppStateRequest, AppStateResponse>,
  exp_state: CurrentOffsets,
) -> bool {
  for _ in 0..attempts {
    let AppStateResponse::State(current_state) = state_invoker.request(AppStateRequest::GetState).await;

    if exp_state == current_state {
      return true;
    }

    tokio::time::sleep(tick).await;
  }

  return false;
}
