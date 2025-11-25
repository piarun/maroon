use crate::{
  fiber::{Fiber, RunResult},
  trace::TraceEvent,
};
use dsl::ir::{FiberType, FutureLabel};
use generated::maroon_assembler::{
  BookSnapshot, GlobalHeap, Heap, Level, SelectArm, SetPrimitiveValue, StackEntry, State, StepResult,
  TestIncrementTask, Trade, Value,
};

#[test]
fn test_future_response() {
  let mut fiber = Fiber::new(FiberType::new("testTaskExecutorIncrementer"), 0);
  let run_result = fiber.run();

  assert_eq!(
    RunResult::Select(vec![SelectArm::Queue {
      queue_name: "testTasks".to_string(),
      bind: "f_task".to_string(),
      next: State::TestTaskExecutorIncrementerMainIncrement,
    }]),
    run_result
  );

  let input_task = TestIncrementTask {
    inStrValue: 10,
    inStrRespFutureId: "my_test_future_id".to_string(),
    inStrRespQueueName: "my_test_queue_name".to_string(),
  };

  fiber.assign_local_and_push_next(
    "f_task".to_string(),
    Value::TestIncrementTask(input_task.clone()),
    State::TestTaskExecutorIncrementerMainIncrement,
  );

  let second_result = fiber.run();
  assert_eq!(
    RunResult::SetValues(vec![
      SetPrimitiveValue::Future {
        id: "my_test_future_id".to_string(),
        value: Value::TestIncrementTask(TestIncrementTask { inStrValue: 11, ..input_task.clone() })
      },
      SetPrimitiveValue::QueueMessage {
        queue_name: "my_test_queue_name".to_string(),
        value: Value::TestIncrementTask(TestIncrementTask { inStrValue: 11, ..input_task.clone() })
      }
    ]),
    second_result
  );

  // make sure that fiber can successfully continue and finish
  let final_run = fiber.run();
  assert_eq!(RunResult::Done(Value::Unit(())), final_run);

  assert_eq!(
    r#"start function
f_task=TestIncrementTask(TestIncrementTask { inStrValue: 0, inStrRespFutureId: "", inStrRespQueueName: "" })
f_respFutureId=
f_respQueueName=
after increment
f_task=TestIncrementTask(TestIncrementTask { inStrValue: 11, inStrRespFutureId: "my_test_future_id", inStrRespQueueName: "my_test_queue_name" })
f_respFutureId=my_test_future_id
f_respQueueName=my_test_queue_name"#,
    fiber.dbg_out
  );
}

