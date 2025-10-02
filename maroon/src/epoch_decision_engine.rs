use common::logical_clock::Timer;
use common::logical_time::LogicalTimeAbsoluteMs;
use libp2p::PeerId;
use log::debug;
use std::{collections::HashSet, usize};

/// takes signals(node updates, timer ticks, latest epoch time, etc) and makes decision if it's time to send new epoch or not
pub struct EpochDecisionEngine<T: Timer> {
  id: PeerId,
  nodes: Vec<PeerId>, // sorted

  /// periods between epochs <br>
  /// this parameter only says **when** you should start a new epoch <br>
  /// however due to multiple reasons a new epoch might not start exactly after this period
  tick_delta: LogicalTimeAbsoluteMs,

  // latest peer_id that commited and timestamp in milis, when it happened
  latest_epoch: Option<(PeerId, LogicalTimeAbsoluteMs)>,

  timer: T,
}

pub fn new_decider(
  id: PeerId,
  tick_delta: LogicalTimeAbsoluteMs,
) -> EpochDecisionEngine<common::logical_clock::MonotonicTimer> {
  EpochDecisionEngine::<common::logical_clock::MonotonicTimer>::new(
    id,
    tick_delta,
    common::logical_clock::MonotonicTimer::new(),
  )
}

impl<T: Timer> EpochDecisionEngine<T> {
  pub fn new(
    id: PeerId,
    tick_delta: LogicalTimeAbsoluteMs,
    timer: T,
  ) -> EpochDecisionEngine<T> {
    EpochDecisionEngine { id, nodes: vec![], latest_epoch: None, tick_delta: tick_delta, timer }
  }

  pub fn update_node_ids(
    &mut self,
    ids: &HashSet<PeerId>,
  ) {
    self.nodes = ids.iter().copied().collect();
    self.nodes.sort();
  }

  pub fn update_latest_epoch(
    &mut self,
    commiter_id: PeerId,
    commit_time: LogicalTimeAbsoluteMs,
  ) {
    self.latest_epoch = Some((commiter_id, commit_time))
  }

  pub fn should_send(&self) -> bool {
    let start_time = self.latest_epoch.map(|x| x.1).unwrap_or(LogicalTimeAbsoluteMs(0));
    let latest_commiter_id = self.latest_epoch.map(|x| x.0);

    let position = calculate_position(&self.nodes, self.id, latest_commiter_id);
    let delta_time = LogicalTimeAbsoluteMs::from_millis((position as u64 + 1) * self.tick_delta.as_millis());

    let next_publish_timestamp = start_time + delta_time;

    let current_timestamp = self.timer.from_start();

    let distance = next_publish_timestamp.abs_diff(&current_timestamp);
    let should = current_timestamp > next_publish_timestamp;
    let count = self.nodes.len();

    debug!("[{position} of {count}:{should}:{distance}] cur: {current_timestamp} next: {next_publish_timestamp}");
    should
  }
}

/// represents ordered `nodes` as a ring and finds an offset of self_id from latest_commiter_id
fn calculate_position(
  nodes: &Vec<PeerId>,
  self_id: PeerId,
  latest_commiter_id: Option<PeerId>,
) -> usize {
  let latest_commiter_id: PeerId = latest_commiter_id.unwrap_or(self_id);

  let mut self_position = 0;
  let mut last_commiter_position = 0;
  let total = nodes.len();

  for (pos, id) in nodes.iter().enumerate() {
    if *id == latest_commiter_id {
      last_commiter_position = pos;
    }

    if *id == self_id {
      self_position = pos;
    }
  }

  if self_position == last_commiter_position {
    return self_position;
  } else {
    return (total - last_commiter_position + self_position) % total;
  }
}

#[cfg(test)]
mod tests {
  use crate::epoch_decision_engine::calculate_position;

  use super::*;
  use common::logical_clock::test_helpers::MockTimer;
  use std::time::Duration;

