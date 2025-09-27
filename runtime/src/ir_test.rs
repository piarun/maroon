use crate::ir_spec::sample_ir;
use dsl::ir::*;

#[test]
fn simple_ir() {
  let ir: IR = sample_ir();
  let (valid, explanation) = ir.is_valid();
  assert!(valid, "{explanation}");
}
