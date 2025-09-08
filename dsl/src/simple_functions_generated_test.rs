use crate::simple_functions_generated::*;

#[test]
fn add_function() {
  let mut some_t = Task::new();

  let (entries, _) = global_prepare_add(14, 16);
  some_t.stack.extend(entries);
  some_t.run();

  assert_eq!(30, global_result_add(&some_t.stack));
}

#[test]
fn sub_add_function() {
  let mut some_t = Task::new();

  let (entries, _) = global_prepare_subAdd(6, 5, 4);
  some_t.stack.extend(entries);
  some_t.run();

  assert_eq!(7, global_result_subAdd(&some_t.stack));
}

#[test]
fn factorial_function() {
  let mut some_t = Task::new();

  let (entries, _) = global_prepare_factorial(3);
  some_t.stack.extend(entries);
  some_t.run();

  assert_eq!(6, global_result_factorial(&some_t.stack));
}

#[test]
fn b_search_function() {
  let mut some_t = Task::new();

  let search_elements = vec![1, 2, 3, 4, 5, 6, 7];
  let (entries, _) = global_prepare_binarySearch(4, 0, (search_elements.len() - 1) as u64);
  some_t.stack.extend(entries);
  some_t.heap = Heap::Global(GlobalHeap { binarySearchValues: search_elements });
  some_t.run();

  assert_eq!(Some(3), global_result_binarySearch(&some_t.stack));
}

pub struct Task {
  pub stack: Vec<StackEntry>,
  pub heap: Heap,
}

impl Task {
  fn new() -> Task {
    Task { stack: vec![], heap: Heap::Global(GlobalHeap { binarySearchValues: vec![] }) }
  }

  fn print_stack(
    &self,
    mark: &str,
  ) {
    println!("StackState:{}", mark);
    for elem in &self.stack {
      println!("    {:?}", elem);
    }
  }

  fn run(&mut self) {
    loop {
      self.print_stack("");
      let Some(head) = self.stack.pop() else {
        break;
      };

      let StackEntry::State(state) = head else {
        // if no next state - return
        self.stack.push(head);
        break;
      };

      let arguments_number = func_args_count(&state);
      if arguments_number > self.stack.len() {
        panic!("miss amount of variables: need {arguments_number}, have {}", self.stack.len());
      }

      // index on stack where current function starts
      // StackEntry::Retrn is not here, only arguments + local_vars
      let start = self.stack.len() - arguments_number;

      // println!("Star {}", start);
      // println!("Vars: {:?}", &self.stack[start..]);
      // self.print_stack("BeforeGlobalStep");

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
          // self.print_stack("After_first_return");
        }
        StepResult::GoTo(state) => {
          // if it's call a function in the same fiber - straightforward
          // TODO: add here cross-fiber async-await shit?
          // But async await has it's own IR state Step::Await
          // then I should check somehow that this is only normal local state, and not async one?
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
        _ => {}
      }
    }
  }
}

/*
stack:

retVal<None>
argA
argB
func1
addRetVal<None>
argA<3>
argB<4>
addEntry


retVal<None>
argA
argB
func1
addRetVal<7>
argA<3>
argB<4>
addDone



*/
