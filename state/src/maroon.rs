use clap::Parser;
use core::panic;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTimeAbsoluteMs(u64);

impl LogicalTimeAbsoluteMs {
  pub fn from_millis(millis: u64) -> Self {
    Self(millis)
  }

  #[allow(dead_code)]
  pub fn as_millis(&self) -> u64 {
    self.0
  }
}

impl std::fmt::Display for LogicalTimeAbsoluteMs {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::ops::Add<LogicalTimeDeltaMs> for LogicalTimeAbsoluteMs {
  type Output = LogicalTimeAbsoluteMs;

  fn add(
    self,
    rhs: LogicalTimeDeltaMs,
  ) -> Self::Output {
    LogicalTimeAbsoluteMs(self.0 + rhs.as_millis())
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTimeDeltaMs(u64);

impl LogicalTimeDeltaMs {
  pub fn from_millis(millis: u64) -> Self {
    Self(millis)
  }

  pub fn as_millis(&self) -> u64 {
    self.0
  }
}

impl std::fmt::Display for LogicalTimeDeltaMs {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl From<u64> for LogicalTimeDeltaMs {
  fn from(millis: u64) -> Self {
    Self::from_millis(millis)
  }
}

impl From<u64> for LogicalTimeAbsoluteMs {
  fn from(millis: u64) -> Self {
    Self::from_millis(millis)
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaroonTaskId(u64);

impl MaroonTaskId {
  pub fn from_u64(id: u64) -> Self {
    Self(id)
  }
}

impl std::fmt::Display for MaroonTaskId {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl From<u64> for MaroonTaskId {
  fn from(id: u64) -> Self {
    Self::from_u64(id)
  }
}

#[derive(Debug, Clone)]
pub struct NextTaskIdGenerator {
  next_task_id: u64,
}

impl NextTaskIdGenerator {
  pub fn new() -> Self {
    Self { next_task_id: 1 }
  }

  fn next_task_id(&mut self) -> MaroonTaskId {
    let task_id = MaroonTaskId::from_u64(self.next_task_id);
    self.next_task_id += 1;
    task_id
  }
}

#[derive(Parser)]
pub struct Args {
  #[arg(long, default_value = "3000")]
  pub port: u16,
}

pub trait Timer: Send + Sync + 'static {
  fn millis_since_start(&self) -> LogicalTimeAbsoluteMs;
}

pub struct WallTimeTimer {
  start_time: std::time::Instant,
}

impl WallTimeTimer {
  pub fn new() -> Self {
    Self { start_time: std::time::Instant::now() }
  }
}

impl Timer for WallTimeTimer {
  fn millis_since_start(&self) -> LogicalTimeAbsoluteMs {
    LogicalTimeAbsoluteMs::from_millis(self.start_time.elapsed().as_millis() as u64)
  }
}

pub trait Writer: Send + Sync + 'static {
  fn write_text(
    &self,
    text: impl Into<String> + Send,
    timestamp: Option<LogicalTimeAbsoluteMs>,
  ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send
  where
    Self: Send;
}

#[derive(Clone, Debug)]
pub enum MaroonTaskState {
  Completed,
  DelayedMessageTaskBegin,
  DelayedMessageTaskExecute,
  DivisorsTaskBegin,
  DivisorsTaskIteration,
  DivisorsPrintAndMoveOn,
  FibonacciTaskBegin,
  FibonacciTaskCalculate,
  FibonacciTaskResult,
  FibonacciTaskStep,
  FactorialEntry,
  FactorialRecursiveCall,
  FactorialRecursionPostWrite,
  FactorialRecursionPostSleep,
  FactorialRecursionPostRecursiveCall,
  FactorialDone,
  SenderSendMessage,
  WaiterWaitMessageAwaiting,
}

// NOTE(dkorolev): These dedicated types are easier for manual development.
// NOTE(dkorolev): They will all be "type-erased" later on, esp. once we get to the DSL part.
#[derive(Debug, Clone)]
pub enum MaroonTaskStackEntryValue {
  DelayInputMs(u64),
  DelayInputMessage(String), // TODO(dkorolev): This `String` contradicts the `u64` promise, but not really.
  FactorialInput(u64),
  FactorialArgument(u64),
  FactorialReturnValue(u64),
  SenderInputMessage(String),
  AwaiterInputMessage(String),
}

// For a given `S`, if the maroon stack contains `State(S)`, how many entries above it are its local stack vars.
const fn maroon_task_state_local_vars_count(e: &MaroonTaskState) -> usize {
  match e {
    MaroonTaskState::FactorialEntry => 1,                      // [FactorialInput]
    MaroonTaskState::FactorialRecursiveCall => 1,              // [FactorialArgument]
    MaroonTaskState::FactorialRecursionPostWrite => 1,         // [FactorialArgument]
    MaroonTaskState::FactorialRecursionPostSleep => 1,         // [FactorialArgument]
    MaroonTaskState::FactorialRecursionPostRecursiveCall => 2, // [FactorialArgument, FactorialReturnValue]
    MaroonTaskState::FactorialDone => 2,                       // [FactorialInput, FactorialReturnValue]
    MaroonTaskState::DelayedMessageTaskBegin => 2,             // [DelayInputMs, DelayInputMessage]
    MaroonTaskState::DelayedMessageTaskExecute => 2,           // [DelayInputMs, DelayInputMessage]
    MaroonTaskState::SenderSendMessage => 1,                   // [SenderInputMessage]
    MaroonTaskState::WaiterWaitMessageAwaiting => 1,           // [AwaiterInputMessage]
    _ => 0,
  }
}

// For a given `S`, if the maroon stack contains `Retrn(S)`, how many entries above it are its local stack vars.
#[cfg(test)]
const fn maroon_task_state_return_local_vars_count(e: &MaroonTaskState) -> usize {
  match e {
    MaroonTaskState::FactorialDone => 1,                       // [FactorialInput]
    MaroonTaskState::FactorialRecursionPostRecursiveCall => 1, // [FactorialArgument]
    _ => 0,
  }
}

// NOTE(dkorolev): The convention so far is the following:
// * The "last" element of the stack is the state of the task.
// * It is known up front how many entries before this state are its "local variables", including parameters.
// * This known mapping is effectively hard-coded. It will come from the DSL later on.
#[derive(Debug, Clone)]
pub enum MaroonTaskStackEntry {
  // The state to execute next, linearly.
  State(MaroonTaskState),
  // The state to return to on `Return`, with the stack populated with the return value.
  Retrn(MaroonTaskState),
  // The value, all `u64`-s for now, but strongly typed for simplicity of the demo.
  Value(MaroonTaskStackEntryValue),
}

#[derive(Debug)]
pub struct MaroonTaskStack {
  pub maroon_stack_entries: Vec<MaroonTaskStackEntry>,
}

#[derive(Clone, Debug)]
pub struct MaroonTaskHeapDivisors {
  pub n: u64,
  pub i: u64,
}

#[derive(Clone, Debug)]
pub struct MaroonTaskHeapFibonacci {
  pub n: u64,
  pub index: u64,
  pub a: u64,
  pub b: u64,
  pub delay_ms: LogicalTimeDeltaMs,
}

#[derive(Clone, Debug)]
pub enum MaroonTaskHeap {
  Empty,
  Divisors(MaroonTaskHeapDivisors),
  Fibonacci(MaroonTaskHeapFibonacci),
}

// TODO(dkorolev): Do not copy "stack vars" back and forth.
#[derive(Debug)]
pub enum MaroonStepResult {
  Done,
  Next(Vec<MaroonTaskStackEntry>),
  Sleep(LogicalTimeDeltaMs, Vec<MaroonTaskStackEntry>),
  Write(String, Vec<MaroonTaskStackEntry>),
  Return(MaroonTaskStackEntryValue),

  // right now string will be broadcasted, just for simplicity
  Send(String, Vec<MaroonTaskStackEntry>),
}

fn global_step(
  state: MaroonTaskState,
  vars: Vec<MaroonTaskStackEntryValue>,
  heap: &mut MaroonTaskHeap,
) -> MaroonStepResult {
  match state {
    MaroonTaskState::SenderSendMessage => {
      let Some(MaroonTaskStackEntryValue::SenderInputMessage(msg)) = vars.get(0) else {
        panic!("Unexpected arguments")
      };

      MaroonStepResult::Send(
        format!("[{}] went through state", &msg),
        vec![MaroonTaskStackEntry::State(MaroonTaskState::Completed)],
      )
    }
    MaroonTaskState::WaiterWaitMessageAwaiting => {
      let Some(MaroonTaskStackEntryValue::AwaiterInputMessage(msg)) = vars.get(0) else {
        panic!("Unexpected arguments")
      };

      MaroonStepResult::Write(format!("got: {msg}"), vec![MaroonTaskStackEntry::State(MaroonTaskState::Completed)])
    }
    MaroonTaskState::DelayedMessageTaskBegin => {
      let (msg, delay_ms) = match (vars.get(0), vars.get(1)) {
        (
          Some(MaroonTaskStackEntryValue::DelayInputMessage(msg)),
          Some(MaroonTaskStackEntryValue::DelayInputMs(delay_ms)),
        ) => (msg.clone(), *delay_ms),
        _ => panic!("Unexpected arguments in DelayedMessageTaskBegin: {:?}", vars),
      };

      MaroonStepResult::Sleep(
        LogicalTimeDeltaMs::from_millis(delay_ms),
        vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMs(delay_ms)),
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::DelayInputMessage(msg)),
          MaroonTaskStackEntry::State(MaroonTaskState::DelayedMessageTaskExecute),
        ],
      )
    }
    MaroonTaskState::DelayedMessageTaskExecute => {
      let (msg, delay_ms) = match (vars.get(0), vars.get(1)) {
        (
          Some(MaroonTaskStackEntryValue::DelayInputMessage(msg)),
          Some(MaroonTaskStackEntryValue::DelayInputMs(delay_ms)),
        ) => (msg, *delay_ms),
        _ => panic!("Unexpected arguments in DelayedMessageTaskExecute: {:?}", vars),
      };

      MaroonStepResult::Write(
        format!("{} after {}ms", &msg, delay_ms),
        vec![MaroonTaskStackEntry::State(MaroonTaskState::Completed)],
      )
    }
    MaroonTaskState::DivisorsTaskBegin => {
      if let MaroonTaskHeap::Divisors(data) = heap {
        data.i = data.n;
        MaroonStepResult::Next(vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskIteration)])
      } else {
        panic!("Heap type mismatch for `DivisorsTaskBegin`.");
      }
    }
    MaroonTaskState::DivisorsTaskIteration => {
      if let MaroonTaskHeap::Divisors(data) = heap {
        let mut i = data.i;
        while i > 0 && data.n % i != 0 {
          i -= 1;
        }
        if i == 0 {
          MaroonStepResult::Write(format!("{}!", data.n), vec![MaroonTaskStackEntry::State(MaroonTaskState::Completed)])
        } else {
          data.i = i;
          MaroonStepResult::Sleep(
            LogicalTimeDeltaMs::from_millis(i * 10),
            vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsPrintAndMoveOn)],
          )
        }
      } else {
        panic!("Heap type mismatch for `DivisorsTaskIteration`.");
      }
    }
    MaroonTaskState::DivisorsPrintAndMoveOn => {
      if let MaroonTaskHeap::Divisors(data) = heap {
        let result = MaroonStepResult::Write(
          format!("{}%{}==0", data.n, data.i),
          vec![MaroonTaskStackEntry::State(MaroonTaskState::DivisorsTaskIteration)],
        );
        data.i -= 1;
        result
      } else {
        panic!("Heap type mismatch for `DivisorsPrintAndMoveOn`.");
      }
    }
    MaroonTaskState::FibonacciTaskBegin => {
      if let MaroonTaskHeap::Fibonacci(data) = heap {
        if data.n <= 1 {
          MaroonStepResult::Write(
            format!("fib{}={}", data.n, data.n),
            vec![MaroonTaskStackEntry::State(MaroonTaskState::Completed)],
          )
        } else {
          data.index = 1;
          data.a = 0;
          data.b = 1;
          MaroonStepResult::Next(vec![MaroonTaskStackEntry::State(MaroonTaskState::FibonacciTaskCalculate)])
        }
      } else {
        panic!("Heap type mismatch for `DivisorsTaskBegin`.");
      }
    }
    MaroonTaskState::FibonacciTaskCalculate => {
      if let MaroonTaskHeap::Fibonacci(data) = heap {
        if data.index >= data.n {
          MaroonStepResult::Next(vec![MaroonTaskStackEntry::State(MaroonTaskState::FibonacciTaskResult)])
        } else {
          let delay = 5 * data.index;
          data.delay_ms = LogicalTimeDeltaMs::from_millis(delay);
          MaroonStepResult::Write(
            format!("fib{}[{}]={}", data.index, data.n, data.b),
            vec![MaroonTaskStackEntry::State(MaroonTaskState::FibonacciTaskStep)],
          )
        }
      } else {
        panic!("Heap type mismatch for `FibonacciTaskCalculate`.");
      }
    }
    MaroonTaskState::FibonacciTaskStep => {
      if let MaroonTaskHeap::Fibonacci(data) = heap {
        let next_index = data.index + 1;
        let next_a = data.b;
        let next_b = data.a + data.b;
        data.index = next_index;
        data.a = next_a;
        data.b = next_b;
        MaroonStepResult::Sleep(
          data.delay_ms,
          vec![MaroonTaskStackEntry::State(MaroonTaskState::FibonacciTaskCalculate)],
        )
      } else {
        panic!("Heap type mismatch for `FibonacciTaskStep`.");
      }
    }
    MaroonTaskState::FibonacciTaskResult => {
      if let MaroonTaskHeap::Fibonacci(data) = heap {
        MaroonStepResult::Write(
          format!("fib{}={}", data.n, data.b),
          vec![MaroonTaskStackEntry::State(MaroonTaskState::Completed)],
        )
      } else {
        panic!("Heap type mismatch for `FibonacciTaskResult`.");
      }
    }
    MaroonTaskState::FactorialEntry => {
      let n = match vars.get(0) {
        Some(MaroonTaskStackEntryValue::FactorialInput(n)) => *n,
        _ => panic!("Unexpected arguments in FactorialEntry: {:?}", vars),
      };

      MaroonStepResult::Next(vec![
        // This input should be preserved on the stack when `Retrn` takes place.
        MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialInput(n)),
        // This is the state to `Return` to.
        MaroonTaskStackEntry::Retrn(MaroonTaskState::FactorialDone),
        // This the argument to the function, which is the state right below this one.
        MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialArgument(n)),
        // And this is the function to be called, which will ultimately `Return` into `FactorialDone`.
        MaroonTaskStackEntry::State(MaroonTaskState::FactorialRecursiveCall),
      ])
    }
    MaroonTaskState::FactorialRecursiveCall => {
      let n = match vars.get(0) {
        Some(MaroonTaskStackEntryValue::FactorialArgument(n)) => *n,
        _ => panic!("Unexpected arguments in FactorialRecursiveCall: {:?}", vars),
      };

      MaroonStepResult::Write(
        format!("f({n})"),
        vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialArgument(n)),
          MaroonTaskStackEntry::State(MaroonTaskState::FactorialRecursionPostWrite),
        ],
      )
    }
    MaroonTaskState::FactorialRecursionPostWrite => {
      let n = match vars.get(0) {
        Some(MaroonTaskStackEntryValue::FactorialArgument(n)) => *n,
        _ => panic!("Unexpected arguments in FactorialRecursionPostWrite: {:?}", vars),
      };

      MaroonStepResult::Sleep(
        LogicalTimeDeltaMs::from_millis(n * 50),
        vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialArgument(n)),
          MaroonTaskStackEntry::State(MaroonTaskState::FactorialRecursionPostSleep),
        ],
      )
    }
    MaroonTaskState::FactorialRecursionPostSleep => {
      let n = match vars.get(0) {
        Some(MaroonTaskStackEntryValue::FactorialArgument(n)) => *n,
        _ => panic!("Unexpected arguments in FactorialRecursionPostSleep: {:?}", vars),
      };

      if n > 1 {
        MaroonStepResult::Next(vec![
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialArgument(n)),
          MaroonTaskStackEntry::Retrn(MaroonTaskState::FactorialRecursionPostRecursiveCall),
          MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::FactorialArgument(n - 1)),
          MaroonTaskStackEntry::State(MaroonTaskState::FactorialRecursiveCall),
        ])
      } else {
        MaroonStepResult::Return(MaroonTaskStackEntryValue::FactorialReturnValue(1))
      }
    }
    MaroonTaskState::FactorialRecursionPostRecursiveCall => {
      let (a, n) = match (vars.get(0), vars.get(1)) {
        (
          Some(MaroonTaskStackEntryValue::FactorialReturnValue(a)),
          Some(MaroonTaskStackEntryValue::FactorialArgument(n)),
        ) => (*a, *n),
        _ => panic!("Unexpected arguments in FactorialRecursionPostRecursiveCall: {:?}", vars),
      };

      MaroonStepResult::Return(MaroonTaskStackEntryValue::FactorialReturnValue(a * n))
    }
    MaroonTaskState::FactorialDone => {
      let (r, n) = match (vars.get(0), vars.get(1)) {
        (
          Some(MaroonTaskStackEntryValue::FactorialReturnValue(r)),
          Some(MaroonTaskStackEntryValue::FactorialInput(n)),
        ) => (*r, *n),
        _ => panic!("Unexpected arguments in FactorialDone: {:?}", vars),
      };

      MaroonStepResult::Write(format!("{n}!={r}"), vec![MaroonTaskStackEntry::State(MaroonTaskState::Completed)])
    }
    MaroonTaskState::Completed => MaroonStepResult::Done,
  }
}

