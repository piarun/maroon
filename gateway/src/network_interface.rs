use protocol::node2gw::{Transaction, TxUpdate};

/// Input for p2p layer from higher modules perspective
#[derive(Debug, Clone)]
pub enum Outbox {
  NewTransaction(Transaction),
}

/// Input for the layer that lives on top of p2p layer. Output for p2p Layer
#[derive(Debug, Clone)]
pub enum Inbox {
  TxUpdates(Vec<TxUpdate>),
}
