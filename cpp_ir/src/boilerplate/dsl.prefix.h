// TODO(dkorolev): Rename `MAROON` into `NMSPC` some time soon.

#define CURRENT_FOR_CPP14

#include "../src/ir.h"

#include "../current/typesystem/serialization/json.h"

struct Ctx final {
  MaroonIRScenarios out;

  std::string current_maroon_name;
  std::string current_type_name;
  std::string current_fiber_name;
  std::string current_function_name;

  uint32_t next_placeholder_idx = 0;
  std::vector<std::pair<uint32_t, size_t>> blocks_stack;  // Where to insert the newly completed blocks.

  std::vector<std::unique_ptr<MaroonIRBlock>> current_fn_blocks_stack;

  // The value is the first line index where this `OPTIONAL<>` is used.
  std::map<std::string, std::map<std::string, uint32_t>> optional_types_per_maroon;

  bool finalized = false;

  bool InFunction() const { return !current_fn_blocks_stack.empty(); }

  void EnterFunction(MaroonIRFunction& fn, uint32_t line) {
    if (!current_fn_blocks_stack.empty()) {
      std::cerr << "NE" << std::endl;
      std::exit(1);
    }
    current_fn_blocks_stack.push_back(std::unique_ptr<MaroonIRBlock>(new MaroonIRBlock()));
    current_fn_blocks_stack.back()->line = line;
  }

  void LeaveFunction() {
    if (current_fn_blocks_stack.size() != 1) {
      std::cerr << "Internal invariant failed: Should be at exactly one block depth at function end." << std::endl;
    }
    out.maroon[current_maroon_name].fibers[current_fiber_name].functions[current_function_name].body =
        std::move(*current_fn_blocks_stack.back());
    current_fn_blocks_stack.clear();
    current_function_name = "";
  }

  uint32_t EnterBlock(uint32_t line) {
    MaroonIRBlockPlaceholder _p;
    auto const key = _p._idx = ++next_placeholder_idx;
    _p.line = line;
    size_t const saved_index = AddToBlock(std::move(_p));
    blocks_stack.push_back({key, saved_index});
    current_fn_blocks_stack.push_back(std::unique_ptr<MaroonIRBlock>(new MaroonIRBlock()));
    return key;
  }

  size_t AddToBlock(MaroonIRStmtOrBlock c) {
    size_t i = current_fn_blocks_stack.back()->code.size();
    current_fn_blocks_stack.back()->code.push_back(std::move(c));
    return i;
  }

  MaroonIRStmtOrBlock ExtractLastStmt() {
    MaroonIRStmtOrBlock res = std::move(current_fn_blocks_stack.back()->code.back());
    current_fn_blocks_stack.back()->code.pop_back();
    return res;
  }

  size_t BlocksDepth() const { return current_fn_blocks_stack.size(); }

  void AddVarToBlock(MaroonIRVar var) { current_fn_blocks_stack.back()->vars.push_back(std::move(var)); }

  void AddArgToFunction(std::string const& type) {
    out.maroon[current_maroon_name].fibers[current_fiber_name].functions[current_function_name].args.push_back(type);
  }

  void MarkInnerBlockAsCompleted(size_t user_key) {
    if (blocks_stack.empty()) {
      std::cerr << "WTF0!" << std::endl;
      std::exit(1);
    }
    auto key_idx = blocks_stack.back();
    auto key = key_idx.first;
    auto idx = key_idx.second;
    blocks_stack.pop_back();
    if (key != user_key) {
      std::cerr << "WTF1!" << std::endl;
      std::exit(1);
    }
    std::unique_ptr<MaroonIRBlock> blk = std::move(current_fn_blocks_stack.back());
    current_fn_blocks_stack.pop_back();
    if (current_fn_blocks_stack.back()->code.empty()) {
      std::cerr << "WTF2!" << std::endl;
      std::exit(1);
    }
    if (idx >= current_fn_blocks_stack.back()->code.size()) {
      std::cerr << "WTF3" << std::endl;
      std::exit(1);
    }
    if (!Exists<MaroonIRBlockPlaceholder>(current_fn_blocks_stack.back()->code[idx])) {
      std::cerr << "WTF4!" << std::endl;
      std::exit(1);
    }
    if (Value<MaroonIRBlockPlaceholder>(current_fn_blocks_stack.back()->code[idx])._idx != key) {
      std::cerr << "WTF5!" << std::endl;
      std::exit(1);
    }
    blk->line = Value<MaroonIRBlockPlaceholder>(current_fn_blocks_stack.back()->code[idx]).line;

    current_fn_blocks_stack.back()->code[idx] = std::move(*blk);
  }

