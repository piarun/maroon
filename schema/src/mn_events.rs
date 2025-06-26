use std::time::{SystemTime, UNIX_EPOCH};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};

pub type Gid = PeerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Eid(pub u64);

impl Eid {
  pub fn new_random() -> Self {
    let mut n = rand::random::<u64>() % 1_000_000_000_000_000;
    n += 9_000_000_000_000_000;
    Eid(n)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandBody {
  TextMessageCommand(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEventBody {
  GatewayConnected { gid: Gid },
  GatewayDisconnected { gid: Gid },
  MaroonNodeUp,
  MaroonNodeDown,
  GatewaySentCommand { eid: Eid, mnid: PeerId, body: CommandBody },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
  pub timestamp_micros: u128,
  pub emitter: PeerId, // the author of event
  pub body: LogEventBody,
}

pub fn now_microsec() -> u128 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros()
}
