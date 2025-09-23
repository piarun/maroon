use crate::simple_function::ir::sample_ir;
use crate::simple_function::{self, generated::*};
use crate::{
  ir::{FiberType, IR, LogicalTimeAbsoluteMs},
  simple_function::{generated::Heap, task::*},
};
use std::hash::Hash;
use std::thread::sleep;
use std::time::Duration;
use std::{
  collections::{BinaryHeap, HashMap, LinkedList, VecDeque},
  env::var,
};

pub trait Timer: Send + Sync + 'static {
  fn from_start(&self) -> LogicalTimeAbsoluteMs;
  fn monotonic_now_system(&self) -> std::time::SystemTime;
}

// Runtime-only Future identifier. Unique per-fiber using suffixing policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FutureId(pub String);
impl std::fmt::Display for FutureId {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl FutureId {
  pub fn new(id: impl Into<String>) -> Self {
    Self(id.into())
  }

  pub fn from_label(
    label: crate::ir::FutureLabel,
    unique_id: u64,
  ) -> Self {
    Self(format!("{}_{}", label.0, unique_id))
  }
}

pub struct MonotonicTimer {
  instant: std::time::Instant,
  system: std::time::SystemTime,
}

impl MonotonicTimer {
  pub fn new() -> MonotonicTimer {
    MonotonicTimer { instant: std::time::Instant::now(), system: std::time::SystemTime::now() }
  }

  // create a timer that already has `elapsed` time accrued
  #[cfg(test)]
  pub fn with_elapsed(elapsed: std::time::Duration) -> Self {
    let now_instant = std::time::Instant::now();
    let now_system = std::time::SystemTime::now();

    let instant = now_instant.checked_sub(elapsed).unwrap_or(now_instant);
    let system = now_system.checked_sub(elapsed).unwrap_or(now_system);

    MonotonicTimer { instant, system }
  }
  #[cfg(test)]
  pub fn with_elapsed_ms(ms: u64) -> Self {
    MonotonicTimer::with_elapsed(std::time::Duration::from_millis(ms))
  }
}

impl Timer for MonotonicTimer {
  fn from_start(&self) -> LogicalTimeAbsoluteMs {
    LogicalTimeAbsoluteMs(self.instant.elapsed().as_millis() as u64)
  }

  fn monotonic_now_system(&self) -> std::time::SystemTime {
    self.system + self.instant.elapsed()
  }
}

#[derive(Debug, Clone)]
struct TaskBlueprint {
  // TODO: make it `UniqueU64BlobId` from `common` crate
  // global_id, the same that is coming from gateways, globally unique
  global_id: u64,

  fiber_type: FiberType,
  // function key to provide an information which function should be executed, ex: `add` or `sub`...
  function_key: String,
  // input parameters for the function
  init_values: Vec<Value>,
}

#[derive(Debug)]
struct ScheduledBlob {
  when: LogicalTimeAbsoluteMs,
  what: FutureId,
}
impl std::fmt::Display for ScheduledBlob {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{} {}", self.when, self.what)
  }
}

impl Eq for ScheduledBlob {}

impl Ord for ScheduledBlob {
  fn cmp(
    &self,
    other: &Self,
  ) -> std::cmp::Ordering {
    // it will be used in BinaryHeap, so I do this intentionally to not do Reverse() all the time
    other.when.cmp(&self.when)
  }
}

impl PartialEq for ScheduledBlob {
  fn eq(
    &self,
    other: &Self,
  ) -> bool {
    self.when == other.when
  }
}

impl PartialOrd for ScheduledBlob {
  fn partial_cmp(
    &self,
    other: &Self,
  ) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}
struct Runtime<T: Timer> {
  // Execution priority
  // Executors goes to the next step only if there is no work on previous steps
  //
  // - active_fibers
  // - fiber_in_message_queue
  // - active_tasks
  //
  // 1. run scheduled operations(if there are)
  // 2. run until there are no active_fibers
  // 3. take in_message from fiber_in_message_queue, convert it into active_fibers, go to step 1
  // 4. take taskBlueprint from active_tasks, convert it into active_fibers, go to step 1

  // this is the input for the engine, here new tasks from commited epochs will be coming in the commited order
  active_tasks: LinkedList<(LogicalTimeAbsoluteMs, VecDeque<TaskBlueprint>)>,

