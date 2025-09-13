use crate::simple_function::generated::*;
use crate::simple_function::task::*;

#[test]
fn add_function() {
  let mut some_t = Task::new(Heap::default(), "global.add", vec![Value::U64(4), Value::U64(8)]);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(12)), result);
}

#[test]
fn sub_add_function() {
  let mut some_t = Task::new(Heap::default(), "global.subAdd", vec![Value::U64(6), Value::U64(5), Value::U64(4)]);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(7)), result);
}

#[test]
fn factorial_function() {
  let mut some_t = Task::new(Heap::default(), "global.factorial", vec![Value::U64(3)]);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(6)), result);
}

#[test]
fn b_search_function() {
  let search_elements = vec![1, 2, 3, 4, 5, 6, 7];
  let elements_len = search_elements.len() as u64;

  let heap = Heap { global: GlobalHeap { binarySearchValues: search_elements }, application: ApplicationHeap {} };

  let mut some_t =
    Task::new(heap, "global.binary_search", vec![Value::U64(4), Value::U64(0), Value::U64(elements_len - 1)]);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::OptionU64(Some(3))), result);
}
