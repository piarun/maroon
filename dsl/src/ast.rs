#[derive(Debug, Clone, PartialEq)]
pub struct Program {
  pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
  Struct(StructDef),
  Function(Function),
  Statement(Statement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
  pub name: String,
  pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
  pub name: String,
  pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
  pub name: String,
  pub params: Vec<Param>,
  pub ret: TypeName,
  pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
  pub name: String,
  pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
  pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
  VarDecl(VarDecl),
  Return(Expr),
  If { cond: Expr, then_blk: Block, else_blk: Option<Block> },
  Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
  pub mutability: Mutability,
  pub name: String,
  pub ty: TypeName,
  pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mutability {
  Immutable,
  Mutable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeName {
  StringTy,
  I32,
  I64,
  Void, // For functions that don't return anything
  Array(Box<TypeName>),
  Map(Box<TypeName>, Box<TypeName>),
  Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
  Int(i64),
  Str(String),
  Ident(String),
  ArrayLit(Vec<Expr>),
  MapLit(Vec<(Expr, Expr)>),
  StructLit { name: String, fields: Vec<StructLitField> },
  Call { name: String, args: Vec<Expr> },
  SyncCall { name: String, args: Vec<Expr> },
  Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLitField {
  pub name: String,
  pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
  Add,
  Sub,
  Mul,
  Div,
  Eq,
  Ne,
  Gt,
  Lt,
  Ge,
  Le,
}
