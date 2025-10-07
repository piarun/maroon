use crate::network_interface::{Inbox, Outbox};
use crate::p2p::P2P;
use axum::extract::ws::{Message, WebSocket};
use common::duplex_channel::create_a_b_duplex_pair;
use generated::maroon_assembler::Value;
use log::error;
use protocol::node2gw::{Meta, Transaction, TxStatus};
use protocol::transaction::{FiberType, TaskBlueprint};
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::{
  broadcast,
  mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use types::range_key::{KeyRange, UniqueU64BlobId, full_interval_for_range};

#[derive(Debug, Clone, Serialize)]
pub enum MonitorEvent {
  NewRequest { id: UniqueU64BlobId, fiber_type: FiberType, function_key: String, init_values: Vec<Value> },
  TxUpdate { meta: Meta, result: Option<Value> },
}

// if the last param is None - the request will still go and will be executed
// it's just result is not interesting for the requester
struct NewRequest {
  id: UniqueU64BlobId,
  blueprint: TaskBlueprint,
  response_socket: Option<WebSocket>,
}

pub struct Gateway {
  p2p_sender: Option<UnboundedSender<Outbox>>,
  p2p_receiver: Option<UnboundedReceiver<Inbox>>,
  p2p: Option<P2P>,

  new_request_sender: UnboundedSender<NewRequest>,
  new_request_receiver: Option<UnboundedReceiver<NewRequest>>,

  interval_left: UniqueU64BlobId,
  interval_right: UniqueU64BlobId,

  monitor_tx: broadcast::Sender<MonitorEvent>,
}

impl Gateway {
  pub fn new(
    range: KeyRange,
    node_urls: Vec<String>,
  ) -> Result<Gateway, Box<dyn std::error::Error>> {
    let (a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Outbox, Inbox>();
    let (new_request_sender, new_request_receiver) = mpsc::unbounded_channel::<NewRequest>();
    let (monitor_tx, _monitor_rx) = broadcast::channel::<MonitorEvent>(1024);

    let mut p2p = P2P::new(node_urls, b2a_endpoint)?;
    // TODO: prepare works in background and you can't start sending requests immediately when you created Gateway
    // I need to create some sort of state/flags or block the thread that can prevent sending requests before initialization even happened
    p2p.prepare().map_err(|e| format!("prepare: {}", e))?;

    let interval = full_interval_for_range(range);

    Ok(Gateway {
      p2p_sender: Some(a2b_endpoint.sender),
      p2p_receiver: Some(a2b_endpoint.receiver),
      p2p: Some(p2p),
      new_request_sender,
      new_request_receiver: Some(new_request_receiver),
      interval_left: interval.start(),
      interval_right: interval.end(),
      monitor_tx,
    })
  }

  pub fn monitor_subscribe(&self) -> broadcast::Receiver<MonitorEvent> {
    self.monitor_tx.subscribe()
  }

  pub async fn start_in_background(&mut self) {
    let p2p = self.p2p.take().expect("can be called only once");

    let mut p2p_receiver = self.p2p_receiver.take().expect("cant take twice");
    let p2p_sender = self.p2p_sender.take().expect("cant take twice");
    let mut new_request_receiver = self.new_request_receiver.take().expect("cant take twice");
    let monitor_tx = self.monitor_tx.clone();

    tokio::spawn(async move {
      p2p.start_event_loop().await;
    });

    let mut ws_registry = HashMap::<UniqueU64BlobId, WebSocket>::new();
    tokio::spawn(async move {
      // let p2p_sender = p2p_sender;
      loop {
        tokio::select! {
          Some(inbox) = p2p_receiver.recv() => {
            handle_inbox(inbox, &mut ws_registry, &monitor_tx).await;
          }
          Some(req) = new_request_receiver.recv() => {
            handle_send_new_request(&p2p_sender, req, &mut ws_registry, &monitor_tx);
          }
        }
      }
    });
  }

  pub async fn send_request(
    &mut self,
    blueprint: TaskBlueprint,
    response_socket: Option<WebSocket>,
  ) {
    if self.interval_left >= self.interval_right {
      todo!("request new key range");
    }

    let id = self.interval_left;
    self.interval_left += UniqueU64BlobId(1);

    if let Err(e) = self.new_request_sender.send(NewRequest { id, blueprint, response_socket }) {
      error!("gateway new request: {e}");
    }
  }
}

fn handle_send_new_request(
  sender: &UnboundedSender<Outbox>,
  request: NewRequest,
  ws_registry: &mut HashMap<UniqueU64BlobId, WebSocket>,
  monitor_tx: &broadcast::Sender<MonitorEvent>,
) {
  let NewRequest { id, blueprint, response_socket } = request;
  let bp_for_monitor = blueprint.clone();
  let _ = sender.send(Outbox::NewTransaction(Transaction { meta: Meta { id, status: TxStatus::Created }, blueprint }));
  if let Some(sock) = response_socket {
    ws_registry.insert(id, sock);
  }

  let _ = monitor_tx.send(MonitorEvent::NewRequest {
    id,
    fiber_type: bp_for_monitor.fiber_type,
    function_key: bp_for_monitor.function_key,
    init_values: bp_for_monitor.init_values,
  });
}

async fn handle_inbox(
  inbox: Inbox,
  ws_registry: &mut HashMap<UniqueU64BlobId, WebSocket>,
  monitor_tx: &broadcast::Sender<MonitorEvent>,
) {
  match inbox {
    Inbox::TxUpdates(tx_updates) => {
      for update in tx_updates {
        let socket = ws_registry.get_mut(&update.meta.id);
        if let Some(socket) = socket {
          let payload = serde_json::to_string(&update).unwrap_or_else(|_| format!("{:?}", update));
          if let Err(e) = socket.send(Message::Text(payload.into())).await {
            error!("send ws response: {e}");
          };
          if update.meta.status == TxStatus::Finished {
            ws_registry.remove(&update.meta.id);
          }
        }

        let _ = monitor_tx.send(MonitorEvent::TxUpdate { meta: update.meta, result: update.result });
      }
    }
  }
}
