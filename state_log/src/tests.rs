use core::time;
use r2d2_redis::redis::Commands;
use r2d2_redis::redis::streams::StreamRangeReply;
use r2d2_redis::{RedisConnectionManager, r2d2::Pool};
use testcontainers::{
  GenericImage, ImageExt,
  core::{IntoContainerPort, WaitFor},
  runners::AsyncRunner,
};
use tokio;

use crate::logger::Sender;
use crate::logger::log;
use crate::logger::log_event_sender;

#[tokio::test(flavor = "multi_thread")]
async fn test_redis() {
  let container = GenericImage::new("redis", "8.0.2-alpine")
    .with_exposed_port(6379.tcp())
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .with_network("bridge")
    .with_env_var("DEBUG", "1")
    .start()
    .await
    .expect("Failed to start Redis");

  let port = container.get_host_port_ipv4(6379).await.unwrap();
  let redis_url = format!("redis://127.0.0.1:{}", port);

  _ = log_event_sender(Some(Sender::new(redis_url.clone())));

  log(schema::mn_events::LogEvent {
    timestamp_micros: 2131,
    emitter: libp2p::PeerId::random(),
    body: schema::mn_events::LogEventBody::MaroonNodeDown,
  });

  let mut counter = 3;
  let mut saved = false;
  while counter > 0 {
    let manager = RedisConnectionManager::new(redis_url.clone()).unwrap();
    let pool = Pool::builder().build(manager).unwrap();
    let mut conn = pool.get().unwrap();

    let result: StreamRangeReply = conn.xrange("state_log_stream", "-", "+").unwrap();
    counter -= 1;

    if result.ids.len() == 1 {
      saved = true;
      break;
    }
    tokio::time::sleep(time::Duration::from_millis(50)).await;
  }

  assert!(saved);
}
