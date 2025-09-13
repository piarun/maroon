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
mod simple_f_ir_spec {
  include!("src/simple_function/ir.rs");
}

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
  // Invalidate build when inputs change.
  println!("cargo:rerun-if-changed=src/ir.rs");
  println!("cargo:rerun-if-changed=src/codegen.rs");

  let to_gen = vec![(simple_f_ir_spec::sample_ir(), "simple_function")];

  for info in &to_gen {
    println!("cargo:rerun-if-changed=src/{}/ir.rs", info.1);
    println!("cargo:rerun-if-changed=src/{}/generated.rs", info.1);
  }

  for info in &to_gen {
    // Build IR and generate code.
    let code = codegen::generate_rust_types(&info.0);

    // Write into the crate source tree for later compilation.
    let mut out_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    out_file.push(format!("src/{}/generated.rs", info.1));
    if let Some(parent) = out_file.parent() {
      let _ = fs::create_dir_all(parent);
    }
    fs::write(&out_file, code).expect("write generated types");

    // Best-effort formatting; ignore failure if rustfmt isn't installed.
    let _ = Command::new("rustfmt").arg(&out_file).status();
  }
}
