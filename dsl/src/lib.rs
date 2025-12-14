pub mod ast;
pub mod codegen;
pub mod ir;
pub mod parser;

// The sketch file contains exploratory DSL code that is not yet valid Rust.
// Keep it out of the default build until macros and semantics are wired in.
// #[cfg(feature = "dsl_sketch")]
mod dsl;
mod dsl_to_ir;
#[cfg(test)]
mod ir_test;
#[cfg(test)]
mod parsed_ast_test;