  fn new_test(
    id: PeerId,
    tick_delta: LogicalTimeAbsoluteMs,
    now_ms: Duration,
  ) -> EpochDecisionEngine<MockTimer> {
    use common::logical_time::LogicalTimeAbsoluteMs;
    EpochDecisionEngine::new(
      id,
      tick_delta,
      MockTimer::new(LogicalTimeAbsoluteMs::from_millis(now_ms.as_millis() as u64)),
    )
  }

  #[test]
  fn test_calculate_position() {
    let mut p_ids = ordered_peer_ids(5);

    let peer_id_1 = p_ids.remove(0);
    let peer_id_2 = p_ids.remove(0);
    let peer_id_3 = p_ids.remove(0);
    let peer_id_4 = p_ids.remove(0);
    let peer_id_5 = p_ids.remove(0);

    struct Case {
      nodes: Vec<PeerId>,
      self_id: PeerId,
      latest_id: Option<PeerId>,
      exp_position: usize,
    }

    let cases = [
      Case {
        nodes: vec![peer_id_1.clone(), peer_id_2.clone()],
        self_id: peer_id_2.clone(),
        latest_id: None,
        exp_position: 1,
      },
      Case {
        nodes: vec![peer_id_1.clone(), peer_id_2.clone()],
        self_id: peer_id_2.clone(),
        latest_id: Some(peer_id_2.clone()),
        exp_position: 1,
      },
      Case {
        nodes: vec![peer_id_1.clone(), peer_id_2.clone()],
        self_id: peer_id_2.clone(),
        latest_id: Some(peer_id_5.clone()), // latest publisher is unknown => ignored in the order
        exp_position: 1,
      },
      Case {
        nodes: vec![peer_id_1.clone(), peer_id_2.clone(), peer_id_3.clone(), peer_id_4.clone()],
        self_id: peer_id_1.clone(),
        latest_id: Some(peer_id_3.clone()),
        exp_position: 2,
      },
      Case {
        nodes: vec![peer_id_1.clone(), peer_id_2.clone(), peer_id_3.clone(), peer_id_4.clone()],
        self_id: peer_id_3.clone(),
        latest_id: Some(peer_id_2.clone()),
        exp_position: 1,
      },
    ];

    for (i, c) in cases.iter().enumerate() {
      assert_eq!(c.exp_position, calculate_position(&c.nodes, c.self_id, c.latest_id), "case index: {i}");
    }
  }

  #[test]
  fn test_decider() {
    let mut p_ids = ordered_peer_ids(4);

    let p_id_1 = p_ids.remove(0);
    let p_id_2 = p_ids.remove(0);
    let p_id_3 = p_ids.remove(0);
    let p_id_4 = p_ids.remove(0);

    let mut decider = new_test(p_id_2, LogicalTimeAbsoluteMs::from_millis(60), Duration::from_millis(0));
    assert!(!decider.should_send()); // by default position is far: u128::MAX

    decider.timer.advance(common::logical_time::LogicalTimeAbsoluteMs::from_millis(70));
    assert!(decider.should_send()); // time to publish, even though there is no info about nodes

    decider.update_node_ids(&HashSet::from([p_id_2]));
    assert!(decider.should_send()); // only itself, nothing published, time is right

    decider.update_node_ids(&HashSet::from([p_id_2, p_id_1]));
    assert!(!decider.should_send()); // self is second, nothing published, time is not yet because it's on second position now

    decider.timer.advance(common::logical_time::LogicalTimeAbsoluteMs::from_millis(60));
    assert!(decider.should_send()); // time ticked, p_id_1 havent published, time to publish for p_id_2

    decider.update_latest_epoch(p_id_1, LogicalTimeAbsoluteMs::from_millis(60));
    assert!(!decider.should_send()); // imitate p_id_1 published so time for the p_id_2 publish moved

    decider.update_node_ids(&HashSet::from([p_id_2, p_id_3, p_id_4]));
    assert!(decider.should_send()); // update nodes, p_id_2 is first again
  }

  fn ordered_peer_ids(n: usize) -> Vec<PeerId> {
    let mut p_ids = Vec::<PeerId>::new();
    p_ids.resize_with(n, PeerId::random);
    p_ids.sort();

    p_ids
  }
}