  void ConsiderOptionalType(std::string const& type, uint32_t line) {
    uint32_t& placeholder = optional_types_per_maroon[current_maroon_name][type];
    if (!placeholder) {
      placeholder = line;
    }
  }

  void Finalize() {
    if (finalized) {
      std::cerr << "Internal error, `Finalize()` already called." << std::endl;
      std::exit(1);
    }
    finalized = true;

    for (auto const cit : optional_types_per_maroon) {
      auto& maroon_types = out.maroon[cit.first].types;
      for (auto const& t : cit.second) {
        std::string const name = "OPTIONAL_" + t.first;
        if (maroon_types.count(name)) {
          std::cerr << "Internal error, type `" << name << "` should not be defined explicitly." << std::endl;
          std::exit(1);
        }
        MaroonIRType out_t;
        out_t.line = t.second;
        MaroonIRTypeDefOptional def;
        def.type = t.first;
        out_t.def = std::move(def);
        maroon_types[name] = std::move(out_t);
      }
    }
  }
};

struct RegisterMaroon final {
  Ctx& ctx;
  bool entered = false;

  RegisterMaroon(Ctx& ctx, std::string const& name, uint32_t line) : ctx(ctx) {
    if (ctx.out.maroon.count(name)) {
      std::cerr << "`MAROON(" << name << ")` is defined more than once." << std::endl;
      std::exit(1);
    }
    ctx.current_maroon_name = name;
    ctx.out.maroon[ctx.current_maroon_name].line = line;
  }

  ~RegisterMaroon() {
    if (!entered) {
      std::cerr << "FOO1" << std::endl;
      std::exit(1);
    }
  }

  void operator<<(std::function<void()> f) {
    if (ctx.current_maroon_name.empty()) {
      std::cerr << "FOO2" << std::endl;
      std::exit(1);
    }
    if (entered) {
      std::cerr << "FOO3" << std::endl;
      std::exit(1);
    }
    entered = true;
    f();
    ctx.current_maroon_name = "";
  }
};

struct RegisterFiber final {
  Ctx& ctx;
  bool entered = false;

  RegisterFiber(Ctx& ctx, std::string const& name, uint32_t line) : ctx(ctx) {
    if (ctx.current_maroon_name.empty()) {
      std::cerr << "`FIBER(" << name << ")` should be defined within some `MAROON()`." << std::endl;
      std::exit(1);
    }
    if (ctx.out.maroon[ctx.current_maroon_name].fibers.count(name)) {
      std::cerr << "`FIBER(" << name << ")` is defined more than once in `MAROON(" << ctx.current_maroon_name << ")`."
                << std::endl;
      std::exit(1);
    }
    ctx.current_fiber_name = name;
    ctx.out.maroon[ctx.current_maroon_name].fibers[ctx.current_fiber_name].line = line;
  }

  ~RegisterFiber() {
    if (!entered) {
      std::cerr << "BAR1" << std::endl;
      std::exit(1);
    }
  }

  void operator<<(std::function<void()> f) {
    if (ctx.current_fiber_name.empty()) {
      std::cerr << "BAR2" << std::endl;
      std::exit(1);
    }
    if (entered) {
      std::cerr << "BAR3" << std::endl;
      std::exit(1);
    }
    entered = true;
    f();
    ctx.current_fiber_name = "";
  }
};

struct RegisterType final {
  Ctx& ctx;
  bool entered = false;

