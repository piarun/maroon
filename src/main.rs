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

struct WebSocketWriter {
  sender: mpsc::Sender<String>,
  _task: tokio::task::JoinHandle<()>,
}

impl WebSocketWriter {
  fn new(socket: WebSocket) -> Self {
    let (sender, mut receiver) = mpsc::channel::<String>(100);
    let mut socket = socket;

    let task = tokio::spawn(async move {
      while let Some(text) = receiver.recv().await {
        let _ = socket.send(Message::Text(text.into())).await;
      }
    });

    Self { sender, _task: task }
  }
}

impl Writer for WebSocketWriter {
  async fn write_text(
    &self, text: String, _timestamp: Option<LogicalTimeAbsoluteMs>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.sender.send(text).await.map_err(Box::new)?;
    Ok(())
  }
}

async fn delay_handler<T: Timer>(
  ws: WebSocketUpgrade, Path((t, s)): Path<(u64, String)>, State(state): State<Arc<AppState<T, WebSocketWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| delay_handler_ws(socket, state.timer.millis_since_start(), t, s, state))
}

async fn delay_handler_ws<T: Timer>(
  socket: WebSocket, ts: LogicalTimeAbsoluteMs, t: u64, s: String, state: Arc<AppState<T, WebSocketWriter>>,
) {
  state
    .schedule(
      Arc::new(WebSocketWriter::new(socket)),
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

async fn divisors_handler<T: Timer>(
  ws: WebSocketUpgrade, Path(a): Path<u64>, State(state): State<Arc<AppState<T, WebSocketWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| divisors_handler_ws(socket, state.timer.millis_since_start(), a, state))
}

async fn divisors_handler_ws<T: Timer>(
  socket: WebSocket, ts: LogicalTimeAbsoluteMs, n: u64, state: Arc<AppState<T, WebSocketWriter>>,
) {
  state
    .schedule(
      Arc::new(WebSocketWriter::new(socket)),
      MaroonTaskStack { maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskBegin)] },
      MaroonTaskHeap::Divisors(MaroonTaskHeapDivisors { n, i: n }),
      ts,
      format!("Divisors of {}", n),
    )
    .await;
}

async fn fibonacci_handler<T: Timer>(
  ws: WebSocketUpgrade, Path(n): Path<u64>, State(state): State<Arc<AppState<T, WebSocketWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| fibonacci_handler_ws(socket, state.timer.millis_since_start(), n, state))
}

async fn fibonacci_handler_ws<T: Timer>(
  socket: WebSocket, ts: LogicalTimeAbsoluteMs, n: u64, state: Arc<AppState<T, WebSocketWriter>>,
) {
  state
    .schedule(
      Arc::new(WebSocketWriter::new(socket)),
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
  ws: WebSocketUpgrade, Path(n): Path<u64>, State(state): State<Arc<AppState<T, WebSocketWriter>>>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| factorial_handler_ws(socket, state.timer.millis_since_start(), n, state))
}

async fn factorial_handler_ws<T: Timer>(
  socket: WebSocket, ts: LogicalTimeAbsoluteMs, n: u64, state: Arc<AppState<T, WebSocketWriter>>,
) {
  state
    .schedule(
      Arc::new(WebSocketWriter::new(socket)),
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
    })),
    quit_tx,
    timer,
  });

  let app = Router::new()
    .route("/", get(root_handler))
    .route("/delay/{t}/{s}", get(delay_handler))
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
    _ = execute_pending_operations(Arc::clone(&app_state)) => {
      unreachable!();
    }
  }

  println!("rust ws state machine demo down");
}

#[cfg(test)]
mod tests {
  use super::*;
  struct MockTimer {
    current_time: Arc<std::sync::Mutex<LogicalTimeAbsoluteMs>>,
  }

  impl MockTimer {
    fn new(initial_time: u64) -> Self {
      Self { current_time: Arc::new(std::sync::Mutex::new(LogicalTimeAbsoluteMs::from_millis(initial_time))) }
    }

    fn set_time(&self, time_ms: u64) {
      let mut current = self.current_time.lock().unwrap();
      *current = LogicalTimeAbsoluteMs::from_millis(time_ms);
    }
  }

  impl Timer for MockTimer {
    fn millis_since_start(&self) -> LogicalTimeAbsoluteMs {
      *self.current_time.lock().unwrap()
    }
  }

