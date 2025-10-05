use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LogicalTimeAbsoluteMs(pub u64);

impl LogicalTimeAbsoluteMs {
  pub fn from_millis(ms: u64) -> Self {
    LogicalTimeAbsoluteMs(ms)
  }

  pub fn as_millis(&self) -> u64 {
    self.0
  }

  /// ```rust
  /// use types::logical_time::LogicalTimeAbsoluteMs;
  ///
  /// assert_eq!(LogicalTimeAbsoluteMs::from_millis(10), LogicalTimeAbsoluteMs::from_millis(10).abs_diff(&LogicalTimeAbsoluteMs::from_millis(20)));
  /// assert_eq!(LogicalTimeAbsoluteMs::from_millis(10), LogicalTimeAbsoluteMs::from_millis(20).abs_diff(&LogicalTimeAbsoluteMs::from_millis(10)));
  /// assert_eq!(LogicalTimeAbsoluteMs::from_millis(0), LogicalTimeAbsoluteMs::from_millis(10).abs_diff(&LogicalTimeAbsoluteMs::from_millis(10)));
  ///
  /// ```
  pub fn abs_diff(
    &self,
    second: &LogicalTimeAbsoluteMs,
  ) -> LogicalTimeAbsoluteMs {
    if self.0 > second.0 {
      return LogicalTimeAbsoluteMs::from_millis(self.0 - second.0);
    } else {
      return LogicalTimeAbsoluteMs::from_millis(second.0 - self.0);
    }
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
