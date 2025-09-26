// Build script for generating runtime code from IR.
// Use the real `dsl` crate as a build-dependency for IR and codegen.
// This avoids path/include complexity and keeps types consistent.
use dsl as _dsl_crate;

mod simple_f_ir_spec {
  include!("src/ir_spec.rs");
}

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
  // Invalidate build when inputs change.
  println!("cargo:rerun-if-changed=../dsl/src/ir.rs");
  println!("cargo:rerun-if-changed=../dsl/src/codegen.rs");
  println!("cargo:rerun-if-changed=src/ir_spec.rs");
  println!("cargo:rerun-if-changed=src/generated.rs");

  // Build IR and generate code.
  let code = _dsl_crate::codegen::generate_rust_types(&simple_f_ir_spec::sample_ir());

  // Write into the crate source tree for later compilation.
  let mut out_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"));
  out_file.push("src/generated.rs");
  if let Some(parent) = out_file.parent() {
    let _ = fs::create_dir_all(parent);
  }
  fs::write(&out_file, code).expect("write generated types");

  // Best-effort formatting; ignore failure if rustfmt isn't installed.
  let _ = Command::new("rustfmt").arg(&out_file).status();
}
