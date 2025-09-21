use crate::{
  ir::{FiberType, Func, FutureId},
  simple_function::generated::*,
};

#[derive(Clone, Debug)]
pub struct Fiber {
  stack: Vec<StackEntry>,
  heap: Heap,
  // holds an information for which function this task was created for
  // used for preparing the stack before run and for getting the result
  function_key: String,

  pub f_type: FiberType,
  pub options: Options,
}

// TODO: don't like this name
#[derive(Clone, Debug)]
pub struct Options {
  // not None if there is binded task that is awaiting finishing this future_id
  // TODO: not sure it's a good way to put that kind of information inside the task
  //    why task should know if it's binded to smth or not?
  pub future_id: Option<FutureId>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunResult {
  Done(Value),
  // futureId, varBind
  Await(FutureId, String),
  AsyncCall { f_type: FiberType, func: String, args: Vec<Value>, future_id: FutureId },
}

impl std::fmt::Display for Fiber {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter,
  ) -> std::fmt::Result {
    write!(f, "{}", self.function_key)
  }
}

impl Fiber {
  // Create an empty fiber with a default heap and no loaded task.
  pub fn new(f_type: FiberType) -> Fiber {
    Fiber {
      f_type,
      stack: Vec::new(),
      heap: Heap::default(),
      function_key: String::new(),
      options: Options { future_id: None },
    }
  }

  pub fn new_with_heap(
    f_type: FiberType,
    heap: Heap,
  ) -> Fiber {
    Fiber { f_type, stack: Vec::new(), heap: heap, function_key: String::new(), options: Options { future_id: None } }
  }

  // load a task into this fiber, clearing the current stack but preserving the heap
  // TODO: should I check and if stack is not empty - panic?
  pub fn load_task(
    &mut self,
    func_name: impl Into<String>,
    init_values: Vec<Value>,
    options: Option<Options>,
  ) {
    self.stack.clear();
    self.function_key = format!("{}.{}", self.f_type, func_name.into());
    let f = get_prepare_fn(self.function_key.as_str());
    self.stack = f(init_values);
    self.options = options.unwrap_or(Options { future_id: None });
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

  // Assigns a value to the first matching named value entry from the back (top) of the stack.
  pub fn assign_local(
    &mut self,
    name: String,
    val: Value,
  ) {
    if let Some(StackEntry::Value(_, slot)) =
      self.stack.iter_mut().rev().find(|se| matches!(se, StackEntry::Value(n, _) if *n == name))
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
        StepResult::SendToFiber { f_type, func, args, next, future_id } => {
          // Continue to `next` and bubble up async call details
          self.stack.push(StackEntry::State(next));
          return RunResult::AsyncCall { f_type, func, args, future_id: FutureId(future_id) };
        }
        _ => {}
      }
    }
  }
}
