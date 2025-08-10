use axum::{
  Router,
  extract::Path,
  extract::ws::{Message, WebSocket},
  extract::{State, WebSocketUpgrade},
  response::IntoResponse,
  routing::get,
  serve,
};
use clap::Parser;
use std::collections::BinaryHeap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{net::TcpListener, sync::mpsc};

mod maroon;
use maroon::*;

enum MaroonWriter {
  WebSocket(mpsc::Sender<String>),
  Printer,
}

impl MaroonWriter {
  fn new_websocket(socket: WebSocket) -> Self {
    let (sender, mut receiver) = mpsc::channel::<String>(100);
    let mut socket = socket;

    tokio::spawn(async move {
      while let Some(text) = receiver.recv().await {
        let _ = socket.send(Message::Text(text.into())).await;
      }
    });

    Self::WebSocket(sender)
  }

  fn new_printer() -> Self {
    Self::Printer
  }
}

impl Writer for MaroonWriter {
  async fn write_text(
    &self,
    text: impl Into<String> + Send,
    _timestamp: Option<LogicalTimeAbsoluteMs>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    match self {
      MaroonWriter::WebSocket(sender) => {
        sender.send(text.into()).await.map_err(Box::new)?;
        Ok(())
      }
      MaroonWriter::Printer => {
        println!("USW: {}", text.into());
        Ok(())
      }
    }
  }
}

async fn delay_handler<T: Timer>(
  ws: WebSocketUpgrade,
  Path((t, s)): Path<(u64, String)>,
  State(state): State<Arc<AppState<T, MaroonWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| delay_handler_ws(socket, state.timer.millis_since_start(), t, s, state))
}

async fn delay_handler_ws<T: Timer>(
  socket: WebSocket,
  ts: LogicalTimeAbsoluteMs,
  t: u64,
  s: String,
  state: Arc<AppState<T, MaroonWriter>>,
) {
  state
    .schedule(
      Arc::new(MaroonWriter::new_websocket(socket)),
      MaroonTaskStack {
        maroon_stack_entries: vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMs(t)),
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMessage(s.clone())),
          MaroonTaskStackEntry::State(MaroonTaskState::DelayedMessageTaskBegin),
        ],
      },
      MaroonTaskHeap::Empty,
      ts,
      format!("Delayed by {}ms: `{}`.", t, s),
    )
    .await;
}

async fn send_handler<T: Timer>(
  ws: WebSocketUpgrade,
  Path(s): Path<String>,
  State(state): State<Arc<AppState<T, MaroonWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| send_handler_ws(socket, state.timer.millis_since_start(), s, state))
}

async fn send_handler_ws<T: Timer>(
  socket: WebSocket,
  ts: LogicalTimeAbsoluteMs,
  s: String,
  state: Arc<AppState<T, MaroonWriter>>,
) {
  state
    .schedule(
      Arc::new(MaroonWriter::new_websocket(socket)),
      MaroonTaskStack {
        maroon_stack_entries: vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::SenderInputMessage(s.clone())),
          MaroonTaskStackEntry::State(MaroonTaskState::SenderSendMessage),
        ],
      },
      MaroonTaskHeap::Empty,
      ts,
      format!("Sent: `{}`.", s),
    )
    .await;
}

async fn receive_handler<T: Timer>(
  ws: WebSocketUpgrade,
  State(state): State<Arc<AppState<T, MaroonWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| receive_handler_ws(socket, state))
}

async fn receive_handler_ws<T: Timer>(
  socket: WebSocket,
  state: Arc<AppState<T, MaroonWriter>>,
) {
  state
    .park_awaiter(
      Arc::new(MaroonWriter::new_websocket(socket)),
      MaroonTaskStack { maroon_stack_entries: vec![] },
      MaroonTaskHeap::Empty,
      format!("Receive sending message."),
    )
    .await
}

async fn divisors_handler<T: Timer>(
  ws: WebSocketUpgrade,
  Path(a): Path<u64>,
  State(state): State<Arc<AppState<T, MaroonWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| divisors_handler_ws(socket, state.timer.millis_since_start(), a, state))
}

async fn divisors_handler_ws<T: Timer>(
  socket: WebSocket,
  ts: LogicalTimeAbsoluteMs,
  n: u64,
  state: Arc<AppState<T, MaroonWriter>>,
) {
  state
    .schedule(
      Arc::new(MaroonWriter::new_websocket(socket)),
      MaroonTaskStack { maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskBegin)] },
      MaroonTaskHeap::Divisors(MaroonTaskHeapDivisors { n, i: n }),
      ts,
      format!("Divisors of {}", n),
    )
    .await;
}

async fn fibonacci_handler<T: Timer>(
  ws: WebSocketUpgrade,
  Path(n): Path<u64>,
  State(state): State<Arc<AppState<T, MaroonWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| fibonacci_handler_ws(socket, state.timer.millis_since_start(), n, state))
}

