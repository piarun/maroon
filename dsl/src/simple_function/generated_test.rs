use crate::simple_function::generated::*;
use crate::simple_function::task::*;

#[test]
fn add_function() {
  let (entries, _) = global_prepare_add(14, 16);

  let mut some_t =
    Task::new(entries, Heap::default(), |stack, _| RunResult::Done(Value::U64(global_result_add(&stack))));
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(30)), result);
}

#[test]
fn sub_add_function() {
  let (entries, _) = global_prepare_subAdd(6, 5, 4);

  let mut some_t =
    Task::new(entries, Heap::default(), |stack, _| RunResult::Done(Value::U64(global_result_subAdd(&stack))));
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(7)), result);
}

#[test]
fn factorial_function() {
  let (entries, _) = global_prepare_factorial(3);

  let mut some_t =
    Task::new(entries, Heap::default(), |stack, _| RunResult::Done(Value::U64(global_result_factorial(&stack))));
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(6)), result);
}

#[test]
fn b_search_function() {
  let search_elements = vec![1, 2, 3, 4, 5, 6, 7];

  let (entries, _) = global_prepare_binarySearch(4, 0, (search_elements.len() - 1) as u64);
  let heap = Heap { global: GlobalHeap { binarySearchValues: search_elements }, application: ApplicationHeap {} };

  let mut some_t =
    Task::new(entries, heap, |stack, _| RunResult::Done(Value::OptionU64(global_result_binarySearch(&stack))));
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::OptionU64(Some(3))), result);
}
