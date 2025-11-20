use common::logical_time::LogicalTimeAbsoluteMs;
use dsl::ir::*;
use std::collections::HashMap;

// Shared IR specification used by build.rs (via include!) and by tests.
pub fn sample_ir() -> IR {
  IR {
    fibers: HashMap::from([
      (
        FiberType::new("root"),
        Fiber {
          fibers_limit: 0,
          heap: HashMap::new(),
          in_messages: vec![],
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func{in_vars: vec![],out: Type::Void, locals: vec![LocalVar("counter", Type::UInt64)], entry: StepId::new("entry"),steps: vec![
                (
                  StepId::new("entry"),
                  Step::RustBlock { binds: vec!["counter".to_string()], code: "0".to_string(), next: StepId::new("start_work") },
                ),
                (
                  StepId::new("start_work"),
                  Step::RustBlock { binds: vec!["counter".to_string()], code: "counter + 1".to_string(), next: StepId::new("compare") },
                ),
                (
                  StepId::new("compare"),
                  Step::If { cond: Expr::Equal(Box::new(Expr::Var("counter".to_string())), Box::new(Expr::UInt64(2))), then_: StepId::new("return"), else_: StepId::new("start_work") },
                ),
                (
                  StepId::new("return"),
                  Step::ReturnVoid,
                ),
              ]},
            ),
          ]),
        }
      ),
      (
        FiberType::new("global"),
        Fiber {
          fibers_limit: 100,
          heap: HashMap::from([("binary_search_values".to_string(), Type::Array(Box::new(Type::UInt64)))]),
          in_messages: vec![],
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals:vec![],
                entry: StepId::new("entry"),
                steps: vec![(StepId::new("entry"), Step::ReturnVoid)],
              },
            ),
            (
              "add".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sum", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["sum".to_string()],
                      code: "a+b".to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("sum".to_string()) }),
                ],
              },
            ),
            (
              "sub".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sub", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["sub".to_string()],
                      code: r#"
let out = a - b;
out
"#
                      .to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("sub".to_string()) }),
                ],
              },
            ),
            (
              "mult".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("mult", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["mult".to_string()],
                      code: "a*b".to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("mult".to_string()) }),
                ],
              },
            ),
            (
              "div".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("div", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["div".to_string()],
                      code: "a/b".to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("div".to_string()) }),
                ],
              },
            ),
            (
              // factorial(n) { if n == 1 { return 1 } return n * factorial(n - 1) }
              "factorial".to_string(),
              Func {
                in_vars: vec![InVar("n", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![
                  LocalVar("fac_call_res", Type::UInt64),
                  LocalVar("subtract_res", Type::UInt64),
                  LocalVar("result", Type::UInt64),
                ],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::If {
                      cond: Expr::Equal(Box::new(Expr::Var("n".to_string())), Box::new(Expr::UInt64(1))),
                      then_: StepId::new("return_1"),
                      else_: StepId::new("subtract"),
                    },
                  ),
                  (StepId::new("return_1"), Step::Return { value: RetValue::UInt64(1) }),
                  (
                    StepId::new("subtract"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                      args: vec![Expr::Var("n".to_string()), Expr::UInt64(1)],
                      bind: Some("subtract_res".to_string()),
                      ret_to: StepId::new("factorial_call"),
                    },
                  ),
                  (
                    StepId::new("factorial_call"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "factorial".to_string() },
                      args: vec![Expr::Var("subtract_res".to_string())],
                      bind: Some("fac_call_res".to_string()),
                      ret_to: StepId::new("multiply"),
                    },
                  ),
                  (
                    StepId::new("multiply"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "mult".to_string() },
                      args: vec![Expr::Var("n".to_string()), Expr::Var("fac_call_res".to_string())],
                      bind: Some("result".to_string()),
                      ret_to: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
                ],
              },
            ),
            (
              "subAdd".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64), InVar("c", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sumAB", Type::UInt64), LocalVar("subABC", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "add".to_string() },
                      args: vec![Expr::Var("a".to_string()), Expr::Var("b".to_string())],
                      bind: Some("sumAB".to_string()),
                      ret_to: StepId::new("sub_sum"),
                    },
                  ),
                  (
                    StepId::new("sub_sum"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                      args: vec![Expr::Var("sumAB".to_string()), Expr::Var("c".to_string())],
                      bind: Some("subABC".to_string()),
                      ret_to: StepId::new("finalize"),
                    },
                  ),
                  (StepId::new("finalize"), Step::Return { value: RetValue::Var("subABC".to_string()) }),
                ],
              },
            ),
            (
              // binary_search IR
              "binary_search".to_string(),
              Func {
                in_vars: vec![InVar("e", Type::UInt64), InVar("left", Type::UInt64), InVar("right", Type::UInt64)],
                out: Type::Option(Box::new(Type::UInt64)),
                locals: vec![
                  LocalVar("div", Type::UInt64),
                  LocalVar("v_by_index_div", Type::UInt64),
                  LocalVar("fac_call_res", Type::Option(Box::new(Type::UInt64))),
                ],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::If {
                      cond: Expr::Greater(
                        Box::new(Expr::Var("left".to_string())),
                        Box::new(Expr::Var("right".to_string())),
                      ),
                      then_: StepId::new("return_None"),
                      else_: StepId::new("calculate_div"),
                    },
                  ),
                  (StepId::new("return_None"), Step::Return { value: RetValue::None }),
                  (
                    StepId::new("calculate_div"),
                    Step::RustBlock {
                      binds: vec!["div".to_string(), "v_by_index_div".to_string()],
                      code: r#"
                    let o_div = (left + right) / 2;
                    let s = &heap.global;
                    (o_div, s.binarySearchValues[o_div as usize])
                    "#
                      .to_string(),
                      next: StepId::new("return_if_equal"),
                    },
                  ),
                  (
                    StepId::new("return_if_equal"),
                    Step::If {
                      cond: Expr::Equal(
                        Box::new(Expr::Var("v_by_index_div".to_string())),
                        Box::new(Expr::Var("e".to_string())),
                      ),
                      then_: StepId::new("return_found"),
                      else_: StepId::new("cmp_less"),
                    },
                  ),
                  (
                    StepId::new("return_found"),
                    Step::Return { value: RetValue::Some(Box::new(RetValue::Var("div".to_string()))) },
                  ),
                  (
                    StepId::new("cmp_less"),
                    Step::If {
                      cond: Expr::Less(
                        Box::new(Expr::Var("v_by_index_div".to_string())),
                        Box::new(Expr::Var("e".to_string())),
                      ),
                      then_: StepId::new("go_right"),
                      else_: StepId::new("go_left_check_overflow"),
                    },
                  ),
                  (
                    StepId::new("go_right"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "add".to_string() },
                      args: vec![Expr::Var("div".to_string()), Expr::UInt64(1)],
                      bind: Some("left".to_string()),
                      ret_to: StepId::new("recursive_call"),
                    },
                  ),
                  (
                    StepId::new("go_left_check_overflow"),
                    Step::If {
                      cond: Expr::Less(Box::new(Expr::Var("div".to_string())), Box::new(Expr::UInt64(0))),
                      then_: StepId::new("return_None"),
                      else_: StepId::new("go_left"),
                    },
                  ),
                  (
                    StepId::new("go_left"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "sub".to_string() },
                      args: vec![Expr::Var("div".to_string()), Expr::UInt64(1)],
                      bind: Some("right".to_string()),
                      ret_to: StepId::new("recursive_call"),
                    },
                  ),
                  (
                    StepId::new("recursive_call"),
                    Step::Call {
                      target: FuncRef { fiber: "global".to_string(), func: "binary_search".to_string() },
                      args: vec![
                        Expr::Var("e".to_string()),
                        Expr::Var("left".to_string()),
                        Expr::Var("right".to_string()),
                      ],
                      bind: Some("fac_call_res".to_string()),
                      ret_to: StepId::new("return_result"),
                    },
                  ),
                  (StepId::new("return_result"), Step::Return { value: RetValue::Var("fac_call_res".to_string()) }),
                ],
              },
            ),
          ]),
        },
      ),
      (
        FiberType::new("application"),
        Fiber {
          fibers_limit: 2,
          heap: HashMap::new(),
          in_messages: vec![MessageSpec("async_foo", vec![("a", Type::UInt64), ("b", Type::UInt64)])],
          // (StepId::new("await_in_message"), Step::Await(())),
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals:vec![],
                entry: StepId::new("entry"),
                steps: vec![(StepId::new("entry"), Step::ReturnVoid)],
              },
            ),
            (
              "async_foo".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("sum", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::SendToFiber {
                      fiber: "global".to_string(),
                      message: "add".to_string(),
                      args: vec![
                        ("a".to_string(), Expr::Var("a".to_string())),
                        ("b".to_string(), Expr::Var("b".to_string())),
                      ],
                      next: StepId::new("await"),
                      future_id: FutureLabel::new("async_add_future_1"),
                    },
                  ),
                  (
                    StepId::new("await"),
                    Step::Await(AwaitSpec {
                      bind: Some("sum".to_string()),
                      ret_to: StepId::new("return"),
                      future_id: FutureLabel::new("async_add_future_1"),
                    }),
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("sum".to_string()) }),
                ],
              },
            ),
            (
              "sleep_and_pow".to_string(),
              Func {
                in_vars: vec![InVar("a", Type::UInt64), InVar("b", Type::UInt64)],
                out: Type::UInt64,
                locals: vec![LocalVar("pow", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::ScheduleTimer {
                      ms: LogicalTimeAbsoluteMs(20),
                      next: StepId::new("await"),
                      future_id: FutureLabel::new("sleep_and_pow_entry_future"),
                    },
                  ),
                  (
                    StepId::new("await"),
                    Step::Await(AwaitSpec {
                      bind: None,
                      ret_to: StepId::new("calc"),
                      future_id: FutureLabel::new("sleep_and_pow_entry_future"),
                    }),
                  ),
                  (
                    StepId::new("calc"),
                    Step::RustBlock {
                      binds: vec!["pow".to_string()],
                      code: "a.pow(b as u32)".to_string(),
                      next: StepId::new("return".to_string()),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("pow".to_string()) }),
                ],
              },
            ),
          ]),
        },
      ),
      (
        FiberType::new("order_book"),
        Fiber {
          fibers_limit: 1,
          heap: HashMap::from([
            ("bids_prices".to_string(), Type::MaxQueue(Box::new(Type::UInt64))),
            ("asks_prices".to_string(), Type::MinQueue(Box::new(Type::UInt64))),
            (
              "bids_by_price".to_string(),
              Type::Map(Box::new(Type::UInt64), Box::new(Type::Array(Box::new(Type::Custom("Order".to_string()))))),
            ),
            (
              "asks_by_price".to_string(),
              Type::Map(Box::new(Type::UInt64), Box::new(Type::Array(Box::new(Type::Custom("Order".to_string()))))),
            ),
            (
              "orders_index".to_string(),
              Type::Map(Box::new(Type::UInt64), Box::new(Type::Custom("OrderIndex".to_string()))),
            ),
          ]),
          in_messages: vec![],
          funcs: HashMap::from([
            (
              "main".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Void,
                locals:vec![],
                entry: StepId::new("entry"),
                steps: vec![(StepId::new("entry"), Step::ReturnVoid)],
              },
            ),
            (
              "add_buy".to_string(),
              Func {
                in_vars: vec![InVar("id", Type::UInt64), InVar("price", Type::UInt64), InVar("qty", Type::UInt64)],
                out: Type::Array(Box::new(Type::Custom("Trade".to_string()))),
                locals: vec![LocalVar("result", Type::Array(Box::new(Type::Custom("Trade".to_string()))))],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["result".to_string()],
                      code: r#"
let ob = &mut heap.orderBook;
let mut remaining = qty;
let mut trades: Vec<Trade> = Vec::new();

// Clean stale top asks and match while price allows
loop {
  // Find current best ask
  let best_ask = loop {
    if let Some(top) = ob.asksPrices.peek() {
      let p = top.0;
      if let Some(level) = ob.asksByPrice.get(&p) {
        if !level.is_empty() { break Some(p); }
      }
      // stale level
      ob.asksPrices.pop();
      continue;
    } else { break None; }
  };

  match best_ask {
    Some(ap) if ap <= price && remaining > 0 => {
      // Execute against this level FIFO
      if let Some(level) = ob.asksByPrice.get_mut(&ap) {
        while remaining > 0 && !level.is_empty() {
          let maker = &mut level[0];
          if maker.qty <= remaining {
            let trade_qty = maker.qty;
            remaining -= trade_qty;
            trades.push(Trade { price: ap, qty: trade_qty, takerId: id, makerId: maker.id });
            level.remove(0);
          } else {
            maker.qty -= remaining;
            trades.push(Trade { price: ap, qty: remaining, takerId: id, makerId: maker.id });
            remaining = 0;
          }
        }
        if level.is_empty() {
          ob.asksByPrice.remove(&ap);
        }
      }
      // continue loop to next level or exit if remaining==0
    }
    _ => break,
  }
}

// If remaining, add to bids book
if remaining > 0 {
  ob.bidsByPrice.entry(price).or_default().push(Order { id, price, qty: remaining });
  ob.bidsPrices.push(price);
  ob.ordersIndex.insert(id, OrderIndex { side: "buy".to_string(), price });
}

trades
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
                ],
              },
            ),
            (
              "add_sell".to_string(),
              Func {
                in_vars: vec![InVar("id", Type::UInt64), InVar("price", Type::UInt64), InVar("qty", Type::UInt64)],
                out: Type::Array(Box::new(Type::Custom("Trade".to_string()))),
                locals: vec![LocalVar("result", Type::Array(Box::new(Type::Custom("Trade".to_string()))))],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["result".to_string()],
                      code: r#"
let ob = &mut heap.orderBook;
let mut remaining = qty;
let mut trades: Vec<Trade> = Vec::new();

// Clean stale top bids and match while price allows
loop {
  // Find current best bid
  let best_bid = loop {
    if let Some(&bp) = ob.bidsPrices.peek() {
      if let Some(level) = ob.bidsByPrice.get(&bp) {
        if !level.is_empty() { break Some(bp); }
      }
      // stale
      ob.bidsPrices.pop();
      continue;
    } else { break None; }
  };

  match best_bid {
    Some(bp) if bp >= price && remaining > 0 => {
      if let Some(level) = ob.bidsByPrice.get_mut(&bp) {
        while remaining > 0 && !level.is_empty() {
          let maker = &mut level[0];
          if maker.qty <= remaining {
            let trade_qty = maker.qty;
            remaining -= trade_qty;
            trades.push(Trade { price: bp, qty: trade_qty, takerId: id, makerId: maker.id });
            level.remove(0);
          } else {
            maker.qty -= remaining;
            trades.push(Trade { price: bp, qty: remaining, takerId: id, makerId: maker.id });
            remaining = 0;
          }
        }
        if level.is_empty() { ob.bidsByPrice.remove(&bp); }
      }
    }
    _ => break,
  }
}

if remaining > 0 {
  ob.asksByPrice.entry(price).or_default().push(Order { id, price, qty: remaining });
  ob.asksPrices.push(std::cmp::Reverse(price));
  ob.ordersIndex.insert(id, OrderIndex { side: "sell".to_string(), price });
}

trades
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
                ],
              },
            ),
            (
              "cancel".to_string(),
              Func {
                in_vars: vec![InVar("id", Type::UInt64)],
                out: Type::UInt64, // 1 if canceled, 0 otherwise
                locals: vec![LocalVar("result", Type::UInt64)],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["result".to_string()],
                      code: r#"
let ob = &mut heap.orderBook;
let mut ok = 0u64;
if let Some(idx) = ob.ordersIndex.remove(&id) {
  let price = idx.price;
  if idx.side == "buy" {
    if let Some(level) = ob.bidsByPrice.get_mut(&price) {
      if let Some(pos) = level.iter().position(|o| o.id == id) { level.remove(pos); ok = 1; if level.is_empty() { ob.bidsByPrice.remove(&price); } }
    }
  } else {
    if let Some(level) = ob.asksByPrice.get_mut(&price) {
      if let Some(pos) = level.iter().position(|o| o.id == id) { level.remove(pos); ok = 1; if level.is_empty() { ob.asksByPrice.remove(&price); } }
    }
  }
}
ok
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
                ],
              },
            ),
            (
              "best_bid".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Option(Box::new(Type::UInt64)),
                locals: vec![LocalVar("result", Type::Option(Box::new(Type::UInt64)))],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["result".to_string()],
                      code: r#"
let ob = &mut heap.orderBook;
loop {
  if let Some(&bp) = ob.bidsPrices.peek() {
    if let Some(level) = ob.bidsByPrice.get(&bp) { if !level.is_empty() { break Some(bp); } }
    ob.bidsPrices.pop();
  } else { break None; }
}
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
                ],
              },
            ),
            (
              "best_ask".to_string(),
              Func {
                in_vars: vec![],
                out: Type::Option(Box::new(Type::UInt64)),
                locals: vec![LocalVar("result", Type::Option(Box::new(Type::UInt64)))],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["result".to_string()],
                      code: r#"
let ob = &mut heap.orderBook;
loop {
  if let Some(top) = ob.asksPrices.peek() {
    let ap = top.0; // Reverse(u64)
    if let Some(level) = ob.asksByPrice.get(&ap) { if !level.is_empty() { break Some(ap); } }
    ob.asksPrices.pop();
  } else { break None; }
}
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
                ],
              },
            ),
            (
              "top_n_depth".to_string(),
              Func {
                in_vars: vec![InVar("n", Type::UInt64)],
                out: Type::Custom("BookSnapshot".to_string()),
                locals: vec![LocalVar("result", Type::Custom("BookSnapshot".to_string()))],
                entry: StepId::new("entry"),
                steps: vec![
                  (
                    StepId::new("entry"),
                    Step::RustBlock {
                      binds: vec!["result".to_string()],
                      code: r#"
let ob = &mut heap.orderBook;

let mut bids_depth: Vec<Level> = Vec::new();
let mut asks_depth: Vec<Level> = Vec::new();

// Bids: highest first
{
  let mut tmp = ob.bidsPrices.clone();
  let mut seen = std::collections::HashSet::<u64>::new();
  while (bids_depth.len() as u64) < n {
    if let Some(bp) = tmp.pop() {
      if seen.contains(&bp) { continue; }
      if let Some(level) = ob.bidsByPrice.get(&bp) {
        if !level.is_empty() {
          let qty = level.iter().map(|o| o.qty).sum::<u64>();
          bids_depth.push(Level { price: bp, qty });
          seen.insert(bp);
        }
      }
    } else { break; }
  }
}

// Asks: lowest first
{
  let mut tmp = ob.asksPrices.clone();
  let mut seen = std::collections::HashSet::<u64>::new();
  while (asks_depth.len() as u64) < n {
    if let Some(std::cmp::Reverse(ap)) = tmp.pop() {
      if seen.contains(&ap) { continue; }
      if let Some(level) = ob.asksByPrice.get(&ap) {
        if !level.is_empty() {
          let qty = level.iter().map(|o| o.qty).sum::<u64>();
          asks_depth.push(Level { price: ap, qty });
          seen.insert(ap);
        }
      }
    } else { break; }
  }
}

BookSnapshot { bids: bids_depth, asks: asks_depth }
"#.to_string(),
                      next: StepId::new("return"),
                    },
                  ),
                  (StepId::new("return"), Step::Return { value: RetValue::Var("result".to_string()) }),
                ],
              },
            ),
          ]),
        },
      ),
    ]),
    types: vec![
      Type::Struct(
        "Order".to_string(),
        vec![
          StructField { name: "id".to_string(), ty: Type::UInt64 },
          StructField { name: "price".to_string(), ty: Type::UInt64 },
          StructField { name: "qty".to_string(), ty: Type::UInt64 },
        ],
        String::new(),
      ),
      Type::Struct(
        "Trade".to_string(),
        vec![
          StructField { name: "price".to_string(), ty: Type::UInt64 },
          StructField { name: "qty".to_string(), ty: Type::UInt64 },
          StructField { name: "taker_id".to_string(), ty: Type::UInt64 },
          StructField { name: "maker_id".to_string(), ty: Type::UInt64 },
        ],
        String::new(),
      ),
      Type::Struct(
        "OrderIndex".to_string(),
        vec![StructField { name: "side".to_string(), ty: Type::String }, StructField { name: "price".to_string(), ty: Type::UInt64 }],
        String::new(),
      ),
      Type::Struct(
        "Level".to_string(),
        vec![StructField { name: "price".to_string(), ty: Type::UInt64 }, StructField { name: "qty".to_string(), ty: Type::UInt64 }],
        String::new(),
      ),
      Type::Struct(
        "BookSnapshot".to_string(),
        vec![
          StructField { name: "bids".to_string(), ty: Type::Array(Box::new(Type::Custom("Level".to_string()))) },
          StructField { name: "asks".to_string(), ty: Type::Array(Box::new(Type::Custom("Level".to_string()))) },
        ],
        String::new(),
      ),
  ],
  }
}
