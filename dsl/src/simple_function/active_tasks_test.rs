use crate::simple_function::generated::*;
use crate::simple_function::ir::sample_ir;
use crate::{
  ir::{FiberType, FutureId, IR},
  simple_function::{generated::Heap, task::*},
};
use std::hash::Hash;
use std::{
  collections::{BinaryHeap, HashMap, LinkedList, VecDeque},
  env::var,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTimeAbsoluteMs(u64);

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

struct Runtime {
  // Execution priority
  // Executors goes to the next step only if there is no work on previous steps
  //
  // - active_fibers
  // - fiber_in_message_queue
  // - active_tasks
  //
  // 1. run until there are no active_fibers
  // 2. take in_message from fiber_in_message_queue, convert it into active_fibers, go to step 1
  // 3. take taskBlueprint from active_tasks, convert it into active_fibers, go to step 1

  // this is the input for the engine, here new tasks from commited epochs will be coming in the commited order
  active_tasks: LinkedList<(LogicalTimeAbsoluteMs, VecDeque<TaskBlueprint>)>,

  // fibers that can be executed
  active_fibers: VecDeque<Fiber>,
  // fibers that have some tasks, but can't be executed because they're awaiting something
  parked_fibers: HashMap<FutureId, FiberBox>,

  // created but idle fibers
  fiber_pool: HashMap<FiberType, Vec<Fiber>>,

  // queue for in_messages that will be executed in the order when fiber is available
  // TODO: this one should have predictable order
  fiber_in_message_queue: HashMap<FiberType, VecDeque<FiberInMessage>>,

  fiber_limiter: HashMap<FiberType, u64>,

  // results. key - is global_id from TaskBlueprint
  // TODO: make it `UniqueU64BlobId` from `common` crate
  results: HashMap<u64, Value>,
}

struct FiberBox {
  fiber: Fiber,

  // information to which variable on stack we should bind the result for the `fiber`
  // is used when fiber is parked and awaits some result
  result_var_bind: String,
}

struct FiberInMessage {
  fiber_type: FiberType,
  function_name: String,
  args: Vec<Value>,
  options: Option<Options>,
}

impl Runtime {
  pub fn new(ir: IR) -> Runtime {
    Runtime {
      fiber_limiter: ir.fibers.into_iter().map(|fi| (fi.0, fi.1.fibers_limit)).collect(),
      active_fibers: VecDeque::new(),
      active_tasks: LinkedList::new(),
      parked_fibers: HashMap::new(),
      fiber_pool: HashMap::new(),
      fiber_in_message_queue: HashMap::new(),
      results: HashMap::new(),
    }
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
    return Some(Fiber::new(f_type.clone()));
  }

  pub fn run(&mut self) {
    let mut counter = 0;
    loop {
      counter += 1;
      if counter > 100 {
        // TODO: just for tests, and for now, actually it should be an infinite loop
        return;
      }

      while let Some(mut fiber) = self.active_fibers.pop_front() {
        match fiber.run() {
          RunResult::Done(result) => {
            println!("FIBER {} IS FINISHED. Result: {:?}", &fiber, result);

            let options = fiber.options.clone();
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

            task_box.fiber.assign_local(task_box.result_var_bind, result);
            self.active_fibers.push_front(task_box.fiber);
          }
          RunResult::AsyncCall { f_type, func, args, future_id } => {
            println!("ASYNC CALL: {:?}", &future_id);

            if let Some(mut available_fiber) = self.get_fiber(&f_type) {
              available_fiber.load_task(func, args, Some(Options { future_id: Some(future_id), global_id: None }));
              // TODO: in that case when task will be finished with work - asynced available_fiber will be taken for execution
              self.active_fibers.push_front(available_fiber);
            } else {
              self.fiber_in_message_queue.entry(f_type.clone()).or_default().push_back(FiberInMessage {
                fiber_type: f_type,
                function_name: func,
                args,
                options: Some(Options { future_id: Some(future_id), global_id: None }),
              });
            }

            self.active_fibers.push_front(fiber);
          }
          RunResult::Await(future_id, var_bind) => {
            println!("AWAIT: {:?}", &future_id);

            // specify bind parameters here
            self.parked_fibers.insert(future_id, FiberBox { fiber: fiber, result_var_bind: var_bind });
          }
        }
      }

      // TODO: get tasks from fiber_in_message_queue and convert them into active_fibers

      // NEXT_STEP_2:
      // if all fibers are finished or awaiting, we can pick the next task and start a new fiber
      //

      'process_active_tasks: loop {
        // TODO: get current logical time
        let now = LogicalTimeAbsoluteMs(0);

        let Some((time_stamp, mut current_queue)) = self.active_tasks.pop_front() else {
          // TODO: not break but some sleep, if there are no next elements or select or smth
          println!("nothing in active_tasks");
          break;
        };

        if time_stamp < now {
          // TODO: not continue but some sleep, since we shouldn't work on it yet + we shouldn't just waste CPU cycles
          println!("smth in active_tasks, but not now");
          self.active_tasks.push_front((time_stamp, current_queue));
          break;
        }

        while let Some(blueprint) = current_queue.pop_front() {
          if let Some(mut fiber) = self.get_fiber(&blueprint.fiber_type) {
            fiber.load_task(
              blueprint.function_key,
              blueprint.init_values,
              Some(Options { future_id: None, global_id: Some(blueprint.global_id) }),
            );
            self.active_fibers.push_back(fiber);

            if !current_queue.is_empty() {
              self.active_tasks.push_front((time_stamp, current_queue));
            }
            break 'process_active_tasks;
          } else {
            self.fiber_in_message_queue.entry(blueprint.fiber_type.clone()).or_default().push_back(FiberInMessage {
              fiber_type: blueprint.fiber_type,
              function_name: blueprint.function_key,
              args: blueprint.init_values,
              options: Some(Options { future_id: None, global_id: Some(blueprint.global_id) }),
            });
          }
        }
      }
    }
  }
}

#[test]
fn some_test() {
  let mut rt = Runtime::new(sample_ir());

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
