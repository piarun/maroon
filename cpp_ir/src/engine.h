// NOTE(dkorolev): This is the somewhat ugly piece of code to "execute" the "post-DSL" boilerplate.

#pragma once

#include <vector>
#include <functional>
#include <iostream>
#include <sstream>

#include "../current/bricks/exception.h"
#include "../current/typesystem/typename.h"

struct MaroonLegalInit final {};  // NOTE(dkorolev): Keeping this for verbosity of constructors.

struct MaroonTypeBase {
  virtual ~MaroonTypeBase() = default;
  virtual char const* const MAROON_type_name() const = 0;
  virtual void MAROON_display(std::ostream&) const = 0;
};

struct MAROON_TYPE_U64 final : MaroonTypeBase {
  uint64_t value;
  MAROON_TYPE_U64(MaroonLegalInit, uint64_t value = uint64_t()) : value(value) {}
  MAROON_TYPE_U64& operator=(uint64_t v) {
    value = v;
    return *this;
  }
  static char const* const MAROON_type_name_static() { return "U64"; }
  char const* const MAROON_type_name() const override { return "U64"; }
  void MAROON_display(std::ostream& os) const override { os << value; }
};

inline MAROON_TYPE_U64 U64(uint64_t v) { return MAROON_TYPE_U64(MaroonLegalInit(), v); }

struct MAROON_TYPE_BOOL final : MaroonTypeBase {
  bool value;
  MAROON_TYPE_BOOL(MaroonLegalInit, bool value = bool()) : value(value) {}
  MAROON_TYPE_BOOL& operator=(bool v) {
    value = v;
    return *this;
  }
  static char const* const MAROON_type_name_static() { return "BOOL"; }
  char const* const MAROON_type_name() const override { return "BOOL"; }
  void MAROON_display(std::ostream& os) const override { os << std::boolalpha << value; }
};

inline MAROON_TYPE_BOOL BOOL(bool v) { return MAROON_TYPE_BOOL(MaroonLegalInit(), v); }

#define MAROON_BASE_TYPES_CSV MAROON_TYPE_U64, MAROON_TYPE_BOOL

template <class F>
bool MAROON_standard_dispatch(current::variant::object_base_t* val, F&& f) {
  if (auto* instance = dynamic_cast<MAROON_TYPE_U64*>(val)) {
    f(val);
    return true;
  } else if (auto* instance = dynamic_cast<MAROON_TYPE_BOOL*>(val)) {
    f(val);
    return true;
  }
  return false;
}

// TODO(dkorolev): This is kinda ugly, although seemingly necessary — need to reconcile for the future.

#define DEFINE_BINARY_OP(type, op1, op2)                       \
  inline type operator op1(type const& lhs, type const& rhs) { \
    return type(MaroonLegalInit(), lhs.value op1 rhs.value);   \
  }                                                            \
  inline type& operator op2(type & lhs, type const& rhs) {     \
    lhs.value op2 rhs.value;                                   \
    return lhs;                                                \
  }

#define DEFINE_BOOLEAN_OP(type, op) \
  inline bool operator op(type const& lhs, type const& rhs) { return lhs.value op rhs.value; }

DEFINE_BINARY_OP(MAROON_TYPE_U64, +, +=)
DEFINE_BINARY_OP(MAROON_TYPE_U64, -, -=)
DEFINE_BINARY_OP(MAROON_TYPE_U64, *, *=)
DEFINE_BOOLEAN_OP(MAROON_TYPE_U64, ==)
DEFINE_BOOLEAN_OP(MAROON_TYPE_U64, !=)
DEFINE_BOOLEAN_OP(MAROON_TYPE_U64, <)
DEFINE_BOOLEAN_OP(MAROON_TYPE_U64, <=)
DEFINE_BOOLEAN_OP(MAROON_TYPE_U64, >)
DEFINE_BOOLEAN_OP(MAROON_TYPE_U64, >=)

struct MAROON_INSTANCE_NONE final {};
static MAROON_INSTANCE_NONE NONE;

// NOTE(dkorolev): This is ugly, but we can not initialize from other vars' values yet :-(
struct MAROON_INSTANCE_PLACEHOLDER final {};
static MAROON_INSTANCE_PLACEHOLDER _;

