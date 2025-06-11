use maroon::app::Params;
use std::future;
use std::num::NonZeroUsize;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  env_logger::init();

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
  let _shutdown = maroon_stack.start();

  // forever pause current state in order to prevent killing the process
  // later will be replaced with something else. Don't know with what
  future::pending::<()>().await;

  Ok(())
}