pub struct MaroonTask<W: Writer> {
  pub description: String,
  pub writer: Arc<W>,
  pub maroon_stack: MaroonTaskStack,
  pub maroon_heap: MaroonTaskHeap,
}

pub struct TimestampedMaroonTask {
  scheduled_timestamp: LogicalTimeAbsoluteMs,
  task_id: MaroonTaskId,
}

impl TimestampedMaroonTask {
  pub fn new(
    scheduled_timestamp: LogicalTimeAbsoluteMs,
    task_id: MaroonTaskId,
  ) -> Self {
    Self { scheduled_timestamp, task_id }
  }
}

impl Eq for TimestampedMaroonTask {}

impl PartialEq for TimestampedMaroonTask {
  fn eq(
    &self,
    other: &Self,
  ) -> bool {
    self.scheduled_timestamp == other.scheduled_timestamp
  }
}

impl Ord for TimestampedMaroonTask {
  fn cmp(
    &self,
    other: &Self,
  ) -> Ordering {
    // Reversed order by design.
    other.scheduled_timestamp.cmp(&self.scheduled_timestamp)
  }
}

impl PartialOrd for TimestampedMaroonTask {
  fn partial_cmp(
    &self,
    other: &Self,
  ) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

pub struct MaroonRuntime<W: Writer> {
  pub task_id_generator: NextTaskIdGenerator,

