use crate::fiber::*;
use crate::wait_registry::{WaitKey, WaitRegistry};
use common::duplex_channel::Endpoint;
use common::logical_clock::Timer;
use common::logical_time::LogicalTimeAbsoluteMs;
use common::range_key::UniqueU64BlobId;
use dsl::ir::{FiberType, IR};
use generated::maroon_assembler::{
  CreatePrimitiveValue, SetPrimitiveValue, StackEntry, SuccessBindKind, Value, pub_to_private, wrap_future_id,
};
use std::collections::{BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBlueprint {
  pub global_id: UniqueU64BlobId,
  pub source: TaskBPSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskBPSource {
  FiberFunc {
    fiber_type: FiberType,
    // function key to provide an information which function should be executed, ex: `add` or `sub`...
    function_key: String,
    // input parameters for the function
    init_values: Vec<Value>,
  },
  Queue {
    q_name: String,
    value: Value,
  },
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
  // deprecating
  parked_fibers: HashMap<FutureId, FiberBox>,

  scheduled: BinaryHeap<ScheduledBlob>,

  // created but idle fibers
  // deprecating
  fiber_pool: HashMap<FiberType, Vec<Fiber>>,

  // queue for in_messages that will be executed in the order when fiber is available
  // Vec - for predictable order
  // deprecating
  fiber_in_message_queue: Vec<(FiberType, VecDeque<FiberInMessage>)>,

  // deprecating
  fiber_limiter: HashMap<FiberType, u64>,

  timer: T,

  // monotonically increasing id for newly created fibers
  next_fiber_id: u64,
  // monotonically increasing id for newly created futures (via Create)
  next_created_future_id: u64,
  /// only for futures that are linked to the external messages(that are coming from gateways)
  public_futures: HashMap<String, UniqueU64BlobId>,

  /// message queues
  /// key - queue name
  /// value - queue of messages
  queue_messages: HashMap<String, VecDeque<Value>>,
  /// order of non-empty queues in which I should check queues
  /// when smth adds message to the empty `queue_messages` - it should add queueName to this queue
  /// when smth works with this list it should:
  /// 1. pop_front
  /// 2. find queue in `queue_messages`
  /// 3. If a waiter exists for queue, pop one message and wake it; otherwise rotate q without poppingpop front message from step 2
  /// 4. if queue is empty - don't add name here
  /// 5. requeue q only if queue_messages[q] is still non-empty
  non_empty_queues: VecDeque<String>,
  /// push resolved futures with their results
  /// fiber awakening will happen in the same order as resolved futures get into the queue
  resolved_futures: VecDeque<(FutureId, Value)>,

  /// Shared debug output sink used by all fibers, safe to share with tests
  /// I don't think it's a good way of doing it longterm,
  ///  but I don't see obvious problems and limitations right now that would justify more advanced solution
  dbg_out: Arc<Mutex<String>>,

  /// Registry for fibers awaiting on multiple sources (queues/futures) via Select
  wait_index: WaitRegistry,
  /// parked fibers that are awaiting smth
  /// key - fiber_id
  awaiting_fibers: HashMap<u64, Fiber>,
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
      next_created_future_id: 0,
      public_futures: HashMap::new(),
      awaiting_fibers: HashMap::new(),

      queue_messages: HashMap::new(),
      non_empty_queues: VecDeque::new(),
      resolved_futures: VecDeque::new(),

      dbg_out: Arc::new(Mutex::new(String::new())),

      wait_index: WaitRegistry::default(),

      interface,
    }
  }

  /// Returns a clone of the debug output handle for external readers (e.g., tests).
  pub fn debug_handle(&self) -> Arc<Mutex<String>> {
    self.dbg_out.clone()
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

  pub async fn run(
    &mut self,
    root_type: String,
  ) {
    let root = Fiber::new(FiberType(root_type), 0, &vec![]);
    self.next_fiber_id = 1;
    self.active_fibers.push_back(root);

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
        // Accumulate debug output locally, then append with a single lock
        let mut local_dbg = String::new();
        local_dbg.push_str(&format!("--- start {}:{} ---\n", fiber.f_type, fiber.unique_id));
        let res = fiber.run(&mut local_dbg);
        local_dbg.push_str(&format!("--- await {}:{} ---\n", fiber.f_type, fiber.unique_id));
        match res {
          RunResult::Done(result) => {
            local_dbg.push_str(&format!("--- exit {}:{} ---\n", fiber.f_type, fiber.unique_id));

            let options = fiber.context.clone();
            // TODO: when fiber type won't be a string - remove this clone
            self.fiber_pool.entry(fiber.f_type.clone()).or_default().push(fiber);

            if let Some(global_id) = options.global_id {
              // I'm ignoring an error here
              // because if there is an error - the receiving channel is closed
              // if it's closed due to shutdown or some error, doesn't matter => current level errors don't really matter
              self.interface.send((global_id, result.clone()));
            }

            if let Some(future_id) = options.future_id {
              if let Some(mut task_box) = self.parked_fibers.remove(&future_id) {
                if let Some(var) = task_box.result_var_bind {
                  task_box.fiber.assign_local(var, result);
                }
                self.active_fibers.push_front(task_box.fiber);
              }
            }
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
          RunResult::AwaitOld(future_id, var_bind) => {
            // legacy variant: same handling as Await
            self.parked_fibers.insert(future_id, FiberBox { fiber: fiber, result_var_bind: var_bind });
          }
          RunResult::ScheduleTimer { ms, future_id } => {
            self.scheduled.push(ScheduledBlob { when: self.timer.from_start() + ms, what: future_id });
            self.active_fibers.push_front(fiber);
          }
          RunResult::Select(states) => {
            self.wait_index.register_select(fiber.unique_id, states);
            self.awaiting_fibers.insert(fiber.unique_id, fiber);
          }
          RunResult::CreateFibers { details } => {
            for (f_type, init_vars) in details {
              local_dbg.push_str(&format!("created: {:?}:{}. init_vars:\n", f_type, self.next_fiber_id));
              for v in init_vars.iter() {
                local_dbg.push_str(&format!("    {:?}\n", v));
              }
              let nf = Fiber::new(f_type, self.next_fiber_id, &init_vars);
              self.next_fiber_id += 1;
              self.active_fibers.push_back(nf);
            }
            self.active_fibers.push_front(fiber);
          }
          RunResult::SetValues(values) => {
            for v in values {
              match v {
                SetPrimitiveValue::QueueMessage { queue_name, value } => {
                  if let Some(queue) = self.queue_messages.get_mut(&queue_name) {
                    let is_empty = queue.is_empty();
                    queue.push_back(value);
                    if is_empty {
                      self.non_empty_queues.push_back(queue_name);
                    }
                  } else {
                    // todo: how to send an error here? should I send an error here?
                    panic!("it means smb is trying to send value to non-existing queue");
                    // self.queue_messages.insert(queue_name.clone(), VecDeque::from(vec![value]));
                    // self.non_empty_queues.push_back(queue_name);
                  }
                }
                SetPrimitiveValue::Future { id, value } => {
                  if let Some(u_id) = self.public_futures.remove(&id) {
                    self.interface.send((u_id, value));
                  } else {
                    self.resolved_futures.push_back((FutureId(id), value));
                  }
                }
              }
            }
            // continue immediately, no need to wait anything
            self.active_fibers.push_front(fiber);
          }
          RunResult::Create { primitives, success_next, success_binds, success_kinds, fail_next, fail_binds } => {
            let mut candidate_queues = HashSet::<String>::new();
            let mut errors = Vec::<Option<String>>::with_capacity(primitives.len());
            let mut has_error = false;

            // Validate and compute ids for all primitives first (atomic behavior)
            for primitive in primitives.iter() {
              match primitive {
                CreatePrimitiveValue::Queue { name, public: _ } => {
                  if self.queue_messages.contains_key(name) || candidate_queues.contains(name) {
                    errors.push(Some("already_exists".to_string()));
                    has_error = true;
                  } else {
                    errors.push(None);
                    candidate_queues.insert(name.clone());
                  }
                }
                CreatePrimitiveValue::Future => {
                  errors.push(None);
                }
              }
            }

            if has_error {
              // Bind per-primitive Option<String> errors and go to fail branch
              for (idx, var_name) in fail_binds.iter().enumerate() {
                let v = match errors.get(idx).cloned().unwrap_or(None) {
                  Some(e) => Value::OptionString(Some(e)),
                  None => Value::OptionString(None),
                };
                fiber.assign_local(var_name.clone(), v);
              }
              fiber.stack.push(StackEntry::State(fail_next));
              self.active_fibers.push_front(fiber);
            } else {
              let mut ids = Vec::<String>::with_capacity(primitives.len());

              // Apply creations for all primitives since validation succeeded
              for primitive in primitives {
                match primitive {
                  CreatePrimitiveValue::Queue { name, public: _ } => {
                    ids.push(name.clone());
                    self.queue_messages.insert(name, VecDeque::new());
                  }
                  CreatePrimitiveValue::Future => {
                    ids.push(format!("{}", self.next_created_future_id));
                    self.next_created_future_id += 1;
                  }
                }
              }
              // Bind success ids into locals
              for (idx, var_name) in success_binds.iter().enumerate() {
                let id = ids.get(idx).cloned().expect("no way it doesn't exist");
                let v = match success_kinds.get(idx) {
                  Some(SuccessBindKind::String) | None => Value::String(id),
                  Some(SuccessBindKind::Future(kind)) => wrap_future_id(kind.clone(), id),
                };
                fiber.assign_local(var_name.clone(), v);
              }
              fiber.stack.push(StackEntry::State(success_next));
              self.active_fibers.push_front(fiber);
            }
          }
        }
        if !local_dbg.is_empty() {
          if let Ok(mut g) = self.dbg_out.lock() {
            g.push_str(&local_dbg);
          }
        }
      }

      {
        // try to resolve fiber for a resolved Future if any is available
        if let Some((future_id, value)) = self.resolved_futures.pop_front() {
          if let Some(awaiter) = self.wait_index.wake_one(&WaitKey::Future(future_id.clone())) {
            // It's not possible that I have smth in wait_index but don't have it in awaiting_fibers
            // if fiber has been removed from awaiting_fibers it should be removed from wait_index as well, no exceptions
            let mut w_fiber = self.awaiting_fibers.remove(&awaiter.fiber_id).expect("data consistency violation");
            if let Some(bind_var) = awaiter.bind {
              w_fiber.assign_local_and_push_next(bind_var, value, awaiter.next);
            } else {
              w_fiber.push_next(awaiter.next);
            }
            self.active_fibers.push_front(w_fiber);
          } else {
            // if nobody is here for this future - probably it's because they haven't started to await it yet, but they will at some point
            // that's why I'm pushing it back to the queue
            //
            // TODO: potential memory leak if we create future but for some reason are not waiting for it
            // that's quite critical and will hit us at some point if won't be fixed
            // I don't know yet where it should be fixed, probably on IR/DSL level? like in Rust?
            // if variable with future is dropped - we can remove it, if not yet - then it's needed
            self.resolved_futures.push_back((future_id, value));
          }
        }
      }

      // try to get message from next message_queue and run fiber
      // maybe later I should add some index here so we don't hit here if one of the condition doesn't match
      if let Some(q_name) = self.non_empty_queues.pop_front() {
        if let Some(awaiter_info) = self.wait_index.wake_one(&WaitKey::Queue(q_name.clone())) {
          let mut fb = self
            .awaiting_fibers
            .remove(&awaiter_info.fiber_id)
            .expect("if fiber is in wait_index, it should be in awaiters. Otherwise data consistency is violated");

          let m_queue = self
            .queue_messages
            .get_mut(&q_name)
            .expect("should be here and non empty. Otherwise it shouldn't end up in non_empty_queues");
          let v = m_queue.pop_front().expect("should be non empty. Otherwise it shouldn't end up in non_empty_queues");

          // Bind the dequeued message into the awaiting fiber and push its next state
          if let Some(bind_name) = awaiter_info.bind {
            fb.assign_local_and_push_next(bind_name, v, awaiter_info.next);
          } else {
            // No bind requested; just continue to the next state
            fb.stack.push(StackEntry::State(awaiter_info.next));
          }

          self.active_fibers.push_front(fb);
          if !m_queue.is_empty() {
            self.non_empty_queues.push_back(q_name);
          }
          continue 'main_loop;
        } else {
          self.non_empty_queues.push_back(q_name);
        }
      }

      // below are old parts that I'll delete soon
      // but I'm keeping them for now to not make all the tests red immediately
      // but I'll be slowly rewriting them to a new API
      //
      //
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

      // this part I won't delete because it reads messages from external source
      // and puts them where they should be
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
          match blueprint.source {
            TaskBPSource::FiberFunc { fiber_type, function_key, init_values } => {
              if let Some(mut fiber) = self.get_fiber(&fiber_type) {
                fiber.load_task(
                  function_key.clone(),
                  init_values.clone(),
                  Some(RunContext { future_id: None, global_id: Some(blueprint.global_id) }),
                );
                self.active_fibers.push_back(fiber);

                if !current_queue.is_empty() {
                  self.active_tasks.push_front((time_stamp, current_queue));
                }
                break 'process_active_tasks;
              } else {
                self.push_fiber_in_message(
                  &fiber_type,
                  FiberInMessage {
                    fiber_type: fiber_type.clone(),
                    function_name: function_key,
                    args: init_values,
                    context: Some(RunContext { future_id: None, global_id: Some(blueprint.global_id) }),
                  },
                );
              }
            }
            TaskBPSource::Queue { q_name, value } => {
              if let Some(queue) = self.queue_messages.get_mut(&q_name) {
                let was_empty = queue.is_empty();
                // here I can have only messages that `can`` be passed from the outside
                // so for them this function won't fail but for other types it will panic
                let p_value = pub_to_private(value, format!("{}", self.next_created_future_id));
                self.public_futures.insert(format!("{}", self.next_created_future_id), blueprint.global_id);
                self.next_created_future_id += 1;

                queue.push_back(p_value);
                if was_empty {
                  // if it was empty => not in non_empty_queues => adding
                  self.non_empty_queues.push_back(q_name);
                }
              }
            }
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
  use generated::maroon_assembler::{TestCreateQueueMessage, TestCreateQueueMessagePub};
  use std::fmt::Debug;
  use tokio::sync::mpsc::UnboundedReceiver;

  use super::*;

  #[tokio::test(flavor = "multi_thread")]
  async fn sleep_test() {
    let (a2b_runtime, b2a_runtime) =
      create_a_b_duplex_pair::<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>();

    let mut rt = Runtime::new(MonotonicTimer::new(), sample_ir(), b2a_runtime);

    tokio::spawn(async move {
      rt.run("root".to_string()).await;
    });

    _ = a2b_runtime.send((
      LogicalTimeAbsoluteMs(10),
      vec![TaskBlueprint {
        global_id: UniqueU64BlobId(9),
        source: TaskBPSource::FiberFunc {
          fiber_type: FiberType::new("application"),
          function_key: "sleep_and_pow".to_string(),
          init_values: vec![Value::U64(2), Value::U64(4)],
        },
      }],
    ));

    compare_channel_data_with_exp(vec![(UniqueU64BlobId(9), Value::U64(16))], a2b_runtime.receiver).await;
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn create_queues_and_external_communication() {
    let (a2b_runtime, b2a_runtime) =
      create_a_b_duplex_pair::<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>();

    let mut rt = Runtime::new(MonotonicTimer::new(), sample_ir(), b2a_runtime);
    let debug_out = rt.debug_handle();
    tokio::spawn(async move {
      rt.run("testCreateQueue".to_string()).await;
    });

    _ = a2b_runtime.send((
      LogicalTimeAbsoluteMs(0),
      vec![TaskBlueprint {
        global_id: UniqueU64BlobId(9),
        source: TaskBPSource::Queue {
          q_name: "randomQueueName".to_string(),
          value: Value::TestCreateQueueMessagePub(TestCreateQueueMessagePub { value: 10 }),
        },
      }],
    ));

    tokio::time::sleep(Duration::from_millis(10)).await;

    compare_channel_data_with_exp(vec![(UniqueU64BlobId(9), Value::U64(12))], a2b_runtime.receiver).await;

    let result = debug_out.lock();
    assert_eq!(
      r#"--- start testCreateQueue:0 ---
--- await testCreateQueue:0 ---
--- start testCreateQueue:0 ---
value=TestCreateQueueMessage(TestCreateQueueMessage { value: 0, publicFutureId: FutureU64("") })
f_queueName=randomQueueName
created_queue_name=
f_queueCreationError=OptionString(Some("already_exists"))
f_future_id_response=FutureU64(FutureU64(""))
f_res_inc=0
--- await testCreateQueue:0 ---
--- start testCreateQueue:0 ---
value=TestCreateQueueMessage(TestCreateQueueMessage { value: 0, publicFutureId: FutureU64("") })
f_queueName=randomQueueName
created_queue_name=randomQueueName
f_queueCreationError=OptionString(None)
f_future_id_response=FutureU64(FutureU64(""))
f_res_inc=0
--- await testCreateQueue:0 ---
--- start testCreateQueue:0 ---
value=TestCreateQueueMessage(TestCreateQueueMessage { value: 10, publicFutureId: FutureU64("0") })
f_queueName=randomQueueName
created_queue_name=randomQueueName
f_queueCreationError=OptionString(None)
f_future_id_response=FutureU64(FutureU64("0"))
f_res_inc=12
--- await testCreateQueue:0 ---
--- start testCreateQueue:0 ---
--- await testCreateQueue:0 ---
--- exit testCreateQueue:0 ---
"#,
      result.expect("should be object").as_str()
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn creating_fiber_cross_fiber_communication() {
    let (_a2b_runtime, b2a_runtime) =
      create_a_b_duplex_pair::<(LogicalTimeAbsoluteMs, Vec<TaskBlueprint>), (UniqueU64BlobId, Value)>();

    let mut rt = Runtime::new(MonotonicTimer::new(), sample_ir(), b2a_runtime);
    let debug_out = rt.debug_handle();
    tokio::spawn(async move {
      rt.run("testRootFiber".to_string()).await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let result = debug_out.lock();
    assert_eq!(
      r#"--- start testRootFiber:0 ---
--- await testRootFiber:0 ---
--- start testRootFiber:0 ---
--- await testRootFiber:0 ---
created: FiberType("testCalculator"):1. init_vars:
    String("rootQueue")
created: FiberType("testCalculator"):2. init_vars:
    String("rootQueue")
--- start testRootFiber:0 ---
--- await testRootFiber:0 ---
--- start testRootFiber:0 ---
--- await testRootFiber:0 ---
--- start testRootFiber:0 ---
--- await testRootFiber:0 ---
--- start testCalculator:1 ---
request=TestCalculatorTask(TestCalculatorTask { a: 0, b: 0, responseFutureId: FutureU64("") })
result=0
respFutureId=FutureU64(FutureU64(""))
--- await testCalculator:1 ---
--- start testCalculator:2 ---
request=TestCalculatorTask(TestCalculatorTask { a: 0, b: 0, responseFutureId: FutureU64("") })
result=0
respFutureId=FutureU64(FutureU64(""))
--- await testCalculator:2 ---
--- start testCalculator:1 ---
got task from the queue
request=TestCalculatorTask(TestCalculatorTask { a: 10, b: 15, responseFutureId: FutureU64("0") })
result=0
respFutureId=FutureU64(FutureU64(""))
--- await testCalculator:1 ---
--- start testCalculator:1 ---
--- await testCalculator:1 ---
--- exit testCalculator:1 ---
--- start testCalculator:2 ---
got task from the queue
request=TestCalculatorTask(TestCalculatorTask { a: 2, b: 4, responseFutureId: FutureU64("1") })
result=0
respFutureId=FutureU64(FutureU64(""))
--- await testCalculator:2 ---
--- start testCalculator:2 ---
--- await testCalculator:2 ---
--- exit testCalculator:2 ---
--- start testRootFiber:0 ---
--- await testRootFiber:0 ---
--- start testRootFiber:0 ---
rootQueueName=rootQueue
calculatorTask=TestCalculatorTask(TestCalculatorTask { a: 10, b: 15, responseFutureId: FutureU64("0") })
calculatorTask2=TestCalculatorTask(TestCalculatorTask { a: 2, b: 4, responseFutureId: FutureU64("1") })
responseFutureId=FutureU64(FutureU64("0"))
responseFutureId2=FutureU64(FutureU64("1"))
responseFromCalculator=8
responseFromCalculator2=150
createQueueError=OptionString(None)
createFutureError=OptionString(None)
createFutureError2=OptionString(None)
--- await testRootFiber:0 ---
--- exit testRootFiber:0 ---
"#,
      result.expect("should be object").as_str()
    );
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
