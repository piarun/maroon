use crate::{
  ir::{Func, FutureId},
  simple_function::{generated::Heap, task::*},
};
use std::collections::{BinaryHeap, HashMap, LinkedList, VecDeque};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTimeAbsoluteMs(u64);

struct Runtime {
  active_tasks: LinkedList<(LogicalTimeAbsoluteMs, VecDeque<Task>)>,
  //   pub time_unbounded: BinaryHeap<>
  time_unbounded: HashMap<FutureId, Task>,
  //   pub time_bounded: BinaryHeap<>
}

struct TaskBox {
  task: Task,
  result_var_bind: String,
}

impl Runtime {
  pub fn run(&mut self) {
    loop {
      // TODO: get current logical time
      let now = LogicalTimeAbsoluteMs(0);

      let current_queue = self.active_tasks.front_mut();
      if current_queue.is_none() {
        // TODO: not continue but some sleep, if there is no next elements
        continue;
      }

      let (time_stamp, tasks) = current_queue.unwrap();
      if *time_stamp < now {
        // TODO: not continue but some sleep, since we shouldn't work on it yet + we shouldn't just waste CPU cycles
        continue;
      }

      while let Some(task) = tasks.pop_front() {
        let mut task = task;
        match task.run() {
          RunResult::Done(result) => {
            println!("TASK {} IS FINISHED. Result: {:?}", task, result);
            let Some(future_id) = task.options.future_id else {
              continue;
            };
            let Some(task_to_restore) = self.time_unbounded.remove(&future_id) else {
              continue;
            };

            // TODO: how to pass result into task_to_restore??
            tasks.push_front(task_to_restore);

            //  TODO: add smth in order to return the result(it might be gateway through some chains)
          }
          RunResult::AsyncCall { fiber, func, args, future_id } => {
            let t = Task::new(Heap::default(), fiber + "." + &func, args, Some(Options { future_id: Some(future_id) }));
            // TODO: in that case when task will be finished with work - asynced t will be taken for execution
            tasks.push_front(t);
            tasks.push_front(task);
          }
          RunResult::Await(future_id) => {
            // specify bind parameters here
            self.time_unbounded.insert(future_id, task);
          }
        }
      }
    }
  }
}
