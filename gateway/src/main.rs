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
use serde::Deserialize;
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

// Order Book specific WS endpoints for convenience.
#[derive(Deserialize)]
struct AddOrderReq {
  id: u64,
  price: u64,
  qty: u64,
}

#[derive(Deserialize)]
struct DepthReq {
  n: u64,
}

async fn ob_add_buy_ws(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |mut socket| async move {
    let payload: AddOrderReq = match socket.recv().await {
      Some(Ok(Message::Text(t))) => match serde_json::from_str::<AddOrderReq>(&t) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      Some(Ok(Message::Binary(b))) => match serde_json::from_slice::<AddOrderReq>(&b) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      _ => {
        let _ = socket.send(Message::Text("error: expected {id,price,qty}".into())).await;
        return;
      }
    };
    let mut gateway = gw.lock().await;
    let bp = TaskBlueprint {
      fiber_type: FiberType::new("order_book"),
      function_key: "add_buy".to_string(),
      init_values: vec![Value::U64(payload.id), Value::U64(payload.price), Value::U64(payload.qty)],
    };
    gateway.send_request(bp, Some(socket)).await;
  })
}

async fn ob_add_sell_ws(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |mut socket| async move {
    let payload: AddOrderReq = match socket.recv().await {
      Some(Ok(Message::Text(t))) => match serde_json::from_str::<AddOrderReq>(&t) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      Some(Ok(Message::Binary(b))) => match serde_json::from_slice::<AddOrderReq>(&b) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      _ => {
        let _ = socket.send(Message::Text("error: expected {id,price,qty}".into())).await;
        return;
      }
    };
    let mut gateway = gw.lock().await;
    let bp = TaskBlueprint {
      fiber_type: FiberType::new("order_book"),
      function_key: "add_sell".to_string(),
      init_values: vec![Value::U64(payload.id), Value::U64(payload.price), Value::U64(payload.qty)],
    };
    gateway.send_request(bp, Some(socket)).await;
  })
}

async fn ob_best_bid_ws(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| async move {
    let mut gateway = gw.lock().await;
    let bp = TaskBlueprint {
      fiber_type: FiberType::new("order_book"),
      function_key: "best_bid".to_string(),
      init_values: vec![],
    };
    gateway.send_request(bp, Some(socket)).await;
  })
}

async fn ob_best_ask_ws(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| async move {
    let mut gateway = gw.lock().await;
    let bp = TaskBlueprint {
      fiber_type: FiberType::new("order_book"),
      function_key: "best_ask".to_string(),
      init_values: vec![],
    };
    gateway.send_request(bp, Some(socket)).await;
  })
}

async fn ob_top_n_depth_ws(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |mut socket| async move {
    let payload: DepthReq = match socket.recv().await {
      Some(Ok(Message::Text(t))) => match serde_json::from_str::<DepthReq>(&t) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      Some(Ok(Message::Binary(b))) => match serde_json::from_slice::<DepthReq>(&b) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      _ => {
        let _ = socket.send(Message::Text("error: expected {n}".into())).await;
        return;
      }
    };
    let mut gateway = gw.lock().await;
    let bp = TaskBlueprint {
      fiber_type: FiberType::new("order_book"),
      function_key: "top_n_depth".to_string(),
      init_values: vec![Value::U64(payload.n)],
    };
    gateway.send_request(bp, Some(socket)).await;
  })
}

async fn ob_cancel_ws(
  State(gw): State<Arc<tokio::sync::Mutex<Gateway>>>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |mut socket| async move {
    #[derive(Deserialize)]
    struct CancelReq {
      id: u64,
    }
    let payload: CancelReq = match socket.recv().await {
      Some(Ok(Message::Text(t))) => match serde_json::from_str::<CancelReq>(&t) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      Some(Ok(Message::Binary(b))) => match serde_json::from_slice::<CancelReq>(&b) {
        Ok(v) => v,
        Err(e) => {
          let _ = socket.send(Message::Text(format!("error: {}", e).into())).await;
          return;
        }
      },
      _ => {
        let _ = socket.send(Message::Text("error: expected {id}".into())).await;
        return;
      }
    };
    let mut gateway = gw.lock().await;
    let bp = TaskBlueprint {
      fiber_type: FiberType::new("order_book"),
      function_key: "cancel".to_string(),
      init_values: vec![Value::U64(payload.id)],
    };
    gateway.send_request(bp, Some(socket)).await;
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
    // Order Book WS routes
    .route("/ws/order_book/add_buy", get(ob_add_buy_ws))
    .route("/ws/order_book/add_sell", get(ob_add_sell_ws))
    .route("/ws/order_book/best_bid", get(ob_best_bid_ws))
    .route("/ws/order_book/best_ask", get(ob_best_ask_ws))
    .route("/ws/order_book/top_n_depth", get(ob_top_n_depth_ws))
    .route("/ws/order_book/cancel", get(ob_cancel_ws))
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
