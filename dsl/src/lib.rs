pub mod ast;
pub mod codegen;
pub mod ir;
pub mod parser;

mod generated_types;

#[cfg(test)]
mod generated_test;
#[cfg(test)]
mod ir_test;
#[cfg(test)]
mod parsed_ast_test;