  // fibers that can be executed
  active_fibers: VecDeque<Fiber>,
  // fibers that have some tasks, but can't be executed because they're awaiting something
  parked_fibers: HashMap<FutureId, FiberBox>,

  scheduled: BinaryHeap<ScheduledBlob>,

  // created but idle fibers
  fiber_pool: HashMap<FiberType, Vec<Fiber>>,

  // queue for in_messages that will be executed in the order when fiber is available
  // Vec - for predictable order
  fiber_in_message_queue: Vec<(FiberType, VecDeque<FiberInMessage>)>,

  fiber_limiter: HashMap<FiberType, u64>,

  // results. key - is global_id from TaskBlueprint
  // TODO: make it `UniqueU64BlobId` from `common` crate
  results: HashMap<u64, Value>,

  timer: T,

  // monotonically increasing id for newly created fibers
  next_fiber_id: u64,
}

struct FiberBox {
  fiber: Fiber,

  // information to which variable on stack we should bind the result for the `fiber`
  // is used when fiber is parked and awaits some result
  result_var_bind: Option<String>,
}

#[derive(Debug)]
struct FiberInMessage {
  fiber_type: FiberType,
  function_name: String,
  args: Vec<Value>,
  context: Option<RunContext>,
}

impl<T: Timer> Runtime<T> {
  pub fn new(
    timer: T,
    ir: IR,
  ) -> Runtime<T> {
    Runtime {
      fiber_limiter: ir.fibers.iter().map(|fi| (fi.0.clone(), fi.1.fibers_limit)).collect(),
      active_fibers: VecDeque::new(),
      active_tasks: LinkedList::new(),
      parked_fibers: HashMap::new(),
      scheduled: BinaryHeap::new(),
      fiber_pool: HashMap::new(),
      fiber_in_message_queue: ir.fibers.iter().map(|f| (f.0.clone(), VecDeque::default())).collect(),
      results: HashMap::new(),
      timer: timer,
      next_fiber_id: 0,
    }
  }

  pub fn dump(&self) {
    println!(
      r"------STATE---------
time: {}
scheduled: 
{}
active fibers:
{}
in_message queue:
{}
fiber_pool:
{}
limiter:
{}
-----END STATE------",
      self.timer.from_start(),
      self.scheduled.iter().map(|s| format!("  t:{} f:{}", s.when, s.what)).collect::<Vec<String>>().join("\n"),
      self
        .active_fibers
        .iter()
        .map(|s| format!("  {}:{}.{}", s.unique_id, s.f_type, s.function_key))
        .collect::<Vec<String>>()
        .join("\n"),
      self.fiber_in_message_queue.iter().map(|s| format!("  {} {:?}", s.0, s.1)).collect::<Vec<String>>().join("\n"),
      self.fiber_pool.iter().map(|s| format!("  {} {:?}", s.0, s.1)).collect::<Vec<String>>().join("\n"),
      self.fiber_limiter.iter().map(|s| format!("  {} {:?}", s.0, s.1)).collect::<Vec<String>>().join("\n")
    );
  }

  pub fn push_fiber_in_message(
    &mut self,
    _type: &FiberType,
    msg: FiberInMessage,
  ) {
    self
      .fiber_in_message_queue
      .iter_mut()
      .find(|(t, _)| t == _type)
      .expect("IR initalization was incorrect, all types should be here")
      .1
      .push_back(msg);
  }

  pub fn next_batch(
    &mut self,
    time: LogicalTimeAbsoluteMs,
    blueprints: VecDeque<TaskBlueprint>,
  ) {
    self.active_tasks.push_back((time, blueprints));
  }

  // returns None if there is no idle Fibers and the limit has reached
  pub fn get_fiber(
    &mut self,
    f_type: &FiberType,
  ) -> Option<Fiber> {
    if let Some(fiber) = self.fiber_pool.get_mut(f_type).and_then(Vec::pop) {
      return Some(fiber);
    }

    let limit = self.fiber_limiter.get_mut(f_type).expect("you shouldnt create tasks that are not part of ir");

    if *limit == 0 {
      return None;
    }

    *limit -= 1;
    let id = self.next_fiber_id;
    self.next_fiber_id += 1;
    return Some(Fiber::new(f_type.clone(), id));
  }

