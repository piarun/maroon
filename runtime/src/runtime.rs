use crate::fiber::*;
use common::duplex_channel::Endpoint;
use common::logical_clock::Timer;
use common::logical_time::LogicalTimeAbsoluteMs;
use common::range_key::UniqueU64BlobId;
use dsl::ir::{FiberType, IR};
use generated::maroon_assembler::Value;
use std::collections::{BinaryHeap, HashMap, LinkedList, VecDeque};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBlueprint {
  pub global_id: UniqueU64BlobId,

  pub fiber_type: FiberType,
  // function key to provide an information which function should be executed, ex: `add` or `sub`...
  pub function_key: String,
  // input parameters for the function
  pub init_values: Vec<Value>,
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
    write!(f, "{}@{}", self.when, self.what)
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

pub type Input = (LogicalTimeAbsoluteMs, Vec<TaskBlueprint>);
pub type Output = (UniqueU64BlobId, Value);
// TODO: Don't like these names, I think it makes sense to have them.
// It provides a bit more clarity and clearnes, but should think more no naming
pub type B2AEndpoint = Endpoint<Output, Input>;
pub type A2BEndpoint = Endpoint<Input, Output>;

pub struct Runtime<T: Timer> {
  // communication interface
  interface: B2AEndpoint,

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
    interface: Endpoint<(UniqueU64BlobId, Value), (LogicalTimeAbsoluteMs, Vec<TaskBlueprint>)>,
  ) -> Runtime<T> {
    Runtime {
      fiber_limiter: ir.fibers.iter().map(|fi| (fi.0.clone(), fi.1.fibers_limit)).collect(),
      active_fibers: VecDeque::new(),
      active_tasks: LinkedList::new(),
      parked_fibers: HashMap::new(),
      scheduled: BinaryHeap::new(),
      fiber_pool: HashMap::new(),
      fiber_in_message_queue: ir.fibers.iter().map(|f| (f.0.clone(), VecDeque::default())).collect(),
      timer: timer,
      next_fiber_id: 0,

      interface,
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

  fn push_fiber_in_message(
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
    return Some(Fiber::new_empty(f_type.clone(), id));
  }

  pub fn has_available_fiber(
    &self,
    f_type: &FiberType,
  ) -> bool {
    !self.fiber_pool.get(f_type).is_none_or(Vec::is_empty) || self.fiber_limiter.get(f_type).is_some_and(|x| *x > 0)
  }
  pub async fn run(&mut self) {
    'main_loop: loop {
      let now = self.timer.from_start();

      // take scheduled Fibers and push them to active_fibers if it's time to work on them
      if let Some(blob) = self.scheduled.peek() {
        if now >= blob.when {
          let blob = self.scheduled.pop().unwrap();

          if let Some(task_box) = self.parked_fibers.remove(&blob.what) {
            self.active_fibers.push_front(task_box.fiber);
          };
        }
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
              // I'm ignoring an error here
              // because if there is an error - the receiving channel is closed
              // if it's closed due to shutdown or some error, doesn't matter => current level errors don't really matter
              self.interface.send((global_id, result.clone()));
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
          RunResult::Select(_states) => {}
          RunResult::SetValues(_values) => {}
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

        let mut next = self.active_tasks.pop_front();
        if next.is_none() {
          if self.interface.receiver.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
            break;
          } else {
            let (time, requests) = self.interface.receiver.recv().await.expect("checked, not empty");
            next = Some((time, VecDeque::from(requests)));
          }
        }

        let Some((time_stamp, mut current_queue)) = next else {
          break;
        };

        if time_stamp > now {
          // TODO: not continue but some sleep, since we shouldn't work on it yet + we shouldn't just waste CPU cycles
          // but it should be probably not just sleep, but select or smth
          let sleep_distance = time_stamp.0 - now.0;
          tokio::time::sleep(Duration::from_millis(sleep_distance)).await;
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

#[cfg(test)]
mod tests {
  use crate::ir_spec::sample_ir;
  use common::duplex_channel::create_a_b_duplex_pair;
  use common::logical_clock::MonotonicTimer;
  use std::fmt::Debug;
  use tokio::sync::mpsc::UnboundedReceiver;

  use super::*;

  #[tokio::test(flavor = "multi_thread")]
  async fn some_test() {
    let (a2b_runtime, b2a_runtime) =
      create_a_b_duplex_pair::<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>();

    let mut rt = Runtime::new(MonotonicTimer::new(), sample_ir(), b2a_runtime);

    tokio::spawn(async move {
      rt.run().await;
    });

    _ = a2b_runtime.send((
      LogicalTimeAbsoluteMs(10),
      vec![
        TaskBlueprint {
          global_id: UniqueU64BlobId(300),
          fiber_type: FiberType::new("application"),
          function_key: "async_foo".to_string(),
          init_values: vec![Value::U64(4), Value::U64(8)],
        },
        TaskBlueprint {
          global_id: UniqueU64BlobId(1),
          fiber_type: FiberType::new("application"),
          function_key: "async_foo".to_string(),
          init_values: vec![Value::U64(0), Value::U64(8)],
        },
      ],
    ));

    compare_channel_data_with_exp(
      vec![(UniqueU64BlobId(300), Value::U64(12)), (UniqueU64BlobId(1), Value::U64(8))],
      a2b_runtime.receiver,
    )
    .await;
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn sleep_test() {
    let (a2b_runtime, b2a_runtime) =
      create_a_b_duplex_pair::<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>();

    let mut rt = Runtime::new(MonotonicTimer::new(), sample_ir(), b2a_runtime);

    tokio::spawn(async move {
      rt.run().await;
    });

    _ = a2b_runtime.send((
      LogicalTimeAbsoluteMs(10),
      vec![TaskBlueprint {
        global_id: UniqueU64BlobId(9),
        fiber_type: FiberType::new("application"),
        function_key: "sleep_and_pow".to_string(),
        init_values: vec![Value::U64(2), Value::U64(4)],
      }],
    ));

    compare_channel_data_with_exp(vec![(UniqueU64BlobId(9), Value::U64(16))], a2b_runtime.receiver).await;
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn multiple_await() {
    let (a2b_runtime, b2a_runtime) =
      create_a_b_duplex_pair::<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>();
    let mut rt = Runtime::new(MonotonicTimer::new(), sample_ir(), b2a_runtime);

    tokio::spawn(async move {
      rt.run().await;
    });

    // Cases to cover:
    // - many awaiting fibers of the same function
    // - IR has limitation for application - 2, so some of them will be executed immediately, some will go to in_message queue
    _ = a2b_runtime.send((
      LogicalTimeAbsoluteMs(10),
      vec![
        TaskBlueprint {
          global_id: UniqueU64BlobId(9),
          fiber_type: FiberType::new("application"),
          function_key: "sleep_and_pow".to_string(),
          init_values: vec![Value::U64(2), Value::U64(4)],
        },
        TaskBlueprint {
          global_id: UniqueU64BlobId(10),
          fiber_type: FiberType::new("application"),
          function_key: "sleep_and_pow".to_string(),
          init_values: vec![Value::U64(2), Value::U64(8)],
        },
        TaskBlueprint {
          global_id: UniqueU64BlobId(300),
          fiber_type: FiberType::new("global"),
          function_key: "add".to_string(),
          init_values: vec![Value::U64(2), Value::U64(8)],
        },
        TaskBlueprint {
          global_id: UniqueU64BlobId(11),
          fiber_type: FiberType::new("application"),
          function_key: "sleep_and_pow".to_string(),
          init_values: vec![Value::U64(2), Value::U64(7)],
        },
        TaskBlueprint {
          global_id: UniqueU64BlobId(12),
          fiber_type: FiberType::new("application"),
          function_key: "sleep_and_pow".to_string(),
          init_values: vec![Value::U64(2), Value::U64(7)],
        },
        TaskBlueprint {
          global_id: UniqueU64BlobId(13),
          fiber_type: FiberType::new("application"),
          function_key: "sleep_and_pow".to_string(),
          init_values: vec![Value::U64(2), Value::U64(7)],
        },
      ],
    ));

    compare_channel_data_with_exp(
      vec![
        (UniqueU64BlobId(300), Value::U64(10)),
        (UniqueU64BlobId(9), Value::U64(16)),
        (UniqueU64BlobId(10), Value::U64(256)),
        (UniqueU64BlobId(11), Value::U64(128)),
        (UniqueU64BlobId(12), Value::U64(128)),
        (UniqueU64BlobId(13), Value::U64(128)),
      ],
      a2b_runtime.receiver,
    )
    .await;
  }

  async fn compare_channel_data_with_exp<T: PartialEq + Debug>(
    expected: Vec<T>,
    mut ch: UnboundedReceiver<T>,
  ) {
    for exp in expected.into_iter() {
      let got = ch.recv().await.expect("result channel closed early");
      assert_eq!(exp, got);
    }
    // Ensure there are no extra messages
    assert!(ch.try_recv().is_err());
  }
}