#[test]
fn test_select_resume_mechanism() {
  let mut some_t = Fiber::new(FiberType::new("testSelectQueue"), 0);
  let run_result = some_t.run();
  let expected_selects_on_first_step = vec![
    SelectArm::Queue {
      queue_name: "counterStartQueue".to_string(),
      bind: "counter".to_string(),
      next: State::TestSelectQueueMainStartWork,
    },
    SelectArm::Future {
      future_id: FutureLabel::new("testSelectQueue_future_1".to_string()),
      bind: Some("responseFromFut".to_string()),
      next: State::TestSelectQueueMainIncFromFut,
    },
  ];

  assert_eq!(RunResult::Select(expected_selects_on_first_step.clone()), run_result);
  assert_eq!(
    vec![TraceEvent {
      state: State::TestSelectQueueMainEntry,
      result: StepResult::Select(expected_selects_on_first_step.clone())
    }],
    some_t.trace_sink
  );

  // `split` fiber, and further I'll run both select return branches
  let mut queue_response = some_t.clone();
  let mut future_response = some_t;

  // imitation resumes from runtime
  {
    // queue imitation
    // we pass counter == 1 - so counter will start from 1
    queue_response.assign_local_and_push_next(
      "counter".to_string(),
      Value::U64(1),
      State::TestSelectQueueMainStartWork,
    );

    // future imitation
    // we pass responseFromFut == 2 - so counter will start from 1
    future_response.assign_local_and_push_next(
      "responseFromFut".to_string(),
      Value::U64(2),
      State::TestSelectQueueMainIncFromFut,
    );
  }

  // Continue execution; should complete
  {
    let queue_run_result = queue_response.run();
    assert_eq!(RunResult::Done(Value::Unit(())), queue_run_result);

    let future_run_result = future_response.run();
    assert_eq!(RunResult::Done(Value::Unit(())), future_run_result);
  }

  // check traces
  {
    let expected_inc_and_compare_tail = vec![
      TraceEvent {
        state: State::TestSelectQueueMainStartWork,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(0, Value::U64(2))]),
          StackEntry::State(State::TestSelectQueueMainCompare),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainCompare,
        result: StepResult::GoTo(State::TestSelectQueueMainStartWork),
      },
      TraceEvent {
        state: State::TestSelectQueueMainStartWork,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(0, Value::U64(3))]),
          StackEntry::State(State::TestSelectQueueMainCompare),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainCompare,
        result: StepResult::GoTo(State::TestSelectQueueMainReturn),
      },
      TraceEvent { state: State::TestSelectQueueMainReturn, result: StepResult::ReturnVoid },
    ];

    // Build expected vectors using extend (extend returns (), so build first then assert)
    let mut expected_queue_trace = vec![TraceEvent {
      state: State::TestSelectQueueMainEntry,
      result: StepResult::Select(expected_selects_on_first_step.clone()),
    }];
    expected_queue_trace.extend(expected_inc_and_compare_tail.clone());
    assert_eq!(expected_queue_trace, queue_response.trace_sink);

    let mut expected_future_trace = vec![
      TraceEvent { state: State::TestSelectQueueMainEntry, result: StepResult::Select(expected_selects_on_first_step) },
      TraceEvent {
        state: State::TestSelectQueueMainIncFromFut,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(0, Value::U64(1))]),
          StackEntry::State(State::TestSelectQueueMainCompare),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainCompare,
        result: StepResult::GoTo(State::TestSelectQueueMainStartWork),
      },
    ];
    expected_future_trace.extend(expected_inc_and_compare_tail);
    assert_eq!(expected_future_trace, future_response.trace_sink);
  }
}

#[test]
fn add_function() {
  let mut some_t = Fiber::new_empty(FiberType::new("global"), 1);
  some_t.load_task("add", vec![Value::U64(4), Value::U64(8)], None);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(12)), result);
}

#[test]
fn sub_add_function() {
  let mut some_t = Fiber::new_empty(FiberType::new("global"), 1);
  some_t.load_task("subAdd", vec![Value::U64(6), Value::U64(5), Value::U64(4)], None);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(7)), result);
}

