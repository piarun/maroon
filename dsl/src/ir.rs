use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StepId(pub String);

impl StepId {
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
  pub heap: HashMap<String, Type>,
  pub init_vars: Vec<InVar>,

  pub funcs: HashMap<String, Func>,
}

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
  /// `ret_to` is the continuation step in the caller
  /// bind - local variable into which response will be written
  /// TODO: allow it only for in-fiber calls. Cross-fiber calls - through queues
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

  /// creates runtime primitives atomically(all or none)
  /// goes to success if all primitives are created
  /// goes to fail if at least one primitive can't be created
  Create {
    primitives: Vec<RuntimePrimitive>,
    success: SuccessCreateBranch,
    fail: FailCreateBranch,
  },

  /// Spawns one or more new fibers and continues immediately
  ///
  /// Semantics:
  /// - Each entry in `details` starts a new fiber of the given type at its `main` function
  ///   with the provided `init_vars`
  /// - There is no success/failure branching; creation is best-effort and non-blocking
  CreateFibers {
    details: Vec<CreateFiberDetail>,
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
pub struct CreateFiberDetail {
  /// Target fiber type name as declared in `IR.fibers`
  /// Must match an existing fiber key. The new fiber starts at its `main` function
  pub f_name: FiberType,

  /// Arguments for the spawned fiber's `main` function
  /// These are positional and must match the target fiber's `init_vars`
  /// in length, types and order.
  pub init_vars: Vec<LocalVarRef>,
}

#[derive(Debug, Clone)]
pub struct SuccessCreateBranch {
  /// where to go in case of success
  pub next: StepId,
  /// ids of created primitives will be put here in the same order as requested
  /// LocalVars should have type String for queues and Type::Future for futures
  pub id_binds: Vec<LocalVarRef>,
}

#[derive(Debug, Clone)]
pub struct FailCreateBranch {
  /// where to go in case of failure
  pub next: StepId,
  /// errors for creating primitives will be listed here in the same order as requested
  /// error_binds should bind to Option<String> local variable
  /// None - if no error for the given primitive
  pub error_binds: Vec<LocalVarRef>,
}

#[derive(Debug, Clone)]
pub enum RuntimePrimitive {
  Future,
  /// `name` should be unique and should reference LocalVar typed as String
  /// if `public` == true - new messages can come not from other fibers but from gateways as well
  /// TODO: provide message types here as well so I can do some transpile and compile-time checks on types
  Queue {
    name: LocalVarRef,
    public: bool,
  },
  /// creates a future that will be resolved after provided amount of milliseconds
  Schedule {
    ms_var: LocalVarRef,
  },
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
    /// variable ref where queue id is located
    future_id: LocalVarRef,
  },
  Queue {
    /// variable ref where queue name is located
    queue_name: LocalVarRef,
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
  Bool(bool),
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
  Bool(bool),
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
  Bool,
  Void,
  Map(Box<Type>, Box<Type>),
  Array(Box<Type>),
  /// struct with user-provided Rust impl (e.g., custom Ord/PartialEq/Eq or other helper functions). The impl code is emitted as-is
  Struct(String, Vec<StructField>, String),
  /// ordered priority queues stored in heap. Elements must implement Ord
  MaxQueue(Box<Type>),
  MinQueue(Box<Type>),
  Option(Box<Type>),
  /// Typed future handle. Represents a runtime future id associated with a payload of type T
  Future(Box<Type>),
  /// reference to types defined in IR.types
  Custom(String),
  /// Same as struct but public queues must create messages only via this construction
  /// because it adds up some runtime things
  /// but this type can be used by any other piece of code with no issues
  PubQueueMessage {
    name: String,
    fields: Vec<StructField>,
    /// some rust code that you can put here and it will be translated as is
    /// this approach sounds not great IMHO, but I think it will be resolved one or another way
    /// when we come to DSL implementation
    rust_additions: String,
  },
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

        if let Some(correct) = uses_correct_variables(self, fiber.1, func.1) {
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
    // Validate PubQueueMessage types contain mandatory `public_future_id: String` field
    for t in &self.types {
      if let Type::PubQueueMessage { name, fields, .. } = t {
        let mut found = false;
        for f in fields {
          if f.name == "public_future_id" {
            // Accept String (legacy) or Future<_>
            if f.ty == Type::String {
              found = true;
              break;
            }
            if let Type::Future(_) = f.ty {
              found = true;
              break;
            }
          }
        }
        if !found {
          explanation.push_str(&format!(
            "PubQueueMessage '{}' must include field public_future_id: String or Future<T>\n",
            name
          ));
        }
      }
    }

    (explanation.len() == 0, explanation)
  }
}

fn uses_correct_variables(
  ir: &IR,
  fiber: &Fiber,
  f: &Func,
) -> Option<String> {
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
  // include fiber init_vars as in-scope vars for all functions within the fiber
  for InVar(name, ty) in &fiber.init_vars {
    // do not treat shadowing as error here; last-in wins
    vars_map.entry(name.to_string()).or_insert_with(|| ty.clone());
  }

  for (id, step) in &f.steps {
    match step {
      Step::Debug(_, _) => {}
      Step::DebugPrintVars(_) => {}
      Step::CreateFibers { details, .. } => {
        for d in details {
          // Fiber must exist
          if let Some(target_fiber) = ir.fibers.get(d.f_name.0.as_str()) {
            // init_vars arity must match
            if d.init_vars.len() != target_fiber.init_vars.len() {
              explanation.push_str(&format!(
                "{:?} CreateFibers '{}' expects {} init vars, got {}\n",
                id,
                d.f_name,
                target_fiber.init_vars.len(),
                d.init_vars.len()
              ));
            }
            // type-check positional init vars
            for (idx, var_ref) in d.init_vars.iter().enumerate() {
              if let Some(t) = vars_map.get(var_ref.0) {
                if let Some(InVar(_pname, pty)) = target_fiber.init_vars.get(idx) {
                  if t != pty {
                    explanation.push_str(&format!(
                      "{:?} CreateFibers '{}' arg {} type mismatch: expected {:?}, got {:?}\n",
                      id, d.f_name, idx, pty, t
                    ));
                  }
                }
              } else {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, var_ref.0));
              }
            }
          } else {
            explanation.push_str(&format!("{:?} CreateFibers references unknown fiber '{}'\n", id, d.f_name));
          }
        }
      }
      Step::Call { target, args, bind, .. } => {
        for expr in args {
          collect_vars_from_expr(expr, &vars_map, &mut explanation, id);
        }
        // Type-check against callee signature
        if let Some(callee) = ir.fibers.get(target.fiber.as_str()).and_then(|ff| ff.funcs.get(target.func.as_str())) {
          if callee.in_vars.len() != args.len() {
            explanation.push_str(&format!(
              "{:?} call {}.{} expects {} args, got {}\n",
              id,
              target.fiber,
              target.func,
              callee.in_vars.len(),
              args.len()
            ));
          }
          for (idx, aexpr) in args.iter().enumerate() {
            if let Some(InVar(pname, pty)) = callee.in_vars.get(idx) {
              if let Some(at) = infer_expr_type(ir, f, aexpr, &vars_map) {
                if &at != pty {
                  explanation.push_str(&format!(
                    "{:?} call arg '{}' type mismatch: expected {:?}, got {:?}\n",
                    id, pname, pty, at
                  ));
                }
              }
            }
          }
          if let Some(LocalVarRef(name)) = bind {
            if let Some(bt) = vars_map.get::<str>(name) {
              if &callee.out != bt {
                explanation.push_str(&format!(
                  "{:?} bind '{}' type mismatch: expected {:?}, got {:?}\n",
                  id, name, callee.out, bt
                ));
              }
            }
          }
        }
        if let Some(LocalVarRef(name)) = bind {
          if !vars_map.contains_key::<str>(name) {
            explanation.push_str(&format!("{:?} references {} that is not defined\n", id, name));
          }
        }
      }
      Step::Return { value } => {
        collect_vars_from_ret_value(value, &vars_map, &mut explanation, id);
        let inferred = infer_ret_value_type(ir, f, value, &vars_map);
        if !return_type_compatible(&f.out, value, inferred.as_ref()) {
          if let Some(rty) = inferred {
            explanation
              .push_str(&format!("{:?} return type mismatch: function returns {:?}, got {:?}\n", id, f.out, rty));
          } else {
            explanation.push_str(&format!("{:?} return type mismatch: function returns {:?}\n", id, f.out));
          }
        }
      }
      Step::ReturnVoid => {}
      Step::If { cond, .. } => {
        collect_vars_from_expr(cond, &vars_map, &mut explanation, id);
        // best-effort checks: if we can infer the cond type it must be Bool
        if let Some(cty) = infer_expr_type(ir, f, cond, &vars_map) {
          if cty != Type::Bool {
            explanation.push_str(&format!(
              "{:?} condition must be Bool, got {:?}\n",
              id, cty
            ));
          }
        }
        // for comparison nodes, also check operand compatibility
        if let Expr::Equal(l, r) | Expr::Greater(l, r) | Expr::Less(l, r) = cond {
          let lt = infer_expr_type(ir, f, l, &vars_map);
          let rt = infer_expr_type(ir, f, r, &vars_map);
          if let (Some(lt), Some(rt)) = (lt, rt) {
            if std::mem::discriminant(&lt) != std::mem::discriminant(&rt) {
              explanation.push_str(&format!(
                "{:?} condition operand types mismatch: left {:?}, right {:?}\n",
                id, lt, rt
              ));
            }
            if matches!(cond, Expr::Greater(_, _) | Expr::Less(_, _)) && lt != Type::UInt64 {
              explanation.push_str(&format!(
                "{:?} comparison expects UInt64 operands, got {:?}\n",
                id, lt
              ));
            }
          }
        }
      }
      Step::Let { local, expr, .. } => {
        if !vars_map.contains_key::<str>(local.as_str()) {
          explanation.push_str(&format!("{:?} references {} that is not defined\n", id, local));
        }
        collect_vars_from_expr(expr, &vars_map, &mut explanation, id);
        if let (Some(lt), Some(rt)) = (vars_map.get(local), infer_expr_type(ir, f, expr, &vars_map)) {
          if lt != &rt {
            explanation.push_str(&format!("{:?} let '{}' type mismatch: expected {:?}, got {:?}\n", id, local, lt, rt));
          }
        }
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
            AwaitSpec::Queue { queue_name, message_var, .. } => {
              if let Some(t) = vars_map.get(queue_name.0) {
                if *t != Type::String {
                  explanation
                    .push_str(&format!("{:?} queue_name '{}' must be String, got {:?}\n", id, queue_name.0, t));
                }
              } else {
                // Allow referencing fiber-level init vars by name
                let is_init_var = ir.fibers.values().any(|f| f.init_vars.iter().any(|iv| iv.0 == queue_name.0));
                if !is_init_var {
                  explanation.push_str(&format!("{:?} references {} that is not defined\n", id, queue_name.0));
                }
              }
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
              if let Some(t) = vars_map.get(f_var_queue_name.0) {
                if *t != Type::String {
                  explanation
                    .push_str(&format!("{:?} queue id '{}' must be String, got {:?}\n", id, f_var_queue_name.0, t));
                }
              }
              if !vars_map.contains_key::<str>(var_name.0) {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, var_name.0));
              }
            }
            SetPrimitive::Future { f_var_name, var_name } => {
              // future variable must exist and be of type Future<T>
              if let Some(fty) = vars_map.get(f_var_name.0) {
                match fty {
                  Type::Future(inner_ty) => {
                    // value variable must exist and match inner T
                    if let Some(vty) = vars_map.get(var_name.0) {
                      if vty != inner_ty.as_ref() {
                        explanation.push_str(&format!(
                          "{:?} SetValues::Future value '{}' type mismatch: expected {:?}, got {:?}\n",
                          id, var_name.0, inner_ty, vty
                        ));
                      }
                    } else {
                      explanation.push_str(&format!("{:?} references {} that is not defined\n", id, var_name.0));
                    }
                  }
                  other => {
                    explanation.push_str(&format!(
                      "{:?} future handle '{}' must be Future<T>, got {:?}\n",
                      id, f_var_name.0, other
                    ));
                  }
                }
              } else {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, f_var_name.0));
              }
            }
          }
        }
      }
      Step::Create { primitives, success, fail } => {
        // length checks
        if success.id_binds.len() != primitives.len() {
          explanation.push_str(&format!(
            "{:?} Create: id_binds length {} does not match primitives {}\n",
            id,
            success.id_binds.len(),
            primitives.len()
          ));
        }
        if fail.error_binds.len() != primitives.len() {
          explanation.push_str(&format!(
            "{:?} Create: error_binds length {} does not match primitives {}\n",
            id,
            fail.error_binds.len(),
            primitives.len()
          ));
        }
        for (i, p) in primitives.iter().enumerate() {
          // success id bind type must match the primitive kind:
          // - Queue => String
          // - Future => Future<T>
          if let Some(b) = success.id_binds.get(i) {
            match vars_map.get(b.0) {
              Some(t) => match p {
                RuntimePrimitive::Queue { .. } => {
                  if *t != Type::String {
                    explanation
                      .push_str(&format!("{:?} Create: success bind '{}' must be String, got {:?}\n", id, b.0, t));
                  }
                }
                RuntimePrimitive::Future => match t {
                  Type::Future(_) => {}
                  other => explanation
                    .push_str(&format!("{:?} Create: success bind '{}' must be Future<T>, got {:?}\n", id, b.0, other)),
                },
                RuntimePrimitive::Schedule { .. } => match t {
                  Type::Future(inner) if **inner == Type::Void => {}
                  Type::Future(inner) => explanation.push_str(&format!(
                    "{:?} Create: success bind '{}' for Schedule must be Future<Void>, got Future<{:?}>\n",
                    id, b.0, inner
                  )),
                  other => explanation.push_str(&format!(
                    "{:?} Create: success bind '{}' for Schedule must be Future<Void>, got {:?}\n",
                    id, b.0, other
                  )),
                },
              },
              None => explanation.push_str(&format!("{:?} references {} that is not defined\n", id, b.0)),
            }
          }
          // fail bind must be Option<String>
          if let Some(b) = fail.error_binds.get(i) {
            match vars_map.get(b.0) {
              Some(Type::Option(inner)) if **inner == Type::String => {}
              Some(other) => explanation
                .push_str(&format!("{:?} Create: error bind '{}' must be Option<String>, got {:?}\n", id, b.0, other)),
              None => explanation.push_str(&format!("{:?} references {} that is not defined\n", id, b.0)),
            }
          }
          // primitive-specific checks
          match p {
            RuntimePrimitive::Future => {}
            RuntimePrimitive::Queue { name: qname, public: _ } => {
              if let Some(t) = vars_map.get(qname.0) {
                if *t != Type::String {
                  explanation
                    .push_str(&format!("{:?} Create: queue name var '{}' must be String, got {:?}\n", id, qname.0, t));
                }
              } else {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, qname.0));
              }
            }
            RuntimePrimitive::Schedule { ms_var } => {
              if let Some(t) = vars_map.get(ms_var.0) {
                if *t != Type::UInt64 {
                  explanation
                    .push_str(&format!("{:?} Create: schedule ms var '{}' must be U64, got {:?}\n", id, ms_var.0, t));
                }
              } else {
                explanation.push_str(&format!("{:?} references {} that is not defined\n", id, ms_var.0));
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

// Best-effort type inference for simple Expr/RetValue shapes
fn infer_expr_type(
  ir: &IR,
  _func: &Func,
  expr: &Expr,
  vars: &HashMap<String, Type>,
) -> Option<Type> {
  match expr {
    Expr::UInt64(_) => Some(Type::UInt64),
    Expr::Str(_) => Some(Type::String),
    Expr::Bool(_) => Some(Type::Bool),
    Expr::Var(LocalVarRef(name)) => vars.get::<str>(name).cloned(),
    Expr::Equal(a, b) | Expr::Greater(a, b) | Expr::Less(a, b) => {
      // comparisons yield Bool; operand validation is done by callers
      let _ = (infer_expr_type(ir, _func, a, vars), infer_expr_type(ir, _func, b, vars));
      Some(Type::Bool)
    }
    Expr::IsSome(inner) => {
      let _ = infer_expr_type(ir, _func, inner, vars);
      Some(Type::Bool)
    }
    Expr::Unwrap(inner) => match infer_expr_type(ir, _func, inner, vars) {
      Some(Type::Option(inner)) => Some(*inner.clone()),
      _ => None,
    },
    Expr::GetField(base, field) => {
      let bty = infer_expr_type(ir, _func, base, vars)?;
      resolve_struct_field_type(ir, &bty, field)
    }
    Expr::StructUpdate { base, updates } => {
      let bty = infer_expr_type(ir, _func, base, vars)?;
      // Validate updates if possible
      if let Some(fields) = resolve_struct_fields(ir, &bty) {
        for (fname, e) in updates {
          if let Some(fty) = fields.iter().find(|sf| &sf.name == fname).map(|sf| sf.ty.clone()) {
            if let Some(ety) = infer_expr_type(ir, _func, e, vars) {
              if fty != ety {
                // mismatch; still return base type
                // caller will add message elsewhere if needed
              }
            }
          }
        }
      }
      Some(bty)
    }
  }
}

fn infer_ret_value_type(
  ir: &IR,
  _func: &Func,
  rv: &RetValue,
  vars: &HashMap<String, Type>,
) -> Option<Type> {
  match rv {
    RetValue::Var(LocalVarRef(name)) => vars.get::<str>(name).cloned(),
    RetValue::UInt64(_) => Some(Type::UInt64),
    RetValue::Str(_) => Some(Type::String),
    RetValue::Bool(_) => Some(Type::Bool),
    RetValue::Some(inner) => infer_ret_value_type(ir, _func, inner, vars).map(|t| Type::Option(Box::new(t))),
    RetValue::None => Some(Type::Option(Box::new(Type::Void))), // marker for Option<_>
  }
}

fn return_type_compatible(
  func_out: &Type,
  rv: &RetValue,
  inferred: Option<&Type>,
) -> bool {
  match (func_out, rv, inferred) {
    // None is compatible with any Option<T>
    (Type::Option(_), RetValue::None, _) => true,
    // Some(x) must match Option<T>
    (Type::Option(_), RetValue::Some(_), Some(Type::Option(_))) => true,
    // Direct value must match exactly
    (fo, _, Some(it)) => fo == it,
    // If we can't infer, be permissive (don’t fail)
    (_, _, None) => true,
  }
}

fn resolve_struct_fields<'a>(
  ir: &'a IR,
  t: &'a Type,
) -> Option<&'a Vec<StructField>> {
  match t {
    Type::Struct(_name, fields, _) => Some(fields),
    // Treat PubQueueMessage as a struct for field/type resolution
    Type::PubQueueMessage { name: _, fields, .. } => Some(fields),
    Type::Custom(name) => ir.types.iter().find_map(|tt| match tt {
      Type::Struct(n, fields, _) if n == name => Some(fields),
      Type::PubQueueMessage { name: n, fields, .. } if n == name => Some(fields),
      _ => None,
    }),
    _ => None,
  }
}

fn resolve_struct_field_type(
  ir: &IR,
  t: &Type,
  field: &str,
) -> Option<Type> {
  let fields = resolve_struct_fields(ir, t)?;
  fields.iter().find(|sf| sf.name == field).map(|sf| sf.ty.clone())
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
    Expr::Bool(_) => {}
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
    RetValue::UInt64(_) | RetValue::Str(_) | RetValue::Bool(_) | RetValue::None => {}
  }
}