  RegisterType(Ctx& ctx, std::string const& name, uint32_t line) : ctx(ctx) {
    if (ctx.current_maroon_name.empty()) {
      std::cerr << "`TYPE(" << name << ")` should be defined within some `MAROON()`." << std::endl;
      std::exit(1);
    }
    if (ctx.out.maroon[ctx.current_maroon_name].types.count(name)) {
      std::cerr << "Type `" << name << "` is defined more than once in `MAROON(" << ctx.current_maroon_name << ")`."
                << std::endl;
      std::exit(1);
    }
    ctx.current_type_name = name;
    auto& r = ctx.out.maroon[ctx.current_maroon_name].types[ctx.current_type_name];
    r.line = line;
    r.def = MaroonIRTypeDefStruct();
  }

  ~RegisterType() {
    if (!entered) {
      std::exit(1);
    }
  }

  void operator<<(std::function<void()> f) {
    if (ctx.current_type_name.empty()) {
      std::exit(1);
    }
    if (entered) {
      std::exit(1);
    }
    entered = true;
    f();
    ctx.current_type_name = "";
  }
};

struct RegisterEnum final {
  Ctx& ctx;
  bool entered = false;

  RegisterEnum(Ctx& ctx, std::string const& name, uint32_t line) : ctx(ctx) {
    if (ctx.current_maroon_name.empty()) {
      std::cerr << "`ENUM(" << name << ")` should be defined within some `MAROON()`." << std::endl;
      std::exit(1);
    }
    if (ctx.out.maroon[ctx.current_maroon_name].types.count(name)) {
      std::cerr << "Type `" << name << "` is defined more than once in `MAROON(" << ctx.current_maroon_name << ")`."
                << std::endl;
      std::exit(1);
    }
    ctx.current_type_name = name;
    auto& r = ctx.out.maroon[ctx.current_maroon_name].types[ctx.current_type_name];
    r.line = line;
    r.def = MaroonIRTypeDefEnum();
  }

  ~RegisterEnum() {
    if (!entered) {
      std::exit(1);
    }
  }

  void operator<<(std::function<void()> f) {
    if (ctx.current_type_name.empty()) {
      std::exit(1);
    }
    if (entered) {
      std::exit(1);
    }
    entered = true;
    f();
    ctx.current_type_name = "";
  }
};

struct RegisterMatchEnumStmt final {
  Ctx& ctx;

  MaroonIRMatchEnumStmt match;

  RegisterMatchEnumStmt(Ctx& ctx, std::string var, uint32_t line) : ctx(ctx) {
    match.line = line;
    match.var = var;
    ctx.current_fn_blocks_stack.push_back(std::unique_ptr<MaroonIRBlock>(new MaroonIRBlock()));
  }

  ~RegisterMatchEnumStmt() {
    ctx.current_fn_blocks_stack.pop_back();
    ctx.AddToBlock(std::move(match));
  }

  void AddArms(std::vector<MaroonIRMatchEnumStmtArm> arms) {
    for (auto& arm : arms) {
      MaroonIRMatchEnumStmtArm in = std::move(arm);

      if (Exists(in.capture)) {
        if (in.code.vars.empty()) {
          std::cerr << "Internal error: should have a var in arm's block." << std::endl;
          std::exit(1);
        }
        if (!Exists<MaroonIRVarEnumCaseCapture>(in.code.vars.back())) {
          std::cerr << "Internal error: should have a captured enum var in arm's block." << std::endl;
          std::exit(1);
        }
        Value<MaroonIRVarEnumCaseCapture>(in.code.vars.back()).src = match.var;
      }

      match.arms.push_back(std::move(in));
    }
  }
};

inline MaroonIRBlock AsBlock(Optional<std::string> capture, std::string key, uint32_t line, MaroonIRStmtOrBlock in) {
  MaroonIRBlock block;
  if (Exists<MaroonIRBlock>(in)) {
    block = std::move(Value<MaroonIRBlock>(in));
  } else {
    block.line = line;
    block.code.push_back(std::move(in));
  }
  if (Exists(capture)) {
    MaroonIRVarEnumCaseCapture captured_var;
    captured_var.name = std::move(Value(capture));
    captured_var.key = std::move(key);
    captured_var.src = "THIS_WILL_BE_REPLACED_BY_SOURCE_VAR_NAME";
    block.vars.push_back(std::move(captured_var));
  }
  return block;
}

