use crate::p2p::P2P;
use axum::extract::ws::{Message, WebSocket};
use common::duplex_channel::{Endpoint, create_a_b_duplex_pair};
use futures::SinkExt;
use log::info;
use protocol::gm_request_response::{Request, Response};
use protocol::node2gw::{Meta, Transaction, TxStatus};
use protocol::transaction::TaskBlueprint;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use types::range_key::{KeyRange, UniqueU64BlobId, full_interval_for_range};
pub struct Gateway {
  p2p_sender: UnboundedSender<Request>,
  p2p_receiver: Option<UnboundedReceiver<Response>>,
  p2p: Option<P2P>,

  interval_left: UniqueU64BlobId,
  interval_right: UniqueU64BlobId,

  // tx_id -> channel to the websocket writer
  ws_registry: Arc<Mutex<HashMap<UniqueU64BlobId, UnboundedSender<Message>>>>,
}

impl Gateway {
  pub fn new(
    range: KeyRange,
    node_urls: Vec<String>,
  ) -> Result<Gateway, Box<dyn std::error::Error>> {
    let (a2b_endpoint, b2a_endpoint) = create_a_b_duplex_pair::<Request, Response>();

    let mut p2p = P2P::new(node_urls, b2a_endpoint)?;
    // TODO: prepare works in background and you can't start sending requests immediately when you created Gateway
    // I need to create some sort of state/flags or block the thread that can prevent sending requests before initialization even happened
    p2p.prepare().map_err(|e| format!("prepare: {}", e))?;

    let interval = full_interval_for_range(range);

    Ok(Gateway {
      p2p_sender: a2b_endpoint.sender,
      p2p_receiver: Some(a2b_endpoint.receiver),
      p2p: Some(p2p),
      interval_left: interval.start(),
      interval_right: interval.end(),
      ws_registry: Arc::new(Mutex::new(HashMap::new())),
    })
  }

  pub async fn start_in_background(&mut self) {
    let p2p = self.p2p.take().expect("can be called only once");

    let mut receiver = self.p2p_receiver.take().expect("cant take twice");

    tokio::spawn(async move {
      p2p.start_event_loop().await;
    });

    let ws_registry = self.ws_registry.clone();
    tokio::spawn(async move {
      while let Some(msg) = receiver.recv().await {
        match msg {
          Response::Node2GWTxUpdate(tx_updates) => {
            for update in tx_updates {
              // try to forward update to a registered websocket for this tx_id
              let maybe_tx = {
                let mut map = ws_registry.lock().await;
                map.remove_entry(&update.meta.id).map(|e| e.1)
              };
              if let Some(tx) = maybe_tx {
                let payload = serde_json::to_string(&update).unwrap_or_else(|_| format!("{:?}", update));
                let _ = tx.send(Message::Text(payload.into()));
              }
            }
          }
          _ => {}
        }
      }
    });
  }

  pub async fn send_request(
    &mut self,
    blueprint: TaskBlueprint,
    mut response_socket: WebSocket,
  ) {
    if self.interval_left >= self.interval_right {
      todo!("request new key range");
    }

    let id = self.interval_left;
    self.interval_left += UniqueU64BlobId(1);

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    {
      let mut map = self.ws_registry.lock().await;
      map.insert(id, tx);
    }

    tokio::spawn(async move {
      while let Some(msg) = rx.recv().await {
        if response_socket.send(msg).await.is_err() {
          break;
        }
      }
      _ = response_socket.close().await;
    });

    let _ = self
      .p2p_sender
      .send(Request::NewTransaction(Transaction { meta: Meta { id, status: TxStatus::Created }, blueprint }));

    if let Some(sender) = self.ws_registry.lock().await.get(&id).cloned() {
      let _ = sender.send(Message::Text(format!("request created. id: {}", id).into()));
    }
  }
}
