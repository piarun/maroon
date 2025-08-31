use crate::generated_types::*;

#[test]
fn primitive_one_tick_function() {
  let vars =
    vec![StackEntry::Value("a".to_string(), Value::U64(2)), StackEntry::Value("b".to_string(), Value::U64(10))];
  let result = global_step(State::GlobalAddEntry, &vars, &mut Heap::Global(GlobalHeap {}));
  if let StepResult::Return(Value::U64(12)) = result {
  } else {
    panic!("add should return 12");
  }
}

#[test]
fn add_function() {
  let mut some_t = Task::new();
  some_t.put_add_task(14, 16);
  some_t.run();
  assert_eq!(vec![StackEntry::Value("ret".to_string(), Value::U64(30))], some_t.stack);
}

#[test]
fn sub_add_function() {
  let mut some_t = Task::new();
  some_t.put_sub_add_task(6, 5, 4);
  some_t.run();

  assert_eq!(vec![StackEntry::Value("ret".to_string(), Value::U64(7))], some_t.stack);
}

#[test]
fn factorial_function() {
  let mut some_t = Task::new();
  some_t.put_factorial_task(3);
  some_t.run();

  assert_eq!(vec![StackEntry::Value("ret".to_string(), Value::U64(6))], some_t.stack);
}

pub struct Task {
  pub stack: Vec<StackEntry>,
  pub heap: Heap,
}

impl Task {
  fn new() -> Task {
    Task { stack: vec![], heap: Heap::Global(GlobalHeap {}) }
  }

  fn put_factorial_task(
    &mut self,
    n: u64,
  ) {
    self.stack.push(StackEntry::Value("ret".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::Retrn(Some(1)));
    self.stack.push(StackEntry::Value("n".to_string(), Value::U64(n)));
    self.stack.push(StackEntry::Value("facCallRes".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::Value("result".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::Value("subtractRes".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::State(State::GlobalFactorialEntry));
  }

  fn put_add_task(
    &mut self,
    a: u64,
    b: u64,
  ) {
    self.stack.push(StackEntry::Value("ret".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::Retrn(Some(1)));
    self.stack.push(StackEntry::Value("a".to_string(), Value::U64(a)));
    self.stack.push(StackEntry::Value("b".to_string(), Value::U64(b)));
    self.stack.push(StackEntry::State(State::GlobalAddEntry));
  }

  fn put_sub_add_task(
    &mut self,
    a: u64,
    b: u64,
    c: u64,
  ) {
    self.stack.push(StackEntry::Value("ret".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::Retrn(Some(1)));
    // This experimental flow requires its own states; leaving as-is but unused.
    self.stack.push(StackEntry::Value("a".to_string(), Value::U64(a)));
    self.stack.push(StackEntry::Value("b".to_string(), Value::U64(b)));
    self.stack.push(StackEntry::Value("c".to_string(), Value::U64(c)));
    self.stack.push(StackEntry::Value("sumAB".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::Value("subABC".to_string(), Value::U64(0)));
    self.stack.push(StackEntry::State(State::GlobalSubAddEntry));
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
          self.stack.extend(stack_entries);
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
