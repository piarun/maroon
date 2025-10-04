mod fiber;
pub mod ir_spec;
// Re-export IR types so generated code can refer to `crate::ir::...`.
pub use dsl::ir;
#[cfg(test)]
mod generated_test;
#[cfg(test)]
mod ir_test;
pub mod runtime;
