use std::{collections::HashMap, fs, path::Path};

use crate::ir::*;
use quote::ToTokens;
use syn::parse::Parser;
use syn::{
  Arm, Expr, ExprBlock, ExprMacro, ExprPath, ExprStruct, Lit, LitBool, LitStr, Member, Pat, PatIdent, PatTuple,
  PatTupleStruct, PathArguments, Token, parse::Parse, parse_file, punctuated::Punctuated,
};

#[test]
fn minimal_dsl_test() {
  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/dsl.rs");
  let ir = transpile_dsl_to_ir(path).expect("transpile");

  // Write a snapshot of the IR into generated_ir.rs for inspection/editing
  if let Err(e) = write_ir_snapshot(&ir) {
    eprintln!("failed to write IR snapshot: {}", e);
  }
}

pub fn transpile_dsl_to_ir(file_path: &str) -> Result<IR, String> {
  let src = fs::read_to_string(Path::new(file_path)).map_err(|e| format!("failed to read {}: {}", file_path, e))?;

  let file = parse_file(&src).map_err(|e| format!("parse error: {}", e))?;
  let mut fibers: HashMap<FiberType, Fiber> = HashMap::new();

  for item in file.items {
    if let syn::Item::Macro(m) = item {
      // match fiber!( ... )
      if let Some(path_ident) = m.mac.path.get_ident() {
        if path_ident == "fiber" {
          if let Ok(parsed) = syn::parse2::<FiberMacroInput>(m.mac.tokens.clone()) {
            let name = parsed.name.value();
            let fiber = make_fiber_from_body(&parsed.body);
            fibers.insert(FiberType::new(name), fiber);
          }
        }
      }
    }
  }

  Ok(IR { types: vec![], fibers })
}

