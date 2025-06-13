use common::clock::{Clock, SystemClock};
use libp2p::PeerId;
use std::{collections::HashSet, time::Duration, u128, usize};

/// takes signals(node updates, timer ticks, latest epoch time, etc) and makes decision if it's time to send new epoch or not
pub struct Decider<C: Clock> {
  id: PeerId,
  nodes: Vec<PeerId>, // sorted

  tick_delta: u128,

  // latest peer_id that commited and timestamp in milis, when it happened
  latest_epoch: Option<(PeerId, u128)>,

  clock: C,
}

pub fn new_decider(id: PeerId, tick_delta: Duration) -> Decider<SystemClock> {
  Decider::<SystemClock>::new(id, tick_delta, SystemClock {})
}

impl<C: Clock> Decider<C> {
  pub fn new(id: PeerId, tick_delta: Duration, clock: C) -> Decider<C> {
    Decider { id, nodes: vec![], latest_epoch: None, tick_delta: tick_delta.as_millis(), clock }
  }

  pub fn update_node_ids(&mut self, ids: &HashSet<PeerId>) {
    self.nodes = ids.iter().copied().collect();
    self.nodes.sort();
  }

  pub fn update_latest_epoch(&mut self, commiter_id: PeerId, commit_time: Duration) {
    self.latest_epoch = Some((commiter_id, commit_time.as_millis()))
  }

  pub fn should_send(&self) -> bool {
    let next_publish_timestamp: u128 = match self.latest_epoch {
      Some((latest_commiter, latest_commited_time)) => {
        let position = calculate_position(&self.nodes, self.id, Some(latest_commiter)) as u128;

        latest_commited_time + (position + 1) * self.tick_delta
      }
      None => {
        let position = calculate_position(&self.nodes, self.id, None) as u128;

        (position + 1) * self.tick_delta
      }
    };

    let current_timestamp = self.clock.now().as_millis();

    println!("{current_timestamp}, {next_publish_timestamp}");
    current_timestamp > next_publish_timestamp
  }
}

/// represents ordered `nodes` as a ring and finds an offset of self_id from latest_commiter_id
fn calculate_position(nodes: &Vec<PeerId>, self_id: PeerId, latest_commiter_id: Option<PeerId>) -> usize {
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
  use crate::app::decider::calculate_position;

  use super::*;
  use common::clock::test_helpers::MockClock;

  fn new_test(id: PeerId, tick_delta: Duration, now: Duration) -> Decider<MockClock> {
    Decider::new(id, tick_delta, MockClock::new(now))
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

    let mut decider = new_test(p_id_2, Duration::from_millis(60), Duration::from_secs(0));
    assert!(!decider.should_send()); // by default position is far: u128::MAX

    decider.clock.advance(Duration::from_millis(70));
    assert!(decider.should_send()); // time to publish, even though there is no info about nodes

    decider.update_node_ids(&HashSet::from([p_id_2]));
    assert!(decider.should_send()); // only itself, nothing published, time is right

    decider.update_node_ids(&HashSet::from([p_id_2, p_id_1]));
    assert!(!decider.should_send()); // self is second, nothing published, time is not yet because it's on second position now

    decider.clock.advance(Duration::from_millis(60));
    assert!(decider.should_send()); // time ticked, p_id_1 havent published, time to publish for p_id_2

    decider.update_latest_epoch(p_id_1, Duration::from_millis(60));
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
