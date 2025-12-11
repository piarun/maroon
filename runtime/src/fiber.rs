use dsl::ir::FiberType;
use generated::maroon_assembler::{
  CreatePrimitiveValue, Heap, SelectArm, SetPrimitiveValue, StackEntry, State, StepResult, SuccessBindKind, Value,
  func_args_count, get_heap_init_fn, get_prepare_fn, global_step,
};

use crate::trace::TraceEvent;

#[derive(Clone, Debug)]
pub struct Fiber {
  pub stack: Vec<StackEntry>,
  pub heap: Heap,
  /// holds an information for which function this task was created for
  /// used for preparing the stack before run and for getting the result
  pub function_key: String,

  pub f_type: FiberType,
  pub unique_id: u64,

  /// here we put full fiber history
  /// right now - pairs (state, result), later maybe more
  /// TODO: make it optional, so if I don't want - I won't include it
  pub trace_sink: Vec<TraceEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunResult {
  Done,
  /// Select arms matching IR: can await either futures or queue messages
  Select(Vec<SelectArm>),
  /// Broadcast primitive updates to runtime; fiber has already queued next state
  SetValues(Vec<SetPrimitiveValue>),
  /// Spawn new fibers via runtime; fiber already queued next state
  CreateFibers {
    details: Vec<(FiberType, Vec<Value>)>,
  },
  /// Request to atomically create primitives; runtime will decide branch
  Create {
    primitives: Vec<CreatePrimitiveValue>,
    success_next: State,
    success_binds: Vec<String>,
    success_kinds: Vec<SuccessBindKind>,
    fail_next: State,
    fail_binds: Vec<String>,
  },
}

impl std::fmt::Display for Fiber {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter,
  ) -> std::fmt::Result {
    write!(f, r"{}", self.function_key)
  }
}

/// Runtime-only Future identifier. Unique per-fiber using suffixing policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FutureId(pub String);
impl std::fmt::Display for FutureId {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "fut{}", self.0)
  }
}

impl Fiber {
  /// creates a Fiber able to run from main function
  pub fn new(
    f_type: FiberType,
    unique_id: u64,
    init_vars: &Vec<Value>,
  ) -> Fiber {
    let f_name = format!("{}.{}", f_type, "main");
    let f = get_prepare_fn(f_name.as_str());
    let heap = {
      let hif = get_heap_init_fn(&f_type);
      hif(init_vars.clone())
    };

    Fiber { f_type, unique_id, stack: f(vec![]), heap: heap, function_key: f_name, trace_sink: vec![] }
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

  /// Assigns a value to the first matching named value entry from the back (top) of the stack.
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

  /// Assign a local and push the next state onto the stack (used for queue-await resume paths)
  pub fn assign_local_and_push_next(
    &mut self,
    name: String,
    val: Value,
    next: State,
  ) {
    self.assign_local(name, val);
    self.stack.push(StackEntry::State(next));
  }

  /// Push next state on stack
  pub fn push_next(
    &mut self,
    next: State,
  ) {
    self.stack.push(StackEntry::State(next));
  }

  /// Runs until finished and gets the result or until parked for awaiting async results
  pub fn run(
    &mut self,
    sink: &mut dyn std::fmt::Write,
  ) -> RunResult {
    loop {
      let head_opt = self.stack.pop();
      if head_opt.is_none() {
        // Empty stack indicates completion
        return RunResult::Done;
      }
      let head = head_opt.unwrap();

      let StackEntry::State(state) = head else {
        // if no next state - return
        self.stack.push(head);
        return RunResult::Done;
      };

      let arguments_number = func_args_count(&state);
      if arguments_number > self.stack.len() {
        panic!("miss amount of variables: need {arguments_number}, have {}", self.stack.len());
      }

      // index on stack where current function starts
      // StackEntry::Retrn is not here, only arguments + local_vars
      let start = self.stack.len() - arguments_number;

      let state_cp = state.clone();
      let result = global_step(state, &self.stack[start..], &mut self.heap);
      self.trace_sink.push(TraceEvent { state: state_cp, result: result.clone() });

      match result {
        StepResult::Debug(msg, next) => {
          let _ = sink.write_str(msg);
          let _ = sink.write_char('\n');
          self.stack.push(StackEntry::State(next));
        }
        StepResult::DebugPrintVars(next) => {
          for se in &self.stack[start..] {
            if let StackEntry::Value(name, val) = se {
              let _ = sink.write_str(name);
              let _ = sink.write_char('=');
              match val {
                Value::U64(x) => {
                  let _ = sink.write_fmt(format_args!("{}", x));
                }
                Value::String(s) => {
                  let _ = sink.write_str(s);
                }
                Value::Unit(_) => {
                  let _ = sink.write_str("()");
                }
                _ => {
                  // Fallback to Debug formatting for other types
                  let _ = sink.write_fmt(format_args!("{:?}", val));
                }
              }
              let _ = sink.write_char('\n');
            }
          }
          self.stack.push(StackEntry::State(next));
        }
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
        StepResult::ReturnVoid => {
          // function returns no value
          // clean up the current frame
          // (drop args/locals) and pop the return marker without
          // binding anything into the caller frame
          self.stack.truncate(start);

          // since we're returning from function we should have a record of return 'address' info
          let StackEntry::Retrn(_return_instruction) =
            self.stack.pop().expect("stack is corrupted. No return instruction")
          else {
            panic!("there is no return instruction on stack. Stack is corrupted");
          };
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
        StepResult::Select(arms) => {
          return RunResult::Select(arms);
        }
        StepResult::CreateFibers { details, next } => {
          self.stack.push(StackEntry::State(next));
          return RunResult::CreateFibers { details };
        }
        StepResult::Create { primitives, success_next, success_binds, success_kinds, fail_next, fail_binds } => {
          // Do not push next yet; runtime will decide the branch and re-queue us
          return RunResult::Create { primitives, success_next, success_binds, success_kinds, fail_next, fail_binds };
        }
        StepResult::SetValues { values, next } => {
          self.stack.push(StackEntry::State(next));
          return RunResult::SetValues(values);
        }
        StepResult::Done | StepResult::Todo(_) => {
          // No-op control signals for now; continue stepping if any state remains
        }
      }
    }
  }
}
