use log::error;
use r2d2_redis::redis::{Commands, RedisResult};
use r2d2_redis::{RedisConnectionManager, r2d2::Pool};
use schema::log_events::LogEvent;
use serde_json;
use std::sync::OnceLock;
use tokio;
use tokio::sync::mpsc::{self, UnboundedSender};

const STATE_LOG_STREAM: &str = "state_log_stream";

pub fn log(event: LogEvent) {
  if let Err(e) = log_event_sender().send(event) {
    error!("redis pipe error: {e}");
  };
}

fn log_event_sender() -> &'static UnboundedSender<LogEvent> {
  static COUNTER: OnceLock<UnboundedSender<LogEvent>> = OnceLock::new();
  COUNTER.get_or_init(|| {
    let (sender, mut receiver) = mpsc::unbounded_channel::<LogEvent>();
    tokio::spawn(async move {
      // TODO: handle unwraps somehow differently?
      // because redis should be opt in

      let redis_url: String = std::env::var("REDIS_URL").map_err(|e| format!("REDIS_URL not set: {}", e)).unwrap();

      let manager = RedisConnectionManager::new(redis_url).unwrap();
      let pool = Pool::builder().build(manager).unwrap();
      let mut conn = pool.get().unwrap();

      while let Some(event) = receiver.recv().await {
        let pairs = &[("event", serde_json::to_string(&event).unwrap())];
        let res: RedisResult<()> = conn.xadd(STATE_LOG_STREAM, "*", pairs);
        if let Err(e) = res {
          error!("stream push to redis: {e}");
        }
      }
    });
    sender
  })
}

// testing in front of redis

// #[tokio::test(flavor = "multi_thread")]
// async fn redis_connection_pool_test() {
//   use schema::{Cid, log_events::LogEventBody};

//   log_state_event(LogEvent {
//     timestamp_micros: 3312312,
//     body: LogEventBody::ClientConnected { cid: Cid(13512312312) },
//   });
//   log_state_event(LogEvent {
//     timestamp_micros: 3312312,
//     body: LogEventBody::ClientDisconnected { cid: Cid(13512312312) },
//   });
// }
