use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub trait Clock {
  fn now(&self) -> std::time::Duration;
}

pub struct SystemClock;

impl SystemClock {
  pub fn new() -> SystemClock {
    SystemClock {}
  }
}

impl Clock for SystemClock {
  fn now(&self) -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("not negative")
  }
}

pub mod test_helpers;