fn make_fiber_from_body(body: &syn::Block) -> Fiber {
  // Find `fn main` inside the block and transpile its body into IR
  let mut func = Func { in_vars: vec![], out: Type::Void, locals: vec![], steps: vec![] };

  let mut pre_code = String::new();
  let mut post_code = String::new();
  let mut create_step: Option<Step> = None;
  let mut success_code = String::new();
  let mut fail_code = String::new();

  let mut queue_name_exprs: Vec<String> = Vec::new();
  let mut queue_name_locals: Vec<String> = Vec::new();
  let mut success_binds: Vec<LocalVarRef> = Vec::new();
  let mut fail_binds: Vec<LocalVarRef> = Vec::new();
  let mut primitives: Vec<RuntimePrimitive> = Vec::new();

  for stmt in &body.stmts {
    if let syn::Stmt::Item(syn::Item::Fn(f)) = stmt {
      if f.sig.ident == "main" {
        // Inspect main body statements
        // Find a match on mrn_create_primitives!(vec![...])
        let mut before_match: Vec<syn::Stmt> = Vec::new();
        let mut after_match: Vec<syn::Stmt> = Vec::new();
        let mut in_match = false;

        for s in &f.block.stmts {
          match s {
            syn::Stmt::Expr(Expr::Match(m), _) => {
              // Check match target is mrn_create_primitives!(...)
              if let Expr::Macro(ExprMacro { mac, .. }) = &*m.expr {
                if mac.path.is_ident("mrn_create_primitives") {
                  // parse primitives from the inner vec![]
                  if let Ok(Expr::Macro(inner_vec)) = syn::parse2::<Expr>(mac.tokens.clone()) {
                    if inner_vec.mac.path.is_ident("vec") {
                      // Parse elements as comma-separated Expr
                      if let Ok(elems) = syn::punctuated::Punctuated::<Expr, Token![,]>::parse_terminated
                        .parse2(inner_vec.mac.tokens.clone())
                      {
                        for el in elems.into_iter() {
                          match el {
                            Expr::Path(ExprPath { path, .. }) => {
                              if path.segments.last().map(|s| s.ident == "Future").unwrap_or(false) {
                                primitives.push(RuntimePrimitive::Future);
                              }
                            }
                            Expr::Struct(ExprStruct { path, fields, .. }) => {
                              if path.segments.last().map(|s| s.ident == "Queue").unwrap_or(false) {
                                let mut q_name_expr_str = None;
                                let mut public = false;
                                for fv in fields {
                                  if let Member::Named(ident) = fv.member.clone() {
                                    if ident == "name" {
                                      let s = strip_ws_outside_strings(&fv.expr.to_token_stream().to_string());
                                      q_name_expr_str = Some(s);
                                    } else if ident == "public" {
                                      public = match fv.expr {
                                        Expr::Lit(ref l) => match &l.lit {
                                          Lit::Bool(b) => b.value(),
                                          _ => false,
                                        },
                                        _ => false,
                                      };
                                    }
                                  }
                                }
                                // Synthesize temp local for queue name
                                let idx = queue_name_exprs.len();
                                let tmp_name = format!("__mrn_qname_{}", idx);
                                queue_name_locals.push(tmp_name.clone());
                                queue_name_exprs
                                  .push(q_name_expr_str.unwrap_or_else(|| "\"queue\".to_string()".to_string()));
                                primitives.push(RuntimePrimitive::Queue {
                                  name: LocalVarRef(Box::leak(tmp_name.clone().into_boxed_str())),
                                  public,
                                });
                              }
                            }
                            _ => {}
                          }
                        }
                      }
                    }
                  }

                  // Parse arms for Ok/Err binds and capture body code
                  for Arm { pat, body, .. } in &m.arms {
                    if let Pat::Tuple(PatTuple { elems, .. }) = pat {
                      // classify by first element path (Ok/Err)
                      let mut is_ok = None;
                      let mut names: Vec<String> = Vec::new();
                      for (i, p) in elems.iter().enumerate() {
                        match p {
                          Pat::TupleStruct(PatTupleStruct { path, elems: inner, .. }) => {
                            let last = path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                            if i == 0 {
                              if last == "Ok" {
                                is_ok = Some(true);
                              } else if last == "Err" {
                                is_ok = Some(false);
                              }
                            }
                            // expect single ident inside
                            if let Some(first) = inner.first() {
                              if let Pat::Ident(PatIdent { ident, .. }) = first {
                                names.push(ident.to_string());
                              }
                            }
                          }
                          _ => {}
                        }
                      }
                      if let Some(true) = is_ok {
                        for (i, n) in names.iter().enumerate() {
                          func.locals.push(LocalVar(
                            Box::leak(n.clone().into_boxed_str()),
                            match primitives.get(i) {
                              Some(RuntimePrimitive::Future) => Type::Future(Box::new(Type::Void)),
                              Some(RuntimePrimitive::Queue { .. }) => Type::String,
                              _ => Type::String,
                            },
                          ));
                          success_binds.push(LocalVarRef(Box::leak(n.clone().into_boxed_str())));
                        }
                        // capture success body code
                        success_code = match &**body {
                          Expr::Block(ExprBlock { block, .. }) => stringify_block_stmts(block),
                          other => strip_ws_outside_strings(&other.to_token_stream().to_string()),
                        };
                      } else if let Some(false) = is_ok {
                        for n in names {
                          let leaked = Box::leak(n.clone().into_boxed_str());
                          func.locals.push(LocalVar(leaked, Type::Option(Box::new(Type::String))));
                          fail_binds.push(LocalVarRef(leaked));
                        }
                        // capture fail body code
                        fail_code = match &**body {
                          Expr::Block(ExprBlock { block, .. }) => stringify_block_stmts(block),
                          other => strip_ws_outside_strings(&other.to_token_stream().to_string()),
                        };
                      }
                    }
                  }

                  in_match = true;
                }
              }
              if !in_match {
                before_match.push(s.clone());
              }
            }
            _ => {
              if in_match {
                after_match.push(s.clone());
              } else {
                before_match.push(s.clone());
              }
            }
          }
        }

        // Prepare code snippets
        pre_code = stringify_block_stmts(&syn::Block { brace_token: f.block.brace_token, stmts: before_match });
        post_code = stringify_block_stmts(&syn::Block { brace_token: f.block.brace_token, stmts: after_match });

        // Synthesize locals for queue temp names
        for qn in &queue_name_locals {
          func.locals.push(LocalVar(Box::leak(qn.clone().into_boxed_str()), Type::String));
        }

        // Build steps: only emit Create flow if we actually matched mrn_create_primitives.
        let mut steps: Vec<(StepId, Step)> = Vec::new();
        if in_match {
          let entry_id = StepId::new("entry");
          let after_entry_id = StepId::new("create_primitives");
          let after_match_id = StepId::new("after_match");
          let success_id = StepId::new("after_create_success");
          let fail_id = StepId::new("after_create_fail");

          // Entry RustBlock: perform any pre_code and compute queue name temps if any
          let entry_binds: Vec<LocalVarRef> =
            queue_name_locals.iter().map(|n| LocalVarRef(Box::leak(n.clone().into_boxed_str()))).collect();
          let entry_code = if entry_binds.is_empty() {
            pre_code.clone()
          } else if queue_name_exprs.len() == 1 {
            // single expression
            format!("{}{}", pre_code, queue_name_exprs[0])
          } else {
            // tuple of expressions
            let joined = queue_name_exprs.join(",");
            format!("{}({})", pre_code, joined)
          };
          steps.push((
            entry_id.clone(),
            Step::RustBlock { binds: entry_binds, code: entry_code, next: after_entry_id.clone() },
          ));

          // Create step
          create_step = Some(Step::Create {
            primitives: primitives.clone(),
            success: SuccessCreateBranch { next: success_id.clone(), id_binds: success_binds.clone() },
            fail: FailCreateBranch { next: fail_id.clone(), error_binds: fail_binds.clone() },
          });
          steps.push((after_entry_id.clone(), create_step.clone().unwrap()));

          // Success and fail continuation blocks
          steps.push((
            success_id.clone(),
            Step::RustBlock { binds: vec![], code: success_code.clone(), next: after_match_id.clone() },
          ));
          steps.push((
            fail_id.clone(),
            Step::RustBlock { binds: vec![], code: fail_code.clone(), next: after_match_id.clone() },
          ));

          // After match
          if !post_code.is_empty() {
            steps.push((
              after_match_id.clone(),
              Step::RustBlock { binds: vec![], code: post_code.clone(), next: StepId::new("return") },
            ));
          }
          steps.push((StepId::new("return"), Step::ReturnVoid));
        } else {
          // No create primitives: just a simple RustBlock with the entire function body and ReturnVoid
          let entry_id = StepId::new("entry");
          let code = stringify_block_stmts(&f.block);
          steps.push((entry_id, Step::RustBlock { binds: vec![], code, next: StepId::new("return") }));
          steps.push((StepId::new("return"), Step::ReturnVoid));
        }

        func.steps = steps;
        break;
      }
    }
  }

  // Fallback: if we didn't generate specialized steps, keep the original behavior
  if func.steps.is_empty() {
    for stmt in &body.stmts {
      if let syn::Stmt::Item(syn::Item::Fn(f)) = stmt {
        if f.sig.ident == "main" {
          let main_code = stringify_block_stmts(&f.block);
          func.steps.push((
            StepId::new("entry"),
            Step::RustBlock { binds: vec![], code: main_code, next: StepId::new("return") },
          ));
          func.steps.push((StepId::new("return"), Step::ReturnVoid));
        }
      }
    }
  }

  Fiber { heap: HashMap::new(), init_vars: vec![], funcs: HashMap::from([("main".to_string(), func)]) }
}

