pub mod ast;
pub mod ir;
pub mod parser;
pub mod codegen;

#[cfg(test)]
mod ir_test;
#[cfg(test)]
mod parsed_ast_test;
