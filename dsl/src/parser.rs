use crate::ast::{
  BinOp, Block, Expr, Function, Item, Mutability, Param, Program, Statement, StructDef, StructField, TypeName, VarDecl,
};

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct DSLParser;

pub fn parse_program(input: &str) -> Result<Program, String> {
  let mut pairs = DSLParser::parse(Rule::program, input).map_err(|e| e.to_string())?;
  let root = pairs.next().ok_or_else(|| "program: missing root".to_string())?;

  let mut items = Vec::new();
  for pair in root.into_inner() {
    match pair.as_rule() {
      Rule::item => items.push(parse_item(pair)?),
      Rule::EOI => {}
      _ => {}
    }
  }
  Ok(Program { items })
}

fn parse_item(pair: Pair<Rule>) -> Result<Item, String> {
  let mut inner = pair.into_inner();
  let p = inner.next().ok_or_else(|| "item: missing inner".to_string())?;
  match p.as_rule() {
    Rule::struct_def => Ok(Item::Struct(parse_struct_def(p)?)),
    Rule::function => Ok(Item::Function(parse_function(p)?)),
    Rule::statement => Ok(Item::Statement(parse_statement(p)?)),
    _ => Err("item: unexpected rule".into()),
  }
}

fn parse_struct_def(pair: Pair<Rule>) -> Result<StructDef, String> {
  let mut inner = pair.into_inner();
  let name_pair = inner.next().ok_or_else(|| "struct_def: missing name".to_string())?;
  let name = name_pair.as_str().to_string();

  let mut fields = Vec::new();
  if let Some(fields_pair) = inner.next() {
    match fields_pair.as_rule() {
      Rule::struct_fields => {
        for f in fields_pair.into_inner() {
          if f.as_rule() == Rule::struct_field {
            fields.push(parse_struct_field(f)?);
          }
        }
      }
      _ => {}
    }
  }

  Ok(StructDef { name, fields })
}

fn parse_struct_field(pair: Pair<Rule>) -> Result<StructField, String> {
  let mut inner = pair.into_inner();
  let name = inner.next().ok_or_else(|| "struct_field: missing name".to_string())?.as_str().to_string();
  let ty_pair = inner.next().ok_or_else(|| "struct_field: missing type".to_string())?;
  let ty = parse_type_name(ty_pair)?;
  Ok(StructField { name, ty })
}

fn parse_function(pair: Pair<Rule>) -> Result<Function, String> {
  let mut inner = pair.into_inner();

  let name = inner.next().ok_or_else(|| "function: missing name".to_string())?.as_str().to_string();

  // optional
  let mut params = Vec::new();
  let next = inner.next().ok_or_else(|| "function: missing after name".to_string())?;
  let (ret_and_rest, had_params) = match next.as_rule() {
    Rule::param_list => {
      for p in next.into_inner() {
        if p.as_rule() == Rule::param {
          params.push(parse_param(p)?);
        }
      }
      (inner.next().ok_or("function: missing return type")?, true)
    }
    Rule::type_name => (next, false),
    _ => return Err("function: unexpected after name".into()),
  };

  // return type
  let ret_ty = if had_params { parse_type_name(ret_and_rest)? } else { parse_type_name(ret_and_rest)? };

  // body
  let body_pair = inner.next().ok_or_else(|| "function: missing body block".to_string())?;
  let body = parse_block(body_pair)?;

  Ok(Function { name, params, ret: ret_ty, body })
}

fn parse_param(pair: Pair<Rule>) -> Result<Param, String> {
  let mut inner = pair.into_inner();
  let name = inner.next().ok_or_else(|| "param: missing name".to_string())?.as_str().to_string();
  let ty_pair = inner.next().ok_or_else(|| "param: missing type".to_string())?;
  let ty = parse_type_name(ty_pair)?;
  Ok(Param { name, ty })
}

fn parse_block(pair: Pair<Rule>) -> Result<Block, String> {
  let mut statements = Vec::new();
  for p in pair.into_inner() {
    if p.as_rule() == Rule::statement {
      statements.push(parse_statement(p)?);
    }
  }
  Ok(Block { statements })
}

