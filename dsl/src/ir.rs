use common::logical_time::LogicalTimeAbsoluteMs;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StepId(pub String);

impl StepId {
  pub fn new(id: impl Into<String>) -> Self {
    Self(id.into())
  }
}

// Note: Runtime `FutureId` moved into runtime modules.

// IR-only identifier to label futures for awaits/links.
// This is not used by the runtime which works with concrete `FutureId`s.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FutureLabel(pub String);

impl FutureLabel {
  pub fn new(id: impl Into<String>) -> Self {
    Self(id.into())
  }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FiberType(pub String);
impl std::fmt::Display for FiberType {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl FiberType {
  pub fn new(id: impl Into<String>) -> Self {
    Self(id.into())
  }
}

impl std::borrow::Borrow<str> for FiberType {
  fn borrow(&self) -> &str {
    self.0.as_str()
  }
}

#[derive(Debug, Clone)]
pub struct IR {
  pub types: Vec<Type>,
  pub fibers: HashMap<FiberType, Fiber>,
}

#[derive(Debug, Clone)]
pub struct Fiber {
  // Limit of independent runtime fibers that can be created from this IR fiber definition
  pub fibers_limit: u64,
  pub heap: HashMap<String, Type>,
  // input queue messages that fiber accepts
  pub in_messages: Vec<MessageSpec>,
  pub funcs: HashMap<String, Func>,
}

#[derive(Debug, Clone)]
pub struct MessageSpec(pub &'static str, pub Vec<(&'static str, Type)>); // (func_name, [(var_name, type)])

#[derive(Debug, Clone)]
pub struct Func {
  pub in_vars: Vec<InVar>,
  pub out: Type,
  pub locals: Vec<LocalVar>,
  pub entry: StepId,
  pub steps: Vec<(StepId, Step)>,
}

#[derive(Debug, Clone)]
pub struct InVar(pub &'static str, pub Type);

#[derive(Debug, Clone)]
pub struct LocalVar(pub &'static str, pub Type);

#[derive(Debug, Clone)]
pub enum Step {
  ScheduleTimer { ms: LogicalTimeAbsoluteMs, next: StepId, future_id: FutureLabel },
  // send a message to a fiber (by name) of a specific kind with arguments, then continue
  // doesn't awaits by default. I think that makes sense?
  // but it can be used with await
  // args: (name on the incoming side, variable)
  SendToFiber { fiber: String, message: String, args: Vec<(String, Expr)>, next: StepId, future_id: FutureLabel },
  Await(AwaitSpec),
  // `ret_to` is the continuation step in the caller
  // bind - local variable into which response will be written
  // THINK: should I get rid of call and alway do it through SendToFiber+Await?
  Call { target: FuncRef, args: Vec<Expr>, bind: Option<String>, ret_to: StepId },
  Return { value: RetValue },
  ReturnVoid,
  If { cond: Expr, then_: StepId, else_: StepId },
  Let { local: String, expr: Expr, next: StepId },
  // Inline Rust block that can perform any amount of computations.
  //     However we'll be aiming to keep it 'relatively small'.
  //     The block must be pure computational with no side effects.
  // `binds` are the local/param names to write results into (in order).
  // `code` is the Rust body that computes and returns the values.
  //     All function params and locals are available in scope for this block.
  RustBlock { binds: Vec<String>, code: String, next: StepId },
  // TODO: Block with local variables that can look at variables of this function
  // but other parts of the function can't access this block's variables
  // ex: for loop

  // TODO: Builtin step for "library" functions
  // Builtin { opcode: Opcode, args: Vec<Expr>, bind: Option<String>, ret_to: StepId },

  // suspends fiber until a message is available
  // if several messages are available at the same time - runtime will pick the first matching arm
  SelectQueue { arms: Vec<QueueAwaitSpec> },
}

#[derive(Debug, Clone)]
pub enum Opcode {
  SubU64,
}

#[derive(Debug, Clone)]
pub struct AwaitSpec {
  pub bind: Option<String>,
  pub ret_to: StepId,
  pub future_id: FutureLabel,
}

#[derive(Debug, Clone)]
pub struct QueueAwaitSpec {
  // TODO: make queue not name but type?
  // Or just check in validate step:
  // - this queue exists
  // - message type is the same as `message_var` type
  pub queue_name: String,
  /// variable name - where message from the queue will be put
  /// TODO: check types of messages that they match
  pub message_var: String,
  /// next step
  pub next: StepId,
}

#[derive(Debug, Clone)]
pub enum Expr {
  UInt64(u64),
  Str(String),
  Var(String),
  Equal(Box<Expr>, Box<Expr>),
  Greater(Box<Expr>, Box<Expr>),
  Less(Box<Expr>, Box<Expr>),
  IsSome(Box<Expr>),
  Unwrap(Box<Expr>),
  GetField(Box<Expr>, String),
  StructUpdate { base: Box<Expr>, updates: Vec<(String, Expr)> },
}

#[derive(Debug, Clone)]
pub enum RetValue {
  // Return a variable by name
  Var(String),
  // Return a literal
  UInt64(u64),
  Str(String),
  // Return an Option constructor
  Some(Box<RetValue>),
  None,
}

#[derive(Debug, Clone)]
pub struct FuncRef {
  pub fiber: String,
  pub func: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
  UInt64,
  String,
  Void,
  Map(Box<Type>, Box<Type>),
  Array(Box<Type>),
  // struct with user-provided Rust impl (e.g., custom Ord/PartialEq/Eq or other helper functions). The impl code is emitted as-is
  Struct(String, Vec<StructField>, String),
  // ordered priority queues stored in heap. Elements must implement Ord
  MaxQueue(Box<Type>),
  MinQueue(Box<Type>),
  Option(Box<Type>),
  // reference to types defined in IR.types
  Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
  pub name: String,
  pub ty: Type,
}

impl IR {
  pub fn is_valid(&self) -> (bool, String) {
    // TODO: all branches have the same end
    let mut explanation = String::new();
    let mut has_root_fiber = false;
    for fiber in self.fibers.iter() {
      if fiber.0 == &FiberType::new("root") {
        has_root_fiber = true
      }
      let mut has_main_function = false;
      for func in fiber.1.funcs.iter() {
        let mut has_entry = false; // each function should start with 'entry' stepId
        for step in func.1.steps.iter() {
          if step.0 == StepId::new("entry") {
            has_entry = true;
            break;
          }
        }
        if *func.0 == "main".to_string() {
          has_main_function = true;

          if func.1.out != Type::Void {
            explanation.push_str(&format!("{} main function can only return void", fiber.0));
          }
        }
        if !has_entry {
          explanation.push_str(&format!("{}.{} doesnt have step 'entry'\n", fiber.0, func.0));
        }
      }
      if !has_main_function {
        explanation.push_str(&format!("{} doesnt have 'main' function\n", fiber.0));
      }
    }

    if !has_root_fiber {
      explanation.push_str("no 'root' fiber\n");
    }
    (explanation.len() == 0, explanation)
  }
}
