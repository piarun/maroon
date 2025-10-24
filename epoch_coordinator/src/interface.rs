use super::epoch::Epoch;
use std::fmt;
use tokio::sync::{
  mpsc::{UnboundedReceiver, UnboundedSender},
  watch::{Receiver, Sender},
};

// TODO: should I download epoch history through this coordinator?
// For example node was offline or
// if yes - add an interface
// if no - where to download it? p2p? s3?

// a pair interface to ControllerInterface. Is used in epoch coordinator itself
pub struct Interface {
  pub receiver: Receiver<Option<EpochRequest>>,
  pub sender: UnboundedSender<EpochUpdates>,
}

// a pair interface to Interface. Is used by some other components that want to communicate with epoch coordinator
pub struct ControllerInterface {
  pub receiver: UnboundedReceiver<EpochUpdates>,
  pub sender: Sender<Option<EpochRequest>>,
}

pub fn create_interface_pair() -> (Interface, ControllerInterface) {
  let (ec_tx, ec_rx) = tokio::sync::watch::channel::<Option<EpochRequest>>(None);
  let (ec_tx_u, ec_rx_u) = tokio::sync::mpsc::unbounded_channel::<EpochUpdates>();

  (Interface { receiver: ec_rx, sender: ec_tx_u }, ControllerInterface { receiver: ec_rx_u, sender: ec_tx })
}

#[derive(Debug, Clone)]
pub struct EpochRequest {
  pub epoch: Epoch,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EpochUpdates {
  /// when a new epoch detected
  New(Epoch),
}

#[derive(Debug)]
pub enum CommitError {
  CommitFailed(String),
}

impl fmt::Display for CommitError {
  fn fmt(
    &self,
    f: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    match self {
      CommitError::CommitFailed(msg) => write!(f, "Failed to commit epoch: {}", msg),
    }
  }
}
