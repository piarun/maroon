use crate::ir::FiberType;
use crate::simple_function::fiber::*;
use crate::simple_function::generated::*;

#[test]
fn add_function() {
  let mut some_t = Fiber::new(FiberType::new("global"), 1);
  some_t.load_task("add", vec![Value::U64(4), Value::U64(8)], None);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(12)), result);
}

#[test]
fn sub_add_function() {
  let mut some_t = Fiber::new(FiberType::new("global"), 1);
  some_t.load_task("subAdd", vec![Value::U64(6), Value::U64(5), Value::U64(4)], None);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(7)), result);
}

#[test]
fn factorial_function() {
  let mut some_t = Fiber::new(FiberType::new("global"), 1);
  some_t.load_task("factorial", vec![Value::U64(3)], None);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(6)), result);
}

#[test]
fn b_search_function() {
  let search_elements = vec![1, 2, 3, 4, 5, 6, 7];
  let elements_len = search_elements.len() as u64;

  let heap = Heap { global: GlobalHeap { binarySearchValues: search_elements }, ..Default::default() };

  // initialize heap for this fiber before loading the task
  let mut some_t = Fiber::new_with_heap(FiberType::new("global"), heap, 1);

  some_t.load_task("binary_search", vec![Value::U64(4), Value::U64(0), Value::U64(elements_len - 1)], None);
  let result = some_t.run();
  assert_eq!(RunResult::Done(Value::OptionU64(Some(3))), result);

  some_t.load_task("binary_search", vec![Value::U64(10), Value::U64(0), Value::U64(elements_len - 1)], None);
  let result = some_t.run();
  assert_eq!(RunResult::Done(Value::OptionU64(None)), result);
}
