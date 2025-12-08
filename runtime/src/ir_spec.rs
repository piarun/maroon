use dsl::ir::*;
use std::collections::HashMap;

// Shared IR specification used by build.rs (via include!) and by tests.
pub fn sample_ir() -> IR {
  IR {
    fibers: HashMap::from([
      (
        FiberType::new("root"),
        Fiber {
          fibers_limit: 0,
          init_vars: vec![],
          heap: HashMap::new(),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func{in_vars: vec![],out: Type::Void, locals: vec![], steps: vec![
                (
                  StepId::new("entry"),
                  Step::ReturnVoid,
                ),
              ]},
            ),
          ]),
        }
      ),
      (
        // fiber for testing select mechanism
        // awaits a new start counter value and then starts count
        FiberType::new("testSelectQueue"),
        Fiber {
          fibers_limit: 0,
          init_vars: vec![],
          heap: HashMap::new(),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func{
                in_vars: vec![],
                out: Type::Void,
                locals: vec![LocalVar("counter", Type::UInt64), LocalVar("responseFromFut", Type::UInt64), LocalVar("counterStartQueueName", Type::String), LocalVar("futureId", Type::String)], 
                steps: vec![
                (
                  StepId::new("entry"),
                Step::Let { local: "counterStartQueueName".to_string(), expr: Expr::Str("counterStartQueue".to_string()), next: StepId::new("init_future_id") },
                ),
                (
                  StepId::new("init_future_id"),
                  Step::Let { local: "futureId".to_string(), expr: Expr::Str("testSelectQueue_future_1".to_string()), next: StepId::new("select_counter") },
                ),
                (
                  StepId::new("select_counter"),
                  Step::Select { arms: vec![
                    AwaitSpec::Queue{
                      queue_name: LocalVarRef("counterStartQueueName"),
                      message_var: LocalVarRef("counter"),
                      next: StepId::new("start_work"),
                    },
                    AwaitSpec::Future {
                      // doesn't matter how this future ended up here for tests
                      // in real life this future should be created or passed somehow
                      bind: Some(LocalVarRef("responseFromFut")),
                      ret_to: StepId::new("inc_from_fut"),
                      future_id: LocalVarRef("futureId"),
                    }
                  ] },
                ),
                (
                  // added this artificial step to see the difference in path in tests
                  StepId::new("inc_from_fut"),
                  Step::RustBlock { binds: vec![LocalVarRef("counter")], code: "responseFromFut - 1".to_string(), next: StepId::new("compare") },
                ),
                (
                  StepId::new("start_work"),
                  Step::RustBlock { binds: vec![LocalVarRef("counter")], code: "counter + 1".to_string(), next: StepId::new("compare") },
                ),
                (
                  StepId::new("compare"),
                  Step::If { cond: Expr::Equal(Box::new(Expr::Var(LocalVarRef("counter"))), Box::new(Expr::UInt64(3))), then_: StepId::new("return"), else_: StepId::new("start_work") },
                ),
                (
                  StepId::new("return"),
                  Step::ReturnVoid,
                ),
              ]},
            ),
          ]),
        }
      ),
      (
        FiberType::new("testTaskExecutorIncrementer"),
        Fiber {
          fibers_limit: 0,
          init_vars: vec![
            InVar("in_taskQueueName", Type::String),
          ],
          heap: HashMap::new(),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func{in_vars: vec![],out: Type::Void,
                locals: vec![
                  // I make such weird names to make sure that in tests I don't use the same strings and conversion happens correctly
                  // I also want to explicitly verify names conversion, because right now it jumps between snake and camel case, which should be fixed for sure
                  LocalVar("f_task", Type::Custom("TestIncrementTask".to_string())),
                  LocalVar("f_respFutureId", Type::Future(Box::new(Type::Custom("TestIncrementTask".to_string())))),
                  LocalVar("f_respQueueName", Type::String),
                  LocalVar("f_tasksQueueName", Type::String),
                ],
                steps: vec![
                (
                  StepId::new("entry"),
                  Step::Debug("start function", StepId::new("init_queue_name")),
                ),
                (
                  StepId::new("init_queue_name"),
                  Step::RustBlock {
                    binds: vec![LocalVarRef("f_tasksQueueName")], 
                    code: "heap.testTaskExecutorIncrementer.in_vars.inTaskqueuename.clone()".to_string(), 
                    next: StepId::new("debug_vars"),
                  }
                ),
                (
                  StepId::new("debug_vars"),
                  Step::DebugPrintVars(StepId::new("await")),
                ),
                (
                  StepId::new("await"),
                  Step::Select { arms: vec![
                    AwaitSpec::Queue{
                      queue_name: LocalVarRef("f_tasksQueueName"),
                      message_var: LocalVarRef("f_task"),
                      next: StepId::new("increment"),
                    },
                  ] },
                ),
                (
                  StepId::new("increment"),
                  Step::RustBlock { binds: vec![LocalVarRef("f_task"), LocalVarRef("f_respQueueName"), LocalVarRef("f_respFutureId")], code: r#"
                    let mut t_m = fTask;
                    t_m.inStrValue += 1;
                    (t_m.clone(), t_m.inStrRespQueueName, FutureTestIncrementTask(t_m.inStrRespFutureId))
                  "#.
                  to_string(), next: StepId::new("debug2") },
                ),
                (
                  StepId::new("debug2"),
                  Step::Debug("after increment", StepId::new("debug_vars2")),
                ),
                (
                  StepId::new("debug_vars2"),
                  Step::DebugPrintVars(StepId::new("return_result")),
                ),
                (
                  StepId::new("return_result"),
                  Step::SetValues {
                    values: vec![
                      SetPrimitive::Future { f_var_name: LocalVarRef("f_respFutureId"), var_name: LocalVarRef("f_task") },
                      SetPrimitive::QueueMessage { f_var_queue_name: LocalVarRef("f_respQueueName"), var_name: LocalVarRef("f_task") },
                    ],
                    next: StepId::new("return"),
                  },
                ),
                (
                  StepId::new("return"),
                  Step::ReturnVoid,
                ),
              ]},
            ),
          ]),
        }
      ),
      (
        // fiber for testing create queue mechanism
        FiberType::new("testCreateQueue"),
        Fiber {
          fibers_limit: 0,
          init_vars: vec![],
          heap: HashMap::new(),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func{
                in_vars: vec![],
                out: Type::Void,
                locals: vec![
                  LocalVar("value", Type::Custom("TestCreateQueueMessage".to_string())), 
                  LocalVar("f_queueName", Type::String),
                  LocalVar("created_queue_name", Type::String),
                  LocalVar("f_queueCreationError", Type::Option(Box::new(Type::String))),
                  LocalVar("f_future_id_response", Type::Future(Box::new(Type::UInt64))),
                  LocalVar("f_res_inc", Type::UInt64),
                ],
                steps: vec![
                (
                  StepId::new("entry"),
                  Step::Let { local: "f_queueName".to_string(), expr: Expr::Str("randomQueueName".to_string()), next: StepId::new("wrong_queue_creation") },
                ),
                (
                  StepId::new("wrong_queue_creation"),
                  Step::Create {
                    primitives: vec![
                      RuntimePrimitive::Queue { name: LocalVarRef("f_queueName"), public: true },
                      RuntimePrimitive::Queue { name: LocalVarRef("f_queueName"), public: true },
                    ],
                    success: SuccessCreateBranch { next: StepId::new("return"), id_binds: vec![LocalVarRef("created_queue_name"), LocalVarRef("created_queue_name")] }, 
                    fail: FailCreateBranch { next: StepId::new("debug_vars"), error_binds: vec![LocalVarRef("f_queueCreationError"), LocalVarRef("f_queueCreationError")] }, 
                  },
                ),
                (
                  StepId::new("debug_vars"),
                  Step::DebugPrintVars(StepId::new("clean_up")),
                ),
                (
                  StepId::new("clean_up"),
                  Step::RustBlock {
                    binds: vec![
                      LocalVarRef("created_queue_name"),
                      LocalVarRef("f_queueCreationError"),
                    ],
                    code: r#"(String::new(), None)"#.to_string(), 
                    next: StepId::new("correct_creation"), 
                  },
                ),
                (
                  StepId::new("correct_creation"),
                  Step::Create {
                    primitives: vec![
                      RuntimePrimitive::Queue { name: LocalVarRef("f_queueName"), public: true },
                    ],
                    success: SuccessCreateBranch { next: StepId::new("debug_vars_2"), id_binds: vec![LocalVarRef("created_queue_name")] }, 
                    fail: FailCreateBranch { next: StepId::new("return"), error_binds: vec![LocalVarRef("f_queueCreationError")] }, 
                  },
                ),
                (
                  StepId::new("debug_vars_2"),
                  Step::DebugPrintVars(StepId::new("await_on_queue")),
                ),
                (
                  StepId::new("await_on_queue"),
                  Step::Select {
                    arms: vec![AwaitSpec::Queue {
                      queue_name: LocalVarRef("created_queue_name"), 
                      message_var: LocalVarRef("value"), 
                      next: StepId::new("extract_fut_and_inc"),
                    }],
                  },
                ),
                (
                  StepId::new("extract_fut_and_inc"),
                  Step::RustBlock {
                    binds: vec![
                      LocalVarRef("f_future_id_response"),
                      LocalVarRef("f_res_inc"),
                    ],
                    code: "(value.publicFutureId, value.value + 2)".to_string(), 
                    next: StepId::new("debug_vars_3"),
                  },
                ),
                (
                  StepId::new("debug_vars_3"),
                  Step::DebugPrintVars(StepId::new("answer")),
                ),
                (
                  StepId::new("answer"),
                  Step::SetValues {
                    values: vec![SetPrimitive::Future {
                      f_var_name: LocalVarRef("f_future_id_response"), 
                      var_name: LocalVarRef("f_res_inc"),
                    }],
                    next: StepId::new("return"),
                  },
                ),
                (
                  StepId::new("return"),
                  Step::ReturnVoid,
                ),
              ]},
            ),
          ]),
        }
      ),
      (
        FiberType::new("testRootFiber"),
        Fiber {
          fibers_limit: 0,
          init_vars: vec![],
          heap: HashMap::new(),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals: vec![
                  LocalVar("rootQueueName", Type::String),
                  LocalVar("calculatorTask", Type::Custom("TestCalculatorTask".to_string())),
                  LocalVar("calculatorTask2", Type::Custom("TestCalculatorTask".to_string())),
                  LocalVar("responseFutureId", Type::Future(Box::new(Type::UInt64))),
                  LocalVar("responseFutureId2", Type::Future(Box::new(Type::UInt64))),
                  LocalVar("responseFromCalculator", Type::String),
                  LocalVar("responseFromCalculator2", Type::String),
                  LocalVar("createQueueError", Type::Option(Box::new(Type::String))),
                  LocalVar("createFutureError", Type::Option(Box::new(Type::String))),
                  LocalVar("createFutureError2", Type::Option(Box::new(Type::String))),
                ],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::Let { 
                      local: "rootQueueName".to_string(), 
                      expr: Expr::Str("rootQueue".to_string()), 
                      next: StepId::new("create_queueues"),
                    },
                  ),
                  (
                    StepId::new("create_queueues"),
                    Step::Create { 
                      primitives: vec![
                        RuntimePrimitive::Queue { 
                          name: LocalVarRef("rootQueueName"), 
                          public: true, 
                        },
                      ], 
                      success: SuccessCreateBranch { next: StepId::new("create_fiber"), id_binds: vec![LocalVarRef("rootQueueName")] }, 
                      fail: FailCreateBranch { next: StepId::new("return_dbg"), error_binds: vec![LocalVarRef("createQueueError")] }, 
                    }
                  ),
                  (
                    StepId::new("create_fiber"),
                    Step::CreateFibers { 
                      details: vec![
                        CreateFiberDetail {
                          f_name: FiberType::new("testCalculator"),
                          init_vars: vec![LocalVarRef("rootQueueName")],
                        },
                        CreateFiberDetail {
                          f_name: FiberType::new("testCalculator"),
                          init_vars: vec![LocalVarRef("rootQueueName")],
                        },
                      ],
                      next: StepId::new("create_future"),
                    },
                  ),
                  (
                    StepId::new("create_future"),
                    Step::Create { 
                      primitives: vec![
                        RuntimePrimitive::Future{},
                        RuntimePrimitive::Future{},
                      ], 
                      success: SuccessCreateBranch { 
                        next: StepId::new("prepareCalculationRequests"), 
                        id_binds: vec![
                          LocalVarRef("responseFutureId"),
                          LocalVarRef("responseFutureId2"),
                        ],
                      }, 
                      fail: FailCreateBranch { next: StepId::new("return_dbg"), error_binds: vec![LocalVarRef("createFutureError"),LocalVarRef("createFutureError2")] }, 
                    },
                  ),
                  (
                    StepId::new("prepareCalculationRequests"),
                    Step::RustBlock { 
                      binds: vec![
                        LocalVarRef("calculatorTask"),
                        LocalVarRef("calculatorTask2"),
                      ], 
                      code: "(TestCalculatorTask{a:10,b:15,responseFutureId: responseFutureId}, TestCalculatorTask{a:2,b:4,responseFutureId: responseFutureId2})".to_string(), 
                      next: StepId::new("send_calculation_request"),
                    },
                  ),
                  (
                    StepId::new("send_calculation_request"),
                    Step::SetValues { 
                      values: vec![
                        SetPrimitive::QueueMessage { 
                          f_var_queue_name: LocalVarRef("rootQueueName"), 
                          var_name: LocalVarRef("calculatorTask"),
                        },
                        SetPrimitive::QueueMessage { 
                          f_var_queue_name: LocalVarRef("rootQueueName"), 
                          var_name: LocalVarRef("calculatorTask2"),
                        },
                      ], 
                      next: StepId::new("await_response"),
                    },
                  ),
                  (
                    StepId::new("await_response"),
                    Step::Select { arms: vec![
                      AwaitSpec::Future { 
                        bind: Some(LocalVarRef("responseFromCalculator")), 
                        ret_to: StepId::new("await_response_2"), 
                        future_id: LocalVarRef("responseFutureId2"),
                      },
                    ] },
                  ),
                  (
                    StepId::new("await_response_2"),
                    Step::Select { arms: vec![
                      AwaitSpec::Future { 
                        bind: Some(LocalVarRef("responseFromCalculator2")), 
                        ret_to: StepId::new("return_dbg"), 
                        future_id: LocalVarRef("responseFutureId"),
                      },
                    ] },
                  ),
                  (
                    StepId::new("return_dbg"),
                    Step::DebugPrintVars(StepId::new("return")),
                  ),
                  (
                    StepId::new("return"),
                    Step::ReturnVoid,
                  ),
                ],
              },
            )
          ]),
        }
      ),
      (
        FiberType::new("testCalculator"),
        Fiber {
          fibers_limit: 0,
          init_vars: vec![
            InVar("calculationRequestsQueueName", Type::String),
          ],
          heap: HashMap::new(),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals: vec![
                  LocalVar("request", Type::Custom("TestCalculatorTask".to_string())),
                  LocalVar("result", Type::UInt64),
                  LocalVar("respFutureId", Type::Future(Box::new(Type::UInt64))),
                ],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::DebugPrintVars(StepId::new("select_queue"))
                  ),
                  (
                    StepId::new("select_queue"),
                    Step::Select { arms: vec![
                      AwaitSpec::Queue { 
                        queue_name: LocalVarRef("calculationRequestsQueueName"), 
                        message_var: LocalVarRef("request"), 
                        next: StepId::new("debug_gotten_task"),
                      },
                    ] },
                  ),
                  (
                    StepId::new("debug_gotten_task"),
                    Step::Debug("got task from the queue",StepId::new("debug_vars"))
                  ),
                  (
                    StepId::new("debug_vars"),
                    Step::DebugPrintVars(StepId::new("calculate"))
                  ),
                  (
                    StepId::new("calculate"),
                    Step::RustBlock { 
                      binds: vec![
                        LocalVarRef("result"), 
                        LocalVarRef("respFutureId"),
                      ], 
                      code: "(request.a * request.b, request.responseFutureId)".to_string(), 
                      next: StepId::new("response"),
                    },
                  ),
                  (
                    StepId::new("response"),
                    Step::SetValues { 
                      values: vec![SetPrimitive::Future { 
                        f_var_name: LocalVarRef("respFutureId"), 
                        var_name: LocalVarRef("result"),
                      }], 
                      next: StepId::new("return"),
                    },
                  ),
                  (
                    StepId::new("return"),
                    Step::ReturnVoid,
                  ),
                ],
              },
            )
          ]),
        }
      ),
      (
        FiberType::new("testRootFiberSleepTest"),
        Fiber {
          fibers_limit: 0,
          heap: HashMap::new(),
          init_vars:vec![],
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                locals: vec![
                  LocalVar("scheduledFutId", Type::Future(Box::new(Type::Void))),
                  LocalVar("createScheduleError", Type::Option(Box::new(Type::String))),
                  LocalVar("await_milliseconds", Type::UInt64),
                ],
                out: Type::Void,
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::Let { local: "await_milliseconds".to_string(), expr: Expr::UInt64(150), next: StepId::new("create_primitives") }
                  ),
                  (
                    StepId::new("create_primitives"),
                    Step::Create { 
                      primitives: vec![RuntimePrimitive::Schedule { ms_var: LocalVarRef("await_milliseconds") }], 
                      success: SuccessCreateBranch { next: StepId::new("seelect"), id_binds: vec![LocalVarRef("scheduledFutId")] }, 
                      fail: FailCreateBranch { next: StepId::new("return_dbg"), error_binds: vec![LocalVarRef("createScheduleError")] },
                    }
                  ),
                  (
                    StepId::new("seelect"),
                    Step::Select { arms: vec![
                      AwaitSpec::Future { 
                        bind: None, 
                        ret_to: StepId::new("return_dbg"), 
                        future_id: LocalVarRef("scheduledFutId"),
                      }] },
                  ),
                  (
                    StepId::new("return_dbg"),
                    Step::DebugPrintVars(StepId::new("return")),
                  ),
                  (
                    StepId::new("return"),
                    Step::ReturnVoid,
                  ),
                ],
              }
            ),
          ]),
        },
      ),
      (
        FiberType::new("global"),
        Fiber {
          fibers_limit: 100,
          init_vars: vec![],
          heap: HashMap::from([("binary_search_values".to_string(), Type::Array(Box::new(Type::UInt64)))]),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals:vec![],
                steps: vec![(StepId::new("entry"), Step::ReturnVoid)],
              },
            ),
            (
              "add".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sum", Type::UInt64)],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("sum")],
                      code: "a+b".to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("sum")) }),
                ],
              },
            ),
            (
              "sub".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sub", Type::UInt64)],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("sub")],
                      code: r#"
let out = a - b;
out
"#
                      .to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("sub")) }),
                ],
              },
            ),
            (
              "mult".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("mult", Type::UInt64)],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("mult")],
                      code: "a*b".to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("mult")) }),
                ],
              },
            ),
            (
              "div".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("div", Type::UInt64)],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("div")],
                      code: "a/b".to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("div")) }),
                ],
              },
            ),
            (
              // factorial(n) { if n == 1 { return 1 } return n * factorial(n - 1) }
              "factorial".to_string(),
              Func {
                in_vars: vec![InVar("n", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![
                  LocalVar("fac_call_res", Type::UInt64),
                  LocalVar("subtract_res", Type::UInt64),
                  LocalVar("result", Type::UInt64),
                ],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::If {
                      cond: Expr::Equal(Box::new(Expr::Var(LocalVarRef("n"))), Box::new(Expr::UInt64(1))),
                      then_: StepId::new("return_1"),
                      else_: StepId::new("subtract"),
                    },
                  ),
                  (StepId::new("return_1"), Step::Return { value: RetValue::UInt64(1) }),
                  (
                    StepId::new("subtract"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                      args: vec![Expr::Var(LocalVarRef("n")), Expr::UInt64(1)],
                      bind: Some(LocalVarRef("subtract_res")),
                      ret_to: StepId::new("factorial_call"),
                    },
                  ),
                  (
                    StepId::new("factorial_call"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "factorial".to_string() },
                      args: vec![Expr::Var(LocalVarRef("subtract_res"))],
                      bind: Some(LocalVarRef("fac_call_res")),
                      ret_to: StepId::new("multiply"),
                    },
                  ),
                  (
                    StepId::new("multiply"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "mult".to_string() },
                      args: vec![Expr::Var(LocalVarRef("n")), Expr::Var(LocalVarRef("fac_call_res"))],
                      bind: Some(LocalVarRef("result")),
                      ret_to: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("result")) }),
                ],
              },
            ),
            (
              "subAdd".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64), InVar("c", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sumAB", Type::UInt64), LocalVar("subABC", Type::UInt64)],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "add".to_string() },
                      args: vec![Expr::Var(LocalVarRef("a")), Expr::Var(LocalVarRef("b"))],
                      bind: Some(LocalVarRef("sumAB")),
                      ret_to: StepId::new("sub_sum"),
                    },
                  ),
                  (
                    StepId::new("sub_sum"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                      args: vec![Expr::Var(LocalVarRef("sumAB")), Expr::Var(LocalVarRef("c"))],
                      bind: Some(LocalVarRef("subABC")),
                      ret_to: StepId::new("finalize"),
                    },
                  ),
                  (StepId::new("finalize"), Step::Return { value: RetValue::Var(LocalVarRef("subABC")) }),
                ],
              },
            ),
            (
              // binary_search IR
              "binary_search".to_string(),
              Func {
                in_vars: vec![InVar("e", Type::UInt64), InVar("left", Type::UInt64), InVar("right", Type::UInt64)],
                out: Type::Option(Box::new(Type::UInt64)),
                locals: vec![
                  LocalVar("div", Type::UInt64),
                  LocalVar("v_by_index_div", Type::UInt64),
                  LocalVar("fac_call_res", Type::Option(Box::new(Type::UInt64))),
                ],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::If {
                      cond: Expr::Greater(
                        Box::new(Expr::Var(LocalVarRef("left"))),
                        Box::new(Expr::Var(LocalVarRef("right"))),
                      ),
                      then_: StepId::new("return_None"),
                      else_: StepId::new("calculate_div"),
                    },
                  ),
                  (StepId::new("return_None"), Step::Return { value: RetValue::None }),
                  (
                    StepId::new("calculate_div"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("div"), LocalVarRef("v_by_index_div")],
                      code: r#"
                    let o_div = (left + right) / 2;
                    let s = &heap.global;
                    (o_div, s.binarySearchValues[o_div as usize])
                    "#
                      .to_string(),
                      next: StepId::new("return_if_equal"),
                    },
                  ),
                  (
                    StepId::new("return_if_equal"),
                    Step::If {
                      cond: Expr::Equal(
                        Box::new(Expr::Var(LocalVarRef("v_by_index_div"))),
                        Box::new(Expr::Var(LocalVarRef("e"))),
                      ),
                      then_: StepId::new("return_found"),
                      else_: StepId::new("cmp_less"),
                    },
                  ),
                  (
                    StepId::new("return_found"),
                    Step::Return { value: RetValue::Some(Box::new(RetValue::Var(LocalVarRef("div")))) },
                  ),
                  (
                    StepId::new("cmp_less"),
                    Step::If {
                      cond: Expr::Less(
                        Box::new(Expr::Var(LocalVarRef("v_by_index_div"))),
                        Box::new(Expr::Var(LocalVarRef("e"))),
                      ),
                      then_: StepId::new("go_right"),
                      else_: StepId::new("go_left_check_overflow"),
                    },
                  ),
                  (
                    StepId::new("go_right"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "add".to_string() },
                      args: vec![Expr::Var(LocalVarRef("div")), Expr::UInt64(1)],
                      bind: Some(LocalVarRef("left")),
                      ret_to: StepId::new("recursive_call"),
                    },
                  ),
                  (
                    StepId::new("go_left_check_overflow"),
                    Step::If {
                      cond: Expr::Less(Box::new(Expr::Var(LocalVarRef("div"))), Box::new(Expr::UInt64(0))),
                      then_: StepId::new("return_None"),
                      else_: StepId::new("go_left"),
                    },
                  ),
                  (
                    StepId::new("go_left"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                      args: vec![Expr::Var(LocalVarRef("div")), Expr::UInt64(1)],
                      bind: Some(LocalVarRef("right")),
                      ret_to: StepId::new("recursive_call"),
                    },
                  ),
                  (
                    StepId::new("recursive_call"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "binary_search".to_string() },
                      args: vec![
                        Expr::Var(LocalVarRef("e")),
                        Expr::Var(LocalVarRef("left")),
                        Expr::Var(LocalVarRef("right")),
                      ],
                      bind: Some(LocalVarRef("fac_call_res")),
                      ret_to: StepId::new("return_result"),
                    },
                  ),
                  (StepId::new("return_result"), Step::Return { value: RetValue::Var(LocalVarRef("fac_call_res")) }),
                ],
              },
            ),
          ]),
        },
      ),
      (
        FiberType::new("application"),
        Fiber {
          fibers_limit: 2,
          init_vars: vec![],
          heap: HashMap::new(),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals:vec![],
                steps: vec![(StepId::new("entry"), Step::ReturnVoid)],
              },
            ),
            (
              "async_foo".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sum", Type::UInt64)],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::SendToFiber {
                      fiber: "global".to_string(),
                      message: "add".to_string(),
                      args: vec![
                        ("a".to_string(), Expr::Var(LocalVarRef("a"))),
                        ("b".to_string(), Expr::Var(LocalVarRef("b"))),
                      ],
                      next: StepId::new("await"),
                      future_id: FutureLabel::new("async_add_future_1"),
                    },
                  ),
                  (
                    StepId::new("await"),
                    Step::Await(AwaitSpecOld::Future {
                      bind: Some(LocalVarRef("sum")),
                      ret_to: StepId::new("return"),
                      future_id: FutureLabel::new("async_add_future_1"),
                    }),
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("sum")) }),
                ],
              },
            ),
          ]),
        },
      ),
      (
        FiberType::new("order_book"),
        Fiber {
          fibers_limit: 1,
          init_vars: vec![],
          heap: HashMap::from([
            ("bids_prices".to_string(), Type::MaxQueue(Box::new(Type::UInt64))),
            ("asks_prices".to_string(), Type::MinQueue(Box::new(Type::UInt64))),
            (
              "bids_by_price".to_string(),
              Type::Map(Box::new(Type::UInt64), Box::new(Type::Array(Box::new(Type::Custom("Order".to_string()))))),
            ),
            (
              "asks_by_price".to_string(),
              Type::Map(Box::new(Type::UInt64), Box::new(Type::Array(Box::new(Type::Custom("Order".to_string()))))),
            ),
            (
              "orders_index".to_string(),
              Type::Map(Box::new(Type::UInt64), Box::new(Type::Custom("OrderIndex".to_string()))),
            ),
          ]),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals:vec![],
                steps: vec![(StepId::new("entry"), Step::ReturnVoid)],
              },
           ),
           (
             "add_buy".to_string(),
             Func {
                in_vars: vec![InVar("id", Type::UInt64), InVar("price", Type::UInt64), InVar("qty", Type::UInt64)],
                out: Type::Array(Box::new(Type::Custom("Trade".to_string()))),
                locals: vec![LocalVar("result", Type::Array(Box::new(Type::Custom("Trade".to_string()))))],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("result")],
                      code: r#"
let ob = &mut heap.orderBook;
let mut remaining = qty;
let mut trades: Vec<Trade> = Vec::new();

// Clean stale top asks and match while price allows
loop {
  // Find current best ask
  let best_ask = loop {
    if let Some(top) = ob.asksPrices.peek() {
      let p = top.0;
      if let Some(level) = ob.asksByPrice.get(&p) {
        if !level.is_empty() { break Some(p); }
      }
      // stale level
      ob.asksPrices.pop();
      continue;
    } else { break None; }
  };

  match best_ask {
    Some(ap) if ap <= price && remaining > 0 => {
      // Execute against this level FIFO
      if let Some(level) = ob.asksByPrice.get_mut(&ap) {
        while remaining > 0 && !level.is_empty() {
          let maker = &mut level[0];
          if maker.qty <= remaining {
            let trade_qty = maker.qty;
            remaining -= trade_qty;
            trades.push(Trade { price: ap, qty: trade_qty, takerId: id, makerId: maker.id });
            level.remove(0);
          } else {
            maker.qty -= remaining;
            trades.push(Trade { price: ap, qty: remaining, takerId: id, makerId: maker.id });
            remaining = 0;
          }
        }
        if level.is_empty() {
          ob.asksByPrice.remove(&ap);
        }
      }
      // continue loop to next level or exit if remaining==0
    }
    _ => break,
  }
}

// If remaining, add to bids book
if remaining > 0 {
  ob.bidsByPrice.entry(price).or_default().push(Order { id, price, qty: remaining });
  ob.bidsPrices.push(price);
  ob.ordersIndex.insert(id, OrderIndex { side: "buy".to_string(), price });
}

trades
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("result")) }),
                ],
              },
            ),
            (
              "add_sell".to_string(),
              Func {
                in_vars: vec![InVar("id", Type::UInt64), InVar("price", Type::UInt64), InVar("qty", Type::UInt64)],
                out: Type::Array(Box::new(Type::Custom("Trade".to_string()))),
                locals: vec![LocalVar("result", Type::Array(Box::new(Type::Custom("Trade".to_string()))))],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("result")],
                      code: r#"
let ob = &mut heap.orderBook;
let mut remaining = qty;
let mut trades: Vec<Trade> = Vec::new();

// Clean stale top bids and match while price allows
loop {
  // Find current best bid
  let best_bid = loop {
    if let Some(&bp) = ob.bidsPrices.peek() {
      if let Some(level) = ob.bidsByPrice.get(&bp) {
        if !level.is_empty() { break Some(bp); }
      }
      // stale
      ob.bidsPrices.pop();
      continue;
    } else { break None; }
  };

  match best_bid {
    Some(bp) if bp >= price && remaining > 0 => {
      if let Some(level) = ob.bidsByPrice.get_mut(&bp) {
        while remaining > 0 && !level.is_empty() {
          let maker = &mut level[0];
          if maker.qty <= remaining {
            let trade_qty = maker.qty;
            remaining -= trade_qty;
            trades.push(Trade { price: bp, qty: trade_qty, takerId: id, makerId: maker.id });
            level.remove(0);
          } else {
            maker.qty -= remaining;
            trades.push(Trade { price: bp, qty: remaining, takerId: id, makerId: maker.id });
            remaining = 0;
          }
        }
        if level.is_empty() { ob.bidsByPrice.remove(&bp); }
      }
    }
    _ => break,
  }
}

if remaining > 0 {
  ob.asksByPrice.entry(price).or_default().push(Order { id, price, qty: remaining });
  ob.asksPrices.push(std::cmp::Reverse(price));
  ob.ordersIndex.insert(id, OrderIndex { side: "sell".to_string(), price });
}

trades
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("result")) }),
                ],
              },
            ),
            (
              "cancel".to_string(),
              Func {
                in_vars: vec![InVar("id", Type::UInt64)],
                out: Type::UInt64, // 1 if canceled, 0 otherwise
                locals: vec![LocalVar("result", Type::UInt64)],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("result")],
                      code: r#"
let ob = &mut heap.orderBook;
let mut ok = 0u64;
if let Some(idx) = ob.ordersIndex.remove(&id) {
  let price = idx.price;
  if idx.side == "buy" {
    if let Some(level) = ob.bidsByPrice.get_mut(&price) {
      if let Some(pos) = level.iter().position(|o| o.id == id) { level.remove(pos); ok = 1; if level.is_empty() { ob.bidsByPrice.remove(&price); } }
    }
  } else {
    if let Some(level) = ob.asksByPrice.get_mut(&price) {
      if let Some(pos) = level.iter().position(|o| o.id == id) { level.remove(pos); ok = 1; if level.is_empty() { ob.asksByPrice.remove(&price); } }
    }
  }
}
ok
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("result")) }),
                ],
              },
            ),
            (
              "best_bid".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Option(Box::new(Type::UInt64)),
                locals: vec![LocalVar("result", Type::Option(Box::new(Type::UInt64)))],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("result")],
                      code: r#"
