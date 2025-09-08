pub mod ast;
pub mod codegen;
pub mod ir;
pub mod parser;

mod simple_functions_generated;
pub use simple_functions_generated::*;

pub mod simple_functions_ir;

#[cfg(test)]
mod parsed_ast_test;
#[cfg(test)]
mod simple_functions_generated_test;
#[cfg(test)]
mod simple_functions_ir_test;