// fiber!("name", { ... })
struct FiberMacroInput {
  name: LitStr,
  _comma: Token![,],
  body: syn::Block,
}

impl Parse for FiberMacroInput {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name: LitStr = input.parse()?;
    let _comma: Token![,] = input.parse()?;
    let body: syn::Block = input.parse()?;
    Ok(FiberMacroInput { name, _comma, body })
  }
}

// --- Snapshot helpers ---

fn write_ir_snapshot(ir: &IR) -> Result<(), String> {
  let file_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/generated_ir.rs");
  let mut content = fs::read_to_string(file_path).map_err(|e| format!("read {}: {}", file_path, e))?;

  // Build the replacement block
  let ir_code = format_ir_as_rust(ir);
  let mut replacement = String::new();
  replacement.push_str("  let ir = ");
  replacement.push_str(&ir_code);
  replacement.push_str(";\n");

  // Ensure header template with imports + test fn signature
  let lines: Vec<&str> = content.lines().collect();
  let mut test_idx: Option<usize> = None;
  let mut sig_idx: Option<usize> = None;
  for (i, line) in lines.iter().enumerate() {
    if test_idx.is_none() && line.trim_start().starts_with("#[test]") {
      test_idx = Some(i);
    }
    if line.contains("fn generated_ir_valid()") {
      sig_idx = Some(i);
      break;
    }
  }
  if let (Some(_t), Some(sig)) = (test_idx, sig_idx) {
    let header_template = "use std::collections::HashMap;\nuse crate::ir::*;\n\n#[test]\nfn generated_ir_valid() {\n";
    let mut rebuilt = String::new();
    // Replace everything up to and including the fn signature line with the header template
    rebuilt.push_str(header_template);
    for i in (sig + 1)..lines.len() {
      rebuilt.push_str(lines[i]);
      rebuilt.push('\n');
    }
    content = rebuilt;
  }

  // Replace the entire `let ir = ...;` statement up to the assert line,
  // to keep the test structure and avoid duplicate blocks.
  let lines: Vec<&str> = content.lines().collect();
  let mut start_idx: Option<usize> = None;
  let mut end_idx: Option<usize> = None;
  for (i, line) in lines.iter().enumerate() {
    if start_idx.is_none() && line.trim_start().starts_with("let ir =") {
      start_idx = Some(i);
    }
    if start_idx.is_some() && line.contains("assert_eq!") {
      end_idx = Some(i);
      break;
    }
  }
  let (start, end) = match (start_idx, end_idx) {
    (Some(s), Some(e)) if s < e => (s, e),
    _ => return Err("could not find replacement window in generated_ir.rs".to_string()),
  };

  let mut new_content = String::new();
  for i in 0..start { new_content.push_str(lines[i]); new_content.push('\n'); }
  new_content.push_str(&replacement);
  for i in end..lines.len() { new_content.push_str(lines[i]); new_content.push('\n'); }
  content = new_content;
  fs::write(file_path, content).map_err(|e| format!("write {}: {}", file_path, e))?;

  // Best-effort formatting with rustfmt (if available)
  match std::process::Command::new("rustfmt").arg("--edition").arg("2021").arg(file_path).status() {
    Ok(status) if status.success() => {}
    Ok(status) => eprintln!("rustfmt exited with status: {}", status),
    Err(err) => eprintln!("rustfmt not available: {}", err),
  }

  Ok(())
}