#define DEFINE_MAROON_OPTIONAL_TYPE(alias, inner)                                        \
  struct MAROON_TYPE_##alias final : MaroonTypeBase {                                    \
    Optional<MAROON_TYPE_##inner> value;                                                 \
    MAROON_TYPE_##alias(MaroonLegalInit, MAROON_INSTANCE_NONE) {}                        \
    MAROON_TYPE_##alias(MaroonLegalInit, MAROON_TYPE_##inner v) : value(std::move(v)) {} \
    MAROON_TYPE_##alias(MAROON_INSTANCE_NONE) {}                                         \
    MAROON_TYPE_##alias(MAROON_TYPE_##inner v) : value(std::move(v)) {}                  \
    MAROON_TYPE_##alias& operator=(MAROON_TYPE_##inner v) {                              \
      value = std::move(v);                                                              \
      return *this;                                                                      \
    }                                                                                    \
    MAROON_TYPE_##alias& operator=(MAROON_INSTANCE_NONE) {                               \
      value = nullptr;                                                                   \
      return *this;                                                                      \
    }                                                                                    \
    static char const* const MAROON_type_name_static() { return #inner; }                \
    char const* const MAROON_type_name() const override { return #inner; }               \
    void MAROON_display(std::ostream& os) const override {                               \
      if (Exists(value)) {                                                               \
        os << "Some(";                                                                   \
        Value(value).MAROON_display(os);                                                 \
        os << ')';                                                                       \
      } else {                                                                           \
        os << "None";                                                                    \
      }                                                                                  \
    }                                                                                    \
    bool _EXISTS() const { return Exists(value); }                                       \
    MAROON_TYPE_##inner const& _VALUE() const { return Value(value); }                   \
    MAROON_TYPE_##inner& _MUTATE() { return Value(value); }                              \
  }

template <class T>
bool EXISTS(T&& x) {
  return x._EXISTS();
}

template <class T>
decltype(std::declval<T const&>()._VALUE()) VALUE(T&& x) {
  // TODO(dkorolev): Need to handle errors / exceptions properly one day.
  return x._VALUE();
}

template <class T>
decltype(std::declval<T&>()._MUTATE()) MUTATE(T&& x) {
  // TODO(dkorolev): Need to handle errors / exceptions properly one day.
  return x._MUTATE();
}

class MaroonDefinition {
 public:
  virtual char const* const maroon_name() const = 0;

 protected:
  ~MaroonDefinition() = default;
};

// TODO(dkorolev): If we agree it's uint32_t, need to make sure the future compiler checks the size of the program.
enum class MaroonStateIndex : uint32_t;
enum class MaroonVarIndex : uint32_t;

struct ImplException : current::Exception {
  using current::Exception::Exception;
};

template <class, class...>
struct MaroonPackArgsImpl;

template <class OUT_TYPE, class... OUT_TYPES, typename IN_TYPE, typename... IN_TYPES>
struct MaroonPackArgsImpl<std::tuple<OUT_TYPE, OUT_TYPES...>, IN_TYPE, IN_TYPES...> final {
  static void DoIt(std::vector<std::unique_ptr<MaroonTypeBase>>& res, IN_TYPE&& x, IN_TYPES&&... xs) {
    res.push_back(std::make_unique<OUT_TYPE>(std::forward<IN_TYPE>(x)));
    MaroonPackArgsImpl<std::tuple<OUT_TYPES...>, IN_TYPES...>::DoIt(res, std::forward<IN_TYPES>(xs)...);
  }
};

template <>
struct MaroonPackArgsImpl<std::tuple<>> {
  static void DoIt(std::vector<std::unique_ptr<MaroonTypeBase>>&) {}
};

template <class OUT_TYPELIST, typename... IN_TYPES>
std::vector<std::unique_ptr<MaroonTypeBase>> pack_args(IN_TYPES&&... args) {
  std::vector<std::unique_ptr<MaroonTypeBase>> res;
  MaroonPackArgsImpl<OUT_TYPELIST, IN_TYPES...>::DoIt(res, std::forward<IN_TYPES>(args)...);
  return res;
}

enum TmpNextStatus { None = 0, Branch, Call, Return };

struct ImplResultCollector final {
  MaroonStateIndex next_idx_;
  TmpNextStatus status_ = TmpNextStatus::None;

  MaroonStateIndex call_idx_;
  std::string call_f_;
  MaroonVarIndex call_retval_var_idx_;

  std::vector<std::unique_ptr<MaroonTypeBase>> call_args_;

  bool has_retval_;
  std::unique_ptr<MaroonTypeBase> retval_;

  void branch(MaroonStateIndex idx) {
    if (status_ != TmpNextStatus::None) {
      CURRENT_THROW(ImplException("TODO(dkorolev): FIXME: Attempted to `IF()` in the wrong place."));
    }
    status_ = TmpNextStatus::Branch;
    next_idx_ = idx;
  }

  void call_ignore_return(size_t number_of_args,
                          MaroonStateIndex idx,
                          std::string f,
                          std::vector<std::unique_ptr<MaroonTypeBase>> args) {
    if (status_ != TmpNextStatus::None) {
      CURRENT_THROW(ImplException("TODO(dkorolev): FIXME: Attempted `CALL()` in the wrong place."));
    }
    if (args.size() != number_of_args) {
      // TODO(dkorolev): The error message should make sense. Including `file:line` perhaps.
      CURRENT_THROW(ImplException("WRONG NUMBER OF ARGS"));
    }
    status_ = TmpNextStatus::Call;
    call_idx_ = idx;
    call_f_ = std::move(f);
    call_retval_var_idx_ = static_cast<MaroonVarIndex>(-1);
    call_args_ = std::move(args);
  }

  void call_capture_return(MaroonVarIndex v,
                           size_t number_of_args,
                           MaroonStateIndex idx,
                           std::string f,
                           std::vector<std::unique_ptr<MaroonTypeBase>> args) {
    if (status_ != TmpNextStatus::None) {
      CURRENT_THROW(ImplException("TODO(dkorolev): FIXME: Attempted `CALL()` in the wrong place."));
    }
    if (args.size() != number_of_args) {
      // TODO(dkorolev): The error message should make sense. Including `file:line` perhaps.
      CURRENT_THROW(ImplException("WRONG NUMBER OF ARGS"));
    }
    status_ = TmpNextStatus::Call;
    call_idx_ = idx;
    call_f_ = std::move(f);
    call_retval_var_idx_ = v;
    call_args_ = std::move(args);
  }

  template <typename T_UNUSED_FUNCTION_RETURN_TYPE>
  void ret() {
    if (status_ != TmpNextStatus::None) {
      CURRENT_THROW(ImplException("TODO(dkorolev): FIXME: Attempted `RETURN()` in the wrong place."));
    }
    status_ = TmpNextStatus::Return;
    has_retval_ = false;
  }

  template <typename T_FUNCTION_RETURN_TYPE, typename T_ARG>
  void ret(T_ARG&& val) {
    static_assert(!std::is_same<T_FUNCTION_RETURN_TYPE, void>::value, "Can't `RETURN(...)` from a `unit` function.");
    if (status_ != TmpNextStatus::None) {
      CURRENT_THROW(ImplException("TODO(dkorolev): FIXME: Attempted `RETURN()` in the wrong place."));
    }
    status_ = TmpNextStatus::Return;
    has_retval_ = true;
    retval_ = std::make_unique<T_FUNCTION_RETURN_TYPE>(std::forward<T_ARG>(val));
  }

  TmpNextStatus status() const { return status_; }
};

struct ImplVar final {
  std::string name;
  std::unique_ptr<MaroonTypeBase> value;
};

struct ImplCallStackEntry final {
  MaroonStateIndex current_idx_;

  std::string f_;
  MaroonVarIndex call_retval_var_idx_;
  std::vector<ImplVar> vars_;

  size_t args_used_ = 0u;
  std::vector<std::unique_ptr<MaroonTypeBase>> args_;

  ImplCallStackEntry() = delete;
  explicit ImplCallStackEntry(MaroonStateIndex idx,
                              std::string f = "",
                              MaroonVarIndex call_retval_var_idx = static_cast<MaroonVarIndex>(-1))
      : current_idx_(idx), f_(std::move(f)), call_retval_var_idx_(call_retval_var_idx) {}

  ImplCallStackEntry(ImplCallStackEntry&&) = default;
  ImplCallStackEntry& operator=(ImplCallStackEntry&&) = default;

  ImplCallStackEntry(ImplCallStackEntry const&) = delete;
  ImplCallStackEntry& operator=(ImplCallStackEntry const&) = delete;
};

inline static std::string StripMaroonTypeNamePrefix(std::string const& s) {
  static std::string prefix = "MAROON_TYPE_";
  if (s.length() >= prefix.length() && s.substr(0u, prefix.length()) == prefix) {
    return s.substr(prefix.length());
  } else {
    return s;
  }
}

struct VariantCaseNameExtractorVisitor final {
  std::string name;
  template <class T>
  void operator()(T const&) {
    name = current::reflection::CurrentTypeName<T>();
  }
};

template <class T>
std::string VariantCaseNameAsString(T const& obj) {
  VariantCaseNameExtractorVisitor visitor;
  obj.Call(visitor);
  return visitor.name;
}

struct ImplEnv final {
  std::ostream& os_;

  std::vector<ImplCallStackEntry> call_stack_;

  explicit ImplEnv(std::ostream& os) : os_(os) {}

  template <typename T>
  void debug(T&& v, char const* const file, int line) {
    std::ostringstream oss;
    oss << std::forward<T>(v);
    std::string const s = oss.str();
    // std::cerr << "Impl DEBUG: " << s << " @ " << file << ':' << line << std::endl;
    // TODO(dkorolev): Tick index / time.
    os_ << s << std::endl;
  }

  template <typename T>
  void debug_expr(char const* const expr, T&& v, char const* const file, int line) {
    std::ostringstream oss;
    oss << expr << '=';
    v.MAROON_display(oss);
    std::string const s = oss.str();
    // std::cerr << "Impl DEBUG: " << s << " @ " << file << ':' << line << std::endl;
    // TODO(dkorolev): Tick index / time.
    os_ << s << std::endl;
  }

  void debug_dump_vars(char const* const file, int line) {
    std::ostringstream oss;
    do_debug_dump_vars(oss, call_stack_.back().vars_, file, line);
    std::string const s = oss.str();
    // std::cerr << "Impl VARS: " << s << " @ " << file << ':' << line << std::endl;
    // TODO(dkorolev): Tick index / time.
    os_ << s << std::endl;
  }

  void do_debug_dump_vars(std::ostringstream& oss, std::vector<ImplVar> const& vars, char const* const file, int line) {
    oss << '[';
    bool first = true;
    for (auto const& v : vars) {
      if (first) {
        first = false;
      } else {
        oss << ',';
      }
      oss << v.name << ':';
      v.value->MAROON_display(oss);
    }
    oss << ']';
  }

  void debug_dump_stack(char const* const file, int line) {
    std::ostringstream oss;
    oss << '<';
    bool first = true;
    for (auto const& v : call_stack_) {
      if (first) {
        first = false;
      } else {
        oss << ',';
      }
      if (!v.f_.empty()) {
        oss << v.f_ << '@';
      }
      do_debug_dump_vars(oss, v.vars_, file, line);
    }
    oss << '>';
    std::string const s = oss.str();
    // std::cerr << "Impl STACK: " << s << " @ " << file << ':' << line << std::endl;
    os_ << s << std::endl;
  }

  void DeclareVar(size_t idx, std::string name, std::unique_ptr<MaroonTypeBase> init) {
    if (idx != call_stack_.back().vars_.size()) {
      std::cerr << "Internal invariant error: corrupted stack." << std::endl;
      std::exit(1);
    }
    ImplVar var;
    var.name = std::move(name);
    var.value = std::move(init);
    call_stack_.back().vars_.push_back(std::move(var));
  }

  template <typename T_VAR>
  void DeclareFunctionArg(size_t idx, std::string name) {
    if (idx != call_stack_.back().vars_.size()) {
      std::cerr << "Internal invariant error: corrupted stack." << std::endl;
      std::exit(1);
    }
    if (call_stack_.back().args_used_ >= call_stack_.back().args_.size()) {
      std::cerr << "Internal invariant error: not enough args, should never happen." << std::endl;
      std::exit(1);
    }
    ImplVar var;
    var.name = std::move(name);
    // TODO(dkorolev): Check that we're not out of `args_used_`!
    var.value = std::move(call_stack_.back().args_[call_stack_.back().args_used_++]);
    if (std::string(T_VAR::MAROON_type_name_static()) != var.value->MAROON_type_name()) {
      std::cerr << "Internal error: function argument type does not match (should not happen)." << std::endl;
      std::exit(1);
    }
    call_stack_.back().vars_.push_back(std::move(var));
  }

  void DeclareCapturedAlias(size_t idx, std::string name) {
    if (idx != call_stack_.back().vars_.size()) {
      std::cerr << "Internal invariant error: corrupted stack." << std::endl;
      std::exit(1);
    }
    ImplVar var;
    var.name = std::move(name);
    // TODO(dkorolev): Uncertain if this is correct to just leave the var handing, but it is never accesssed,
    //                 this piece of logic with `vars_` is just to keep the counters of local vars in sync.
    call_stack_.back().vars_.push_back(std::move(var));
  }

  template <class T_VAR>
  T_VAR& AccessVar(size_t idx, char const* const name) {
    if (idx >= call_stack_.back().vars_.size()) {
      std::cerr << "Internal invariant error: var out of stack." << std::endl;
      std::exit(1);
    }
    if (call_stack_.back().vars_[idx].name != name) {
      std::cerr << "Internal invariant error: corrupted stack, at index " << idx << " expecting var " << name
                << ", have var " << call_stack_.back().vars_[idx].name << std::endl;
      std::exit(1);
    }
    auto& v = call_stack_.back().vars_[idx].value;
    T_VAR* instance = dynamic_cast<T_VAR*>(v.get());
    if (instance) {
      return *instance;
    } else {
      std::ostringstream oss;
      oss << "Attempted to use `" << name << "` of type `" << StripMaroonTypeNamePrefix(v->MAROON_type_name())
          << "` as `" << StripMaroonTypeNamePrefix(T_VAR::MAROON_type_name_static()) << "`.";
      CURRENT_THROW(ImplException(oss.str()));
    }
  }
};

using step_function_t = void (*)(ImplEnv& env, ImplResultCollector& result);
using vars_function_t = void (*)(ImplEnv& env);

struct MaroonStep final {
  step_function_t code;
  size_t num_vars_available_before_step;
  size_t num_vars_declared_for_step;
  vars_function_t new_vars;
};

#define DEBUG(s) MAROON_env.debug(s, __FILE__, __LINE__)
#define DEBUG_EXPR(s) MAROON_env.debug_expr(#s, s, __FILE__, __LINE__)
#define DEBUG_DUMP_VARS() MAROON_env.debug_dump_vars(__FILE__, __LINE__)
#define DEBUG_DUMP_STACK() MAROON_env.debug_dump_stack(__FILE__, __LINE__)

// NOTE(dkorolev): The ugly yet functional way to tell 1-arg vs. 2-args macros.
#define CALL_DISPATCH(_1, _2, _3, NAME, ...) NAME
#define CALL(...) CALL_DISPATCH(__VA_ARGS__, CALL3, CALL2, NONEXISTENT_CALL1)(__VA_ARGS__)

#define CALL2(f, args) \
  MAROON_result.call_ignore_return(NUMBER_OF_ARGS_##f, FN_##f, #f, pack_args<MAROON_F_ARGS_##f> args)
#define CALL3(v, f, args)                                                           \
  static_assert(std::is_same<MAROON_VAR_TYPE_##v, MAROON_F_RETURN_TYPE_##f>::value, \
                "Function call return type mismatch.");                             \
  MAROON_result.call_capture_return(                                                \
      MAROON_VAR_INDEX_##v, NUMBER_OF_ARGS_##f, FN_##f, #f, pack_args<MAROON_F_ARGS_##f> args)

#define RETURN(...) MAROON_result.ret<T_FUNCTION_RETURN_TYPE>(__VA_ARGS__)

template <class T_MAROON, class T_FIBER>
struct MaroonEngine final {
  static_assert(std::is_base_of<MaroonDefinition, T_MAROON>::value, "");
  // TODO(dkorolev): Perhaps add a `static_assert` that this `T_FIBER` is from the right `T_MAROON`.

  // NOTE(dkorolev): This will not compile if there's no `main` in the `global` fiber.
  static_assert(T_FIBER::kIsFiber, "");

  static_assert(T_FIBER::NUMBER_OF_ARGS_main == 0, "");

  std::pair<std::string, std::string> run() {
    try {
      std::ostringstream oss;
      ImplEnv env(oss);

      auto const fiber_steps = T_FIBER::MAROON_steps();

      // TODO(dkorolev): Proper engine =)
      env.call_stack_.push_back(ImplCallStackEntry(T_FIBER::FN_main));
      while (!env.call_stack_.empty()) {
        if (static_cast<uint32_t>(env.call_stack_.back().current_idx_) >= T_FIBER::kStepsCount) {
          CURRENT_THROW(ImplException("Need `RETURN()` at least at the last `STMT()` of the `FN()`."));
        }
        MaroonStep const& step = fiber_steps[static_cast<uint32_t>(env.call_stack_.back().current_idx_)];

        if (env.call_stack_.back().vars_.size() < step.num_vars_available_before_step) {
          std::cerr << "Internal invariant failed: pre-step vars count mismatch." << std::endl;
          std::exit(1);
        }

        if (env.call_stack_.back().vars_.size() > step.num_vars_available_before_step) {
          // Destruct what is no longer needed.
          env.call_stack_.back().vars_.resize(step.num_vars_available_before_step);
        }

        step.new_vars(env);

        if (env.call_stack_.back().vars_.size() !=
            step.num_vars_available_before_step + step.num_vars_declared_for_step) {
          std::cerr << "Internal invariant failed: intra-step vars count mismatch." << std::endl;
          std::exit(1);
        }

        ImplResultCollector result;
        step.code(env, result);

        if (result.status() == TmpNextStatus::Branch) {
          env.call_stack_.back().current_idx_ = static_cast<MaroonStateIndex>(result.next_idx_);
        } else if (result.status() == TmpNextStatus::Call) {
          env.call_stack_.back().current_idx_ =
              static_cast<MaroonStateIndex>(static_cast<uint32_t>(env.call_stack_.back().current_idx_) + 1);
          env.call_stack_.push_back(
              ImplCallStackEntry(result.call_idx_, std::move(result.call_f_), result.call_retval_var_idx_));
          env.call_stack_.back().args_ = std::move(result.call_args_);
        } else if (result.status() == TmpNextStatus::Return) {
          auto const retval_var_idx = env.call_stack_.back().call_retval_var_idx_;
          env.call_stack_.pop_back();
          if (result.has_retval_) {
            if (env.call_stack_.empty()) {
              std::cerr << "Internal error: returning from the top-level of the fiber should have no value."
                        << std::endl;
              std::exit(1);
            }
            if (retval_var_idx != static_cast<MaroonVarIndex>(-1)) {
              env.call_stack_.back().vars_[static_cast<size_t>(retval_var_idx)].value = std::move(result.retval_);
            }
            // NOTE(dkorolev): Perfectly fine to ignore the returned value!
          } else if (retval_var_idx != static_cast<MaroonVarIndex>(-1)) {
            CURRENT_THROW(ImplException("A return value must have been provided."));
          }
        } else {
          // Assume the default is `next`.
          env.call_stack_.back().current_idx_ =
              static_cast<MaroonStateIndex>(static_cast<uint32_t>(env.call_stack_.back().current_idx_) + 1);
        }

        // TODO(dkorolev): Clean up the vars here, not up there.
        // TODO(dkorolev): This will be possible to check once we have object with destructors / `drop`!
      }

      return {oss.str(), ""};
    } catch (ImplException const& e) {
      return {"", e.OriginalDescription()};
    }
  }
};
