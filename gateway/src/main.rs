use axum::{
  http::StatusCode,
  response::IntoResponse,
  routing::{get, post},
  serve,
  extract::{Path, State, ws::WebSocketUpgrade},
  Router, Json,
};
use gateway::core::Gateway;
use generated::maroon_assembler::Value;
use protocol::transaction::{FiberType, TaskBlueprint};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use types::range_key::KeyRange;

async fn summarize_handler(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  Path((a, b)): Path<(u64, u64)>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| async move {
    let mut gateway = gw.lock().await;

    gateway
      .send_request(
        TaskBlueprint {
          fiber_type: FiberType::new("application"),
          function_key: "async_foo".to_string(),
          init_values: vec![Value::U64(a), Value::U64(b)],
        },
        Some(socket),
      )
      .await;
  })
}

async fn new_request_handler(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  Json(blueprint): Json<TaskBlueprint>,
) -> impl IntoResponse {
  let mut gateway = gw.lock().await;
  gateway.send_request(blueprint, None).await;
  StatusCode::ACCEPTED
}

#[tokio::main]
async fn main() {
  env_logger::init();

  let node_urls: Vec<String> = std::env::var("NODE_URLS")
    .unwrap_or("/ip4/127.0.0.1/tcp/3000,/ip4/127.0.0.1/tcp/3001,/ip4/127.0.0.1/tcp/3002".to_string())
    .split(',')
    .map(String::from)
    .collect();

  let key_range = KeyRange(std::env::var("KEY_RANGE").unwrap_or("0".to_string()).parse::<u64>().unwrap());

  let mut gateway_app = Gateway::new(key_range, node_urls).expect("should be ok");
  gateway_app.start_in_background().await;

  // server
  let gw = Router::new()
    .route("/summarize/{a}/{b}", get(summarize_handler))
    .route("/new_request", post(new_request_handler))
    .with_state(Arc::new(tokio::sync::Mutex::new(gateway_app)));

  let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
  let listener = TcpListener::bind(addr).await.unwrap();

  println!("gateway ws server up on {addr}");

  let server = serve(listener, gw);

  let shutdown = async move {
    let _ = tokio::signal::ctrl_c().await;
  };

  tokio::select! {
    _ = server.with_graceful_shutdown(shutdown) => {},
  }

  println!("gateway ws server down");
}
