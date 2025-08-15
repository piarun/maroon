use crate::ast::{Block, Expr, Item, Mutability, Program, Statement};

#[derive(Debug, Clone, PartialEq)]
enum StorageOp {
  Get,
  Create,
  // TODO: delete/update
  // TODO: extend to array/structs. What about primitive variables?
}

#[derive(Debug, Clone, PartialEq)]
enum State {
  // Global storage states
  StorageIdle { storage: String },
  StorageGetItemRequest { storage: String },
  StorageCreateItemRequest { storage: String },

  // Function lifecycle
  FuncEntry { func: String },
  FuncRecursiveCall { func: String },
  FuncCall { func: String, callee: String },
  FuncStorageRequest { func: String, op: StorageOp, storage: String },
  FuncStorageGot { func: String, op: StorageOp, storage: String },
  FuncDone { func: String },
}

fn render_op(op: &StorageOp) -> &'static str {
  match op {
    StorageOp::Get => "Get",
    StorageOp::Create => "Create",
  }
}

fn render_state(s: &State) -> String {
  match s {
    State::StorageIdle { storage } => format!("{}StorageIdle", storage),
    State::StorageGetItemRequest { storage } => format!("{}StorageGetItemRequest", storage),
    State::StorageCreateItemRequest { storage } => format!("{}StorageCreateItemRequest", storage),

    State::FuncEntry { func } => format!("{}Entry", func),
    State::FuncRecursiveCall { func } => format!("{}RecursiveCall", func),
    State::FuncCall { func, callee } => format!("{}Call{}", func, callee),
    State::FuncStorageRequest { func, op, storage } => format!("{}{}{}Request", func, render_op(op), storage),
    State::FuncStorageGot { func, op, storage } => format!("{}{}{}Got", func, render_op(op), storage),
    State::FuncDone { func } => format!("{}Done", func),
  }
}

fn render_states(states: Vec<State>) -> Vec<String> {
  states.into_iter().map(|s| render_state(&s)).collect()
}

pub fn states_from_program(program: &Program) -> Vec<String> {
  let mut states: Vec<State> = vec![];

  // generate states for global storages (top-level mutable variables)
  for el in program.items.iter() {
    let Item::Statement(Statement::VarDecl(var_decl)) = el else {
      continue;
    };

    if Mutability::Mutable == var_decl.mutability {
      let storage_name = capitalize_first(&var_decl.name);
      states.push(State::StorageIdle { storage: storage_name.clone() });
      states.push(State::StorageGetItemRequest { storage: storage_name.clone() });
      states.push(State::StorageCreateItemRequest { storage: storage_name });
    }
  }

  // generate states for each function
  for el in program.items.iter() {
    let Item::Function(func) = el else { continue };

    let prefix = capitalize_first(&func.name);

    states.push(State::FuncEntry { func: prefix.clone() });

    let mut function_calls: Vec<String> = Vec::new();
    let mut storage_states: Vec<(String, StorageOp)> = Vec::new();
    collect_function_calls(&func.body, &mut function_calls);
    collect_storage_states(&func.body, &mut storage_states);

    for (storage, op) in storage_states {
      let storage_title = capitalize_first(&storage);
      states.push(State::FuncStorageRequest { func: prefix.clone(), op: op.clone(), storage: storage_title.clone() });
      states.push(State::FuncStorageGot { func: prefix.clone(), op, storage: storage_title });
    }

    for call_name in function_calls {
      let call_prefix = capitalize_first(&call_name);
      if call_name == func.name {
        states.push(State::FuncRecursiveCall { func: prefix.clone() });
      } else {
        states.push(State::FuncCall { func: prefix.clone(), callee: call_prefix });
      }
    }

    states.push(State::FuncDone { func: prefix });
  }

  render_states(states)
}

fn collect_function_calls(
  block: &Block,
  calls: &mut Vec<String>,
) {
  for statement in &block.statements {
    collect_function_calls_from_statement(statement, calls);
  }
}

fn collect_function_calls_from_statement(
  statement: &Statement,
  calls: &mut Vec<String>,
) {
  match statement {
    Statement::VarDecl(var_decl) => {
      if let Some(init_expr) = &var_decl.init {
        collect_function_calls_from_expr(init_expr, calls);
      }
    }
    Statement::Return(expr) => collect_function_calls_from_expr(expr, calls),
    Statement::If { cond, then_blk, else_blk } => {
      collect_function_calls_from_expr(cond, calls);
      collect_function_calls(then_blk, calls);
      if let Some(else_blk) = else_blk {
        collect_function_calls(else_blk, calls);
      }
    }
    Statement::Expr(expr) => collect_function_calls_from_expr(expr, calls),
  }
}