#[test]
fn factorial_function() {
  let mut some_t = Fiber::new_empty(FiberType::new("global"), 1);
  some_t.load_task("factorial", vec![Value::U64(3)], None);
  let result = some_t.run();

  assert_eq!(RunResult::Done(Value::U64(6)), result);
  assert_eq!(
    vec![
      TraceEvent { state: State::GlobalFactorialEntry, result: StepResult::GoTo(State::GlobalFactorialSubtract) },
      TraceEvent {
        state: State::GlobalFactorialSubtract,
        result: StepResult::Next(vec![
          StackEntry::State(State::GlobalFactorialFactorialCall),
          StackEntry::Retrn(Some(3)),
          StackEntry::Value("a".to_string(), Value::U64(3)),
          StackEntry::Value("b".to_string(), Value::U64(1)),
          StackEntry::Value("sub".to_string(), Value::U64(0)),
          StackEntry::State(State::GlobalSubEntry),
        ]),
      },
      TraceEvent { state: State::GlobalSubEntry, result: StepResult::Return(Value::U64(2)) },
      TraceEvent {
        state: State::GlobalFactorialFactorialCall,
        result: StepResult::Next(vec![
          StackEntry::State(State::GlobalFactorialMultiply),
          StackEntry::Retrn(Some(4)),
          StackEntry::Value("n".to_string(), Value::U64(2)),
          StackEntry::Value("fac_call_res".to_string(), Value::U64(0)),
          StackEntry::Value("subtract_res".to_string(), Value::U64(0)),
          StackEntry::Value("result".to_string(), Value::U64(0)),
          StackEntry::State(State::GlobalFactorialEntry),
        ]),
      },
      TraceEvent { state: State::GlobalFactorialEntry, result: StepResult::GoTo(State::GlobalFactorialSubtract) },
      TraceEvent {
        state: State::GlobalFactorialSubtract,
        result: StepResult::Next(vec![
          StackEntry::State(State::GlobalFactorialFactorialCall),
          StackEntry::Retrn(Some(3)),
          StackEntry::Value("a".to_string(), Value::U64(2)),
          StackEntry::Value("b".to_string(), Value::U64(1)),
          StackEntry::Value("sub".to_string(), Value::U64(0)),
          StackEntry::State(State::GlobalSubEntry),
        ]),
      },
      TraceEvent { state: State::GlobalSubEntry, result: StepResult::Return(Value::U64(1)) },
      TraceEvent {
        state: State::GlobalFactorialFactorialCall,
        result: StepResult::Next(vec![
          StackEntry::State(State::GlobalFactorialMultiply),
          StackEntry::Retrn(Some(4)),
          StackEntry::Value("n".to_string(), Value::U64(1)),
          StackEntry::Value("fac_call_res".to_string(), Value::U64(0)),
          StackEntry::Value("subtract_res".to_string(), Value::U64(0)),
          StackEntry::Value("result".to_string(), Value::U64(0)),
          StackEntry::State(State::GlobalFactorialEntry),
        ]),
      },
      TraceEvent { state: State::GlobalFactorialEntry, result: StepResult::GoTo(State::GlobalFactorialReturn1) },
      TraceEvent { state: State::GlobalFactorialReturn1, result: StepResult::Return(Value::U64(1)) },
      TraceEvent {
        state: State::GlobalFactorialMultiply,
        result: StepResult::Next(vec![
          StackEntry::State(State::GlobalFactorialReturn),
          StackEntry::Retrn(Some(2)),
          StackEntry::Value("a".to_string(), Value::U64(2)),
          StackEntry::Value("b".to_string(), Value::U64(1)),
          StackEntry::Value("mult".to_string(), Value::U64(0)),
          StackEntry::State(State::GlobalMultEntry),
        ]),
      },
      TraceEvent { state: State::GlobalMultEntry, result: StepResult::Return(Value::U64(2)) },
      TraceEvent { state: State::GlobalFactorialReturn, result: StepResult::Return(Value::U64(2)) },
      TraceEvent {
        state: State::GlobalFactorialMultiply,
        result: StepResult::Next(vec![
          StackEntry::State(State::GlobalFactorialReturn),
          StackEntry::Retrn(Some(2)),
          StackEntry::Value("a".to_string(), Value::U64(3)),
          StackEntry::Value("b".to_string(), Value::U64(2)),
          StackEntry::Value("mult".to_string(), Value::U64(0)),
          StackEntry::State(State::GlobalMultEntry),
        ]),
      },
      TraceEvent { state: State::GlobalMultEntry, result: StepResult::Return(Value::U64(6)) },
      TraceEvent { state: State::GlobalFactorialReturn, result: StepResult::Return(Value::U64(6)) },
    ],
    some_t.trace_sink,
  );
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

#[test]
fn order_book_add_no_match_and_best_quotes() {
  let mut ob = Fiber::new_with_heap(FiberType::new("order_book"), Heap::default(), 1);

  // add BUY 100@10 into empty book -> no trades
  ob.load_task("add_buy", vec![Value::U64(1), Value::U64(10), Value::U64(100)], None);
  let r = ob.run();
  assert_eq!(RunResult::Done(Value::ArrayTrade(vec![])), r);

  // best bid should be 10, best ask None
  ob.load_task("best_bid", vec![], None);
  let r = ob.run();
  assert_eq!(RunResult::Done(Value::OptionU64(Some(10))), r);

  ob.load_task("best_ask", vec![], None);
  let r = ob.run();
  assert_eq!(RunResult::Done(Value::OptionU64(None)), r);
}

#[test]
fn order_book_full_match_single_level() {
  let mut ob = Fiber::new_with_heap(FiberType::new("order_book"), Heap::default(), 2);

  // SELL 50@12
  ob.load_task("add_sell", vec![Value::U64(10), Value::U64(12), Value::U64(50)], None);
  let r = ob.run();
  assert_eq!(RunResult::Done(Value::ArrayTrade(vec![])), r);

  // BUY 50@12 fully matches
  ob.load_task("add_buy", vec![Value::U64(11), Value::U64(12), Value::U64(50)], None);
  let r = ob.run();
  let expected = vec![Trade { price: 12, qty: 50, takerId: 11, makerId: 10 }];
  assert_eq!(RunResult::Done(Value::ArrayTrade(expected)), r);

  // Book cleared on asks side
  ob.load_task("best_ask", vec![], None);
  let r = ob.run();
  assert_eq!(RunResult::Done(Value::OptionU64(None)), r);
}

#[test]
fn order_book_partial_match_and_depth() {
  let mut ob = Fiber::new_with_heap(FiberType::new("order_book"), Heap::default(), 3);

  // Seed: SELL 80@12 by maker 100
  ob.load_task("add_sell", vec![Value::U64(100), Value::U64(12), Value::U64(80)], None);
  let _ = ob.run();

  // BUY 50@12 -> trade 50@12, remaining SELL 30@12 stays
  ob.load_task("add_buy", vec![Value::U64(101), Value::U64(12), Value::U64(50)], None);
  let r = ob.run();
  let expected = vec![Trade { price: 12, qty: 50, takerId: 101, makerId: 100 }];
  assert_eq!(RunResult::Done(Value::ArrayTrade(expected)), r);

  // Best ask remains 12
  ob.load_task("best_ask", vec![], None);
  let r = ob.run();
  assert_eq!(RunResult::Done(Value::OptionU64(Some(12))), r);

  // Depth snapshot top 1: asks [12:30], bids []
  ob.load_task("top_n_depth", vec![Value::U64(1)], None);
  let r = ob.run();
  let expected = BookSnapshot { bids: vec![], asks: vec![Level { price: 12, qty: 30 }] };
  assert_eq!(RunResult::Done(Value::BookSnapshot(expected)), r);
}

#[test]
fn order_book_cross_multiple_levels_and_fifo_cancel() {
  let mut ob = Fiber::new_with_heap(FiberType::new("order_book"), Heap::default(), 4);

  // Seed asks: 10@12 (id 201), 20@13 (202), 40@14 (203), plus FIFO on 15
  ob.load_task("add_sell", vec![Value::U64(201), Value::U64(12), Value::U64(10)], None);
  let _ = ob.run();
  ob.load_task("add_sell", vec![Value::U64(202), Value::U64(13), Value::U64(20)], None);
  let _ = ob.run();
  ob.load_task("add_sell", vec![Value::U64(203), Value::U64(14), Value::U64(40)], None);
  let _ = ob.run();

  // Add two sells at same price (FIFO): A=30@15 (204), then B=20@15 (205)
  ob.load_task("add_sell", vec![Value::U64(204), Value::U64(15), Value::U64(30)], None);
  let _ = ob.run();
  ob.load_task("add_sell", vec![Value::U64(205), Value::U64(15), Value::U64(20)], None);
  let _ = ob.run();

  // Aggressive BUY 50@14: matches 10@12, 20@13, 20@14; leaves 20@14
  ob.load_task("add_buy", vec![Value::U64(300), Value::U64(14), Value::U64(50)], None);
  let r = ob.run();
  let expected = vec![
    Trade { price: 12, qty: 10, takerId: 300, makerId: 201 },
    Trade { price: 13, qty: 20, takerId: 300, makerId: 202 },
    Trade { price: 14, qty: 20, takerId: 300, makerId: 203 },
  ];
  assert_eq!(RunResult::Done(Value::ArrayTrade(expected)), r);

  // BUY 40@15 continues: first 20@14 (leftover), then 20 from A=30@15 (FIFO)
  ob.load_task("add_buy", vec![Value::U64(301), Value::U64(15), Value::U64(40)], None);
  let r = ob.run();
  let expected = vec![
    Trade { price: 14, qty: 20, takerId: 301, makerId: 203 },
    Trade { price: 15, qty: 20, takerId: 301, makerId: 204 },
  ];
  assert_eq!(RunResult::Done(Value::ArrayTrade(expected)), r);

  // Cancel B (remaining 20@15)
  ob.load_task("cancel", vec![Value::U64(205)], None);
  let r = ob.run();
  assert_eq!(RunResult::Done(Value::U64(1)), r);
}