inline MaroonIRMatchEnumStmtArm RegisterEnumArm(
    Ctx& ctx, std::string key, std::string capture, uint32_t line, std::function<void()> code) {
  // TODO(dkorolev): Will ultimately require a cleaner check that we're inside the `MATCH` construct; on `pest` level!
  MaroonIRMatchEnumStmtArm arm;
  arm.line = line;
  arm.key = key;
  arm.capture = capture;
  code();
  arm.code = AsBlock(std::move(capture), std::move(key), line, ctx.ExtractLastStmt());
  return arm;
}

inline MaroonIRMatchEnumStmtArm RegisterEnumDefaultArm(Ctx& ctx, uint32_t line, std::function<void()> code) {
  // TODO(dkorolev): Will ultimately require a cleaner check that we're inside the `MATCH` construct; on `pest` level!
  MaroonIRMatchEnumStmtArm arm;
  arm.line = line;
  code();
  arm.code = AsBlock(nullptr, "", line, ctx.ExtractLastStmt());
  return arm;
}

struct RegisterFn final {
  Ctx& ctx;
  bool entered = false;

  RegisterFn(Ctx& ctx, std::string const& name, Optional<std::string> return_type, uint32_t line) : ctx(ctx) {
    if (ctx.current_fiber_name.empty()) {
      std::cerr << "`FN(" << name << ")` should be defined within some `FIBER()`." << std::endl;
      std::exit(1);
    }
    if (ctx.out.maroon[ctx.current_maroon_name].fibers[ctx.current_fiber_name].functions.count(name)) {
      std::cerr << "`FN(" << name << ")` is defined more than once in `FIBER(" << ctx.current_fiber_name
                << ")` of `MAROON(" << ctx.current_maroon_name << ")`." << std::endl;
      std::exit(1);
    }
    MaroonIRFunction& fn = ctx.out.maroon[ctx.current_maroon_name].fibers[ctx.current_fiber_name].functions[name];
    fn.ret = std::move(return_type);
    ctx.current_function_name = name;
    fn.line = line;
    ctx.EnterFunction(fn, line);
  }

  ~RegisterFn() {
    if (!entered) {
      std::cerr << "MEH1" << std::endl;
      std::exit(1);
    }
  }

  void operator<<(std::function<void()> f) {
    if (entered) {
      std::cerr << "MEH2" << std::endl;
      std::exit(1);
    }
    entered = true;
    f();
    ctx.LeaveFunction();
  }
};

inline void RegisterStmt(Ctx& ctx, uint32_t line, std::string const& stmt) {
  if (!ctx.InFunction()) {
    std::cerr << "`STMT()` is only legal inside an `FN()`." << std::endl;
    std::exit(1);
  }

  MaroonIRStmt obj;
  obj.line = line;
  obj.stmt = stmt;

  ctx.AddToBlock(std::move(obj));
}

struct RegisterIf final {
  Ctx& ctx;

  RegisterIf(Ctx& ctx, std::string condition, std::function<void()> yes, std::function<void()> no, uint32_t line)
      : ctx(ctx) {
    if (!ctx.InFunction()) {
      std::cerr << "`IF()` is only legal inside an `FN()`." << std::endl;
      std::exit(1);
    }

    // NOTE(dkorolev): Trivially construct two blocks and then extract them.
    yes();
    no();
    MaroonIRIf cond;
    cond.line = line;
    cond.cond = condition;
    cond.no = ctx.ExtractLastStmt();
    cond.yes = ctx.ExtractLastStmt();
    ctx.AddToBlock(std::move(cond));
  }
};

struct RegisterBlock final {
  Ctx& ctx;
  uint32_t block_key;
  size_t save_stack_depth;

  bool entered = false;

  RegisterBlock(Ctx& ctx, uint32_t line) : ctx(ctx) {
    if (!ctx.InFunction()) {
      std::cerr << "`BLOCK()` is only legal inside an `FN()`." << std::endl;
      std::exit(1);
    }

    block_key = ctx.EnterBlock(line);
    save_stack_depth = ctx.BlocksDepth();
  }