fn format_ir_as_rust(ir: &IR) -> String {
  // We rely on `use crate::ir::*; use std::collections::HashMap;` in the file header

  fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
      match ch {
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c => out.push(c),
      }
    }
    out.push('"');
    out
  }

  fn fmt_type(t: &Type) -> String {
    match t {
      Type::UInt64 => "Type::UInt64".to_string(),
      Type::String => "Type::String".to_string(),
      Type::Void => "Type::Void".to_string(),
      Type::Map(k, v) => format!("Type::Map(Box::new({})), Box::new({}))", fmt_type(k), fmt_type(v))
        .replace(") , Box", ") , Box"),
      Type::Array(inner) => format!("Type::Array(Box::new({}))", fmt_type(inner)),
      Type::Struct(name, fields, add) => format!(
        "Type::Struct({}, vec![{}], {})",
        esc(name),
        fields.iter().map(|f| fmt_struct_field(f)).collect::<Vec<_>>().join(","),
        esc(add)
      ),
      Type::MaxQueue(inner) => format!("Type::MaxQueue(Box::new({}))", fmt_type(inner)),
      Type::MinQueue(inner) => format!("Type::MinQueue(Box::new({}))", fmt_type(inner)),
      Type::Option(inner) => format!("Type::Option(Box::new({}))", fmt_type(inner)),
      Type::Future(inner) => format!("Type::Future(Box::new({}))", fmt_type(inner)),
      Type::Custom(s) => format!("Type::Custom({})", esc(s)),
      Type::PubQueueMessage { name, fields, rust_additions } => format!(
        "Type::PubQueueMessage {{ name: {}, fields: vec![{}], rust_additions: {} }}",
        esc(name),
        fields.iter().map(|f| fmt_struct_field(f)).collect::<Vec<_>>().join(","),
        esc(rust_additions)
      ),
    }
  }

  fn fmt_struct_field(f: &StructField) -> String {
    format!("StructField {{ name: {}, ty: {} }}", esc(&f.name), fmt_type(&f.ty))
  }

  fn fmt_lvar_ref(r: &LocalVarRef) -> String { format!("LocalVarRef({})", esc(r.0)) }
  fn fmt_step_id(s: &StepId) -> String { format!("StepId::new({})", esc(&s.0)) }

  fn fmt_runtime_primitive(rp: &RuntimePrimitive) -> String {
    match rp {
      RuntimePrimitive::Future => "RuntimePrimitive::Future".to_string(),
      RuntimePrimitive::Queue { name, public } => format!(
        "RuntimePrimitive::Queue {{ name: {}, public: {} }}",
        fmt_lvar_ref(name),
        public
      ),
      RuntimePrimitive::Schedule { ms_var } => format!(
        "RuntimePrimitive::Schedule {{ ms_var: {} }}",
        fmt_lvar_ref(ms_var)
      ),
    }
  }

  fn fmt_step(pair: &(StepId, Step)) -> String {
    match &pair.1 {
      Step::RustBlock { binds, code, next } => format!(
        "({}, Step::RustBlock {{ binds: vec![{}], code: {}.to_string(), next: {} }})",
        fmt_step_id(&pair.0),
        binds.iter().map(fmt_lvar_ref).collect::<Vec<_>>().join(","),
        esc(code),
        fmt_step_id(next)
      ),
      Step::Create { primitives, success, fail } => format!(
        "({}, Step::Create {{ primitives: vec![{}], success: SuccessCreateBranch {{ next: {}, id_binds: vec![{}] }}, fail: FailCreateBranch {{ next: {}, error_binds: vec![{}] }} }})",
        fmt_step_id(&pair.0),
        primitives.iter().map(fmt_runtime_primitive).collect::<Vec<_>>().join(","),
        fmt_step_id(&success.next),
        success.id_binds.iter().map(fmt_lvar_ref).collect::<Vec<_>>().join(","),
        fmt_step_id(&fail.next),
        fail.error_binds.iter().map(fmt_lvar_ref).collect::<Vec<_>>().join(","),
      ),
      Step::ReturnVoid => format!("({}, Step::ReturnVoid)", fmt_step_id(&pair.0)),
      Step::Return { value } => format!(
        "({}, Step::Return {{ value: {} }})",
        fmt_step_id(&pair.0),
        match value {
          RetValue::Var(v) => format!("RetValue::Var({})", fmt_lvar_ref(v)),
          RetValue::UInt64(n) => format!("RetValue::UInt64({})", n),
          RetValue::Str(s) => format!("RetValue::Str({})", esc(s)),
          RetValue::Some(inner) => format!("RetValue::Some(Box::new({}))", match &**inner {
            RetValue::UInt64(n) => format!("RetValue::UInt64({})", n),
            RetValue::Str(s) => format!("RetValue::Str({})", esc(s)),
            RetValue::Var(v) => format!("RetValue::Var({})", fmt_lvar_ref(v)),
            RetValue::None => "RetValue::None".to_string(),
            RetValue::Some(_) => "/* nested Some unsupported in snapshot */".to_string(),
          } ),
          RetValue::None => "RetValue::None".to_string(),
        }
      ),
      Step::Debug(msg, next) => format!(
        "({}, Step::Debug({}, {}))",
        fmt_step_id(&pair.0),
        esc(msg),
        fmt_step_id(next)
      ),
      Step::DebugPrintVars(next) => format!(
        "({}, Step::DebugPrintVars({}))",
        fmt_step_id(&pair.0),
        fmt_step_id(next)
      ),
      Step::If { .. } => format!("({}, /* If omitted in snapshot */ Step::ReturnVoid)", fmt_step_id(&pair.0)),
      Step::Let { .. } => format!("({}, /* Let omitted in snapshot */ Step::ReturnVoid)", fmt_step_id(&pair.0)),
      Step::SetValues { .. } => format!("({}, /* SetValues omitted in snapshot */ Step::ReturnVoid)", fmt_step_id(&pair.0)),
      Step::Call { .. } => format!("({}, /* Call omitted in snapshot */ Step::ReturnVoid)", fmt_step_id(&pair.0)),
      Step::Select { .. } => format!("({}, /* Select omitted in snapshot */ Step::ReturnVoid)", fmt_step_id(&pair.0)),
      Step::CreateFibers { .. } => format!(
        "({}, /* CreateFibers omitted in snapshot */ Step::ReturnVoid)",
        fmt_step_id(&pair.0)
      ),
    }
  }

  let mut out = String::new();
  out.push_str(&format!(
    "IR {{ types: vec![{}], fibers: HashMap::from([{}]) }}",
    ir.types.iter().map(|t| fmt_type(t)).collect::<Vec<_>>().join(","),
    {
      let mut fibers = Vec::new();
      for (ft, fdef) in &ir.fibers {
        let locals = fdef
          .funcs
          .get(&"main".to_string())
          .map(|f| f.locals.iter().map(|l| format!("LocalVar({}, {})", esc(l.0), fmt_type(&l.1))).collect::<Vec<_>>().join(","))
          .unwrap_or_else(|| String::new());
        let steps = fdef
          .funcs
          .get(&"main".to_string())
          .map(|f| f.steps.iter().map(fmt_step).collect::<Vec<_>>().join(","))
          .unwrap_or_else(|| String::new());
        let funcs = format!("HashMap::from([(\"main\".to_string(), Func {{ in_vars: vec![], out: Type::Void, locals: vec![{}], steps: vec![{}] }})])", locals, steps);
        fibers.push(format!(
          "(FiberType::new({}), Fiber {{ heap: HashMap::new(), init_vars: vec![], funcs: {} }})",
          esc(&ft.0),
          funcs
        ));
      }
      fibers.join(",")
    }
  ));
  out
}

fn stringify_block_stmts(block: &syn::Block) -> String {
  // Join all statement token streams and strip whitespace outside of string literals
  let combined: String = block.stmts.iter().map(|s| s.to_token_stream().to_string()).collect::<Vec<_>>().join(" ");
  strip_ws_outside_strings(&combined)
}

fn strip_ws_outside_strings(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let mut in_str = false;
  let mut escape = false;
  for ch in input.chars() {
    if in_str {
      out.push(ch);
      if escape {
        escape = false;
      } else if ch == '\\' {
        escape = true;
      } else if ch == '"' {
        in_str = false;
      }
      continue;
    }
    match ch {
      '"' => {
        in_str = true;
        out.push(ch);
      }
      c if c.is_whitespace() => {
        // skip
      }
      c => out.push(c),
    }
  }
  out
}
