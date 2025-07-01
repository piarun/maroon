use log::error;
use r2d2_redis::redis::{Commands, RedisResult};
use r2d2_redis::{RedisConnectionManager, r2d2::Pool};
use schema::mn_events::LogEvent;
use serde_json;
use std::env::VarError;
use std::sync::OnceLock;
use tokio;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

const STATE_LOG_STREAM: &str = "state_log_stream";

pub struct Sender {
  redis_url: String,
}

impl Sender {
  pub fn new(redis_url: String) -> Sender {
    Sender { redis_url }
  }

  pub async fn start_loop(
    self,
    mut receiver: UnboundedReceiver<LogEvent>,
  ) {
    let manager = RedisConnectionManager::new(self.redis_url).unwrap();
    let pool = Pool::builder().build(manager).unwrap();
    let mut conn = pool.get().unwrap();

    while let Some(event) = receiver.recv().await {
      let pairs = &[("event", serde_json::to_string(&event).unwrap())];
      let res: RedisResult<()> = conn.xadd(STATE_LOG_STREAM, "*", pairs);
      if let Err(e) = res {
        error!("stream push to redis: {e}");
        println!("sent : {e}");
      } else {
        println!("sent correctly");
      }
    }
  }
}

pub fn log_event_sender(custom_sender: Option<Sender>) -> &'static UnboundedSender<LogEvent> {
  static COUNTER: OnceLock<UnboundedSender<LogEvent>> = OnceLock::new();
  COUNTER.get_or_init(|| {
    let (sender, receiver) = mpsc::unbounded_channel::<LogEvent>();

    let snd = match custom_sender {
      Some(s) => s,
      None => {
        let redis_url: Result<String, VarError> = std::env::var("REDIS_URL");
        match redis_url {
          Ok(r_u) => Sender::new(r_u),
          Err(_) => {
            println!("LAST RESORT");
            return sender;
          }
        }
      }
    };

    tokio::spawn(async move {
      snd.start_loop(receiver).await;
    });
    sender
  })
}

pub fn log(event: LogEvent) {
  if let Err(e) = log_event_sender(None).send(event) {
    error!("redis pipe error: {e}");
  };
}
