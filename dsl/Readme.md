# DSL

at some point there will be some language that engineers can use, but for now work on DSL is abandoned and we're concentrating an effort on IR(Intermediate Representation) for Maroon step-executable kind of language that is easier to compile into Maroon Assembler

# IR

- Rust based
- Easy [compileable](./src/codegen.rs) into `maroon assembler`
- Featureful enough to write any program
- Async/Await