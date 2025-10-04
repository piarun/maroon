use libp2p::swarm::StreamProtocol;
use libp2p_request_response::{
  self as request_response, Event as RequestResponseEvent, ProtocolSupport,
  json::{self},
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use types::range_key::U64BlobIdClosedInterval;
use crate::transaction::Transaction;

pub type Event = RequestResponseEvent<Request, Response>;
pub type Behaviour = json::Behaviour<Request, Response>;

pub fn create_behaviour() -> json::Behaviour<Request, Response> {
  json::Behaviour::<Request, Response>::new(
    [(StreamProtocol::new("/maroon/p2p_direct/1.0.0"), ProtocolSupport::Full)],
    request_response::Config::default(),
  )
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Request {
  /// request missing transactions for given ranges
  GetMissingTx(Vec<U64BlobIdClosedInterval>),

  /// sends missing transactions
  MissingTx(Vec<Transaction>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Response {
  Ack,
}

