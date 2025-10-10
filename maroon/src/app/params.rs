use std::{num::NonZeroUsize, time::Duration};

use common::logical_time::LogicalTimeAbsoluteMs;

#[derive(Clone, Copy, Debug)]
pub struct Params {
  /// how often node will send state info to other nodes
  /// consensus offset is recalculated on this tick
  pub advertise_period: std::time::Duration,
  /// minimum amount of nodes that should have the same transactions(+ current one) in order to confirm them
  /// TODO: separate pub struct ConsensusAlgoParams in a separate lib/consensus crate with its own test suite?
  pub consensus_nodes: NonZeroUsize,

  /// periods between epochs <br>
  /// this parameter only says **when** you should start a new epoch <br>
  /// however due to multiple reasons a new epoch might not start after this period
  pub epoch_period: LogicalTimeAbsoluteMs,
}

impl Params {
  pub fn default() -> Params {
    Params {
      advertise_period: Duration::from_millis(50), // 20Hz :)
      consensus_nodes: NonZeroUsize::new(2).unwrap(),
      epoch_period: LogicalTimeAbsoluteMs::from_millis(50),
    }
  }

  pub fn set_advertise_period(
    mut self,
    new_period: Duration,
  ) -> Params {
    self.advertise_period = new_period;
    self
  }

  pub fn set_consensus_nodes(
    mut self,
    n_consensus: NonZeroUsize,
  ) -> Params {
    self.consensus_nodes = n_consensus;
    self
  }

  pub fn set_epoch_period(
    mut self,
    new_period: LogicalTimeAbsoluteMs,
  ) -> Params {
    self.epoch_period = new_period;
    self
  }
}