let ob = &mut heap.orderBook;
loop {
  if let Some(&bp) = ob.bidsPrices.peek() {
    if let Some(level) = ob.bidsByPrice.get(&bp) { if !level.is_empty() { break Some(bp); } }
    ob.bidsPrices.pop();
  } else { break None; }
}
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("result")) }),
                ],
              },
            ),
            (
              "best_ask".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Option(Box::new(Type::UInt64)),
                locals: vec![LocalVar("result", Type::Option(Box::new(Type::UInt64)))],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("result")],
                      code: r#"
let ob = &mut heap.orderBook;
loop {
  if let Some(top) = ob.asksPrices.peek() {
    let ap = top.0; // Reverse(u64)
    if let Some(level) = ob.asksByPrice.get(&ap) { if !level.is_empty() { break Some(ap); } }
    ob.asksPrices.pop();
  } else { break None; }
}
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("result")) }),
                ],
              },
            ),
            (
              "top_n_depth".to_string(),
              Func {
                in_vars: vec![InVar("n", Type::UInt64)],
                out: Type::Custom("BookSnapshot".to_string()),
                locals: vec![LocalVar("result", Type::Custom("BookSnapshot".to_string()))],
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec![LocalVarRef("result")],
                      code: r#"
let ob = &mut heap.orderBook;

let mut bids_depth: Vec<Level> = Vec::new();
let mut asks_depth: Vec<Level> = Vec::new();

// Bids: highest first
{
  let mut tmp = ob.bidsPrices.clone();
  let mut seen = std::collections::HashSet::<u64>::new();
  while (bids_depth.len() as u64) < n {
    if let Some(bp) = tmp.pop() {
      if seen.contains(&bp) { continue; }
      if let Some(level) = ob.bidsByPrice.get(&bp) {
        if !level.is_empty() {
          let qty = level.iter().map(|o| o.qty).sum::<u64>();
          bids_depth.push(Level { price: bp, qty });
          seen.insert(bp);
        }
      }
    } else { break; }
  }
}

// Asks: lowest first
{
  let mut tmp = ob.asksPrices.clone();
  let mut seen = std::collections::HashSet::<u64>::new();
  while (asks_depth.len() as u64) < n {
    if let Some(std::cmp::Reverse(ap)) = tmp.pop() {
      if seen.contains(&ap) { continue; }
      if let Some(level) = ob.asksByPrice.get(&ap) {
        if !level.is_empty() {
          let qty = level.iter().map(|o| o.qty).sum::<u64>();
          asks_depth.push(Level { price: ap, qty });
          seen.insert(ap);
        }
      }
    } else { break; }
  }
}

BookSnapshot { bids: bids_depth, asks: asks_depth }
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var(LocalVarRef("result")) }),
                ],
              },
            ),
          ]),
        },
      ),
    ]),
    types: vec![
      Type::Struct(
        "Order".to_string(),
        vec![
          StructField { name: "id".to_string(), ty: Type::UInt64 },
          StructField { name: "price".to_string(), ty: Type::UInt64 },
          StructField { name: "qty".to_string(), ty: Type::UInt64 },
        ],
        String::new(),
      ),
      Type::Struct(
        "Trade".to_string(),
        vec![
          StructField { name: "price".to_string(), ty: Type::UInt64 },
          StructField { name: "qty".to_string(), ty: Type::UInt64 },
          StructField { name: "taker_id".to_string(), ty: Type::UInt64 },
          StructField { name: "maker_id".to_string(), ty: Type::UInt64 },
        ],
        String::new(),
      ),
      Type::Struct(
        "OrderIndex".to_string(),
        vec![StructField { name: "side".to_string(), ty: Type::String }, StructField { name: "price".to_string(), ty: Type::UInt64 }],
        String::new(),
      ),
      Type::Struct(
        "Level".to_string(),
        vec![StructField { name: "price".to_string(), ty: Type::UInt64 }, StructField { name: "qty".to_string(), ty: Type::UInt64 }],
        String::new(),
      ),
      Type::Struct(
        "BookSnapshot".to_string(),
        vec![
          StructField { name: "bids".to_string(), ty: Type::Array(Box::new(Type::Custom("Level".to_string()))) },
          StructField { name: "asks".to_string(), ty: Type::Array(Box::new(Type::Custom("Level".to_string()))) },
        ],
        String::new(),
      ),
      Type::Struct(
        "TestIncrementTask".to_string(),
        vec![
          // I make such weird names to make sure that in tests I don't use the same strings and conversion happens correctly
          StructField { name: "in_str_value".to_string(), ty: Type::UInt64 },
          StructField { name: "in_str_resp_future_id".to_string(), ty: Type::String },
          StructField { name: "in_str_resp_queue_name".to_string(), ty: Type::String },
        ],
        String::new(),
      ),
      Type::PubQueueMessage{
        name:"TestCreateQueueMessage".to_string(),
        fields:vec![
          StructField { name: "value".to_string(), ty: Type::UInt64},
          // just returns the same value as was put as an input
          StructField { name: "public_future_id".to_string(), ty: Type::Future(Box::new(Type::UInt64)) },
        ],
        rust_additions:String::new(),
      },
      Type::Struct(
        "TestCalculatorTask".to_string(),
        vec![
          StructField { name: "a".to_string(), ty: Type::UInt64},
          StructField { name: "b".to_string(), ty: Type::UInt64},
          // will return multiplication of a and b
          StructField { name: "response_future_id".to_string(), ty: Type::Future(Box::new(Type::UInt64)) },
        ],
        String::new(),
      ),
  ],
  }
}
