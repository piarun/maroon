use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Cid(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Gid(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Eid(pub u64);

impl Cid {
  pub fn new_random() -> Self {
    let mut n = rand::random::<u64>() % 1_000_000_000_000_000;
    n += 1_000_000_000_000_000;
    Cid(n)
  }
}

impl Gid {
  pub fn new_random() -> Self {
    let mut n = rand::random::<u64>() % 1_000_000_000_000_000;
    n += 2_000_000_000_000_000;
    Gid(n)
  }
}

impl Eid {
  pub fn new_random() -> Self {
    let mut n = rand::random::<u64>() % 1_000_000_000_000_000;
    n += 9_000_000_000_000_000;
    Eid(n)
  }
}

impl fmt::Display for Cid {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:015}", self.0)
  }
}
impl fmt::Display for Gid {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:015}", self.0)
  }
}
impl fmt::Display for Eid {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:015}", self.0)
  }
}

pub mod log_events {
  use super::*;

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum CommandBody {
    TextMessageCommand(String),
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum LogEventBody {
    ClientConnected { cid: Cid },
    ClientDisconnected { cid: Cid },
    GatewayUp { gid: Gid },
    GatewayDown { gid: Gid },
    ClientSentCommand { cid: Cid, eid: Eid, gid: Gid, body: CommandBody },
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct LogEvent {
    pub timestamp_micros: u64,
    pub body: LogEventBody,
  }
}
