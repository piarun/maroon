use std::{collections::HashMap, fs, path::Path};

use crate::ir::*;
use quote::ToTokens;
use syn::{LitStr, Token, parse::Parse, parse_file};

#[test]
fn minimal_dsl_test() {
  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/dsl.rs");
  let ir = transpile_dsl_to_ir(path).expect("transpile");

  assert_eq!(
    IR {
      types: vec![],
      fibers: HashMap::from([(
        FiberType::new("minimalRoot"),
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
                    code: r#"println("hello");"#.to_string(),
                    next: StepId::new("L28:mrn_create_queues")
                  }
                ),
                (
                  StepId::new("L28:mrn_create_queues"),
                  Step::Create {
                    primitives: vec![],
                    success: SuccessCreateBranch { next: StepId::new("return"), id_binds: vec![] },
                    fail: FailCreateBranch { next: StepId::new("return"), error_binds: vec![] },
                  }
                ),
                (StepId::new("return"), Step::ReturnVoid,)
              ]
            },
          )])
        }
      ),])
    },
    ir,
  );
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
  // Find `fn main` inside the block, and stringify its body (statements only)
  let mut main_code = String::new();
  for stmt in &body.stmts {
    if let syn::Stmt::Item(syn::Item::Fn(f)) = stmt {
      if f.sig.ident == "main" {
        main_code = stringify_block_stmts(&f.block);
        break;
      }
    }
  }
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
          (StepId::new("entry"), Step::RustBlock { binds: vec![], code: main_code, next: StepId::new("return") }),
          (StepId::new("return"), Step::ReturnVoid),
        ],
      },
    )]),
  }
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
