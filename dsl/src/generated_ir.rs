use crate::ir::*;
use std::collections::HashMap;

#[test]
fn generated_ir_valid() {
  let ir = IR {
    types: vec![],
    fibers: HashMap::from([
      (
        FiberType::new("minimalRoot2"),
        Fiber {
          heap: HashMap::new(),
          init_vars: vec![],
          funcs: HashMap::from([(
            "main".to_string(),
            Func {
              in_vars: vec![],
              out: Type::Void,
              locals: vec![],
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::RustBlock {
                    binds: vec![],
                    code: "println!(\"hello\");".to_string(),
                    next: StepId::new("return"),
                  },
                ),
                (StepId::new("return"), Step::ReturnVoid),
              ],
            },
          )]),
        },
      ),
      (
        FiberType::new("minimalRoot"),
        Fiber {
          heap: HashMap::new(),
          init_vars: vec![],
          funcs: HashMap::from([(
            "main".to_string(),
            Func {
              in_vars: vec![],
              out: Type::Void,
              locals: vec![
                LocalVar("queue", Type::String),
                LocalVar("future_1", Type::Future(Box::new(Type::Void))),
                LocalVar("future_2", Type::Future(Box::new(Type::Void))),
                LocalVar("err_1", Type::Option(Box::new(Type::String))),
                LocalVar("err_2", Type::Option(Box::new(Type::String))),
                LocalVar("err_3", Type::Option(Box::new(Type::String))),
                LocalVar("__mrn_qname_0", Type::String),
              ],
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::RustBlock {
                    binds: vec![LocalVarRef("__mrn_qname_0")],
                    code: "println!(\"hello\");\"rootQueue\".to_string()".to_string(),
                    next: StepId::new("create_primitives"),
                  },
                ),
                (
                  StepId::new("create_primitives"),
                  Step::Create {
                    primitives: vec![
                      RuntimePrimitive::Queue { name: LocalVarRef("__mrn_qname_0"), public: false },
                      RuntimePrimitive::Future,
                      RuntimePrimitive::Future,
                    ],
                    success: SuccessCreateBranch {
                      next: StepId::new("after_create_success"),
                      id_binds: vec![LocalVarRef("queue"), LocalVarRef("future_1"), LocalVarRef("future_2")],
                    },
                    fail: FailCreateBranch {
                      next: StepId::new("after_create_fail"),
                      error_binds: vec![LocalVarRef("err_1"), LocalVarRef("err_2"), LocalVarRef("err_3")],
                    },
                  },
                ),
                (
                  StepId::new("after_create_success"),
                  Step::RustBlock {
                    binds: vec![],
                    code: "println!(\"created queues\");".to_string(),
                    next: StepId::new("after_match"),
                  },
                ),
                (
                  StepId::new("after_create_fail"),
                  Step::RustBlock {
                    binds: vec![],
                    code: "println!(\"{:?} {:?} {:?}\",err_1,err_2,err_3);".to_string(),
                    next: StepId::new("after_match"),
                  },
                ),
                (
                  StepId::new("after_match"),
                  Step::RustBlock {
                    binds: vec![],
                    code: "println!(\"return\");".to_string(),
                    next: StepId::new("return"),
                  },
                ),
                (StepId::new("return"), Step::ReturnVoid),
              ],
            },
          )]),
        },
      ),
    ]),
  };
  assert_eq!((true, "".to_string()), ir.is_valid());
}