  struct MockWriter<T: Timer> {
    outputs: Arc<std::sync::Mutex<Vec<String>>>,
    timer: Arc<T>,
  }

  impl<T: Timer> MockWriter<T> {
    fn new_with_timer(timer: Arc<T>) -> Self {
      Self { outputs: Arc::new(std::sync::Mutex::new(Vec::new())), timer }
    }

    fn get_outputs_as_string(&self) -> String {
      self.outputs.lock().unwrap().join(";")
    }

    fn clear_outputs(&self) {
      self.outputs.lock().unwrap().clear();
    }
  }

  impl<T: Timer> Clone for MockWriter<T> {
    fn clone(&self) -> Self {
      Self { outputs: Arc::clone(&self.outputs), timer: Arc::clone(&self.timer) }
    }
  }

  impl<T: Timer> Writer for MockWriter<T> {
    async fn write_text(
      &self, text: String, timestamp: Option<LogicalTimeAbsoluteMs>,
    ) -> Result<(), Box<dyn std::error::Error>> {
      let timer = Arc::clone(&self.timer);
      let outputs = Arc::clone(&self.outputs);
      let time_to_use = timestamp.unwrap_or_else(|| timer.millis_since_start());
      outputs.lock().unwrap().push(format!("{}ms:{text}", time_to_use.as_millis()));
      Ok(())
    }
  }

