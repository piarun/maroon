use std::time::Duration;
use tokio;

pub async fn retry<F: FnMut() -> bool>(
  intervals: Vec<Duration>,
  mut f: F,
) {
  for i in intervals {
    if f() {
      break;
    }

    tokio::time::sleep(i).await;
  }
}

pub fn const_intervals(
  count: usize,
  interval: Duration,
) -> Vec<Duration> {
  vec![interval; count]
}

pub fn exp_intervals(
  count: usize,
  start_interval: Duration,
) -> Vec<Duration> {
  (0..count).map(|i| start_interval * 2_u32.pow(i as u32)).collect()
}
