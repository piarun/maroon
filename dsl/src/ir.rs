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

/// IR-only identifier to label futures for awaits/links.
/// This is not used by the runtime which works with concrete `FutureId`s.
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
  /// Limit of independent runtime fibers that can be created from this IR fiber definition
  pub fibers_limit: u64,
  pub heap: HashMap<String, Type>,
  /// input queue messages that fiber accepts
  pub in_messages: Vec<MessageSpec>,
  pub init_vars: Vec<InVar>,

  pub funcs: HashMap<String, Func>,
}

#[derive(Debug, Clone)]
pub struct MessageSpec(pub &'static str, pub Vec<(&'static str, Type)>); // (func_name, [(var_name, type)])

#[derive(Debug, Clone)]
pub struct Func {
  pub in_vars: Vec<InVar>,
  pub out: Type,
  pub locals: Vec<LocalVar>,
  pub steps: Vec<(StepId, Step)>,
}

#[derive(Debug, Clone)]
pub struct InVar(pub &'static str, pub Type);

#[derive(Debug, Clone)]
pub struct LocalVar(pub &'static str, pub Type);

/// this reference should be used in ir specification where I want to reference LocalVar existed in the current stack frame
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalVarRef(pub &'static str);

#[derive(Debug, Clone)]
pub enum Step {
  ScheduleTimer {
    ms: LogicalTimeAbsoluteMs,
    next: StepId,
    future_id: FutureLabel,
  },
  /// send a message to a fiber (by name) of a specific kind with arguments, then continue
  /// doesn't awaits by default. I think that makes sense?
  /// but it can be used with await
  /// args: (name on the incoming side, variable)
  SendToFiber {
    fiber: String,
    message: String,
    args: Vec<(String, Expr)>,
    next: StepId,
    future_id: FutureLabel,
  },
  Await(AwaitSpec),
  /// `ret_to` is the continuation step in the caller
  /// bind - local variable into which response will be written
  /// THINK: should I get rid of call and alway do it through SendToFiber+Await?
  Call {
    target: FuncRef,
    args: Vec<Expr>,
    bind: Option<LocalVarRef>,
    ret_to: StepId,
  },
  Return {
    value: RetValue,
  },
  ReturnVoid,
  If {
    cond: Expr,
    then_: StepId,
    else_: StepId,
  },
  Let {
    local: String,
    expr: Expr,
    next: StepId,
  },
  /// Inline Rust block that can perform any amount of computations.
  ///     However we'll be aiming to keep it 'relatively small'.
  ///     The block must be pure computational with no side effects.
  /// `binds` are the local/param names to write results into (in order).
  /// `code` is the Rust body that computes and returns the values.
  ///     All function params and locals are available in scope for this block.
  RustBlock {
    binds: Vec<LocalVarRef>,
    code: String,
    next: StepId,
  },
  /// TODO: Block with local variables that can look at variables of this function
  /// but other parts of the function can't access this block's variables
  /// ex: for loop

  /// TODO: Builtin step for "library" functions
  /// Builtin { opcode: Opcode, args: Vec<Expr>, bind: Option<String>, ret_to: StepId },

  /// suspends fiber until a message is available
  /// if several messages are available at the same time - runtime will pick the first matching arm
  Select {
    arms: Vec<AwaitSpec>,
  },

  /// set values to async primitives: queue, future, ...?
  /// doesnt stop Fiber from execution
  SetValues {
    values: Vec<SetPrimitive>,
    next: StepId,
  },

  /// DEBUG section
  /// Prints smth to dbgOut

  /// Prints debug string and then continues to `next` step.
  Debug(&'static str, StepId),
  /// Prints all vars (in the current stack frame) values in the order of
  /// definition in the function, then continues to `next` step.
  DebugPrintVars(StepId),
}

#[derive(Debug, Clone)]
pub enum SetPrimitive {
  QueueMessage {
    /// `f_var_queue_name` - variable where queue name is located
    /// - the one that should be updated with the new value
    f_var_queue_name: LocalVarRef,
    /// ref to a variable from which value should be copied and sent to the queue
    var_name: LocalVarRef,
  },

  Future {
    /// `f_var_name` - variable where future id is located
    /// - the one that should be updated with the new value
    f_var_name: LocalVarRef,
    /// ref to a local variable from which value should be copied and set to Future
    var_name: LocalVarRef,
  },
}

#[derive(Debug, Clone)]
pub enum Opcode {
  SubU64,
}

#[derive(Debug, Clone)]
pub enum AwaitSpec {
  Future {
    bind: Option<LocalVarRef>,
    ret_to: StepId,
    future_id: FutureLabel,
  },
  Queue {
    /// TODO: make queue not name but type?
    /// Or just check in validate step:
    /// - this queue exists
    /// - message type is the same as `message_var` type
    queue_name: String,
    /// variable name - where message from the queue will be put
    /// TODO: check types of messages that they match
    message_var: LocalVarRef,
    /// next step after await is resolved in this arm
    next: StepId,
  },
}

#[derive(Debug, Clone)]
pub enum Expr {
  UInt64(u64),
  Str(String),
  Var(LocalVarRef),
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
  /// Return a variable by name
  Var(LocalVarRef),
  /// Return a literal
  UInt64(u64),
  Str(String),
  /// Return an Option constructor
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
  /// struct with user-provided Rust impl (e.g., custom Ord/PartialEq/Eq or other helper functions). The impl code is emitted as-is
  Struct(String, Vec<StructField>, String),
  /// ordered priority queues stored in heap. Elements must implement Ord
  MaxQueue(Box<Type>),
  MinQueue(Box<Type>),
  Option(Box<Type>),
  /// reference to types defined in IR.types
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

        if let Some(correct) = uses_correct_variables(func.1) {
          explanation.push_str(&correct);
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

fn uses_correct_variables(f: &Func) -> Option<String> {
  let mut explanation = String::new();
  let mut vars_map = HashMap::<String, Type>::new();
  // include parameters as in-scope vars
  for p in &f.in_vars {
    if let Some(previous) = vars_map.insert(p.0.to_string(), p.1.clone()) {
      explanation.push_str(&format!("duplicate var name {} for types: {:?} and {:?}\n", p.0, p.1, previous));
    }
  }
  for v in &f.locals {
    if let Some(previous) = vars_map.insert(v.0.to_string(), v.1.clone()) {
      explanation.push_str(&format!("duplicate var name {} for types: {:?} and {:?}\n", v.0, v.1, previous));
    }
  }

  for (id, step) in &f.steps {
    match step {
      Step::Debug(_, _) => {}
      Step::DebugPrintVars(_) => {}
      Step::ScheduleTimer { .. } => {}
      Step::SendToFiber { args, .. } => {
        for (_name, expr) in args {
          collect_vars_from_expr(expr, &vars_map, &mut explanation, id);
        }
      }
      Step::Await(spec) => match spec {
        AwaitSpec::Future { bind, ret_to: _, future_id: _ } => {
          if let Some(var_ref) = bind {
            if !vars_map.contains_key(var_ref.0) {
              explanation.push_str(&format!("{:?} references {} that is not defined\n", id, var_ref.0));
            }
          }
        }
        AwaitSpec::Queue { queue_name: _, message_var, next: _ } => {
          if !vars_map.contains_key(message_var.0) {
            explanation.push_str(&format!("{:?} references {} that is not defined\n", id, message_var.0));
          }
        }
      },
      Step::Call { args, bind, .. } => {
        for expr in args {
          collect_vars_from_expr(expr, &vars_map, &mut explanation, id);
        }
        if let Some(LocalVarRef(name)) = bind {
          if !vars_map.contains_key::<str>(name) {
            explanation.push_str(&format!("{:?} references {} that is not defined\n", id, name));
          }
        }
      }
      Step::Return { value } => {
        collect_vars_from_ret_value(value, &vars_map, &mut explanation, id);
      }
      Step::ReturnVoid => {}
      Step::If { cond, .. } => {
        collect_vars_from_expr(cond, &vars_map, &mut explanation, id);
      }
      Step::Let { local, expr, .. } => {
        if !vars_map.contains_key::<str>(local.as_str()) {
          explanation.push_str(&format!("{:?} references {} that is not defined\n", id, local));
        }
        collect_vars_from_expr(expr, &vars_map, &mut explanation, id);
      }
      Step::RustBlock { binds, .. } => {
        for b in binds {
          if !vars_map.contains_key::<str>(b.0) {
            explanation.push_str(&format!("{:?} references {} that is not defined\n", id, b.0));
          }
        }
      }
      Step::Select { arms } => {
        for arm in arms {
          match arm {
            AwaitSpec::Future { bind, .. } => {
              if let Some(var_ref) = bind {
                if !vars_map.contains_key::<str>(var_ref.0) {
                  explanation.push_str(&format!("{:?} references {} that is not defined\n", id, var_ref.0));
                }
              }
            }
            AwaitSpec::Queue { message_var, .. } => {
              if !vars_map.contains_key::<str>(message_var.0) {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, message_var.0));
              }
            }
          }
        }
      }
      Step::SetValues { values, .. } => {
        for v in values {
          match v {
            SetPrimitive::QueueMessage { f_var_queue_name, var_name } => {
              if !vars_map.contains_key::<str>(f_var_queue_name.0) {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, f_var_queue_name.0));
              }
              if !vars_map.contains_key::<str>(var_name.0) {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, var_name.0));
              }
            }
            SetPrimitive::Future { f_var_name, var_name } => {
              if !vars_map.contains_key::<str>(f_var_name.0) {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, f_var_name.0));
              }
              if !vars_map.contains_key::<str>(var_name.0) {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, var_name.0));
              }
            }
          }
        }
      }
    }
  }

  if explanation.len() > 0 {
    return Some(explanation);
  }

  return None;
}

