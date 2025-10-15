use maroon::app::Params;
use maroon::metrics;
use schema::mn_events::{LogEvent, LogEventBody, now_microsec};
use std::future;
use std::num::NonZeroUsize;
use std::time::Duration;
use tracing::error;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,maroon=info"));
  tracing_subscriber::registry()
    .with(filter)
    .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
    .init();

  let node_urls: Vec<String> =
    std::env::var("NODE_URLS").map_err(|e| format!("NODE_URLS not set: {}", e))?.split(',').map(String::from).collect();
  let etcd_urls: Vec<String> =
    std::env::var("ETCD_URLS").map_err(|e| format!("ETCD_URLS not set: {}", e))?.split(',').map(String::from).collect();

  let self_url: String = std::env::var("SELF_URL").map_err(|e| format!("SELF_URL not set: {}", e))?;

  let _tick = Duration::from_millis(std::env::var("TICK").unwrap_or("60".to_string()).parse::<u64>().unwrap());

  let consensus_nodes = std::env::var("CONSENSUS_NODES")
    .unwrap_or_else(|_| "2".to_string())
    .parse::<usize>()
    .map(NonZeroUsize::new)
    .unwrap()
    .unwrap();

  let params = Params::default().set_consensus_nodes(consensus_nodes);

  let (maroon_stack, _stack_remote_control) = maroon::stack::MaroonStack::new(node_urls, etcd_urls, self_url, params)?;
  let meter_provider = metrics::init_meter_provider(maroon_stack.id)?;

  let id = maroon_stack.id;
  let _shutdown = maroon_stack.start();

  state_log::log(LogEvent { timestamp_micros: now_microsec(), emitter: id, body: LogEventBody::MaroonNodeUp });

  // TODO: implement proper shutdown
  // forever pause current state in order to prevent killing the process
  // later will be replaced with something else. Don't know with what
  future::pending::<()>().await;

  if let Err(e) = meter_provider.shutdown() {
    error!("meter provider shutdown: {e}");
  }

  state_log::log(LogEvent { timestamp_micros: now_microsec(), emitter: id, body: LogEventBody::MaroonNodeDown });
  Ok(())
}
