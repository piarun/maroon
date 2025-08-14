use crate::{
  ast::{
    BinOp, Block, Expr, Function, Item, Mutability, Param, Program, Statement, StructDef, StructField, StructLitField,
    TypeName, VarDecl,
  },
  parser,
};

#[test]
fn test_parse_program() {
  let input = r#"
        fn factorial(n: i32) -> i32 {
            if n <= 1 {
                return 1
            }
            return n * factorial(n - 1)
        }

        fn multiply(a: i32, b: i32) -> i32 {
          return a * b
        }

        fn noReturnFunction(a: i32) {
          print(a)
        }

        // calls and expression assignment
        let six: i32 = multiply(2,3)
        let result: i32 = factorial(five)

        struct Account {
          id: String,
          amount: i64,
        }

        struct User {
            email: String,
            age: i32,
            bank_account: Account,
            some_array: []i32,
            some_map: map[i32]String
        }

        let users: []User
        var index: map[String]User = {}
    "#;

  let program = parser::parse_program(input);
  println!("{:#?}", program);
  assert!(program.is_ok());

  let expected = Program {
    items: vec![
      Item::Function(Function {
        name: "factorial".to_string(),
        params: vec![Param { name: "n".to_string(), ty: TypeName::I32 }],
        ret: TypeName::I32,
        body: Block {
          statements: vec![
            Statement::If {
              cond: Expr::Binary {
                left: Box::new(Expr::Ident("n".to_string())),
                op: BinOp::Le,
                right: Box::new(Expr::Int(1)),
              },
              then_blk: Block { statements: vec![Statement::Return(Expr::Int(1))] },
              else_blk: None,
            },
            Statement::Return(Expr::Binary {
              left: Box::new(Expr::Ident("n".to_string())),
              op: BinOp::Mul,
              right: Box::new(Expr::Call {
                name: "factorial".to_string(),
                args: vec![Expr::Binary {
                  left: Box::new(Expr::Ident("n".to_string())),
                  op: BinOp::Sub,
                  right: Box::new(Expr::Int(1)),
                }],
              }),
            }),
          ],
        },
      }),
      Item::Function(Function {
        name: "multiply".to_string(),
        params: vec![
          Param { name: "a".to_string(), ty: TypeName::I32 },
          Param { name: "b".to_string(), ty: TypeName::I32 },
        ],
        ret: TypeName::I32,
        body: Block {
          statements: vec![Statement::Return(Expr::Binary {
            left: Box::new(Expr::Ident("a".to_string())),
            op: BinOp::Mul,
            right: Box::new(Expr::Ident("b".to_string())),
          })],
        },
      }),
      Item::Function(Function {
        name: "noReturnFunction".to_string(),
        params: vec![Param { name: "a".to_string(), ty: TypeName::I32 }],
        ret: TypeName::Void,
        body: Block {
          statements: vec![Statement::Expr(Expr::Call {
            name: "print".to_string(),
            args: vec![Expr::Ident("a".to_string())],
          })],
        },
      }),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "six".to_string(),
        ty: TypeName::I32,
        init: Some(Expr::Call { name: "multiply".to_string(), args: vec![Expr::Int(2), Expr::Int(3)] }),
      })),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "result".to_string(),
        ty: TypeName::I32,
        init: Some(Expr::Call { name: "factorial".to_string(), args: vec![Expr::Ident("five".to_string())] }),
      })),
      Item::Struct(StructDef {
        name: "Account".to_string(),
        fields: vec![
          StructField { name: "id".to_string(), ty: TypeName::StringTy },
          StructField { name: "amount".to_string(), ty: TypeName::I64 },
        ],
      }),
      Item::Struct(StructDef {
        name: "User".to_string(),
        fields: vec![
          StructField { name: "email".to_string(), ty: TypeName::StringTy },
          StructField { name: "age".to_string(), ty: TypeName::I32 },
          StructField { name: "bank_account".to_string(), ty: TypeName::Custom("Account".to_string()) },
          StructField { name: "some_array".to_string(), ty: TypeName::Array(Box::new(TypeName::I32)) },
          StructField {
            name: "some_map".to_string(),
            ty: TypeName::Map(Box::new(TypeName::I32), Box::new(TypeName::StringTy)),
          },
        ],
      }),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "users".to_string(),
        ty: TypeName::Array(Box::new(TypeName::Custom("User".to_string()))),
        init: None,
      })),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Mutable,
        name: "index".to_string(),
        ty: TypeName::Map(Box::new(TypeName::StringTy), Box::new(TypeName::Custom("User".to_string()))),
        init: Some(Expr::MapLit(vec![])),
      })),
    ],
  };

  assert_eq!(expected, program.unwrap())
}

