// NOTEs
// in IR - highlevel struct that has different fibers and these fibers have their own typed stacks
// queues between fibers makes sense to keep in IR?
//
// cancelable future

//   type IR {
//     fibers: {
//       key: {
//         is_singleton/daemon: bool
//         heap {}
//         funcs {
//           name: {
//             steps[
//               {

//                 local_vars_for_this_step // on stack
//                 local_vars_out_for_next_step // also on stack
//                 steps[...] // calls of other functions?

//               }
//             ]
//           }
//           in_params_type
//           ret_value_type
//         }
//       }
//     }
//     types {
//       // all types? Or only in/out function types
//     }
//   }

use crate::ir::*;
use crate::codegen::generate_rust_types;
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;

#[test]
fn test_ir() {
  /*
     userManager{
      map[string]User
      get
      set
      delete
     }

     socialScore {
      increment
      decrement
     }

     global {
      add
      sub
     }

  */

  let ir = IR {
    fibers: HashMap::from([
      (
        "global".to_string(),
        Fiber {
          // exact implementation(steps) of primitive(add/sub/div/mult/rand/...) functions in global are not provided
          // only semantics
          // implementation is "provided" by runtime(maybe not the best term?)
          // maybe global is not the best name?
          heap: HashMap::new(),
          in_messages: vec![MessageSpec { name: "Timeout".to_string(), fields: vec![("sec".to_string(), Type::Int)] }],
          funcs: HashMap::from([
            (
              "add".to_string(),
              Func {
                in_vars: vec![
                  InVar { name: "a".to_string(), type_: Type::Int },
                  InVar { name: "b".to_string(), type_: Type::Int },
                ],
                out: Type::Int,
                locals: vec![],
                entry: StepId::new("entry"),
                steps: vec![],
              },
            ),
            (
              "sub".to_string(),
              Func {
                in_vars: vec![
                  InVar { name: "a".to_string(), type_: Type::Int },
                  InVar { name: "b".to_string(), type_: Type::Int },
                ],
                out: Type::Int,
                locals: vec![],
                entry: StepId::new("entry"),
                steps: vec![],
              },
            ),
            (
              "mult".to_string(),
              Func {
                in_vars: vec![
                  InVar { name: "a".to_string(), type_: Type::Int },
                  InVar { name: "b".to_string(), type_: Type::Int },
                ],
                out: Type::Int,
                locals: vec![],
                entry: StepId::new("entry"),
                steps: vec![],
              },
            ),
            (
              "randGen".to_string(),
              Func { in_vars: vec![], out: Type::Int, locals: vec![], entry: StepId::new("entry"), steps: vec![] },
            ),
            (
              // factorial(n) {
              //   if n == 1 { return 1 }
              //   return n * factorial(n - 1)
              // }
              "factorial".to_string(),
              Func {
                in_vars: vec![InVar { name: "n".to_string(), type_: Type::Int }],
                out: Type::Int,
                locals: vec![
                  LocalVar { name: "fac_call_res".to_string(), type_: Type::Int },
                  LocalVar { name: "subtract_res".to_string(), type_: Type::Int },
                  LocalVar { name: "result".to_string(), type_: Type::Int },
                ],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::If {
                      cond: Expr::Equal(Box::new(Expr::Var("n".to_string())), Box::new(Expr::Int(1))),
                      then_: StepId::new("return_1"),
                      else_: StepId::new("subtract"),
                    },
                  ),
                  (StepId::new("return_1"), Step::Return { value: Some(Expr::Int(1)) }),
                  (
                    StepId::new("subtract"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                      args: vec![Expr::Var("n".to_string()), Expr::Int(1)],
                      bind: Some("subtract_res".to_string()),
                      ret_to: StepId::new("factorial_call"),
                    },
                  ),
                  (
                    StepId::new("factorial_call"),
                    Step::Call {
                      // or instead of global.factorial use self.factorial?
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
                  (StepId::new("return"), Step::Return { value: Some(Expr::Var("result".to_string())) }),
                ],
              },
            ),
          ]),
        },
      ),
      (
        "userManager".to_string(),
        Fiber {
          heap: HashMap::from([(
            "users".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Custom("User".to_string()))),
          )]),
          in_messages: vec![MessageSpec {
            name: "GetUser".to_string(),
            fields: vec![("key".to_string(), Type::String)],
          }],
          funcs: HashMap::from([
            (
              "get".to_string(),
              Func {
                in_vars: vec![InVar { name: "key".to_string(), type_: Type::String }],
                out: Type::Option(Box::new(Type::Custom("User".to_string()))),
                locals: vec![],
                entry: StepId::new("entry"),
                steps: vec![],
              },
            ),
            (
              "set".to_string(),
              Func {
                in_vars: vec![
                  InVar { name: "key".to_string(), type_: Type::String },
                  InVar { name: "item".to_string(), type_: Type::Custom("User".to_string()) },
                ],
                out: Type::Void,
                locals: vec![],
                entry: StepId::new("entry"),
                steps: vec![],
              },
            ),
          ]),
        },
      ),
      (
        "socialScore".to_string(),
        Fiber {
          heap: HashMap::new(),
          in_messages: vec![],
          funcs: HashMap::from([("increment".to_string(), {
            // socialScore.increment(userId):
            //   send global.timeout(10)
            //   send userManager.GetUser(userId)
            //   select
            //     user_opt = await GetUserResponse
            //     await Timeout ->
            //   if user_opt is Some:
            //     user = unwrap(user_opt)
            //     new_rating = global.add(user.rating, 1)
            //     updated_user = user{ rating: new_rating }
            //     userManager.set(userId, updated_user)
            //   return
            Func {
              in_vars: vec![InVar { name: "userId".to_string(), type_: Type::String }],
              out: Type::Void,
              locals: vec![
                LocalVar {
                  name: "user_opt".to_string(),
                  type_: Type::Option(Box::new(Type::Custom("User".to_string()))),
                },
                LocalVar { name: "user".to_string(), type_: Type::Custom("User".to_string()) },
                LocalVar { name: "new_rating".to_string(), type_: Type::Int },
                LocalVar { name: "updated_user".to_string(), type_: Type::Custom("User".to_string()) },
              ],
              entry: StepId::new("entry"),
              steps: vec![
                (
                  StepId::new("entry"),
                  Step::SendToFiber {
                    fiber: "userManager".to_string(),
                    message: "GetUser".to_string(),
                    args: vec![("key".to_string(), Expr::Var("userId".to_string()))],
                    next: StepId::new("timeout_start"),
                    future_id: "can_put_anything_unique_here_needed_only_for_awaiting_identification".to_string(),
                  },
                ),
                (
                  StepId::new("timeout_start"),
                  Step::SendToFiber {
                    fiber: "global".to_string(),
                    message: "Timeout".to_string(),
                    args: vec![("sec".to_string(), Expr::Int(10))],
                    next: StepId::new("select"),
                    future_id: "timeout_await_unique_id".to_string(),
                  },
                ),
                (
                  StepId::new("select"),
                  Step::Select {
                    arms: vec![
                      AwaitSpec {
                        bind: Some("user_opt".to_string()),
                        ret_to: StepId::new("check_user"),
                        future_id: "can_put_anything_unique_here_needed_only_for_awaiting_identification".to_string(),
                      },
                      AwaitSpec { bind: None, ret_to: StepId::new("done"), future_id: "timeout_await_unique_id".to_string() },
                    ],
                  },
                ),
                (
                  StepId::new("check_user"),
                  Step::If {
                    cond: Expr::IsSome(Box::new(Expr::Var("user_opt".to_string()))),
                    then_: StepId::new("have_user"),
                    else_: StepId::new("done"),
                  },
                ),
                (
                  StepId::new("have_user"),
                  Step::Let {
                    local: "user".to_string(),
                    expr: Expr::Unwrap(Box::new(Expr::Var("user_opt".to_string()))),
                    next: StepId::new("call_add"),
                  },
                ),
                (
                  StepId::new("call_add"),
                  Step::Call {
                    target: FuncRef { fiber: "global".to_string(), func: "add".to_string() },
                    args: vec![
                      Expr::GetField(Box::new(Expr::Var("user".to_string())), "rating".to_string()),
                      Expr::Int(1),
                    ],
                    bind: Some("new_rating".to_string()),
                    ret_to: StepId::new("update_user"),
                  },
                ),
                (
                  StepId::new("update_user"),
                  Step::Let {
                    local: "updated_user".to_string(),
                    expr: Expr::StructUpdate {
                      base: Box::new(Expr::Var("user".to_string())),
                      updates: vec![("rating".to_string(), Expr::Var("new_rating".to_string()))],
                    },
                    next: StepId::new("set_user"),
                  },
                ),
                (
                  StepId::new("set_user"),
                  Step::Call {
                    target: FuncRef { fiber: "userManager".to_string(), func: "set".to_string() },
                    args: vec![Expr::Var("userId".to_string()), Expr::Var("updated_user".to_string())],
                    bind: None,
                    ret_to: StepId::new("done"),
                  },
                ),
                (StepId::new("done"), Step::Return { value: None }),
              ],
            }
          })]),
        },
      ),
    ]),
    types: vec![Type::Struct(
      "User".to_string(),
      vec![
        StructField { name: "id".to_string(), ty: Type::String },
        StructField { name: "age".to_string(), ty: Type::Int },
        StructField { name: "email".to_string(), ty: Type::String },
        StructField { name: "rating".to_string(), ty: Type::Int },
      ],
    )],
  };

  // Generate Rust code from IR and write it into state/src/generated_types.rs
  let code = generate_rust_types(&ir);
  let mut out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  out_path.pop(); // move from dsl/ to workspace root
  out_path.push("state/src/generated_types.rs");
  if let Some(parent) = out_path.parent() { fs::create_dir_all(parent).unwrap(); }
  fs::write(&out_path, code).expect("write generated types");
}