  pub fn has_available_fiber(
    &self,
    f_type: &FiberType,
  ) -> bool {
    !self.fiber_pool.get(f_type).is_none_or(Vec::is_empty) || self.fiber_limiter.get(f_type).is_some_and(|x| *x > 0)
  }
  pub fn run(&mut self) {
    let mut counter = 0;
    'main_loop: loop {
      counter += 1;
      if counter > 100 {
        // TODO: just for tests, and for now, actually it should be an infinite loop
        return;
      }

      let now = self.timer.from_start();

      // take scheduled Fibers and push them to active_fibers if it's time to work on them
      while let Some(blob) = self.scheduled.peek() {
        if now < blob.when {
          break;
        }

        let blob = self.scheduled.pop().unwrap();

        let Some(task_box) = self.parked_fibers.remove(&blob.what) else {
          continue;
        };

        // TODO: make a reverse order of these extracted tasks
        // because right now I will pick the earliest first
        // but if there are several tasks that should be executed - the earlist one will be executed the latest
        self.active_fibers.push_front(task_box.fiber);
      }

      // work on active fibers(state-machine iterations moves)
      while let Some(mut fiber) = self.active_fibers.pop_front() {
        match fiber.run() {
          RunResult::Done(result) => {
            println!("FIBER {} IS FINISHED. Result: {:?}", &fiber, result);

            let options = fiber.context.clone();
            // TODO: when fiber type won't be a string - remove this clone
            self.fiber_pool.entry(fiber.f_type.clone()).or_default().push(fiber);

            if let Some(global_id) = options.global_id {
              self.results.insert(global_id, result.clone());
            }

            let Some(future_id) = options.future_id else {
              continue;
            };
            let Some(mut task_box) = self.parked_fibers.remove(&future_id) else {
              continue;
            };

            if let Some(var) = task_box.result_var_bind {
              task_box.fiber.assign_local(var, result);
            }
            self.active_fibers.push_front(task_box.fiber);
          }
          RunResult::AsyncCall { f_type, func, args, future_id } => {
            if let Some(mut available_fiber) = self.get_fiber(&f_type) {
              available_fiber.load_task(func, args, Some(RunContext { future_id: Some(future_id), global_id: None }));
              // TODO: in that case when task will be finished with work - asynced available_fiber will be taken for execution
              self.active_fibers.push_front(available_fiber);
            } else {
              self.push_fiber_in_message(
                &f_type,
                FiberInMessage {
                  fiber_type: f_type.clone(),
                  function_name: func,
                  args,
                  context: Some(RunContext { future_id: Some(future_id), global_id: None }),
                },
              );
            }

            self.active_fibers.push_front(fiber);
          }
          RunResult::Await(future_id, var_bind) => {
            // specify bind parameters here
            self.parked_fibers.insert(future_id, FiberBox { fiber: fiber, result_var_bind: var_bind });
          }
          RunResult::ScheduleTimer { ms, future_id } => {
            self.scheduled.push(ScheduledBlob { when: self.timer.from_start() + ms, what: future_id });
            self.active_fibers.push_front(fiber);
          }
        }
      }

      // get in_message and put it into available Fiber(if there is one)
      // here I don't put all of them into work, but pick the first one and immediately start execution
      let to_push: Option<(FiberType, usize)> =
        self.fiber_in_message_queue.iter().enumerate().find_map(|(index, (f_type, in_msg_queue))| {
          if !in_msg_queue.is_empty() && self.has_available_fiber(f_type) {
            Some((f_type.clone(), index))
          } else {
            None
          }
        });

      // I'm doing this in two steps because of borrow protection, I can't iter_mut and (mut self).get_fiber() inside, so I'm finding the index first, and the mutating
      if let Some((f_type, index)) = to_push {
        let msg = self.fiber_in_message_queue.get_mut(index).expect("checked").1.pop_front().expect("checked");
        let mut available_fiber = self.get_fiber(&f_type).expect("checked before");
        available_fiber.load_task(msg.function_name, msg.args, msg.context);
        self.active_fibers.push_front(available_fiber);
        continue 'main_loop;
      };

      'process_active_tasks: loop {
        let now = self.timer.from_start();

        let Some((time_stamp, mut current_queue)) = self.active_tasks.pop_front() else {
          // TODO: a bit smarter sleep, if there are no next elements or select or smth
          println!("nothing in active_tasks. sleep 5ms");
          sleep(Duration::from_millis(5));
          break;
        };

        if time_stamp > now {
          // TODO: not continue but some sleep, since we shouldn't work on it yet + we shouldn't just waste CPU cycles
          // but it should be probably not just sleep, but select or smth
          let sleep_distance = time_stamp.0 - now.0;
          sleep(Duration::from_millis(sleep_distance));
          println!("smth in active_tasks, but not now, sleeping {}ms", sleep_distance);
          self.active_tasks.push_front((time_stamp, current_queue));
          break;
        }

        while let Some(blueprint) = current_queue.pop_front() {
          if let Some(mut fiber) = self.get_fiber(&blueprint.fiber_type) {
            fiber.load_task(
              blueprint.function_key,
              blueprint.init_values,
              Some(RunContext { future_id: None, global_id: Some(blueprint.global_id) }),
            );
            self.active_fibers.push_back(fiber);

            if !current_queue.is_empty() {
              self.active_tasks.push_front((time_stamp, current_queue));
            }
            break 'process_active_tasks;
          } else {
            let ftype = blueprint.fiber_type.clone();
            self.push_fiber_in_message(
              &ftype,
              FiberInMessage {
                fiber_type: ftype.clone(),
                function_name: blueprint.function_key,
                args: blueprint.init_values,
                context: Some(RunContext { future_id: None, global_id: Some(blueprint.global_id) }),
              },
            );
          }
        }
      }
    }
  }
}