#[test]
fn test_expression_in_statement() {
  let input = r#"
        let s: i32 = 10
        if s == 5 {
          let a: i32 = 10 + s
          print(a)
        } else {
          print(4)
        }
    "#;

  let program = parser::parse_program(input);
  println!("{:#?}", program);
  assert!(program.is_ok());

  let expected = Program {
    items: vec![
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "s".to_string(),
        ty: TypeName::I32,
        init: Some(Expr::Int(10)),
      })),
      Item::Statement(Statement::If {
        cond: Expr::Binary {
          left: Box::new(Expr::Ident("s".to_string())),
          op: BinOp::Eq,
          right: Box::new(Expr::Int(5)),
        },
        then_blk: Block {
          statements: vec![
            Statement::VarDecl(VarDecl {
              mutability: Mutability::Immutable,
              name: "a".to_string(),
              ty: TypeName::I32,
              init: Some(Expr::Binary {
                left: Box::new(Expr::Int(10)),
                op: BinOp::Add,
                right: Box::new(Expr::Ident("s".to_string())),
              }),
            }),
            Statement::Expr(Expr::Call { name: "print".to_string(), args: vec![Expr::Ident("a".to_string())] }),
          ],
        },
        else_blk: Some(Block {
          statements: vec![Statement::Expr(Expr::Call { name: "print".to_string(), args: vec![Expr::Int(4)] })],
        }),
      }),
    ],
  };
  assert_eq!(expected, program.unwrap())
}

#[test]
fn test_function_without_return_type() {
  let input = r#"
        fn print_hello() {
            print("Hello")
        }
    "#;

  let program = parser::parse_program(input);
  println!("{:#?}", program);
  assert!(program.is_ok());

  let expected = Program {
    items: vec![Item::Function(Function {
      name: "print_hello".to_string(),
      params: vec![],
      ret: TypeName::Void,
      body: Block {
        statements: vec![Statement::Expr(Expr::Call {
          name: "print".to_string(),
          args: vec![Expr::Str("Hello".to_string())],
        })],
      },
    })],
  };

  assert_eq!(expected, program.unwrap())
}

#[test]
fn test_function_struct_construction() {
  let input = r#"
        struct User {
          id: String,
          email: String,
          age: i32,
        }

        let id: String = "123"
        let email: String = "test@test.com"
        let age: i32 = 10

        let my_user: User = User { id: id, email: email, age: age }
        // map construction still work
        let my_map: map[String]i32 = { "key": 42 }
    "#;

  let program = parser::parse_program(input);
  println!("{:#?}", program);
  assert!(program.is_ok());

  let expected = Program {
    items: vec![
      Item::Struct(StructDef {
        name: "User".to_string(),
        fields: vec![
          StructField { name: "id".to_string(), ty: TypeName::StringTy },
          StructField { name: "email".to_string(), ty: TypeName::StringTy },
          StructField { name: "age".to_string(), ty: TypeName::I32 },
        ],
      }),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "id".to_string(),
        ty: TypeName::StringTy,
        init: Some(Expr::Str("123".to_string())),
      })),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "email".to_string(),
        ty: TypeName::StringTy,
        init: Some(Expr::Str("test@test.com".to_string())),
      })),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "age".to_string(),
        ty: TypeName::I32,
        init: Some(Expr::Int(10)),
      })),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "my_user".to_string(),
        ty: TypeName::Custom("User".to_string()),
        init: Some(Expr::StructLit {
          name: "User".to_string(),
          fields: vec![
            StructLitField { name: "id".to_string(), value: Expr::Ident("id".to_string()) },
            StructLitField { name: "email".to_string(), value: Expr::Ident("email".to_string()) },
            StructLitField { name: "age".to_string(), value: Expr::Ident("age".to_string()) },
          ],
        }),
      })),
      Item::Statement(Statement::VarDecl(VarDecl {
        mutability: Mutability::Immutable,
        name: "my_map".to_string(),
        ty: TypeName::Map(Box::new(TypeName::StringTy), Box::new(TypeName::I32)),
        init: Some(Expr::MapLit(vec![(Expr::Str("key".to_string()), Expr::Int(42))])),
      })),
    ],
  };

  assert_eq!(expected, program.unwrap())
}