fn collect_vars_from_expr(
  expr: &Expr,
  vars: &HashMap<String, Type>,
  explanation: &mut String,
  id: &StepId,
) {
  match expr {
    Expr::Var(LocalVarRef(name)) => {
      if !vars.contains_key::<str>(*name) {
        explanation.push_str(&format!("{:?} references {} that is not defined\n", id, name));
      }
    }
    Expr::Equal(a, b) | Expr::Greater(a, b) | Expr::Less(a, b) => {
      collect_vars_from_expr(a, vars, explanation, id);
      collect_vars_from_expr(b, vars, explanation, id);
    }
    Expr::IsSome(inner) | Expr::Unwrap(inner) => collect_vars_from_expr(inner, vars, explanation, id),
    Expr::GetField(base, _field) => collect_vars_from_expr(base, vars, explanation, id),
    Expr::StructUpdate { base, updates } => {
      collect_vars_from_expr(base, vars, explanation, id);
      for (_fname, e) in updates {
        collect_vars_from_expr(e, vars, explanation, id);
      }
    }
    Expr::UInt64(_) | Expr::Str(_) => {}
  }
}

fn collect_vars_from_ret_value(
  rv: &RetValue,
  vars: &HashMap<String, Type>,
  explanation: &mut String,
  id: &StepId,
) {
  match rv {
    RetValue::Var(LocalVarRef(name)) => {
      if !vars.contains_key::<str>(*name) {
        explanation.push_str(&format!("{:?} references {} that is not defined\n", id, name));
      }
    }
    RetValue::Some(inner) => collect_vars_from_ret_value(inner, vars, explanation, id),
    RetValue::UInt64(_) | RetValue::Str(_) | RetValue::None => {}
  }
}
