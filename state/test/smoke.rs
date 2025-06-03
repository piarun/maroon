#[cfg(test)]
mod tests {
  use state::*;
  use std::sync::Arc;
  use tokio::sync::mpsc;
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
    execute_pending_operations_inner(&mut app_state, true).await;

    let expected1 = vec!["120ms:12%12==0", "180ms:12%6==0", "220ms:12%4==0", "225ms:HI after 225ms"].join(";");
    assert_eq!(expected1, writer.get_outputs_as_string());

    timer.set_time(1000);
    execute_pending_operations_inner(&mut app_state, true).await;

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
    execute_pending_operations_inner(&mut app_state, true).await;

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
    execute_pending_operations_inner(&mut app_state, true).await;
    assert_eq!("0ms:fib1[5]=1", writer.get_outputs_as_string());

    timer.set_time(10);
    execute_pending_operations_inner(&mut app_state, true).await;

    assert_eq!("0ms:fib1[5]=1;5ms:fib2[5]=1", writer.get_outputs_as_string());

    timer.set_time(100);
    execute_pending_operations_inner(&mut app_state, true).await;

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
    execute_pending_operations_inner(&mut app_state, true).await;

    let output_at_100 = writer.get_outputs_as_string();
    assert_eq!(output_at_100, "0ms:f(5)");

    timer.set_time(1000);
    execute_pending_operations_inner(&mut app_state, true).await;

    let output_at_1000 = writer.get_outputs_as_string();
    assert_eq!(output_at_1000, "0ms:f(5);250ms:f(4);450ms:f(3);600ms:f(2);700ms:f(1);750ms:5!=120");
  }
}
