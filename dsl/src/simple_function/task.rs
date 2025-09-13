use crate::{
  ir::{Func, FutureId},
  simple_function::generated::*,
};

pub struct Task {
  stack: Vec<StackEntry>,
  heap: Heap,
  get_result: fn(stack: &Vec<StackEntry>, heap: &Heap) -> RunResult,
}

#[derive(Clone, Debug, PartialEq)]

pub enum RunResult {
  Done(Value),
  Await(FutureId),
  AsyncCall(),
}

impl Task {
  pub fn new(
    stack_init: Vec<StackEntry>,
    heap_init: Heap,
    get_result: fn(stack: &Vec<StackEntry>, heap: &Heap) -> RunResult,
  ) -> Task {
    Task { stack: stack_init, heap: heap_init, get_result }
  }

  pub fn print_stack(
    &self,
    mark: &str,
  ) {
    println!("StackState:{}", mark);
    for elem in &self.stack {
      println!("    {:?}", elem);
    }
  }

  // Runs until finished and gets the resutl or until parked for awaiting async results
  pub fn run(&mut self) -> RunResult {
    loop {
      self.print_stack("");
      let Some(head) = self.stack.pop() else {
        panic!("no way there will be no elements. Can happen only on empty one")
      };

      let StackEntry::State(state) = head else {
        // if no next state - return
        self.stack.push(head);

        return (self.get_result)(&self.stack, &self.heap);
      };

      let arguments_number = func_args_count(&state);
      if arguments_number > self.stack.len() {
        panic!("miss amount of variables: need {arguments_number}, have {}", self.stack.len());
      }

      // index on stack where current function starts
      // StackEntry::Retrn is not here, only arguments + local_vars
      let start = self.stack.len() - arguments_number;

      let result = global_step(state, &self.stack[start..], &mut self.heap);

      match result {
        StepResult::Return(val) => {
          // Drop used arguments
          self.stack.truncate(start);

          // since we're returning from function we should have a record of return 'address' info
          let StackEntry::Retrn(return_instruction) =
            self.stack.pop().expect("stack is corrupted. No return instruction")
          else {
            panic!("there is no return instruction on stack. Stack is corrupted");
          };

          if let Some(offset) = return_instruction {
            let ret_value_bind_index = self.stack.len() - offset;
            let new_entry = if let StackEntry::Value(label, _) = &self.stack[ret_value_bind_index] {
              StackEntry::Value(label.clone(), val)
            } else {
              StackEntry::Value("ret".to_string(), val)
            };
            self.stack[ret_value_bind_index] = new_entry;
          }
        }
        StepResult::GoTo(state) => {
          self.stack.push(StackEntry::State(state));
        }
        StepResult::Next(stack_entries) => {
          // Apply in-frame assignments first, relative to current frame start
          for se in stack_entries {
            match se {
              StackEntry::FrameAssign(updates) => {
                for (ofs, val) in updates {
                  let idx = start + ofs;
                  let new_entry = if let StackEntry::Value(label, _) = &self.stack[idx] {
                    StackEntry::Value(label.clone(), val.clone())
                  } else {
                    StackEntry::Value("_".to_string(), val.clone())
                  };
                  self.stack[idx] = new_entry;
                }
              }
              other => self.stack.push(other),
            }
          }
        }
        StepResult::Await(future_id) => {
          return RunResult::Await(FutureId(future_id));
        }
        _ => {}
      }
    }
  }
}
