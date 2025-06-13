use std::time::Duration;

use crate::clock::Clock;

pub struct MockClock {
  time: Duration,
}

impl MockClock {
  pub fn new(now: Duration) -> MockClock {
    MockClock { time: now }
  }
  pub fn advance(&mut self, d: Duration) {
    self.time += d;
  }

  pub fn set(&mut self, t: Duration) {
    self.time = t
  }
}

impl Clock for MockClock {
  fn now(&self) -> Duration {
    self.time
  }
}
