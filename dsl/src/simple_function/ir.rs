use crate::ir::*;
use std::collections::HashMap;

// Shared IR specification used by build.rs (via include!) and by tests.
pub fn sample_ir() -> IR {
  IR {
    fibers: HashMap::from([(
      "global".to_string(),
      Fiber {
        heap: HashMap::from([("binary_search_values".to_string(), Type::Array(Box::new(Type::UInt64)))]),
        in_messages: vec![],
        funcs: HashMap::from([
          (
            "add".to_string(),
            Func {
              in_vars: vec![
                InVar { name: "a".to_string(), type_: Type::UInt64 },
                InVar { name: "b".to_string(), type_: Type::UInt64 },
              ],
              out: Type::UInt64,
              locals: vec![LocalVar { name: "sum".to_string(), type_: Type::UInt64 }],
              entry: StepId::new("entry"),
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::RustBlock {
                    binds: vec!["sum".to_string()],
                    code: "a+b".to_string(),
                    next: StepId::new("return"),
                  },
                ),
                (StepId::new("return"), Step::Return { value: RetValue::Var("sum".to_string()) }),
              ],
            },
          ),
          (
            "sub".to_string(),
            Func {
              in_vars: vec![
                InVar { name: "a".to_string(), type_: Type::UInt64 },
                InVar { name: "b".to_string(), type_: Type::UInt64 },
              ],
              out: Type::UInt64,
              locals: vec![LocalVar { name: "sub".to_string(), type_: Type::UInt64 }],
              entry: StepId::new("entry"),
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::RustBlock {
                    binds: vec!["sub".to_string()],
                    code: r#"
let out = a - b;
out
"#
                    .to_string(),
                    next: StepId::new("return"),
                  },
                ),
                (StepId::new("return"), Step::Return { value: RetValue::Var("sub".to_string()) }),
              ],
            },
          ),
          (
            "mult".to_string(),
            Func {
              in_vars: vec![
                InVar { name: "a".to_string(), type_: Type::UInt64 },
                InVar { name: "b".to_string(), type_: Type::UInt64 },
              ],
              out: Type::UInt64,
              locals: vec![],
              entry: StepId::new("entry"),
              steps: vec![],
            },
          ),
          (
            "div".to_string(),
            Func {
              in_vars: vec![
                InVar { name: "a".to_string(), type_: Type::UInt64 },
                InVar { name: "b".to_string(), type_: Type::UInt64 },
              ],
              out: Type::UInt64,
              locals: vec![],
              entry: StepId::new("entry"),
              steps: vec![],
            },
          ),
          (
            // factorial(n) { if n == 1 { return 1 } return n * factorial(n - 1) }
            "factorial".to_string(),
            Func {
              in_vars: vec![InVar { name: "n".to_string(), type_: Type::UInt64 }],
              out: Type::UInt64,
              locals: vec![
                LocalVar { name: "fac_call_res".to_string(), type_: Type::UInt64 },
                LocalVar { name: "subtract_res".to_string(), type_: Type::UInt64 },
                LocalVar { name: "result".to_string(), type_: Type::UInt64 },
              ],
              entry: StepId::new("entry"),
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::If {
                    cond: Expr::Equal(Box::new(Expr::Var("n".to_string())), Box::new(Expr::UInt64(1))),
                    then_: StepId::new("return_1"),
                    else_: StepId::new("subtract"),
                  },
                ),
                (StepId::new("return_1"), Step::Return { value: RetValue::UInt64(1) }),
                (
                  StepId::new("subtract"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                    args: vec![Expr::Var("n".to_string()), Expr::UInt64(1)],
                    bind: Some("subtract_res".to_string()),
                    ret_to: StepId::new("factorial_call"),
                  },
                ),
                (
                  StepId::new("factorial_call"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "factorial".to_string() },
                    args: vec![Expr::Var("subtract_res".to_string())],
                    bind: Some("fac_call_res".to_string()),
                    ret_to: StepId::new("multiply"),
                  },
                ),
                (
                  StepId::new("multiply"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "mult".to_string() },
                    args: vec![Expr::Var("n".to_string()), Expr::Var("fac_call_res".to_string())],
                    bind: Some("result".to_string()),
                    ret_to: StepId::new("return"),
                  },
                ),
                (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
              ],
            },
          ),
          (
            "subAdd".to_string(),
            Func {
              in_vars: vec![
                InVar { name: "a".to_string(), type_: Type::UInt64 },
                InVar { name: "b".to_string(), type_: Type::UInt64 },
                InVar { name: "c".to_string(), type_: Type::UInt64 },
              ],
              out: Type::UInt64,
              locals: vec![
                LocalVar { name: "sumAB".to_string(), type_: Type::UInt64 },
                LocalVar { name: "subABC".to_string(), type_: Type::UInt64 },
              ],
              entry: StepId::new("entry"),
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "add".to_string() },
                    args: vec![Expr::Var("a".to_string()), Expr::Var("b".to_string())],
                    bind: Some("sumAB".to_string()),
                    ret_to: StepId::new("sub_sum"),
                  },
                ),
                (
                  StepId::new("sub_sum"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                    args: vec![Expr::Var("sumAB".to_string()), Expr::Var("c".to_string())],
                    bind: Some("subABC".to_string()),
                    ret_to: StepId::new("finalize"),
                  },
                ),
                (StepId::new("finalize"), Step::Return { value: RetValue::Var("subABC".to_string()) }),
              ],
            },
          ),
          (
            // binary_search IR
            "binary_search".to_string(),
            Func {
              in_vars: vec![
                InVar { name: "e".to_string(), type_: Type::UInt64 },
                InVar { name: "left".to_string(), type_: Type::UInt64 },
                InVar { name: "right".to_string(), type_: Type::UInt64 },
              ],
              out: Type::Option(Box::new(Type::UInt64)),
              locals: vec![
                LocalVar { name: "div".to_string(), type_: Type::UInt64 },
                LocalVar { name: "v_by_index_div".to_string(), type_: Type::UInt64 },
                LocalVar { name: "fac_call_res".to_string(), type_: Type::Option(Box::new(Type::UInt64)) },
              ],
              entry: StepId::new("entry"),
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::If {
                    cond: Expr::Greater(
                      Box::new(Expr::Var("left".to_string())),
                      Box::new(Expr::Var("right".to_string())),
                    ),
                    then_: StepId::new("return_None"),
                    else_: StepId::new("calculate_div"),
                  },
                ),
                (StepId::new("return_None"), Step::Return { value: RetValue::None }),
                (
                  StepId::new("calculate_div"),
                  Step::RustBlock {
                    binds: vec!["div".to_string(), "v_by_index_div".to_string()],
                    code: r#"
                    let o_div = (left + right) / 2;
                    let Heap::Global(s) = heap;
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
                      Box::new(Expr::Var("v_by_index_div".to_string())),
                      Box::new(Expr::Var("e".to_string())),
                    ),
                    then_: StepId::new("return_found"),
                    else_: StepId::new("cmp_less"),
                  },
                ),
                (
                  StepId::new("return_found"),
                  Step::Return { value: RetValue::Some(Box::new(RetValue::Var("div".to_string()))) },
                ),
                (
                  StepId::new("cmp_less"),
                  Step::If {
                    cond: Expr::Less(
                      Box::new(Expr::Var("v_by_index_div".to_string())),
                      Box::new(Expr::Var("e".to_string())),
                    ),
                    then_: StepId::new("go_right"),
                    else_: StepId::new("go_left_check_overflow"),
                  },
                ),
                (
                  StepId::new("go_right"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "add".to_string() },
                    args: vec![Expr::Var("div".to_string()), Expr::UInt64(1)],
                    bind: Some("left".to_string()),
                    ret_to: StepId::new("recursive_call"),
                  },
                ),
                (
                  StepId::new("go_left_check_overflow"),
                  Step::If {
                    cond: Expr::Less(Box::new(Expr::Var("div".to_string())), Box::new(Expr::UInt64(0))),
                    then_: StepId::new("return_None"),
                    else_: StepId::new("go_left"),
                  },
                ),
                (
                  StepId::new("go_left"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                    args: vec![Expr::Var("div".to_string()), Expr::UInt64(1)],
                    bind: Some("right".to_string()),
                    ret_to: StepId::new("recursive_call"),
                  },
                ),
                (
                  StepId::new("recursive_call"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "binary_search".to_string() },
                    args: vec![
                      Expr::Var("e".to_string()),
                      Expr::Var("left".to_string()),
                      Expr::Var("right".to_string()),
                    ],
                    bind: Some("fac_call_res".to_string()),
                    ret_to: StepId::new("return_result"),
                  },
                ),
                (StepId::new("return_result"), Step::Return { value: RetValue::Var("fac_call_res".to_string()) }),
              ],
            },
          ),
        ]),
      },
    )]),
    types: vec![],
  }
}
