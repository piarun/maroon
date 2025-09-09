use crate::ir::*;
use crate::simple_function::ir::sample_ir;

#[test]
fn simple_ir() {
  let ir: IR = sample_ir();
  let (valid, explanation) = ir.is_valid();
  assert!(valid, "{explanation}");
}

