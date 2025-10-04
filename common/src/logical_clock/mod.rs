use types::logical_time::LogicalTimeAbsoluteMs;

pub trait Timer: Send + Sync + 'static {
  fn from_start(&self) -> LogicalTimeAbsoluteMs;
  fn monotonic_now_system(&self) -> std::time::SystemTime;
}

#[derive(Clone, Copy)]
pub struct MonotonicTimer {
  instant: std::time::Instant,
  system: std::time::SystemTime,
}

impl MonotonicTimer {
  pub fn new() -> MonotonicTimer {
    MonotonicTimer { instant: std::time::Instant::now(), system: std::time::SystemTime::now() }
  }

  // create a timer that already has `elapsed` time
  #[cfg(test)]
  pub fn with_elapsed(elapsed: std::time::Duration) -> Self {
    let now_instant = std::time::Instant::now();
    let now_system = std::time::SystemTime::now();

    let instant = now_instant.checked_sub(elapsed).unwrap_or(now_instant);
    let system = now_system.checked_sub(elapsed).unwrap_or(now_system);

    MonotonicTimer { instant, system }
  }

  #[cfg(test)]
  pub fn with_elapsed_ms(ms: u64) -> Self {
    MonotonicTimer::with_elapsed(std::time::Duration::from_millis(ms))
  }
}

impl Timer for MonotonicTimer {
  fn from_start(&self) -> LogicalTimeAbsoluteMs {
    LogicalTimeAbsoluteMs(self.instant.elapsed().as_millis() as u64)
  }

  fn monotonic_now_system(&self) -> std::time::SystemTime {
    self.system + self.instant.elapsed()
  }
}

pub mod test_helpers;
