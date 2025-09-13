use crate::{
  ir::{Func, FutureId},
  simple_function::task::*,
};
use std::collections::{BinaryHeap, HashMap, LinkedList, VecDeque};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTimeAbsoluteMs(u64);

pub struct Future {}

pub struct Runtime {
  pub active_tasks: LinkedList<(LogicalTimeAbsoluteMs, VecDeque<Task>)>,
  //   pub time_unbounded: BinaryHeap<>
  pub time_unbounded: HashMap<FutureId, Task>,
  //   pub time_bounded: BinaryHeap<>
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
          RunResult::Done(_) => {
            /*  TODO: add smth in order to return the result(it might be gateway through some chains, it might be awaited fiber/task?)*/
          }
          RunResult::AsyncCall() => {
            // here I need to get

            tasks.push_front(task);
          }
          RunResult::Await(future_id) => {
            self.time_unbounded.insert(future_id, task);

            // TODO:
            // here I need to put result somehow to awaiting_task and add awaiting_task to the current_queue
            //      should I put it in the beginning or end of the queue?
            //      beginning sounds better, because in that case there will be less awaiting tasks
            //      I need to be able to cancel other async tasks if there is Select and one arm has shoot
          }
        }
      }
    }
  }
}
