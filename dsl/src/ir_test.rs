use std::collections::HashMap;

use crate::ir::{Fiber, FiberType, Func, IR, InVar, LocalVar, LocalVarRef, SetPrimitive, Step, StepId, Type};

#[test]
fn is_valid_no_root_fiber() {
  let (valid, explanation) = IR { fibers: HashMap::new(), types: vec![] }.is_valid();
  assert!(!valid);
  assert_eq!("no 'root' fiber\n".to_string(), explanation);
}

#[test]
fn is_valid_variables() {
  let (valid, explanation) = IR {
    fibers: HashMap::from([(
      FiberType::new("root"),
      Fiber {
        fibers_limit: 0,
        heap: HashMap::new(),
        in_messages: vec![],
        funcs: HashMap::from([(
          "f1".to_string(),
          Func {
            in_vars: vec![InVar("a", Type::UInt64)],
            out: Type::UInt64,
            locals: vec![LocalVar("a", Type::UInt64)],
            steps: vec![(
              StepId::new("entry"),
              Step::SetValues {
                values: vec![SetPrimitive::QueueMessage {
                  f_var_queue_name: LocalVarRef("queueName"),
                  var_name: LocalVarRef("a"),
                }],
                next: StepId::new("next"),
              },
            )],
          },
        )]),
      },
    )]),
    types: vec![],
  }
  .is_valid();
  assert!(!valid);
  assert_eq!(
    r#"duplicate var name a for types: UInt64 and UInt64
StepId("entry") references queueName that is not defined
root doesnt have 'main' function
"#
    .to_string(),
    explanation
  );
}
