use crate::{
  ast::{Expr, Function, Item, Mutability, Program, Statement, TypeName},
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
  let ir = program_to_ir(&program);

  assert_eq!(
    ir,
    IR {
      states: vec![
        StateSpec {
          name: "UsersStorageIdle".to_string(),
          kind: StateKind::StorageIdle { storage: "users".to_string() },
          in_vars: vec![],
          out_vars: vec![],
        },
        StateSpec {
          name: "UsersStorageGetItemRequest".to_string(),
          kind: StateKind::StorageRequest { op: StorageOp::Get, storage: "users".to_string() },
          // in my other experiments there were two variables id and maroon_fiber for response, but that variable will be provided by runtime and shouldn't be reflected here
          // or should be?
          in_vars: vec![Var::Input("key".to_string(), TypeName::StringTy)],
          out_vars: vec![Var::Output(TypeName::Option(Box::new(TypeName::Custom("User".to_string()))))],
        },
        StateSpec {
          name: "UsersStorageCreateItemRequest".to_string(),
          kind: StateKind::StorageRequest { op: StorageOp::Create, storage: "users".to_string() },
          in_vars: vec![
            Var::Input("item".to_string(), TypeName::Custom("User".to_string())),
            Var::Input("key".to_string(), TypeName::StringTy)
          ],
          out_vars: vec![],
        },
        StateSpec {
          name: "FindUserByIdEntry".to_string(),
          kind: StateKind::Entry,
          in_vars: vec![Var::Input("id".to_string(), TypeName::StringTy)],
          out_vars: vec![],
        },
        StateSpec {
          name: "FindUserByIdUsersGetRequest".to_string(),
          kind: StateKind::StorageRequest { op: StorageOp::Get, storage: "users".to_string() },
          // on the previous step var name was id(because it was input for findUserById function, now it's key because it's an input for storage fiber get state)
          // don't know yet how magically it will happen but we know that get operation in storage fiber accepts `key` as an input, so it should be renamed by runtime
          in_vars: vec![Var::Input("key".to_string(), TypeName::StringTy)],
          out_vars: vec![],
        },
        StateSpec {
          name: "FindUserByIdUsersGetGot".to_string(),
          kind: StateKind::StorageGot { op: StorageOp::Get, storage: "users".to_string() },
          in_vars: vec![],
          out_vars: vec![Var::Output(TypeName::Option(Box::new(TypeName::Custom("User".to_string()))))],
        },
        StateSpec {
          name: "FindUserByIdDone".to_string(),
          kind: StateKind::Done,
          in_vars: vec![],
          out_vars: vec![Var::Output(TypeName::Option(Box::new(TypeName::Custom("User".to_string()))))],
        },
        StateSpec {
          name: "CreateNewUserEntry".to_string(),
          kind: StateKind::Entry,
          in_vars: vec![
            Var::Input("email".to_string(), TypeName::StringTy),
            Var::Input("age".to_string(), TypeName::I32),
          ],
          out_vars: vec![],
        },
        StateSpec {
          // stateName == funcName+stotrage/fiberName+functionName
          //              CreateNewUser+Users+CreateRequest
          name: "CreateNewUserUsersCreateRequest".to_string(),
          kind: StateKind::StorageRequest { op: StorageOp::Create, storage: "users".to_string() },
          in_vars: vec![
            Var::Input("key".to_string(), TypeName::StringTy),
            Var::Input("item".to_string(), TypeName::Custom("User".to_string()))
          ],
          out_vars: vec![],
        },
        StateSpec {
          name: "CreateNewUserUsersCreateGot".to_string(),
          kind: StateKind::StorageGot { op: StorageOp::Create, storage: "users".to_string() },
          in_vars: vec![],
          out_vars: vec![],
        },
        StateSpec {
          name: "CreateNewUserUsersDone".to_string(),
          kind: StateKind::Done,
          in_vars: vec![],
          out_vars: vec![],
        },
      ],
      storages: vec![StorageSpec {
        name: "users".to_string(),
        ty: TypeName::Map(Box::new(TypeName::StringTy), Box::new(TypeName::Custom("User".to_string())))
      }]
    }
  );
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

enum Var {
  // name, typeName
  Input(String, TypeName),
  // it's an output, just type is needed
  Output(TypeName),
}

#[derive(Debug, Clone, PartialEq)]
struct StorageSpec {
  name: String,
  ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
struct IR {
  states: Vec<StateSpec>,
  storages: Vec<StorageSpec>,
}

fn program_to_ir(program: &Program) -> IR {
  let mut states = Vec::new();
  let mut storages = Vec::new();

  // First pass: collect all mutable variables (storages)
  for item in &program.items {
    if let Item::Statement(Statement::VarDecl(var_decl)) = item {
      if matches!(var_decl.mutability, Mutability::Mutable) {
        storages.push(StorageSpec { name: var_decl.name.clone(), ty: var_decl.ty.clone() });

        states.push(StateSpec {
          name: format!("{}StorageIdle", capitalize(&var_decl.name)),
          kind: StateKind::StorageIdle { storage: var_decl.name.clone() },
          in_vars: vec![],
          out_vars: vec![],
        });

        // Create storage request states for common operations
        // Right now it's known only for map type, TODO: add the same for not-map storages
        // Get operation
        states.push(StateSpec {
          name: format!("{}StorageGetItemRequest", capitalize(&var_decl.name)),
          kind: StateKind::StorageRequest { op: StorageOp::Get, storage: var_decl.name.clone() },
          in_vars: vec![Var::Input("key".to_string(), get_map_key_type(&var_decl.ty))],
          out_vars: vec![Var::Output(TypeName::Option(Box::new(get_map_value_type(&var_decl.ty))))],
        });

        // Create operation (set)
        states.push(StateSpec {
          name: format!("{}StorageCreateItemRequest", capitalize(&var_decl.name)),
          kind: StateKind::StorageRequest { op: StorageOp::Create, storage: var_decl.name.clone() },
          in_vars: vec![
            Var::Input("item".to_string(), get_map_value_type(&var_decl.ty)),
            Var::Input("key".to_string(), get_map_key_type(&var_decl.ty)),
          ],
          out_vars: vec![],
        });
      }
    }
  }

  // Second pass: process functions
  for item in &program.items {
    if let Item::Function(function) = item {
      generate_function_states(function, &mut states);
    }
  }

  IR { states, storages }
}

fn capitalize(s: &str) -> String {
  let mut chars = s.chars();
  match chars.next() {
    None => String::new(),
    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
  }
}

fn get_map_key_type(ty: &TypeName) -> TypeName {
  match ty {
    TypeName::Map(key_ty, _) => (**key_ty).clone(),
    _ => panic!("no way we don't know key type"),
  }
}

fn get_map_value_type(ty: &TypeName) -> TypeName {
  match ty {
    TypeName::Map(_, value_ty) => (**value_ty).clone(),
    _ => panic!("no way we don't know value type"),
  }
}

fn generate_function_states(
  function: &Function,
  states: &mut Vec<StateSpec>,
) {
  let func_name = &function.name;

  states.push(StateSpec {
    name: format!("{}Entry", capitalize(func_name)),
    kind: StateKind::Entry,
    in_vars: function.params.iter().map(|p| Var::Input(p.name.clone(), p.ty.clone())).collect(),
    out_vars: vec![],
  });

  // Find storage names used in this function
  let mut storage_names: Vec<String> = Vec::new();
  collect_storage_names_from_statements(&function.body.statements, &mut storage_names);

  // Process function body to generate states for operations
  for statement in &function.body.statements {
    match statement {
      Statement::Return(expr) => {
        generate_expression_states(func_name, expr, states);

        let out_vars =
          if matches!(function.ret, TypeName::Void) { vec![] } else { vec![Var::Output(function.ret.clone())] };

        states.push(StateSpec {
          name: format!("{}Done", capitalize(func_name)),
          kind: StateKind::Done,
          in_vars: vec![],
          out_vars,
        });
      }
      Statement::VarDecl(var_decl) => {
        if let Some(init_expr) = &var_decl.init {
          generate_expression_states(func_name, init_expr, states);
        }
      }
      Statement::Expr(expr) => {
        generate_expression_states(func_name, expr, states);

        // For functions that don't return anything, create done state after processing expressions
        if matches!(function.ret, TypeName::Void) {
          // Use the first storage name found, or default to generic "Done"
          let done_name = if let Some(storage_name) = storage_names.first() {
            format!("{}{}Done", capitalize(func_name), capitalize(storage_name))
          } else {
            format!("{}Done", capitalize(func_name))
          };

          states.push(StateSpec { name: done_name, kind: StateKind::Done, in_vars: vec![], out_vars: vec![] });
        }
      }
      _ => {}
    }
  }
}

fn collect_storage_names_from_statements(
  statements: &[Statement],
  storage_names: &mut Vec<String>,
) {
  for statement in statements {
    match statement {
      Statement::Return(expr) => collect_storage_names_from_expr(expr, storage_names),
      Statement::VarDecl(var_decl) => {
        if let Some(init_expr) = &var_decl.init {
          collect_storage_names_from_expr(init_expr, storage_names);
        }
      }
      Statement::Expr(expr) => collect_storage_names_from_expr(expr, storage_names),
      _ => {}
    }
  }
}

fn collect_storage_names_from_expr(
  expr: &Expr,
  storage_names: &mut Vec<String>,
) {
  match expr {
    Expr::MethodCall { receiver, name: _, args: _ } => {
      if let Expr::Ident(storage_name) = receiver.as_ref() {
        if !storage_names.contains(storage_name) {
          storage_names.push(storage_name.clone());
        }
      }
    }
    _ => {}
  }
}

fn generate_expression_states(
  func_name: &str,
  expr: &Expr,
  states: &mut Vec<StateSpec>,
) {
  match expr {
    Expr::MethodCall { receiver, name: method_name, args: _ } => {
      if let Expr::Ident(storage_name) = receiver.as_ref() {
        // TODO: Again, it works only for map storage and we assume that get/set functions are `defined`
        // need to implement the same for other types
        match method_name.as_str() {
          "get" => {
            // Generate get request state
            states.push(StateSpec {
              name: format!("{}{}GetRequest", capitalize(func_name), capitalize(storage_name)),
              kind: StateKind::StorageRequest { op: StorageOp::Get, storage: storage_name.clone() },
              in_vars: vec![Var::Input("key".to_string(), TypeName::StringTy)],
              out_vars: vec![],
            });

            // Generate get response state
            states.push(StateSpec {
              name: format!("{}{}GetGot", capitalize(func_name), capitalize(storage_name)),
              kind: StateKind::StorageGot { op: StorageOp::Get, storage: storage_name.clone() },
              in_vars: vec![],
              out_vars: vec![Var::Output(TypeName::Option(Box::new(TypeName::Custom("User".to_string()))))],
            });
          }
          "set" => {
            // Generate create request state
            states.push(StateSpec {
              name: format!("{}{}CreateRequest", capitalize(func_name), capitalize(storage_name)),
              kind: StateKind::StorageRequest { op: StorageOp::Create, storage: storage_name.clone() },
              in_vars: vec![
                Var::Input("key".to_string(), TypeName::StringTy),
                Var::Input("item".to_string(), TypeName::Custom("User".to_string())),
              ],
              out_vars: vec![],
            });

            // Generate create response state
            states.push(StateSpec {
              name: format!("{}{}CreateGot", capitalize(func_name), capitalize(storage_name)),
              kind: StateKind::StorageGot { op: StorageOp::Create, storage: storage_name.clone() },
              in_vars: vec![],
              out_vars: vec![],
            });
          }
          _ => {}
        }
      }
    }
    Expr::SyncCall { name: _call_name, args: _args } => {
      // doesn't generate anything, sync calls are embeded into state, it happens inside
    }
    _ => {
      // Handle other expression types as needed
    }
  }
}
