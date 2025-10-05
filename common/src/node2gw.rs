use crate::transaction::Transaction;
use libp2p::{
  PeerId,
  gossipsub::{Sha256Topic, TopicHash},
};
use serde::{Deserialize, Serialize};

pub const NODE2GW_TOPIC_NAME: &str = "node-2-gw-broadcast";

pub fn node2gw_topic() -> Sha256Topic {
  Sha256Topic::new(NODE2GW_TOPIC_NAME)
}

pub fn node2gw_topic_hash() -> TopicHash {
  node2gw_topic().hash().clone()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GossipMessage {
  pub peer_id: PeerId,

  // This is the only information in that enum that node can gossip to gateways
  pub payload: GossipPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GossipPayload {
  Node2GWTxUpdate(Vec<Transaction>),
}
