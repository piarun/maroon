use std::{collections::HashMap, fmt::Error};

use crate::ir::IR;

#[test]
fn minimal_dsl_test() {
  let ir = transpile_dsl_to_ir("./dsl.rs").expect("compilable");
  assert_eq!(IR { types: vec![], fibers: HashMap::new() }, ir);
}

pub fn transpile_dsl_to_ir(file_path: &str) -> Result<IR, Error> {
  Result::Ok(IR { types: vec![], fibers: HashMap::new() })
}
