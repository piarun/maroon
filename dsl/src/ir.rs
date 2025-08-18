use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StepId(pub String);

impl StepId {
  pub fn new(id: impl Into<String>) -> Self {
    Self(id.into())
  }
}

#[derive(Debug, Clone)]
pub struct IR {
  pub types: Vec<Type>,
  pub fibers: HashMap<String, Fiber>,
}

#[derive(Debug, Clone)]
pub struct Fiber {
  pub heap: HashMap<String, Type>,
  // input queue messages that fiber accepts
  pub in_messages: Vec<MessageSpec>,
  pub funcs: HashMap<String, Func>,
}

#[derive(Debug, Clone)]
pub struct MessageSpec {
  pub name: String,
  pub fields: Vec<(String, Type)>, // (name, type)
}

#[derive(Debug, Clone)]
pub struct Func {
  pub in_vars: Vec<InVar>,
  pub out: Type,
  pub locals: Vec<LocalVar>,
  pub entry: StepId,
  pub steps: Vec<(StepId, Step)>,
}

#[derive(Debug, Clone)]
pub struct InVar {
  pub name: String,
  pub type_: Type,
}

#[derive(Debug, Clone)]
pub struct LocalVar {
  pub name: String,
  pub type_: Type,
}

#[derive(Debug, Clone)]
pub enum Step {
  Sleep { ms: Expr, next: StepId },
  Write { text: Expr, next: StepId },
  // send a message to a fiber (by name) of a specific kind with arguments, then continue
  // doesn't awaits by default. I think that makes sense?
  // but it can be used with await
  SendToFiber { fiber: String, message: String, args: Vec<Expr>, next: StepId },
  Await { message: String, bind: Option<String>, ret_to: StepId },
  // `ret_to` is the continuation step in the caller
  // bind - local variable into which response will be written
  // THINK: should I get rid of call and alway do it through SendToFiber+Await?
  Call { target: FuncRef, args: Vec<Expr>, bind: Option<String>, ret_to: StepId },
  Return { value: Option<Expr> },
  If { cond: Expr, then_: StepId, else_: StepId },
  Let { local: String, expr: Expr, next: StepId },
}

#[derive(Debug, Clone)]
pub enum Expr {
  Int(u64),
  Str(String),
  Var(String),
  IsSome(Box<Expr>),
  Unwrap(Box<Expr>),
  GetField(Box<Expr>, String),
  StructUpdate { base: Box<Expr>, updates: Vec<(String, Expr)> },
}

#[derive(Debug, Clone)]
pub struct FuncRef {
  pub fiber: String,
  pub func: String,
}

#[derive(Debug, Clone)]
pub enum Type {
  Int,
  String,
  Void,
  Map(Box<Type>, Box<Type>),
  Array(Box<Type>),
  Struct(String, Vec<StructField>),
  Option(Box<Type>),
  // reference to types defined in IR.types
  Custom(String),
}

#[derive(Debug, Clone)]
pub struct StructField {
  pub name: String,
  pub ty: Type,
}
