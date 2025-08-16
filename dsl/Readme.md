The `dsl` crate defines a small language to describe agent/workflow logic and turns it into an explicit state machine model. 

- DSL language grammar: grammar.pest
- AST types: ast.rs
- Pest-based parser: parser.rs
- Analysis/state extraction: state_generator.rs