fn parse_statement(pair: Pair<Rule>) -> Result<Statement, String> {
  let mut inner = pair.into_inner();
  let p = inner.next().ok_or_else(|| "statement: missing inner".to_string())?;
  match p.as_rule() {
    Rule::binding_stmt => {
      let decl = parse_binding_stmt(p)?;
      Ok(Statement::VarDecl(decl))
    }
    Rule::return_stmt => {
      let expr_pair = p.into_inner().next().ok_or_else(|| "return: missing expr".to_string())?;
      let expr = parse_expr(expr_pair)?;
      Ok(Statement::Return(expr))
    }
    Rule::if_stmt => parse_if_stmt(p),
    Rule::expr => {
      let expr = parse_expr(p)?;
      Ok(Statement::Expr(expr))
    }
    _ => Err(format!("statement: unexpected rule {:?}", p.as_rule())),
  }
}

fn parse_if_stmt(pair: Pair<Rule>) -> Result<Statement, String> {
  let mut inner = pair.into_inner();
  let cond_pair = inner.next().ok_or_else(|| "if: missing condition".to_string())?;
  let cond = parse_expr(cond_pair)?;
  let then_pair = inner.next().ok_or_else(|| "if: missing then block".to_string())?;
  let then_blk = parse_block(then_pair)?;
  let else_blk = if let Some(next) = inner.next() { Some(parse_block(next)?) } else { None };
  Ok(Statement::If { cond, then_blk, else_blk })
}

fn parse_binding_stmt(pair: Pair<Rule>) -> Result<VarDecl, String> {
  let mut inner = pair.into_inner();

  let kind = inner.next().ok_or_else(|| "binding: missing kind".to_string())?;
  let mutability = match kind.as_str() {
    "let" => Mutability::Immutable,
    "var" => Mutability::Mutable,
    _ => return Err("binding: invalid kind".into()),
  };

  let name = inner.next().ok_or_else(|| "binding: missing name".to_string())?.as_str().to_string();

  let ty_pair = inner.next().ok_or_else(|| "binding: missing type".to_string())?;
  let ty = parse_type_name(ty_pair)?;

  let init = if let Some(expr_or_rest) = inner.next() { Some(parse_expr(expr_or_rest)?) } else { None };

  Ok(VarDecl { mutability, name, ty, init })
}

fn parse_expr(pair: Pair<Rule>) -> Result<Expr, String> {
  debug_assert_eq!(pair.as_rule(), Rule::expr);
  let p = pair.into_inner().next().ok_or_else(|| "expr: missing equality".to_string())?;
  parse_equality(p)
}

fn parse_equality(pair: Pair<Rule>) -> Result<Expr, String> {
  let mut inner = pair.into_inner();
  let mut left = parse_add(inner.next().ok_or_else(|| "equality: missing left".to_string())?)?;

  while let Some(op_or_rest) = inner.next() {
    let op = match op_or_rest.as_rule() {
      Rule::equality_op => match op_or_rest.as_str() {
        "==" => BinOp::Eq,
        "!=" => BinOp::Ne,
        _ => return Err("equality: bad op".into()),
      },
      _ => return Err("equality: expected op".into()),
    };
    let rhs = parse_add(inner.next().ok_or_else(|| "equality: missing right".to_string())?)?;
    left = Expr::Binary { left: Box::new(left), op, right: Box::new(rhs) };
  }
  Ok(left)
}

fn parse_add(pair: Pair<Rule>) -> Result<Expr, String> {
  let mut inner = pair.into_inner();
  let mut left = parse_mul(inner.next().ok_or_else(|| "add: missing left".to_string())?)?;

  while let Some(op_or_rest) = inner.next() {
    let op = match op_or_rest.as_rule() {
      Rule::add_op => match op_or_rest.as_str() {
        "+" => BinOp::Add,
        "-" => BinOp::Sub,
        _ => return Err("add: bad op".into()),
      },
      _ => return Err("add: expected op".into()),
    };
    let rhs = parse_mul(inner.next().ok_or_else(|| "add: missing right".to_string())?)?;
    left = Expr::Binary { left: Box::new(left), op, right: Box::new(rhs) };
  }
  Ok(left)
}

fn parse_mul(pair: Pair<Rule>) -> Result<Expr, String> {
  let mut inner = pair.into_inner();
  let mut left = parse_primary(inner.next().ok_or_else(|| "mul: missing left".to_string())?)?;

  while let Some(op_or_rest) = inner.next() {
    let op = match op_or_rest.as_rule() {
      Rule::mul_op => match op_or_rest.as_str() {
        "*" => BinOp::Mul,
        "/" => BinOp::Div,
        _ => return Err("mul: bad op".into()),
      },
      _ => return Err("mul: expected op".into()),
    };
    let rhs = parse_primary(inner.next().ok_or_else(|| "mul: missing right".to_string())?)?;
    left = Expr::Binary { left: Box::new(left), op, right: Box::new(rhs) };
  }
  Ok(left)
}

