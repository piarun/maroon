#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTimeAbsoluteMs(pub u64);

impl LogicalTimeAbsoluteMs {
  pub fn from_millis(ms: u64) -> Self {
    LogicalTimeAbsoluteMs(ms)
  }

  pub fn as_millis(&self) -> u64 {
    self.0
  }
}

impl std::fmt::Display for LogicalTimeAbsoluteMs {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{}ms", self.0)
  }
}

impl std::ops::Add for LogicalTimeAbsoluteMs {
  type Output = Self;

  fn add(
    self,
    rhs: LogicalTimeAbsoluteMs,
  ) -> Self {
    LogicalTimeAbsoluteMs(self.0 + rhs.0)
  }
}
