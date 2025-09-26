mod fiber;
mod generated;
mod ir_spec;
// Re-export IR types so generated code can refer to `crate::ir::...`.
pub use dsl::ir;
pub mod runtime;
pub mod runtime_timer;

#[cfg(test)]
mod generated_test;
#[cfg(test)]
mod ir_test;