fn parse_primary(pair: Pair<Rule>) -> Result<Expr, String> {
  match pair.as_rule() {
    Rule::primary => {
      let inner = pair.into_inner().next().ok_or_else(|| "primary: empty".to_string())?;
      parse_primary(inner)
    }
    Rule::call => parse_call(pair),
    Rule::identifier => Ok(Expr::Ident(pair.as_str().to_string())),
    Rule::int => {
      let n: i64 = pair.as_str().parse().map_err(|e| format!("invalid int: {e}"))?;
      Ok(Expr::Int(n))
    }
    Rule::string => {
      let s = pair.as_str();
      // strip quotes
      let unquoted = &s[1..s.len() - 1];
      Ok(Expr::Str(unquoted.to_string()))
    }
    Rule::array_lit => parse_array_lit(pair),
    Rule::map_lit => parse_map_lit(pair),
    Rule::expr => parse_expr(pair),
    _ => Err(format!("primary: unexpected rule {:?}", pair.as_rule())),
  }
}

fn parse_call(pair: Pair<Rule>) -> Result<Expr, String> {
  let mut inner = pair.into_inner();
  let name = inner.next().ok_or_else(|| "call: missing name".to_string())?.as_str().to_string();
  let mut args = Vec::new();
  if let Some(next) = inner.next() {
    if next.as_rule() == Rule::arg_list {
      for a in next.into_inner() {
        if a.as_rule() == Rule::expr {
          args.push(parse_expr(a)?);
        }
      }
    } else {
      // no args
    }
  }
  Ok(Expr::Call { name, args })
}

fn parse_array_lit(pair: Pair<Rule>) -> Result<Expr, String> {
  let mut elems = Vec::new();
  for p in pair.into_inner() {
    if p.as_rule() == Rule::expr {
      elems.push(parse_expr(p)?);
    }
  }
  Ok(Expr::ArrayLit(elems))
}

fn parse_map_lit(pair: Pair<Rule>) -> Result<Expr, String> {
  let mut entries = Vec::new();
  for p in pair.into_inner() {
    if p.as_rule() == Rule::map_entry {
      let mut m = p.into_inner();
      let k = parse_expr(m.next().ok_or("map_entry: missing key")?)?;
      let v = parse_expr(m.next().ok_or("map_entry: missing value")?)?;
      entries.push((k, v));
    }
  }
  Ok(Expr::MapLit(entries))
}

fn parse_type_name(pair: Pair<Rule>) -> Result<TypeName, String> {
  let inner = pair.into_inner().next().ok_or_else(|| "type_name: missing type_ref".to_string())?;
  parse_type_ref(inner)
}

fn parse_type_ref(pair: Pair<Rule>) -> Result<TypeName, String> {
  match pair.as_rule() {
    Rule::base_type => match pair.as_str() {
      "String" => Ok(TypeName::StringTy),
      "i32" => Ok(TypeName::I32),
      "i64" => Ok(TypeName::I64),
      _ => Err("unknown base type".into()),
    },
    Rule::array_type => {
      let inner = pair.into_inner().next().ok_or_else(|| "array_type: missing inner type".to_string())?;
      Ok(TypeName::Array(Box::new(parse_type_ref(inner)?)))
    }
    Rule::map_type => {
      let mut it = pair.into_inner();
      let key_t = parse_type_ref(it.next().ok_or("map_type: missing key")?)?;
      let val_t = parse_type_ref(it.next().ok_or("map_type: missing value")?)?;
      Ok(TypeName::Map(Box::new(key_t), Box::new(val_t)))
    }
    Rule::custom_type => {
      let name = pair.into_inner().next().ok_or_else(|| "custom_type: missing ident".to_string())?.as_str().to_string();
      Ok(TypeName::Custom(name))
    }
    Rule::type_ref => {
      let inner = pair.into_inner().next().ok_or_else(|| "type_ref: missing inner".to_string())?;
      parse_type_ref(inner)
    }
    _ => Err(format!("type_ref: unexpected rule {:?}", pair.as_rule())),
  }
}