  #[tokio::test]
  async fn test_state_machine_and_execution() {
    let timer = Arc::new(MockTimer::new(0));
    let (quit_tx, _) = mpsc::channel::<()>(1);
    let mut app_state = Arc::new(AppState {
      fsm: Arc::new(tokio::sync::Mutex::new(MaroonRuntime {
        task_id_generator: NextTaskIdGenerator::new(),
        pending_operations: Default::default(),
        active_tasks: Default::default(),
      })),
      quit_tx,
      timer: Arc::clone(&timer),
    });
    let writer = Arc::new(MockWriter::new_with_timer(Arc::clone(&timer)));

    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack { maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskBegin)] },
        MaroonTaskHeap::Divisors(MaroonTaskHeapDivisors { n: 12, i: 12 }),
        LogicalTimeAbsoluteMs::from_millis(0),
        "Divisors of 12".to_string(),
      )
      .await;
    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack {
          maroon_stack_entries: vec![
            MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMs(225)),
            MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMessage("HI".to_string())),
            MaroonTaskStackEntry::State(MaroonTaskState::DelayedMessageTaskBegin),
          ],
        },
        MaroonTaskHeap::Empty,
        LogicalTimeAbsoluteMs::from_millis(0),
        "Hello".to_string(),
      )
      .await;
    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack {
          maroon_stack_entries: vec![
            MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMs(75)),
            MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMessage("BYE".to_string())),
            MaroonTaskStackEntry::State(MaroonTaskState::DelayedMessageTaskBegin),
          ],
        },
        MaroonTaskHeap::Empty,
        LogicalTimeAbsoluteMs::from_millis(200),
        "Bye".to_string(),
      )
      .await;

    timer.set_time(225);
    execute_pending_operations_inner(&mut app_state).await;

    let expected1 = vec!["120ms:12%12==0", "180ms:12%6==0", "220ms:12%4==0", "225ms:HI after 225ms"].join(";");
    assert_eq!(expected1, writer.get_outputs_as_string());

    timer.set_time(1000);
    execute_pending_operations_inner(&mut app_state).await;

    let expected2 = vec![
      "120ms:12%12==0",
      "180ms:12%6==0",
      "220ms:12%4==0",
      "225ms:HI after 225ms",
      "250ms:12%3==0",
      "270ms:12%2==0",
      "275ms:BYE after 75ms",
      "280ms:12%1==0",
      "280ms:12!",
    ]
    .join(";");
    assert_eq!(expected2, writer.get_outputs_as_string());

    writer.clear_outputs();
    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack { maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskBegin)] },
        MaroonTaskHeap::Divisors(MaroonTaskHeapDivisors { n: 8, i: 8 }),
        LogicalTimeAbsoluteMs::from_millis(10_000),
        "".to_string(),
      )
      .await;
    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack { maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskBegin)] },
        MaroonTaskHeap::Divisors(MaroonTaskHeapDivisors { n: 9, i: 9 }),
        LogicalTimeAbsoluteMs::from_millis(10_001),
        "".to_string(),
      )
      .await;
    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack { maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskBegin)] },
        MaroonTaskHeap::Divisors(MaroonTaskHeapDivisors { n: 3, i: 3 }),
        LogicalTimeAbsoluteMs::from_millis(10_002),
        "".to_string(),
      )
      .await;
    timer.set_time(20_000);
    execute_pending_operations_inner(&mut app_state).await;

    let expected3 = vec![
      "10032ms:3%3==0",
      "10042ms:3%1==0",
      "10042ms:3!",
      "10080ms:8%8==0",
      "10091ms:9%9==0",
      "10120ms:8%4==0",
      "10121ms:9%3==0",
      "10131ms:9%1==0",
      "10131ms:9!",
      "10140ms:8%2==0",
      "10150ms:8%1==0",
      "10150ms:8!",
    ]
    .join(";");
    assert_eq!(expected3, writer.get_outputs_as_string());
  }

  #[tokio::test]
  async fn test_fibonacci_task() {
    let timer = Arc::new(MockTimer::new(0));
    let (quit_tx, _) = mpsc::channel::<()>(1);
    let mut app_state = Arc::new(AppState {
      fsm: Arc::new(tokio::sync::Mutex::new(MaroonRuntime {
        task_id_generator: NextTaskIdGenerator::new(),
        pending_operations: Default::default(),
        active_tasks: Default::default(),
      })),
      quit_tx,
      timer: Arc::clone(&timer),
    });
    let writer = Arc::new(MockWriter::new_with_timer(Arc::clone(&timer)));

    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack {
          maroon_stack_entries: vec![MaroonTaskStackEntry::State(MaroonTaskState::FibonacciTaskBegin)],
        },
        MaroonTaskHeap::Fibonacci(MaroonTaskHeapFibonacci {
          n: 5,
          index: 0,
          a: 0,
          b: 0,
          delay_ms: LogicalTimeDeltaMs::from_millis(0),
        }),
        LogicalTimeAbsoluteMs::from_millis(0),
        "The fifth Fibonacci number".to_string(),
      )
      .await;

    timer.set_time(4);
    execute_pending_operations_inner(&mut app_state).await;
    assert_eq!("0ms:fib1[5]=1", writer.get_outputs_as_string());

    timer.set_time(10);
    execute_pending_operations_inner(&mut app_state).await;

    assert_eq!("0ms:fib1[5]=1;5ms:fib2[5]=1", writer.get_outputs_as_string());

    timer.set_time(100);
    execute_pending_operations_inner(&mut app_state).await;

    let expected = vec!["0ms:fib1[5]=1", "5ms:fib2[5]=1", "15ms:fib3[5]=2", "30ms:fib4[5]=3", "50ms:fib5=5"].join(";");

    assert_eq!(expected, writer.get_outputs_as_string());
  }

  #[tokio::test]
  async fn test_factorial_task() {
    let timer = Arc::new(MockTimer::new(0));
    let (quit_tx, _) = mpsc::channel::<()>(1);
    let mut app_state = Arc::new(AppState {
      fsm: Arc::new(tokio::sync::Mutex::new(MaroonRuntime {
        task_id_generator: NextTaskIdGenerator::new(),
        pending_operations: Default::default(),
        active_tasks: Default::default(),
      })),
      quit_tx,
      timer: Arc::clone(&timer),
    });
    let writer = Arc::new(MockWriter::new_with_timer(Arc::clone(&timer)));

    app_state
      .schedule(
        Arc::clone(&writer),
        MaroonTaskStack {
          maroon_stack_entries: vec![
            MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialInput(5)),
            MaroonTaskStackEntry::State(MaroonTaskState::FactorialEntry),
          ],
        },
        MaroonTaskHeap::Empty,
        LogicalTimeAbsoluteMs::from_millis(0),
        "Factorial of 5".to_string(),
      )
      .await;

    timer.set_time(100);
    execute_pending_operations_inner(&mut app_state).await;

    let output_at_100 = writer.get_outputs_as_string();
    assert_eq!(output_at_100, "0ms:f(5)");

    timer.set_time(1000);
    execute_pending_operations_inner(&mut app_state).await;

    let output_at_1000 = writer.get_outputs_as_string();
    assert_eq!(output_at_1000, "0ms:f(5);250ms:f(4);450ms:f(3);600ms:f(2);700ms:f(1);750ms:5!=120");
  }
}
