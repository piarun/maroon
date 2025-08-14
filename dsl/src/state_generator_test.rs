use crate::{
  ast::{Block, Expr, Item, Program, Statement},
  parser,
};

#[test]
fn test_expression_in_statement() {
  let input = r#"
        fn factorial(n: i32) -> i32 {
            if n == 0 {
                return 1
            } else if n == 1 {
                return 1
            }

            return n * factorial(n - 1)
        }

        fn delayed(t: i64, message: String){
          sleep(t)
          print(message)
        }
    "#;

  let program = parser::parse_program(input);
  assert!(program.is_ok());

  let program = program.unwrap();
  let expected_states = vec![
    "FactorialEntry".to_string(),
    "FactorialRecursiveCall".to_string(),
    "FactorialDone".to_string(),
    "DelayedEntry".to_string(),
    "DelayedCallSleep".to_string(),
    "DelayedCallPrint".to_string(),
    "DelayedDone".to_string(),
  ];

  assert_eq!(expected_states, states_from_program(&program))
}

fn states_from_program(program: &Program) -> Vec<String> {
  let mut steps: Vec<String> = vec![];

  for el in program.items.iter() {
    let Item::Function(func) = el else { continue };

    let mut prefix = func.name.clone();
    prefix.get_mut(0..1).map(|s| {
      s.make_ascii_uppercase();
      &*s
    });

    steps.push(format!("{}Entry", prefix));

    let mut function_calls = Vec::new();
    collect_function_calls(&func.body, &mut function_calls);

    for call_name in function_calls {
      let mut call_prefix = call_name.clone();
      call_prefix.get_mut(0..1).map(|s| {
        s.make_ascii_uppercase();
        &*s
      });

      if call_name == func.name {
        steps.push(format!("{}RecursiveCall", prefix));
      } else {
        steps.push(format!("{}Call{}", prefix, call_prefix));
      }
    }

    steps.push(format!("{}Done", prefix));
  }

  steps
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
