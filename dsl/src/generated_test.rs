use crate::generated_types::*;

#[test]
fn primitive_one_tick_function() {
  let vars = vec![Value::GlobalAddParamA(2), Value::GlobalAddParamB(10)];
  let result = global_step(State::GlobalAddEntry, &vars, &mut Heap::Global(GlobalHeap {}));

  assert_eq!(StepResult::Return(Some(Value::GlobalAddReturn(12))), result);
}

#[test]
fn random_function() {
  let result = global_step(State::GlobalRandGenEntry, &vec![], &mut Heap::Global(GlobalHeap {}));

  if let StepResult::Return(Some(Value::GlobalRandGenReturn(_))) = result {
  } else {
    panic!("failed test, should return some random number");
  }
}
