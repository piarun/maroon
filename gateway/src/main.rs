use axum::{
  Json, Router,
  extract::{
    Path, State,
    ws::{Message, WebSocket, WebSocketUpgrade},
  },
  http::StatusCode,
  response::IntoResponse,
  routing::{get, post},
  serve,
};
use gateway::core::{Gateway, MonitorEvent};
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

async fn monitor_ws_loop(
  mut socket: WebSocket,
  mut rx: tokio::sync::broadcast::Receiver<MonitorEvent>,
) {
  loop {
    match rx.recv().await {
      Ok(evt) => {
        let payload = serde_json::to_string(&evt).unwrap_or_else(|_| format!("{:?}", evt));
        if socket.send(Message::Text(payload.into())).await.is_err() {
          break;
        }
      }
      Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
        continue;
      }
      Err(_) => break,
    }
  }
}

async fn monitor_handler(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| async move {
    let rx = {
      let gateway = gw.lock().await;
      gateway.monitor_subscribe()
    };
    monitor_ws_loop(socket, rx).await;
  })
}

// Generic per-request WebSocket endpoint.
async fn request_ws_handler(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |mut socket| async move {
    // Read the first message as TaskBlueprint JSON.
    let blueprint: TaskBlueprint = match socket.recv().await {
      Some(Ok(Message::Text(t))) => match serde_json::from_str::<TaskBlueprint>(&t) {
        Ok(bp) => bp,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: invalid blueprint json: {}", e).into())).await;
          return;
        }
      },
      Some(Ok(Message::Binary(b))) => match serde_json::from_slice::<TaskBlueprint>(&b) {
        Ok(bp) => bp,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: invalid blueprint json(bin): {}", e).into())).await;
          return;
        }
      },
      _ => {
        let _ = socket.send(Message::Text("error: expected first message with TaskBlueprint".into())).await;
        return;
      }
    };

    let mut gateway = gw.lock().await;
    // Ownership of socket is moved; responses will be sent on this socket by gateway
    gateway.send_request(blueprint, Some(socket)).await;
  })
}

#[tokio::main]
async fn main() {
  env_logger::init();

  let node_urls: Vec<String> = std::env::var("NODE_URLS")
    .unwrap_or("/ip4/127.0.0.1/tcp/3000,/ip4/127.0.0.1/tcp/3001,/ip4/127.0.0.1/tcp/3002".to_string())
    .split(',')
    .map(String::from)
    .collect();

  let server_port = std::env::var("PORT").unwrap_or("5000".to_string()).parse::<u16>().unwrap();
  let key_range = KeyRange(std::env::var("KEY_RANGE").unwrap_or("0".to_string()).parse::<u64>().unwrap());

  let mut gateway_app = Gateway::new(key_range, node_urls).expect("should be ok");
  gateway_app.start_in_background().await;

  // server
  let gw = Router::new()
    .route("/summarize/{a}/{b}", get(summarize_handler))
    .route("/monitor", get(monitor_handler))
    .route("/request", get(request_ws_handler))
    .route("/new_request", post(new_request_handler))
    .with_state(Arc::new(tokio::sync::Mutex::new(gateway_app)));

  let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
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