async fn fibonacci_handler_ws<T: Timer>(
  socket: WebSocket,
  ts: LogicalTimeAbsoluteMs,
  n: u64,
  state: Arc<AppState<T, MaroonWriter>>,
) {
  state
    .schedule(
      Arc::new(MaroonWriter::new_websocket(socket)),
      MaroonTaskStack { maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::FibonacciTaskBegin)] },
      MaroonTaskHeap::Fibonacci(MaroonTaskHeapFibonacci {
        n,
        index: 0,
        a: 0,
        b: 0,
        delay_ms: LogicalTimeDeltaMs::from_millis(0),
      }),
      ts,
      format!("Fibonacci number {}", n),
    )
    .await;
}

async fn factorial_handler<T: Timer>(
  ws: WebSocketUpgrade,
  Path(n): Path<u64>,
  State(state): State<Arc<AppState<T, MaroonWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| factorial_handler_ws(socket, state.timer.millis_since_start(), n, state))
}

async fn factorial_handler_ws<T: Timer>(
  socket: WebSocket,
  ts: LogicalTimeAbsoluteMs,
  n: u64,
  state: Arc<AppState<T, MaroonWriter>>,
) {
  state
    .schedule(
      Arc::new(MaroonWriter::new_websocket(socket)),
      MaroonTaskStack {
        maroon_stack_entries: vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialInput(n)),
          MaroonTaskStackEntry::State(MaroonTaskState::FactorialEntry),
        ],
      },
      MaroonTaskHeap::Empty,
      ts,
      format!("Factorial of {}", n),
    )
    .await;
}

async fn get_user_handler<T: Timer>(
  ws: WebSocketUpgrade,
  Path(id): Path<String>,
  State(state): State<Arc<AppState<T, MaroonWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| get_user_handler_ws(socket, id, state))
}

async fn get_user_handler_ws<T: Timer>(
  socket: WebSocket,
  id: String,
  state: Arc<AppState<T, MaroonWriter>>,
) {
  state
    .schedule(
      Arc::new(MaroonWriter::new_websocket(socket)),
      MaroonTaskStack {
        maroon_stack_entries: vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::RequesterGetUserInput(id.clone())),
          MaroonTaskStackEntry::State(MaroonTaskState::RequesterGetUserRequest),
        ],
      },
      MaroonTaskHeap::Empty,
      state.timer.millis_since_start(),
      format!("Get user {id}"),
    )
    .await;
}

async fn root_handler<T: Timer, W: Writer>(_state: State<Arc<AppState<T, W>>>) -> impl IntoResponse {
  "magic"
}

async fn state_handler<T: Timer, W: Writer>(State(state): State<Arc<AppState<T, W>>>) -> impl IntoResponse {
  let mut response = String::from("Active tasks:\n");
  let mut empty = true;

  for (id, maroon_task) in state.fsm.lock().await.active_tasks.iter() {
    empty = false;
    // TODO(dkorolev): Add the `heap` here as well.
    response.push_str(&format!(
      "Task ID: {}, Description: {}, Stack: {:?}\n",
      id, maroon_task.description, maroon_task.maroon_stack,
    ));
  }

  if empty {
    response = String::from("No active tasks\n");
  }

  println!("STATE: {response}");
  response
}

async fn quit_handler<T: Timer, W: Writer>(State(state): State<Arc<AppState<T, W>>>) -> impl IntoResponse {
  let _ = state.quit_tx.send(()).await;
  "TY\n"
}

#[tokio::main]
async fn main() {
  let args = Args::parse();
  let timer = Arc::new(WallTimeTimer::new());
  let (quit_tx, mut quit_rx) = mpsc::channel::<()>(1);

  let app_state = Arc::new(AppState {
    fsm: Arc::new(tokio::sync::Mutex::new(MaroonRuntime {
      task_id_generator: NextTaskIdGenerator::new(),
      pending_operations: BinaryHeap::<TimestampedMaroonTask>::new(),
      active_tasks: std::collections::HashMap::new(),
      awaiter: None,
      daemon_user_storage: None,
    })),
    quit_tx,
    timer,
  });

  app_state.create_user_storage(Arc::new(MaroonWriter::new_printer())).await;

  let app = Router::new()
    .route("/", get(root_handler))
    .route("/delay/{t}/{s}", get(delay_handler))
    .route("/send/{s}", get(send_handler))
    .route("/receive", get(receive_handler))
    .route("/getUser/{id}", get(get_user_handler))
    .route("/divisors/{n}", get(divisors_handler))
    .route("/fibonacci/{n}", get(fibonacci_handler))
    .route("/factorial/{n}", get(factorial_handler))
    .route("/state", get(state_handler))
    .route("/quit", get(quit_handler))
    .with_state(Arc::clone(&app_state));

  let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
  let listener = TcpListener::bind(addr).await.unwrap();

  println!("rust ws state machine demo up on {addr}");

  let server = serve(listener, app);

  let shutdown = async move {
    quit_rx.recv().await;
  };

  tokio::select! {
    _ = server.with_graceful_shutdown(shutdown) => {},
    _ = execute_pending_operations(Arc::clone(&app_state), false) => {
      unreachable!();
    }
  }

  println!("rust ws state machine demo down");
}