  // any living task is in active_tasks
  // and any taskID is in pending_operations or in awaiter
  pub active_tasks: std::collections::HashMap<MaroonTaskId, MaroonTask<W>>,
  pub pending_operations: BinaryHeap<TimestampedMaroonTask>,
  pub awaiter: Option<MaroonTaskId>,
}

pub struct AppState<T: Timer, W: Writer> {
  pub fsm: Arc<tokio::sync::Mutex<MaroonRuntime<W>>>,
  pub quit_tx: tokio::sync::mpsc::Sender<()>,
  pub timer: Arc<T>,
}

impl<T: Timer, W: Writer> AppState<T, W> {
  pub async fn schedule(
    &self,
    writer: Arc<W>,
    maroon_stack: MaroonTaskStack,
    maroon_heap: MaroonTaskHeap,
    scheduled_timestamp: LogicalTimeAbsoluteMs,
    task_description: String,
  ) {
    let mut fsm = self.fsm.lock().await;
    let task_id = fsm.task_id_generator.next_task_id();
    fsm.active_tasks.insert(task_id, MaroonTask { description: task_description, writer, maroon_stack, maroon_heap });
    fsm.pending_operations.push(TimestampedMaroonTask::new(scheduled_timestamp, task_id))
  }

  pub async fn park_awaiter(
    &self,
    writer: Arc<W>,
    maroon_stack: MaroonTaskStack,
    maroon_heap: MaroonTaskHeap,
    task_description: String,
  ) {
    let mut fsm = self.fsm.lock().await;
    if fsm.awaiter.is_some() {
      panic!("only one awaiter at once")
    }
    let task_id = fsm.task_id_generator.next_task_id();

    fsm.awaiter = Some(task_id);
    fsm.active_tasks.insert(task_id, MaroonTask { description: task_description, writer, maroon_stack, maroon_heap });
  }
}

#[cfg(test)]
fn debug_validate_maroon_stack(stk: &Vec<MaroonTaskStackEntry>) {
  let stack_depth = stk.len();
  if stack_depth == 0 {
    panic!("In `debug_validate_maroon_stack` the `stk` should not be empty.");
  } else {
    let mut i = stack_depth - 1;
    while i > 0 {
      let n = {
        if let MaroonTaskStackEntry::State(state) = &stk[i] {
          maroon_task_state_local_vars_count(&state)
        } else if let MaroonTaskStackEntry::Retrn(state) = &stk[i] {
          maroon_task_state_return_local_vars_count(&state)
        } else {
          panic!("In `debug_validate_maroon_stack` the `stk[0]` should be a state, not {:?}.", stk[i]);
        }
      };
      if n > i {
        let state = &stk[i];
        panic!("In `debug_validate_maroon_stack` expecting {n} arguments for state {state:?}, only have {i} left.",);
      }
      for _ in 0..n {
        i = i - 1;
        if let MaroonTaskStackEntry::Value(_) = &stk[i] {
          // OK
        } else {
          panic!("In `debug_validate_maroon_stack` expecting value, for non-value, pls examine `{:?}`.", stk);
        }
      }
      if i == 0 {
        // All good, traversed the entire stack, which was not empty at the beginning of the call, no issues found.
        break;
      } else {
        i = i - 1
      }
    }
  }
}

#[cfg(not(test))]
fn debug_validate_maroon_stack(_: &Vec<MaroonTaskStackEntry>) {}

pub async fn execute_pending_operations<T: Timer, W: Writer>(
  mut state: Arc<AppState<T, W>>,
  verbose: bool,
) {
  loop {
    execute_pending_operations_inner(&mut state, verbose).await;

    // NOTE(dkorolev): I will eventually rewrite this w/o busy waiting.
    tokio::time::sleep(Duration::from_millis(10)).await;
  }
}

pub async fn execute_pending_operations_inner<T: Timer, W: Writer>(
  state: &mut Arc<AppState<T, W>>,
  verbose: bool,
) {
  loop {
    let mut fsm = state.fsm.lock().await;
    let scheduled_timestamp_cutoff: LogicalTimeAbsoluteMs = state.timer.millis_since_start();
    if let Some((task_id, scheduled_timestamp)) = {
      // The `.map()` -> `.filter()` is to not keep the `.peek()`-ed reference.
      fsm
        .pending_operations
        .peek()
        .map(|t| t.scheduled_timestamp <= scheduled_timestamp_cutoff)
        .filter(|b| *b)
        .and_then(|_| fsm.pending_operations.pop())
        .map(|t| (t.task_id, t.scheduled_timestamp))
    } {
      let mut maroon_task =
        fsm.active_tasks.remove(&task_id).expect("The task just retrieved from `fsm.pending_operations` should exist.");

      if verbose {
        println!("MAROON STEP AT T={scheduled_timestamp}ms");
        for e in &maroon_task.maroon_stack.maroon_stack_entries {
          match e {
            MaroonTaskStackEntry::State(s) => {
              let n = maroon_task_state_local_vars_count(&s);
              println!("  state: {s:?}, uses {n} argument(s) above as its local stack.");
            }
            MaroonTaskStackEntry::Retrn(r) => {
              println!("  retrn: {r:?}, awaiting to be `return`-ed into here.");
            }
            MaroonTaskStackEntry::Value(v) => {
              println!("  value: {v:?}");
            }
          }
        }
      }

      // Before any work is done, let's validate the maroon stack, to be safe.
      debug_validate_maroon_stack(&maroon_task.maroon_stack.maroon_stack_entries);

      let current_stack_entry = maroon_task
        .maroon_stack
        .maroon_stack_entries
        .pop()
        .expect("The active task should have at least one state in call stack.");

      // Extract the state from the top of the stack. It should be a state, not a value.
      let current_state = if let MaroonTaskStackEntry::State(state) = &current_stack_entry {
        state.clone()
      } else {
        panic!("Expected a state at the top of the stack, found a value.");
      };

      // Extract the values.
      // TODO(dkorolev): They should not be extracted, it should just be a wrapper for the future, of course.
      let number_of_vars_in_context = maroon_task_state_local_vars_count(&current_state);
      let mut vars = Vec::new();
      for _ in 0..number_of_vars_in_context {
        if let Some(MaroonTaskStackEntry::Value(v)) = maroon_task.maroon_stack.maroon_stack_entries.pop() {
          vars.push(v);
        } else {
          panic!("The value on the stack appears to be a 'function', not a `value`, aborting.");
        }
      }

      let step_result = global_step(current_state, vars, &mut maroon_task.maroon_heap);
      if verbose {
        println!("MAROON STEP RESULT\n  {step_result:?}");
      }
      match step_result {
        MaroonStepResult::Done => {
          // no need to do anything here since we've removed the task before
        }
        MaroonStepResult::Sleep(sleep_ms, new_states_vec) => {
          for new_state in new_states_vec.into_iter() {
            maroon_task.maroon_stack.maroon_stack_entries.push(new_state);
          }
          debug_validate_maroon_stack(&maroon_task.maroon_stack.maroon_stack_entries);
          let scheduled_timestamp = scheduled_timestamp + sleep_ms;
          fsm.active_tasks.insert(task_id, maroon_task);
          fsm.pending_operations.push(TimestampedMaroonTask::new(scheduled_timestamp, task_id));
        }
        MaroonStepResult::Write(text, new_states_vec) => {
          let _ = maroon_task.writer.write_text(text, Some(scheduled_timestamp)).await;
          for new_state in new_states_vec.into_iter() {
            maroon_task.maroon_stack.maroon_stack_entries.push(new_state);
          }
          debug_validate_maroon_stack(&maroon_task.maroon_stack.maroon_stack_entries);
          fsm.active_tasks.insert(task_id, maroon_task);
          fsm.pending_operations.push(TimestampedMaroonTask::new(scheduled_timestamp, task_id));
        }
        MaroonStepResult::Send(message, new_states_vec) => {
          if let Some(awaiter) = fsm.awaiter.take() {
            let Some(mut awaiting_task) = fsm.active_tasks.remove(&awaiter) else {
              _ = maroon_task
                .writer
                .write_text("internal error. Awaiter is not running", Some(scheduled_timestamp))
                .await;
              continue;
            };

            _ = maroon_task.writer.write_text("message has been sent", Some(scheduled_timestamp)).await;

            for new_state in new_states_vec.into_iter() {
              maroon_task.maroon_stack.maroon_stack_entries.push(new_state);
            }
            debug_validate_maroon_stack(&maroon_task.maroon_stack.maroon_stack_entries);

            fsm.active_tasks.insert(task_id, maroon_task);
            fsm.pending_operations.push(TimestampedMaroonTask::new(scheduled_timestamp, task_id));

            awaiting_task
              .maroon_stack
              .maroon_stack_entries
              .push(MaroonTaskStackEntry::Value(MaroonTaskStackEntryValue::AwaiterInputMessage(message)));
            awaiting_task
              .maroon_stack
              .maroon_stack_entries
              .push(MaroonTaskStackEntry::State(MaroonTaskState::WaiterWaitMessageAwaiting));

            fsm.active_tasks.insert(awaiter, awaiting_task);
            fsm.pending_operations.push(TimestampedMaroonTask::new(scheduled_timestamp, awaiter));
          } else {
            _ = maroon_task.writer.write_text("no awaiter. closing", Some(scheduled_timestamp)).await;
          };
        }
        MaroonStepResult::Next(new_states_vec) => {
          for new_state in new_states_vec.into_iter() {
            maroon_task.maroon_stack.maroon_stack_entries.push(new_state);
          }
          debug_validate_maroon_stack(&maroon_task.maroon_stack.maroon_stack_entries);
          fsm.active_tasks.insert(task_id, maroon_task);
          fsm.pending_operations.push(TimestampedMaroonTask::new(scheduled_timestamp, task_id));
        }
        MaroonStepResult::Return(retval) => {
          let popped = maroon_task.maroon_stack.maroon_stack_entries.pop().expect("Nowhere to `Return` to!");
          if let MaroonTaskStackEntry::Retrn(next_state) = popped {
            maroon_task.maroon_stack.maroon_stack_entries.push(MaroonTaskStackEntry::Value(retval));
            maroon_task.maroon_stack.maroon_stack_entries.push(MaroonTaskStackEntry::State(next_state));
            fsm.active_tasks.insert(task_id, maroon_task);
            fsm.pending_operations.push(TimestampedMaroonTask::new(scheduled_timestamp, task_id));
          } else {
            panic!("Should be `Return`-ing to a state, has `{:?}` on top of the stack instead.", popped);
          }
        }
      }
      if verbose {
        println!("MAROON STEP DONE\n");
      }
    } else {
      break;
    }
  }
}
