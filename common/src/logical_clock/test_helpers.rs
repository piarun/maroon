use super::Timer;
use crate::logical_time::LogicalTimeAbsoluteMs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct MockTimer {
  time: LogicalTimeAbsoluteMs,
}

impl MockTimer {
  pub fn new(now: LogicalTimeAbsoluteMs) -> MockTimer {
    MockTimer { time: now }
  }
  pub fn advance(
    &mut self,
    d: LogicalTimeAbsoluteMs,
  ) {
    self.time = self.time + d;
  }

  pub fn set(
    &mut self,
    t: LogicalTimeAbsoluteMs,
  ) {
    self.time = t
  }
}

impl Timer for MockTimer {
  fn from_start(&self) -> LogicalTimeAbsoluteMs {
    self.time
  }
  fn monotonic_now_system(&self) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(self.time.as_millis())
  }
}
