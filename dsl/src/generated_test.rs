use crate::generated_types::*;

#[test]
fn primitive_one_tick_function() {
  let vars = vec![StackEntry::Value(Value::GlobalAddParamA(2)), StackEntry::Value(Value::GlobalAddParamB(10))];
  let result = global_step(State::GlobalAddEntry, &vars, &mut Heap::Global(GlobalHeap {}));
  if let StepResult::Return(Some(Value::GlobalAddReturn(12))) = result {
  } else {
    panic!("add should return 12");
  }
}

#[test]
fn random_function() {
  let empty: Vec<StackEntry> = vec![];
  let result = global_step(State::GlobalRandGenEntry, &empty, &mut Heap::Global(GlobalHeap {}));

  if let StepResult::Return(_) = result {
  } else {
    panic!("failed test, should return some random number");
  }
}

#[test]
fn add_function() {
  let mut some_t = Task::new();
  some_t.put_add_task(14, 16);
  some_t.run();

  assert_eq!(vec![StackEntry::Retrn(Some(Value::GlobalAddReturn(30)))], some_t.stack);
}

pub struct Task {
  pub stack: Vec<StackEntry>,
  pub heap: Heap,
}

impl Task {
  fn new() -> Task {
    Task { stack: vec![], heap: Heap::Global(GlobalHeap {}) }
  }

  fn put_add_task(
    &mut self,
    a: u64,
    b: u64,
  ) {
    self.stack.push(StackEntry::Retrn(None));
    self.stack.push(StackEntry::Value(Value::GlobalAddParamA(a)));
    self.stack.push(StackEntry::Value(Value::GlobalAddParamB(b)));
    self.stack.push(StackEntry::State(State::GlobalAddEntry));
  }

  fn run(&mut self) {
    loop {
      let Some(head) = self.stack.pop() else {
        break;
      };

      let StackEntry::State(state) = head else {
        // if no next state - return
        self.stack.push(head);
        break;
      };

      let arguments_number = state_args_count(&state);
      if arguments_number > self.stack.len() {
        panic!("miss amount of variables: need {arguments_number}, have {}", self.stack.len());
      }

      let start = self.stack.len() - arguments_number;

      let result = global_step(state, &self.stack[start..], &mut self.heap);

      match result {
        StepResult::Return(opt) => {
          // Drop used arguments
          self.stack.truncate(start);

          let on_top = self.stack.last_mut().expect("should be value after return, otherwise stack is corrupted");
          if let StackEntry::Retrn(slot) = on_top {
            *slot = opt
          } else {
            panic!("on top lays not retrn value, stack is corrupted")
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
