use crate::{
  fiber::{Fiber, RunResult},
  test_helpers::assert_str_eq_by_lines,
  trace::TraceEvent,
};
use dsl::ir::FiberType;
use generated::maroon_assembler::{
  SelectArm, SetPrimitiveValue, StackEntry, State, StepResult, TestIncrementTask, Value,
};

#[test]
fn test_future_response() {
  // Pass init_vars via constructor
  let mut fiber =
    Fiber::new(FiberType::new("testTaskExecutorIncrementer"), 0, &vec![Value::String("testTasks".to_string())]);
  let mut dbg = String::new();
  let run_result = fiber.run(&mut dbg);

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

  let second_result = fiber.run(&mut dbg);
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
  let final_run = fiber.run(&mut dbg);
  assert_eq!(RunResult::Done, final_run);

  assert_eq!(
    r#"start function
f_task=TestIncrementTask(TestIncrementTask { inStrValue: 0, inStrRespFutureId: "", inStrRespQueueName: "" })
f_respFutureId=FutureTestIncrementTask(FutureTestIncrementTask(""))
f_respQueueName=
f_tasksQueueName=testTasks
after increment
f_task=TestIncrementTask(TestIncrementTask { inStrValue: 11, inStrRespFutureId: "my_test_future_id", inStrRespQueueName: "my_test_queue_name" })
f_respFutureId=FutureTestIncrementTask(FutureTestIncrementTask("my_test_future_id"))
f_respQueueName=my_test_queue_name
f_tasksQueueName=testTasks
"#,
    dbg
  );
}

#[test]
fn test_select_resume_mechanism() {
  let mut some_t = Fiber::new(FiberType::new("testSelectQueue"), 0, &vec![]);
  let mut dbg = String::new();
  let run_result = some_t.run(&mut dbg);
  let expected_selects_on_first_step = vec![
    SelectArm::Queue {
      queue_name: "counterStartQueue".to_string(),
      bind: "counter".to_string(),
      next: State::TestSelectQueueMainStartWork,
    },
    SelectArm::FutureVar {
      future_id: "testSelectQueue_future_1".to_string(),
      bind: Some("responseFromFut".to_string()),
      next: State::TestSelectQueueMainIncFromFut,
    },
  ];

  assert_eq!(RunResult::Select(expected_selects_on_first_step.clone()), run_result);
  assert_eq!(
    vec![
      TraceEvent {
        state: State::TestSelectQueueMainEntry,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(2, Value::String("counterStartQueue".to_string()))]),
          StackEntry::State(State::TestSelectQueueMainInitFutureId),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainInitFutureId,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(3, Value::String("testSelectQueue_future_1".to_string()))]),
          StackEntry::State(State::TestSelectQueueMainSelectCounter),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainSelectCounter,
        result: StepResult::Select(expected_selects_on_first_step.clone())
      },
    ],
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
    let queue_run_result = queue_response.run(&mut dbg);
    assert_eq!(RunResult::Done, queue_run_result);

    let future_run_result = future_response.run(&mut dbg);
    assert_eq!(RunResult::Done, future_run_result);
  }

  // check traces
  {
    let expected_inc_and_compare_tail = vec![
      TraceEvent {
        state: State::TestSelectQueueMainStartWork,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(0, Value::U64(2))]),
          StackEntry::State(State::TestSelectQueueMainPrepareCond),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainPrepareCond,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(4, Value::Bool(false))]),
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
          StackEntry::State(State::TestSelectQueueMainPrepareCond),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainPrepareCond,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(4, Value::Bool(true))]),
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
    let mut expected_queue_trace = vec![
      TraceEvent {
        state: State::TestSelectQueueMainEntry,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(2, Value::String("counterStartQueue".to_string()))]),
          StackEntry::State(State::TestSelectQueueMainInitFutureId),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainInitFutureId,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(3, Value::String("testSelectQueue_future_1".to_string()))]),
          StackEntry::State(State::TestSelectQueueMainSelectCounter),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainSelectCounter,
        result: StepResult::Select(expected_selects_on_first_step.clone()),
      },
    ];
    expected_queue_trace.extend(expected_inc_and_compare_tail.clone());
    assert_eq!(expected_queue_trace, queue_response.trace_sink);

    let mut expected_future_trace = vec![
      TraceEvent {
        state: State::TestSelectQueueMainEntry,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(2, Value::String("counterStartQueue".to_string()))]),
          StackEntry::State(State::TestSelectQueueMainInitFutureId),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainInitFutureId,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(3, Value::String("testSelectQueue_future_1".to_string()))]),
          StackEntry::State(State::TestSelectQueueMainSelectCounter),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainSelectCounter,
        result: StepResult::Select(expected_selects_on_first_step),
      },
      TraceEvent {
        state: State::TestSelectQueueMainIncFromFut,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(0, Value::U64(1))]),
          StackEntry::State(State::TestSelectQueueMainPrepareCond),
        ]),
      },
      TraceEvent {
        state: State::TestSelectQueueMainPrepareCond,
        result: StepResult::Next(vec![
          StackEntry::FrameAssign(vec![(4, Value::Bool(false))]),
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
fn fiber_call_different_functions() {
  // Pass init_vars via constructor
  let mut fiber = Fiber::new(
    FiberType::new("testFunctionsCall"),
    0,
    &vec![Value::U64(2), Value::U64(3), Value::U64(5), Value::ArrayU64(vec![1, 2, 3, 4, 5, 6, 7, 8]), Value::U64(4)],
  );
  let mut dbg = String::new();
  let run_result = fiber.run(&mut dbg);
  assert_eq!(RunResult::Done, run_result);

  assert_str_eq_by_lines(
    r#"multResult=0
factorialResult=0
binarySearchResult=OptionU64(None)
binarySearchLeft=0
binarySearchRight=0
multResult=6
factorialResult=0
binarySearchResult=OptionU64(None)
binarySearchLeft=0
binarySearchRight=0
multResult=6
factorialResult=120
binarySearchResult=OptionU64(None)
binarySearchLeft=0
binarySearchRight=0
multResult=6
factorialResult=120
binarySearchResult=OptionU64(Some(3))
binarySearchLeft=0
binarySearchRight=7
"#,
    &dbg,
  );
}