fn collect_function_calls_from_expr(
  expr: &Expr,
  calls: &mut Vec<String>,
) {
  match expr {
    Expr::Call { name, args } => {
      calls.push(name.clone());
      for arg in args {
        collect_function_calls_from_expr(arg, calls);
      }
    }
    Expr::SyncCall { name: _, args } => {
      for arg in args {
        collect_function_calls_from_expr(arg, calls);
      }
    }
    Expr::StructLit { name: _, fields } => {
      for f in fields {
        collect_function_calls_from_expr(&f.value, calls);
      }
    }
    Expr::MethodCall { receiver, name: _, args } => {
      collect_function_calls_from_expr(receiver, calls);
      for arg in args {
        collect_function_calls_from_expr(arg, calls);
      }
    }
    Expr::Binary { left, right, .. } => {
      collect_function_calls_from_expr(left, calls);
      collect_function_calls_from_expr(right, calls);
    }
    Expr::ArrayLit(elements) => {
      for elem in elements {
        collect_function_calls_from_expr(elem, calls);
      }
    }
    Expr::MapLit(entries) => {
      for (key, value) in entries {
        collect_function_calls_from_expr(key, calls);
        collect_function_calls_from_expr(value, calls);
      }
    }
    Expr::Int(_) | Expr::Str(_) | Expr::Ident(_) => {}
  }
}

// Collect storage states: users.get(x) -> ("Users", "Get"), users.set(...) -> ("Users", "Create")
// TODO: add support not only for maps
// TODO: add storage states: delete, update, anything else?
fn collect_storage_states(
  block: &Block,
  states: &mut Vec<(String, StorageOp)>,
) {
  for statement in &block.statements {
    collect_storage_states_from_statement(statement, states);
  }
}

fn collect_storage_states_from_statement(
  statement: &Statement,
  states: &mut Vec<(String, StorageOp)>,
) {
  match statement {
    Statement::VarDecl(var_decl) => {
      if let Some(init_expr) = &var_decl.init {
        collect_storage_states_from_expr(init_expr, states);
      }
    }
    Statement::Return(expr) => collect_storage_states_from_expr(expr, states),
    Statement::If { cond, then_blk, else_blk } => {
      collect_storage_states_from_expr(cond, states);
      collect_storage_states(then_blk, states);
      if let Some(else_blk) = else_blk {
        collect_storage_states(else_blk, states);
      }
    }
    Statement::Expr(expr) => collect_storage_states_from_expr(expr, states),
  }
}

fn collect_storage_states_from_expr(
  expr: &Expr,
  states: &mut Vec<(String, StorageOp)>,
) {
  match expr {
    Expr::MethodCall { receiver, name, args } => {
      // If receiver is an identifier and name is get/set, create states
      if let Expr::Ident(storage_ident) = &**receiver {
        let op = match name.as_str() {
          "get" => Some(StorageOp::Get),
          "set" => Some(StorageOp::Create),
          _ => None,
        };
        if let Some(op) = op {
          let title = capitalize_first(storage_ident);
          states.push((title, op));
        }
      }
      collect_storage_states_from_expr(receiver, states);
      for arg in args {
        collect_storage_states_from_expr(arg, states);
      }
    }
    Expr::Call { args, .. } | Expr::SyncCall { args, .. } => {
      for arg in args {
        collect_storage_states_from_expr(arg, states);
      }
    }
    Expr::StructLit { fields, .. } => {
      for f in fields {
        collect_storage_states_from_expr(&f.value, states);
      }
    }
    Expr::Binary { left, right, .. } => {
      collect_storage_states_from_expr(left, states);
      collect_storage_states_from_expr(right, states);
    }
    Expr::ArrayLit(elements) => {
      for elem in elements {
        collect_storage_states_from_expr(elem, states);
      }
    }
    Expr::MapLit(entries) => {
      for (key, value) in entries {
        collect_storage_states_from_expr(key, states);
        collect_storage_states_from_expr(value, states);
      }
    }
    Expr::Int(_) | Expr::Str(_) | Expr::Ident(_) => {}
  }
}

fn capitalize_first(s: &str) -> String {
  let mut out = s.to_string();
  out.get_mut(0..1).map(|p| {
    p.make_ascii_uppercase();
    &*p
  });
  out
}
