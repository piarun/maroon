use crate::simple_function::generated::*;
use crate::{
  ir::{Func, FutureId},
  simple_function::{generated::Heap, task::*},
};
use std::{
  collections::{BinaryHeap, HashMap, LinkedList, VecDeque},
  env::var,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTimeAbsoluteMs(u64);

struct Runtime {
  active_tasks: LinkedList<(LogicalTimeAbsoluteMs, VecDeque<Task>)>,
  //   pub time_unbounded: BinaryHeap<>
  time_unbounded: HashMap<FutureId, TaskBox>,
  //   pub time_bounded: BinaryHeap<>
}

struct TaskBox {
  task: Task,
  result_var_bind: String,
}

impl Runtime {
  pub fn run(&mut self) {
    let mut counter = 0;
    loop {
      counter += 1;
      if counter > 10 {
        panic!("");
      }

      // TODO: get current logical time
      let now = LogicalTimeAbsoluteMs(0);

      let current_queue = self.active_tasks.front_mut();
      if current_queue.is_none() {
        // TODO: not continue but some sleep, if there is no next elements
        println!("nothing in active_tasks");
        continue;
      }

      let (time_stamp, tasks) = current_queue.unwrap();
      if *time_stamp < now {
        // TODO: not continue but some sleep, since we shouldn't work on it yet + we shouldn't just waste CPU cycles
        println!("smth in active_tasks, but not now");
        continue;
      }

      while let Some(task) = tasks.pop_front() {
        println!("TASKS: {}", &tasks.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n"));
        let mut task = task;
        match task.run() {
          RunResult::Done(result) => {
            println!("TASK {} IS FINISHED. Result: {:?}", task, result);
            let Some(future_id) = task.options.future_id else {
              continue;
            };
            let Some(mut task_box) = self.time_unbounded.remove(&future_id) else {
              continue;
            };

            task_box.task.put_toppest_value_by_name(task_box.result_var_bind, result);
            tasks.push_front(task_box.task);

            //  TODO: add smth in order to return the result for gateway through some chains
          }
          RunResult::AsyncCall { fiber, func, args, future_id } => {
            println!("ASYNC CALL: {:?}", &future_id);
            let t = Task::new(Heap::default(), fiber + "." + &func, args, Some(Options { future_id: Some(future_id) }));
            // TODO: in that case when task will be finished with work - asynced t will be taken for execution
            tasks.push_front(t);
            tasks.push_front(task);
          }
          RunResult::Await(future_id, var_bind) => {
            println!("AWAIT: {:?}", &future_id);

            // specify bind parameters here
            self.time_unbounded.insert(future_id, TaskBox { task: task, result_var_bind: var_bind });
          }
        }
      }
    }
  }
}

#[test]
fn some_test() {
  let mut rt = Runtime {
    active_tasks: LinkedList::from([(
      LogicalTimeAbsoluteMs(10),
      VecDeque::from([Task::new(Heap::default(), "application.async_foo", vec![Value::U64(4), Value::U64(8)], None)]),
    )]),
    time_unbounded: HashMap::new(),
  };

  rt.run();
}