  ~RegisterBlock() {
    if (!entered) {
      std::cout << "FFUUUUU1\n";
      std::exit(1);
    }
  }

  void operator<<(std::function<void()> f) {
    if (entered) {
      std::cout << "FFUUUUU2\n";
      std::exit(1);
    }
    entered = true;
    f();
    if (ctx.BlocksDepth() != save_stack_depth) {
      std::cerr << "Internal error, stack depth mismatch on closing the block." << std::endl;
      std::exit(1);
    }
    ctx.MarkInnerBlockAsCompleted(block_key);
  }
};

inline void SupportOptionalTypes(Ctx& ctx, std::string& type, uint32_t line) {
  static std::string const optional_prefix("OPTIONAL<");
  if (type.substr(0, optional_prefix.length()) == optional_prefix && type.back() == '>') {
    std::string inner = type.substr(optional_prefix.length());
    inner.pop_back();
    ctx.ConsiderOptionalType(inner, line);
    type = "OPTIONAL_" + inner;
  }
}

inline void RegisterVar(Ctx& ctx, std::string name, std::string type, std::string const& init, uint32_t line) {
  if (!ctx.InFunction()) {
    std::cerr << "`VAR()` is only legal inside an `FN()`." << std::endl;
    std::exit(1);
  }

  SupportOptionalTypes(ctx, type, line);

  MaroonIRVarRegular var;
  var.line = line;
  var.name = std::move(name);
  var.type = std::move(type);
  if (!init.empty() && init.front() == '(' && init.back() == ')') {
    var.init = init.substr(1u, init.length() - 2u);
  } else {
    var.init = init;
  }

  ctx.AddVarToBlock(std::move(var));
}

// TODO(dkorolev): Copy-pasted from `RegisterVar`, we can do better.
inline void RegisterArg(Ctx& ctx, std::string name, std::string type, uint32_t line) {
  if (!ctx.InFunction()) {
    std::cerr << "`ARG()` is only legal inside an `FN()`." << std::endl;
    std::exit(1);
  }

  MaroonIRVarFunctionArg var;
  var.line = line;
  var.name = std::move(name);
  var.type = std::move(type);

  ctx.AddArgToFunction(var.type);
  ctx.AddVarToBlock(std::move(var));
}

inline void RegisterField(Ctx& ctx, std::string name, std::string type, uint32_t line) {
  if (ctx.current_type_name.empty()) {
    std::cerr << "`FIELD()` is only legal inside `TYPE()`." << std::endl;
    std::exit(1);
  }

  SupportOptionalTypes(ctx, type, line);

  auto& p = ctx.out.maroon[ctx.current_maroon_name].types[ctx.current_type_name].def;
  if (!Exists<MaroonIRTypeDefStruct>(p)) {
    std::cerr << "`FIELD()` is only legal inside the type that is a proper `TYPE()`." << std::endl;
    std::exit(1);
  }
  MaroonIRTypeDefStructField f;
  f.name = std::move(name);
  f.type = std::move(type);
  Value<MaroonIRTypeDefStruct>(p).fields.push_back(std::move(f));
}

inline void RegisterCase(Ctx& ctx, std::string key, std::string type, uint32_t line) {
  if (ctx.current_type_name.empty()) {
    std::cerr << "`CASE()` is only legal inside `ENUM()`." << std::endl;
    std::exit(1);
  }

  SupportOptionalTypes(ctx, type, line);

  auto& p = ctx.out.maroon[ctx.current_maroon_name].types[ctx.current_type_name].def;
  if (!Exists<MaroonIRTypeDefEnum>(p)) {
    std::cerr << "`CASE()` is only legal inside the type that is a proper `ENUM()`." << std::endl;
    std::exit(1);
  }
  MaroonIRTypeDefEnumCase c;
  c.key = std::move(key);
  c.type = std::move(type);
  Value<MaroonIRTypeDefEnum>(p).cases.push_back(std::move(c));

  // TODO(dkorolev): Should allow no empty `ENUM`-s, right?
}

int main() {
  Ctx ctx;
