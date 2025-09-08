// Build script for the `dsl` crate.
// Generates Rust code from the IR into src/generated_types.rs so the library can include it.

// Reuse the crate's IR and codegen implementations by text-including them here.
// This avoids circular dependencies while keeping a single source of truth.
mod ir {
  include!("src/ir.rs");
}
mod codegen {
  include!("src/codegen.rs");
}
mod ir_spec {
  include!("src/simple_functions_ir.rs");
}

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
  // Invalidate build when inputs change.
  println!("cargo:rerun-if-changed=src/ir.rs");
  println!("cargo:rerun-if-changed=src/codegen.rs");
  println!("cargo:rerun-if-changed=src/ir_spec.rs");
  println!("cargo:rerun-if-changed=src/simple_functions_generated.rs");

  // Build IR and generate code.
  let ir = ir_spec::sample_ir();
  let code = codegen::generate_rust_types(&ir);

  // Write into the crate source tree for later compilation.
  let mut out_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"));
  out_file.push("src/simple_functions_generated.rs");
  if let Some(parent) = out_file.parent() {
    let _ = fs::create_dir_all(parent);
  }
  fs::write(&out_file, code).expect("write generated types");

  // Best-effort formatting; ignore failure if rustfmt isn't installed.
  let _ = Command::new("rustfmt").arg(&out_file).status();
}
