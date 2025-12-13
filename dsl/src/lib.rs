pub mod ast;
pub mod codegen;
pub mod ir;
pub mod parser;

mod dsl_to_ir;
#[cfg(test)]
mod ir_test;
#[cfg(test)]
mod parsed_ast_test;
