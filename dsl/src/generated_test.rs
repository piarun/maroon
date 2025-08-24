use crate::generated_types::*;

#[test]
fn some() {
  let vars = vec![Value::GlobalAddParamA(2), Value::GlobalAddParamB(10)];
  let result = global_step(State::GlobalAddEntry, &vars, &mut Heap::Global(GlobalHeap {}));

  assert_eq!(StepResult::Return(Some(Value::GlobalAddReturn(12))), result);
}
