use crate::ir::*;

pub fn generate_rust_types(ir: &IR) -> String {
  // Helpers for casing and identifiers
  fn to_pascal(s: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for ch in s.chars() {
      if !ch.is_alphanumeric() {
        up = true;
        continue;
      }
      if up {
        out.push(ch.to_ascii_uppercase());
        up = false;
      } else {
        out.push(ch);
      }
    }
    out
  }

  fn to_camel(s: &str) -> String {
    let p = to_pascal(s);
    let mut it = p.chars();
    match it.next() {
      None => String::new(),
      Some(f) => f.to_ascii_lowercase().to_string() + it.as_str(),
    }
  }

  // Type mapping helpers
  fn rust_type_name(t: &Type) -> String {
    match t {
      Type::UInt64 => "u64".into(),
      Type::String => "String".into(),
      Type::Void => "()".into(),
      Type::Map(k, v) => format!("std::collections::HashMap<{}, {}>", rust_type_name(k), rust_type_name(v)),
      Type::Array(t) => format!("Vec<{}>", rust_type_name(t)),
      Type::Struct(n, _, _) => to_pascal(n),
      Type::MaxQueue(t) => format!("std::collections::BinaryHeap<{}>", rust_type_name(t)),
      Type::MinQueue(t) => format!("std::collections::BinaryHeap<std::cmp::Reverse<{}>>", rust_type_name(t)),
      Type::Option(t) => format!("Option<{}>", rust_type_name(t)),
      Type::Custom(n) => to_pascal(n),
    }
  }

  fn value_variant_inner(t: &Type) -> String {
    match t {
      Type::UInt64 => "U64".into(),
      Type::String => "String".into(),
      Type::Struct(n, _, _) | Type::Custom(n) => to_pascal(n),
      Type::Array(inner) => format!("Array{}", value_variant_inner(inner)),
      Type::Option(inner) => format!("Option{}", value_variant_inner(inner)),
      Type::Map(_, _) | Type::MaxQueue(_) | Type::MinQueue(_) | Type::Void => "Unsupported".into(),
    }
  }

  fn default_typed_value(t: &Type) -> String {
    match t {
      Type::UInt64 => "0u64".into(),
      Type::String => "String::new()".into(),
      Type::Void => "()".into(),
      Type::Array(_) => "vec![]".into(),
      Type::Map(_, _) => "std::collections::HashMap::new()".into(),
      Type::MaxQueue(_) | Type::MinQueue(_) => "std::collections::BinaryHeap::new()".into(),
      Type::Option(_) => "None".into(),
      Type::Struct(n, _, _) | Type::Custom(n) => format!("{}::default()", to_pascal(n)),
    }
  }

  fn value_ctor_expr(
    t: &Type,
    e: &str,
  ) -> String {
    let v = value_variant_inner(t);
    format!("Value::{}({})", v, e)
  }

  fn push_extract(
    bind: &str,
    t: &Type,
    idx_expr: &str,
  ) -> String {
    let v = value_variant_inner(t);
    let rust_ty = rust_type_name(t);
    format!(
      "let {bind}: {rust_ty} = if let StackEntry::Value(_, Value::{v}(x)) = &vars[{idx}] {{ x.clone() }} else {{ unreachable!() }};",
      bind = bind,
      rust_ty = rust_ty,
      v = v,
      idx = idx_expr
    )
  }

  // Compute used Value types from all function signatures/locals and messages
  use std::collections::{BTreeMap, BTreeSet};
  let mut used_value_types: BTreeSet<String> = BTreeSet::new();
  let mut struct_defs: Vec<String> = Vec::new();

  // Custom structs
  for t in &ir.types {
    if let Type::Struct(name, mut fields, _impl) = t.clone() {
      let mut s = String::new();
      s.push_str("#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]\n");
      s.push_str(&format!("pub struct {} {{\n", to_pascal(&name)));
      // Ensure fields are alphabetically sorted by name
      fields.sort_by(|a, b| a.name.cmp(&b.name));
      for f in fields {
        s.push_str(&format!("  pub {}: {},\n", f.name, rust_type_name(&f.ty)));
      }
      s.push_str("}\n\n");
      struct_defs.push(s);
    }
  }

  // Per-fiber info
  #[derive(Clone)]
  struct FuncInfo {
    in_vars: Vec<(String, Type)>,
    locals: Vec<(String, Type)>,
    out: Type,
    steps: Vec<(String, Step)>,
  }

  // Map for quick lookup
  let mut fibers: BTreeMap<String, (Fiber, BTreeMap<String, FuncInfo>)> = BTreeMap::new();
  for (ft, f) in &ir.fibers {
    let mut funcs = BTreeMap::new();
    for (fname, func) in &f.funcs {
      let mut steps: Vec<(String, Step)> = Vec::new();
      for (sid, st) in &func.steps {
        steps.push((sid.0.clone(), st.clone()));
      }
      let fi = FuncInfo {
        in_vars: func.in_vars.iter().map(|InVar(n, t)| (n.to_string(), t.clone())).collect(),
        locals: func.locals.iter().map(|LocalVar(n, t)| (n.to_string(), t.clone())).collect(),
        out: func.out.clone(),
        steps,
      };
      funcs.insert(fname.clone(), fi);
      // used value types
      for InVar(_, t) in &func.in_vars {
        used_value_types.insert(value_variant_inner(t));
      }
      used_value_types.insert(value_variant_inner(&func.out));
      for LocalVar(_, t) in &func.locals {
        used_value_types.insert(value_variant_inner(t));
      }
    }
    fibers.insert(ft.0.clone(), (f.clone(), funcs));
  }

  // Collect fibers in deterministic order for subsequent sections
  let mut fibers_sorted_ir: Vec<(&FiberType, &Fiber)> = ir.fibers.iter().collect();
  fibers_sorted_ir.sort_by(|a, b| a.0.0.cmp(&b.0.0));

  // messages: add structs (sorted fibers and fields)
  let mut message_structs: Vec<String> = Vec::new();
  for (ft, f) in &fibers_sorted_ir {
    for MessageSpec(name, fields) in &f.in_messages {
      let mut s = String::new();
      s.push_str("#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]\n");
      s.push_str(&format!("pub struct {}{}Msg {{\n", to_pascal(&ft.0), to_pascal(name)));
      let mut fields_sorted = fields.clone();
      fields_sorted.sort_by(|a, b| a.0.cmp(&b.0));
      for (n, t) in &fields_sorted {
        s.push_str(&format!("  pub {}: {},\n", n, rust_type_name(t)));
        used_value_types.insert(value_variant_inner(t));
      }
      s.push_str("}\n\n");
      message_structs.push(s);
    }
  }

  // Heaps per fiber + combined heap
  let mut heap_structs: Vec<String> = Vec::new();
  for (ft, f) in &fibers_sorted_ir {
    let mut s = String::new();
    s.push_str("#[derive(Clone, Debug, Default)]\n");
    s.push_str(&format!("pub struct {}Heap {{\n", to_pascal(&ft.0)));
    let mut heap_fields: Vec<(&String, &Type)> = f.heap.iter().collect();
    heap_fields.sort_by(|a, b| a.0.cmp(b.0));
    for (n, t) in heap_fields {
      let ty = match t {
        Type::MinQueue(inner) => format!("std::collections::BinaryHeap<std::cmp::Reverse<{}>>", rust_type_name(inner)),
        _ => rust_type_name(t),
      };
      s.push_str(&format!("  pub {}: {},\n", n, ty));
    }
    s.push_str("}\n\n");
    heap_structs.push(s);
  }

  let mut combined_heap = String::new();
  combined_heap.push_str("#[derive(Clone, Debug, Default)]\n");
  combined_heap.push_str("pub struct Heap {\n");
  for (ft, _f) in &fibers_sorted_ir {
    // Keep field name as provided in IR (expects snake_case like order_book)
    combined_heap.push_str(&format!("  pub {}: {}Heap,\n", ft.0, to_pascal(&ft.0)));
  }
  combined_heap.push_str("}\n\n");

  // State enum (with trivial-return suppression)
  let mut state_variants: Vec<(String, String, String)> = Vec::new(); // (variant, fiber, func)
  for (ft, (_f, funcs)) in &fibers {
    for (fname, finfo) in funcs {
      // Identify direct-return RustBlock next steps to suppress from State enum
      use std::collections::BTreeSet as __BTS_STATE;
      let mut suppressed: __BTS_STATE<String> = __BTS_STATE::new();
      for (_, st) in &finfo.steps {
        if let Step::RustBlock { binds, next, .. } = st {
          if binds.len() == 1 {
            if let Some((_, Step::Return { value })) = finfo.steps.iter().find(|(nsid, _)| nsid == &next.0) {
              if let RetValue::Var(vn) = value {
                if vn == &binds[0] {
                  suppressed.insert(next.0.clone());
                }
              }
            }
          }
        }
      }
      for (sid, _) in &finfo.steps {
        if suppressed.contains(sid) {
          continue;
        }
        let v = format!("{}{}{}", to_pascal(ft), to_pascal(fname), to_pascal(sid));
        state_variants.push((v, ft.clone(), fname.clone()));
      }
    }
  }
  state_variants.sort_by(|a, b| a.0.cmp(&b.0));

  let mut state_enum = String::new();
  state_enum.push_str("#[derive(Clone, Debug, PartialEq)]\n");
  state_enum.push_str("pub enum State {\n  Completed,\n  Idle,\n");
  for (v, _, _) in &state_variants {
    state_enum.push_str(&format!("  {},\n", v));
  }
  state_enum.push_str("}\n\n");

  // Value enum (compact-by-type)
  let mut val_enum = String::new();
  val_enum.push_str("#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]\n");
  val_enum.push_str("pub enum Value {\n");
  for vt in &used_value_types {
    // Map inner label to Rust type
    let rust_ty = match vt.as_str() {
      "U64" => "u64".to_string(),
      "String" => "String".to_string(),
      l if l.starts_with("Option") => {
        let inner = &vt[6..];
        let rust_inner = match inner {
          "U64" => "u64".to_string(),
          "String" => "String".to_string(),
          _ => inner.to_string(),
        };
        format!("Option<{}>", rust_inner)
      }
      l if l.starts_with("Array") => {
        let inner = &vt[5..];
        let rust_inner = match inner {
          "U64" => "u64".to_string(),
          "String" => "String".to_string(),
          _ => inner.to_string(),
        };
        format!("Vec<{}>", rust_inner)
      }
      other => other.to_string(),
    };
    val_enum.push_str(&format!("  {}({}),\n", vt, rust_ty));
  }
  val_enum.push_str("}\n\n");

  // Fixed runtime structures used by fiber.rs
  let mut stack_and_step = String::new();
  stack_and_step.push_str("#[derive(Clone, Debug, PartialEq)]\n");
  stack_and_step.push_str("pub enum StackEntry {\n  State(State),\n  // Option<usize> - local index offset back on stack\n  // if it's None - no value will be binded into the local variable of the function that initiated call\n  Retrn(Option<usize>),\n  Value(String, Value),\n  // In-place updates to the current frame (offset -> new Value)\n  FrameAssign(Vec<(usize, Value)>),\n}\n\n");
  stack_and_step.push_str("#[derive(Clone, Debug, PartialEq)]\n");
  stack_and_step.push_str(
    "pub enum StepResult {\n  Done,\n  Next(Vec<StackEntry>),\n  ScheduleTimer { ms: u64, next: State, future_id: FutureLabel },\n  GoTo(State),\n  Select(Vec<State>),\n  // Return can carry an optional value to be consumed by the runtime.\n  Return(Value),\n  ReturnVoid,\n  Todo(String),\n  // Await a future: (future_id, optional bind_var, next_state)\n  Await(FutureLabel, Option<String>, State),\n  // Send a message to a fiber with function and typed args, then continue to `next`.\n  SendToFiber { f_type: FiberType, func: String, args: Vec<Value>, next: State, future_id: FutureLabel },\n}\n"
  );

  // func_args_count mapping: number of in_vars + locals for each state's function
  let mut func_args_count = String::new();
  func_args_count.push_str("pub fn func_args_count(e: &State) -> usize {\n  match e {\n");
  for (v, fty, fname) in &state_variants {
    let finfo = &fibers.get(fty).unwrap().1.get(fname).unwrap();
    let n = finfo.in_vars.len() + finfo.locals.len();
    func_args_count.push_str(&format!("    State::{} => {},\n", v, n));
  }
  func_args_count.push_str("    State::Idle => 0,\n    State::Completed => 0,\n  }\n}\n");

  // Expression printer for conditions and simple values
  fn expr_to_typed_str(e: &Expr) -> String {
    match e {
      Expr::UInt64(v) => format!("{}u64", v),
      Expr::Str(s) => format!("\"{}\".to_string()", s),
      Expr::Var(n) => n.clone(),
      Expr::Equal(a, b) => format!("({} == {})", expr_to_typed_str(a), expr_to_typed_str(b)),
      Expr::Greater(a, b) => format!("({} > {})", expr_to_typed_str(a), expr_to_typed_str(b)),
      Expr::Less(a, b) => format!("({} < {})", expr_to_typed_str(a), expr_to_typed_str(b)),
      Expr::IsSome(a) => format!("({}.is_some())", expr_to_typed_str(a)),
      Expr::Unwrap(a) => format!("({}.unwrap())", expr_to_typed_str(a)),
      Expr::GetField(base, fld) => format!("({}.{})", expr_to_typed_str(base), fld),
      Expr::StructUpdate { base, updates } => {
        let mut s = format!("({{ let mut __tmp = {};", expr_to_typed_str(base));
        for (n, v) in updates {
          s.push_str(&format!(" __tmp.{} = {};", n, expr_to_typed_str(v)));
        }
        s.push_str(" __tmp }})");
        s
      }
    }
  }

  fn ret_value_expr(rv: &RetValue) -> String {
    match rv {
      RetValue::Var(n) => n.clone(),
      RetValue::UInt64(v) => format!("{}u64", v),
      RetValue::Str(s) => format!("\"{}\".to_string()", s),
      RetValue::Some(inner) => format!("Some({})", ret_value_expr(inner)),
      RetValue::None => "None".into(),
    }
  }

  // global_step generation
  let mut global_step = String::new();
  global_step.push_str("pub fn global_step(\n  state: State,\n  vars: &[StackEntry],\n  heap: &mut Heap,\n) -> StepResult {\n  match state {\n    State::Completed => StepResult::Done,\n    State::Idle => panic!(\"shoudnt be here\"),\n");

  // Helper to compute index of a named variable in a function frame
  let mut name_to_index_map: BTreeMap<(String, String, String), usize> = BTreeMap::new(); // key: (fiber, func, var)
  for (fty, (_f, funcs)) in &fibers {
    for (fname, finfo) in funcs {
      for (i, (n, _)) in finfo.in_vars.iter().chain(finfo.locals.iter()).enumerate() {
        name_to_index_map.insert((fty.clone(), fname.clone(), n.clone()), i);
      }
    }
  }

  for (fty, (_f, funcs)) in &fibers {
    for (fname, finfo) in funcs {
      // Compute suppressed steps (direct-return next of RustBlock) for this function
      use std::collections::BTreeSet as __BTS_CODEGEN2;
      let mut suppressed: __BTS_CODEGEN2<String> = __BTS_CODEGEN2::new();
      for (_sid, st) in &finfo.steps {
        if let Step::RustBlock { binds, next, .. } = st {
          if binds.len() == 1 {
            if let Some((_, Step::Return { value })) = finfo.steps.iter().find(|(nsid, _)| nsid == &next.0) {
              if let RetValue::Var(vn) = value {
                if vn == &binds[0] {
                  suppressed.insert(next.0.clone());
                }
              }
            }
          }
        }
      }

      let func_states: Vec<(String, String, Step)> = finfo
        .steps
        .iter()
        .map(|(sid, st)| (format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(sid)), sid.clone(), st.clone()))
        .collect();

      // For each state, emit extractor lines for all function vars
      for (svar, sid, st) in func_states {
        if suppressed.contains(&sid) {
          continue;
        }
        global_step.push_str(&format!("    State::{} => {{\n", svar));
        // Extractors
        for (i, (n, t)) in finfo.in_vars.iter().chain(finfo.locals.iter()).enumerate() {
          global_step.push_str(&format!("      {}\n", push_extract(n, t, &i.to_string())));
        }

        match st {
          Step::If { cond, then_, else_ } => {
            let cond_e = expr_to_typed_str(&cond);
            let then_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&then_.0));
            let else_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&else_.0));
            global_step.push_str(&format!(
              "      if {} {{\n        StepResult::GoTo(State::{})\n      }} else {{\n        StepResult::GoTo(State::{})\n      }}\n",
              cond_e, then_v, else_v
            ));
          }
          Step::RustBlock { binds, code, next } => {
            if binds.len() == 0 {
              // No binds: execute for side effects and continue
              let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&next.0));
              global_step
                .push_str(&format!("      {{ let _out = {{ {} }}; StepResult::GoTo(State::{}) }}\n", code, next_v));
            } else if binds.len() == 1 {
              // Assign single bind via FrameAssign
              let b0 = &binds[0];
              let idx = *name_to_index_map.get(&(fty.clone(), fname.clone(), b0.clone())).unwrap();
              let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&next.0));
              // If the next step is a trivial return of this bind, inline the return and suppress the next state
              if suppressed.contains(&next.0) {
                let ret_v = value_ctor_expr(&finfo.out, "out");
                global_step.push_str(&format!("      {{ let out = {{ {} }}; StepResult::Return({}) }}\n", code, ret_v));
              } else {
                global_step.push_str(&format!(
                  "      {{ let out = {{ {} }}; StepResult::Next(vec![\n        StackEntry::FrameAssign(vec![({}, {})]),\n        StackEntry::State(State::{})\n      ]) }}\n",
                  code,
                  idx,
                  value_ctor_expr(&finfo
                    .locals
                    .iter()
                    .find(|(n, _)| n == b0)
                    .map(|x| &x.1)
                    .unwrap_or(&finfo.out),
                    "out"),
                  next_v
                ));
              }
            } else {
              let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&next.0));
              let mut assigns: Vec<String> = Vec::new();
              for (i, b) in binds.iter().enumerate() {
                let idx = *name_to_index_map.get(&(fty.clone(), fname.clone(), b.clone())).unwrap();
                let valty =
                  finfo.locals.iter().find(|(n, _)| n == b).map(|x| x.1.clone()).unwrap_or_else(|| finfo.out.clone());
                assigns.push(format!("({}, {})", idx, value_ctor_expr(&valty, &format!("o{}", i))));
              }
              global_step.push_str(&format!(
                "      {{ let out = {{ {} }}; let ({}) = out; StepResult::Next(vec![\n        StackEntry::FrameAssign(vec![{}]),\n        StackEntry::State(State::{})\n      ]) }}\n",
                code,
                (0..binds.len()).map(|i| format!("o{}", i)).collect::<Vec<_>>().join(", "),
                assigns.join(", "),
                next_v
              ));
            }
          }
          Step::Return { value } => {
            let vexpr = ret_value_expr(&value);
            let v = value_ctor_expr(&finfo.out, &vexpr);
            global_step.push_str(&format!("      StepResult::Return({})\n", v));
          }
          Step::ReturnVoid => {
            global_step.push_str("      StepResult::ReturnVoid\n");
          }
          Step::Let { local, expr, next } => {
            let idx = *name_to_index_map.get(&(fty.clone(), fname.clone(), local.clone())).unwrap();
            let lty = finfo.locals.iter().find(|(n, _)| *n == local).map(|x| x.1.clone()).unwrap();
            let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&next.0));
            let expr_s = expr_to_typed_str(&expr);
            global_step.push_str(&format!(
              "      StepResult::Next(vec![\n        StackEntry::FrameAssign(vec![({}, {})]),\n        StackEntry::State(State::{})\n      ])\n",
              idx,
              value_ctor_expr(&lty, &expr_s),
              next_v
            ));
          }
          Step::Call { target, args, bind, ret_to } => {
            let (t_fty, t_fn) = (&target.fiber, &target.func);
            let entry = format!("{}{}{}", to_pascal(t_fty), to_pascal(t_fn), to_pascal("entry"));
            let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&ret_to.0));
            let n_parent = finfo.in_vars.len() + finfo.locals.len();
            let ret_marker = if let Some(var) = bind.clone() {
              let idx = *name_to_index_map.get(&(fty.clone(), fname.clone(), var)).unwrap();
              let offset = n_parent + 1 - idx; // see runtime binding logic
              format!("StackEntry::Retrn(Some({}))", offset)
            } else {
              "StackEntry::Retrn(None)".to_string()
            };

            // Build args for callee in order they are declared in callee signature
            let callee = &fibers.get(t_fty).unwrap().1.get(t_fn).unwrap();
            let mut arg_vals: Vec<String> = Vec::new();
            for (i, (_n, t)) in callee.in_vars.iter().enumerate() {
              let e = expr_to_typed_str(&args[i]);
              arg_vals.push(format!("StackEntry::Value(\"{}\".to_string(), {})", "_", value_ctor_expr(t, &e)));
            }
            // Callee locals defaults
            let mut local_vals: Vec<String> = Vec::new();
            for (n, t) in &callee.locals {
              local_vals.push(format!(
                "StackEntry::Value(\"{}\".to_string(), {})",
                n,
                value_ctor_expr(t, &default_typed_value(t))
              ));
            }
            let mut seq: Vec<String> = Vec::new();
            seq.push(format!("StackEntry::State(State::{})", next_v));
            seq.push(ret_marker);
            seq.extend(arg_vals);
            seq.extend(local_vals);
            seq.push(format!("StackEntry::State(State::{})", entry));
            global_step
              .push_str(&format!("      StepResult::Next(vec![\n        {}\n      ])\n", seq.join(",\n        ")));
          }
          Step::Await(AwaitSpec { bind, ret_to, future_id }) => {
            let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&ret_to.0));
            let bind_s = bind.clone().map(|s| format!("\"{}\".to_string()", s));
            let arg = bind_s.unwrap_or("None".into());
            global_step.push_str(&format!(
              "      StepResult::Await(FutureLabel::new(\"{}\"), {}, State::{})\n",
              future_id.0,
              if arg == "None" { "None".into() } else { format!("Some({})", arg) },
              next_v
            ));
          }
          Step::SendToFiber { fiber, message, args, next, future_id } => {
            let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&next.0));
            let mut arg_vals: Vec<String> = Vec::new();
            // We don't know the callee signature here; evaluate as Values from our types
            for e in &args {
              let e_s = expr_to_typed_str(&e.1);
              // Best-effort: assume u64 unless name hints; try lookup by var name
              // We can attempt to resolve from our own var map
              let t = if let Expr::Var(nm) = e.1.clone() {
                if let Some(idx) = name_to_index_map.get(&(fty.clone(), fname.clone(), nm.clone())) {
                  let (_, ty) = finfo
                    .in_vars
                    .iter()
                    .chain(finfo.locals.iter())
                    .enumerate()
                    .find(|(i, _)| i == idx)
                    .map(|(_, x)| x)
                    .unwrap();
                  ty.clone()
                } else {
                  Type::UInt64
                }
              } else {
                Type::UInt64
              };
              arg_vals.push(value_ctor_expr(&t, &e_s));
            }
            global_step.push_str(&format!(
              "      StepResult::SendToFiber {{ f_type: FiberType::new(\"{}\"), func: \"{}\".to_string(), args: vec![{}], next: State::{}, future_id: FutureLabel::new(\"{}\") }}\n",
              fiber,
              message,
              arg_vals.join(", "),
              next_v,
              future_id.0
            ));
          }
          Step::ScheduleTimer { ms, next, future_id } => {
            let next_v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&next.0));
            global_step.push_str(&format!(
              "      StepResult::ScheduleTimer {{ ms: {}u64, next: State::{}, future_id: FutureLabel::new(\"{}\") }}\n",
              ms.0, next_v, future_id.0
            ));
          }
          Step::Select { arms } => {
            let mut states: Vec<String> = Vec::new();
            for a in &arms {
              let v = format!("{}{}{}", to_pascal(fty), to_pascal(fname), to_pascal(&a.ret_to.0));
              states.push(format!("State::{}", v));
            }
            global_step.push_str(&format!("      StepResult::Select(vec![{}])\n", states.join(", ")));
          }
        }
        global_step.push_str("    }\n");
      }
    }
  }

  global_step.push_str("  }\n}\n");

  // Prepare/result helpers and registry
  let mut prep_res_helpers = String::new();
  prep_res_helpers.push_str("// Registry: function key -> (prepare_from_values, result_to_value)\n");
  prep_res_helpers.push_str("pub type PrepareFn = fn(Vec<Value>) -> Vec<StackEntry>;\n");
  prep_res_helpers.push_str("pub type ResultFn = fn(&[StackEntry]) -> Value;\n\n");

  // Per-function prepare/result
  let mut regs_prepare: Vec<(String, String)> = Vec::new();
  let mut regs_result: Vec<(String, String)> = Vec::new();
  let mut per_func_blocks: Vec<String> = Vec::new();
  for (fty, (_f, funcs)) in &fibers {
    for (fname, finfo) in funcs {
      let f_pas = to_pascal(fname);
      let t_pas = to_pascal(fty);
      let entry_state = format!("{}{}{}", t_pas, f_pas, to_pascal("entry"));

      // typed prepare
      let mut sig = format!("pub fn {}_prepare_{}(", to_camel(fty), f_pas);
      sig.push_str(
        &finfo.in_vars.iter().map(|(n, t)| format!("{}: {}", n, rust_type_name(t))).collect::<Vec<_>>().join(", "),
      );
      sig.push_str(") -> (Vec<StackEntry>, Heap) {\n");
      let mut body = String::new();
      body.push_str("  let mut stack: Vec<StackEntry> = Vec::new();\n");
      // ret value slot
      body.push_str(&format!(
        "  stack.push(StackEntry::Value(\"ret\".to_string(), {}));\n",
        value_ctor_expr(&finfo.out, &default_typed_value(&finfo.out))
      ));
      body.push_str("  stack.push(StackEntry::Retrn(Some(1)));\n");
      for (n, t) in &finfo.in_vars {
        body.push_str(&format!("  stack.push(StackEntry::Value(\"{}\".to_string(), {}));\n", n, value_ctor_expr(t, n)));
      }
      for (n, t) in &finfo.locals {
        body.push_str(&format!(
          "  stack.push(StackEntry::Value(\"{}\".to_string(), {}));\n",
          n,
          value_ctor_expr(t, &default_typed_value(t))
        ));
      }
      body.push_str(&format!(
        "  stack.push(StackEntry::State(State::{}));\n  let heap = Heap::default();\n  (stack, heap)\n",
        entry_state
      ));
      body.push_str("}\n\n");

      // typed result
      let mut res = format!(
        "pub fn {}_result_{}(stack: &[StackEntry]) -> {} {{\n  match stack.last() {{\n",
        to_camel(fty),
        f_pas,
        rust_type_name(&finfo.out)
      );
      let v = value_variant_inner(&finfo.out);
      res.push_str(&format!(
        "    Some(StackEntry::Value(_, Value::{}(v))) => v.clone(),\n    _ => unreachable!(\"result not found on stack\"),\n  }}\n}}\n\n",
        v
      ));

      // from_values adapter
      let mut from_v =
        format!("fn {}_prepare_{}_from_values(args: Vec<Value>) -> Vec<StackEntry> {{\n", to_camel(fty), f_pas);
      for (i, (n, t)) in finfo.in_vars.iter().enumerate() {
        let v = value_variant_inner(t);
        from_v.push_str(&format!(
          "  let {}: {} = if let Value::{}(x) = &args[{}] {{ x.clone() }} else {{ unreachable!(\"invalid args for {}.{}\") }};\n",
          n,
          rust_type_name(t),
          v,
          i,
          fty,
          fname
        ));
      }
      from_v.push_str(&format!(
        "  let (stack, _heap) = {}_prepare_{}({});\n  stack\n}}\n\n",
        to_camel(fty),
        f_pas,
        finfo.in_vars.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>().join(", ")
      ));

      // result to value adapter
      let res_v = format!(
        "fn {}_result_{}_value(stack: &[StackEntry]) -> Value {{\n  Value::{}({}_result_{}(stack))\n}}\n\n",
        to_camel(fty),
        f_pas,
        value_variant_inner(&finfo.out),
        to_camel(fty),
        f_pas
      );

      per_func_blocks.push(sig + &body + &res + &from_v + &res_v);
      regs_prepare.push((format!("{}.{}", fty, fname), format!("{}_prepare_{}_from_values", to_camel(fty), f_pas)));
      regs_result.push((format!("{}.{}", fty, fname), format!("{}_result_{}_value", to_camel(fty), f_pas)));
    }
  }

  // Registry fns
  let mut reg_code = String::new();
  reg_code.push_str("pub fn get_prepare_fn(key: &str) -> PrepareFn {\n  match key {\n");
  for (k, f) in &regs_prepare {
    reg_code.push_str(&format!("    \"{}\" => {},\n", k, f));
  }
  reg_code.push_str("    _ => panic!(\"shouldnt be here\"),\n  }\n}\n\n");
  reg_code.push_str("pub fn get_result_fn(key: &str) -> ResultFn {\n  match key {\n");
  for (k, f) in &regs_result {
    reg_code.push_str(&format!("    \"{}\" => {},\n", k, f));
  }
  reg_code.push_str("    _ => panic!(\"shouldnt be here\"),\n  }\n}\n");

  // Header and glue
  let mut out = String::new();
  out.push_str("// Generated by dsl::codegen from IR\n");
  out.push_str("#![allow(dead_code)]\n#![allow(unused_variables)]\n#![allow(non_snake_case)]\n\n");
  out.push_str("use crate::ir::{FiberType, FutureLabel};\nuse serde::{Deserialize, Serialize};\n\n");
  for s in struct_defs {
    out.push_str(&s);
  }
  for s in message_structs {
    out.push_str(&s);
  }
  for s in heap_structs {
    out.push_str(&s);
  }
  out.push_str(&combined_heap);
  out.push_str(&state_enum);
  out.push_str(&val_enum);
  out.push_str(&stack_and_step);
  out.push_str(&func_args_count);
  out.push_str(&global_step);
  // Type aliases for registry
  out.push_str(&prep_res_helpers);
  out.push_str("\n");
  for blk in per_func_blocks {
    out.push_str(&blk);
  }
  out.push_str(&reg_code);

  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  #[test]
  fn generates_types() {
    let ir = IR {
      types: vec![Type::Struct(
        "User".into(),
        vec![StructField { name: "id".into(), ty: Type::String }, StructField { name: "age".into(), ty: Type::UInt64 }],
        String::new(),
      )],
      fibers: HashMap::from([
        (
          FiberType::new("userManager"),
          Fiber {
            fibers_limit: 1,
            heap: HashMap::from([(
              "users".into(),
              Type::Map(Box::new(Type::String), Box::new(Type::Custom("User".into()))),
            )]),
            in_messages: vec![MessageSpec("GetUser", vec![("key", Type::String)])],
            funcs: HashMap::from([(
              "get".into(),
              Func {
                in_vars: vec![InVar("key", Type::String)],
                out: Type::Option(Box::new(Type::Custom("User".into()))),
                locals: vec![],
                entry: StepId::new("entry"),
                steps: vec![(StepId::new("entry"), Step::ReturnVoid)],
              },
            )]),
          },
        ),
        (
          FiberType::new("global"),
          Fiber { fibers_limit: 1, heap: HashMap::new(), in_messages: vec![], funcs: HashMap::new() },
        ),
      ]),
    };

    let code = generate_rust_types(&ir);
    // Spot-check a few important bits are present.
    assert!(code.contains("pub struct User"));
    assert!(code.contains("pub struct UserManagerGetUserMsg"));
    assert!(code.contains("pub struct Heap"));
    assert!(code.contains("pub enum State"));
    assert!(code.contains("UserManagerGetEntry"));
    assert!(code.contains("pub enum Value"));
    // With compact-by-type Value enum, we expect used types only
    assert!(code.contains("String(String)"));
    assert!(code.contains("OptionUser(Option<User>)"));
  }
}
