use crate::{
  ir::{Func, FutureId},
  simple_function::generated::*,
};

#[derive(Clone, Debug)]

pub struct Task {
  stack: Vec<StackEntry>,
  heap: Heap,
  // holds an information for which function this task was created for
  // used for preparing the stack before run and for getting the result
  function_key: String,

  pub options: Options,
}

// TODO: don't like this name
#[derive(Clone, Debug)]

pub struct Options {
  pub future_id: Option<FutureId>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunResult {
  Done(Value),
  // futureId, varBind
  Await(FutureId, String),
  AsyncCall { fiber: String, func: String, args: Vec<Value>, future_id: FutureId },
}

impl std::fmt::Display for Task {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter,
  ) -> std::fmt::Result {
    write!(f, "{}", self.function_key)
  }
}

impl Task {
  pub fn new(
    // TODO: heap_init is a bit hacky right now
    // because, ideally it should be prepared through get_prepare_fn map, but it's ok for now
    // when I'll get more usecases for Heap I'll do smth with it
    heap_init: Heap,
    key: impl Into<String>,
    init_values: Vec<Value>,
    options: Option<Options>,
  ) -> Task {
    let function_key: String = key.into();
    let f = get_prepare_fn(function_key.as_str());
    let stack = f(init_values);
    let options = options.unwrap_or(Options { future_id: None });
    Task { stack, heap: heap_init, function_key, options }
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

  pub fn put_toppest_value_by_name(
    &mut self,
    var_name: String,
    val: Value,
  ) {
    if let Some(StackEntry::Value(_, slot)) =
      self.stack.iter_mut().rev().find(|se| matches!(se, StackEntry::Value(n, _) if *n == var_name))
    {
      *slot = val;
    } else {
      panic!("didnt find the value with the right name, something is completely wrong");
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
        let f = get_result_fn(&self.function_key);
        return RunResult::Done(f(&self.stack));
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
        StepResult::Await(future_id, bind_result, next_state) => {
          // Continue at `next_state` after the future resolves
          self.stack.push(StackEntry::State(next_state));
          return RunResult::Await(FutureId(future_id), bind_result);
        }
        StepResult::SendToFiber { fiber, func, args, next, future_id } => {
          // Continue to `next` and bubble up async call details
          self.stack.push(StackEntry::State(next));
          return RunResult::AsyncCall { fiber, func, args, future_id: FutureId(future_id) };
        }
        _ => {}
      }
    }
  }
}
