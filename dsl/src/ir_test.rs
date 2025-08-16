use crate::{
  ast::{Program, TypeName},
  parser,
};

#[test]
fn test_parse_program() {
  let input = r#"
        struct User {
          id: String,
          email: String,
          age: i32,
        }
        var users: map[String]User = {}
        fn findUserById(id: String) -> Option<User> {
          return users.get(id)
        }
        fn createNewUser(email: String, age: i32) {
          let id: String = sync generateId()
          users.set(id, User { id: id, email: email, age: age })
        }
    "#;

  let program = parser::parse_program(input).expect("should be ok");
  //   let ir = program_to_ir(&program);

  //   assert_eq!(
  //     ir,
  //     IR {
  //       states: vec![
  //         StateSpec {
  //           name: "UsersStorageIdle".to_string(),
  //           kind: StateKind::StorageIdle { storage: "users".to_string() },
  //           in_vars: vec![],
  //           out_vars: vec![],
  //         },
  //         StateSpec {
  //           name: "UsersStorageGetItemRequest".to_string(),
  //           kind: StateKind::StorageRequest { op: StorageOp::Get, storage: "users".to_string() },
  //           in_vars: vec![Var { name: "id".to_string(), ty: TypeName::StringTy }], // in my other experiments there were two variables id and maroon_fiber for response, but that variable will be provided by runtime and shouldn't be reflected here
  //           out_vars: vec![Var { name: "user".to_string(), ty: TypeName::Custom("Option<User>".to_string()) }],
  //         },
  //         StateSpec {
  //           name: "UsersStorageCreateItemRequest".to_string(),
  //           kind: StateKind::StorageRequest { op: StorageOp::Create, storage: "users".to_string() },
  //           in_vars: vec![Var { name: "user".to_string(), ty: TypeName::Custom("User".to_string()) }],
  //           out_vars: vec![],
  //         },
  //         StateSpec {
  //           name: "FindUserByIdEntry".to_string(),
  //           kind: StateKind::Entry,
  //           in_vars: vec![Var { name: "id".to_string(), ty: TypeName::StringTy }],
  //           out_vars: vec![],
  //         },
  //         StateSpec {
  //           name: "FindUserByIdDone".to_string(),
  //           kind: StateKind::Done,
  //           in_vars: vec![Var { name: "id".to_string(), ty: TypeName::StringTy }],
  //           out_vars: vec![],
  //         },
  //       ],
  //       values: vec![],
  //       storages: vec![StorageSpec {
  //         name: "users".to_string(),
  //         ty: TypeName::Map(Box::new(TypeName::StringTy), Box::new(TypeName::Custom("User".to_string())))
  //       }]
  //     }
  //   );
}

#[derive(Debug, Clone, PartialEq)]
struct StateSpec {
  name: String,
  kind: StateKind,
  in_vars: Vec<Var>,
  out_vars: Vec<Var>,
}

#[derive(Debug, Clone, PartialEq)]
enum StorageOp {
  Get,
  Create,
  // TODO: delete/update
  // TODO: extend to array/structs. What about primitive variables?
}

#[derive(Debug, Clone, PartialEq)]
enum StateKind {
  StorageIdle { storage: String },
  StorageRequest { op: StorageOp, storage: String },
  StorageGot { op: StorageOp, storage: String },
  Entry,
  RecursiveCall,
  Call { callee: String },
  Done,
}

#[derive(Debug, Clone, PartialEq)]

struct Var {
  name: String,
  ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
struct VarSpec {}

#[derive(Debug, Clone, PartialEq)]
struct StorageSpec {
  name: String,
  ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
struct IR {
  states: Vec<StateSpec>,
  values: Vec<VarSpec>,
  storages: Vec<StorageSpec>,
}

fn program_to_ir(program: &Program) -> IR {
  IR { states: vec![], values: vec![], storages: vec![] }
}