#[test]
fn some_test() {
  let mut rt = Runtime::new(MonotonicTimer::new(), sample_ir());

  rt.next_batch(
    LogicalTimeAbsoluteMs(10),
    VecDeque::from([
      TaskBlueprint {
        global_id: 300,
        fiber_type: FiberType::new("application"),
        function_key: "async_foo".to_string(),
        init_values: vec![Value::U64(4), Value::U64(8)],
      },
      TaskBlueprint {
        global_id: 1,
        fiber_type: FiberType::new("application"),
        function_key: "async_foo".to_string(),
        init_values: vec![Value::U64(0), Value::U64(8)],
      },
    ]),
  );

  rt.run();

  assert_eq!(HashMap::from([(300, Value::U64(12)), (1, Value::U64(8))]), rt.results);
}

#[test]
fn sleep_test() {
  let mut rt = Runtime::new(MonotonicTimer::with_elapsed_ms(5), sample_ir());

  rt.next_batch(
    LogicalTimeAbsoluteMs(10),
    VecDeque::from([TaskBlueprint {
      global_id: 9,
      fiber_type: FiberType::new("application"),
      function_key: "sleep_and_pow".to_string(),
      init_values: vec![Value::U64(2), Value::U64(4)],
    }]),
  );

  rt.run();

  assert_eq!(HashMap::from([(9, Value::U64(16))]), rt.results);
}

#[test]
fn multiple_await() {
  let mut rt = Runtime::new(MonotonicTimer::with_elapsed_ms(5), sample_ir());

  // Cases to cover:
  // - many awaiting fibers of the same function
  // - IR has limitation for application - 2, so some of them will be executed immediately, some will go to in_message queue
  rt.next_batch(
    LogicalTimeAbsoluteMs(10),
    VecDeque::from([
      TaskBlueprint {
        global_id: 9,
        fiber_type: FiberType::new("application"),
        function_key: "sleep_and_pow".to_string(),
        init_values: vec![Value::U64(2), Value::U64(4)],
      },
      TaskBlueprint {
        global_id: 10,
        fiber_type: FiberType::new("application"),
        function_key: "sleep_and_pow".to_string(),
        init_values: vec![Value::U64(2), Value::U64(8)],
      },
      TaskBlueprint {
        global_id: 11,
        fiber_type: FiberType::new("application"),
        function_key: "sleep_and_pow".to_string(),
        init_values: vec![Value::U64(2), Value::U64(7)],
      },
      TaskBlueprint {
        global_id: 12,
        fiber_type: FiberType::new("application"),
        function_key: "sleep_and_pow".to_string(),
        init_values: vec![Value::U64(2), Value::U64(7)],
      },
      TaskBlueprint {
        global_id: 13,
        fiber_type: FiberType::new("application"),
        function_key: "sleep_and_pow".to_string(),
        init_values: vec![Value::U64(2), Value::U64(7)],
      },
    ]),
  );

  rt.run();

  assert_eq!(
    HashMap::from([
      (9, Value::U64(16)),
      (10, Value::U64(256)),
      (11, Value::U64(128)),
      (12, Value::U64(128)),
      (13, Value::U64(128))
    ]),
    rt.results
  );
}
