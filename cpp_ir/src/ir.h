// This is the schema for the IR.
//
// Life of a test from this directory: (TODO(dkorolev): This directory, or perhaps `cases/`?)
// 1. First from the DSL into the IR-generating code (via the C preprocessor).
// 2. Then from the IR-generating code to the JSON with this DSL file in the IR format (by running the generating code).
// 3. Then from this JSON to the new piece of code, C++ for now, which will be executed (by running `autogen.cc`).
// 4. And finally the resulting generated C++ code is run, as the unit test — because it is a unit test!
//
// TODO(dkorolev): Explain directories and what builds what.
// TODO(dkorolev): Have this header copied to every source/header file here.
// TODO(dkorolev): Turn the above into README.md for this directory and reference it.

#pragma once

#include "../current/typesystem/struct.h"
#include "../current/typesystem/variant.h"

// TODO(dkorolev): Add a `make` target to generate the `.md` describing this schema.

CURRENT_STRUCT(MaroonIRVarRegular) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(name, std::string);
  CURRENT_FIELD(type, std::string);  // NOTE(dkorolev): Would love to `enum` this somehow.
  CURRENT_FIELD(init, std::string);  // NOTE(dkorolev): Not sure I like this as `string`, but works for now.
};

CURRENT_STRUCT(MaroonIRVarFunctionArg) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(name, std::string);
  CURRENT_FIELD(type, std::string);  // NOTE(dkorolev): Would love to `enum` this somehow.
};

CURRENT_STRUCT(MaroonIRVarEnumCaseCapture) {
  CURRENT_FIELD(name, std::string);
  CURRENT_FIELD(key, std::string);
  CURRENT_FIELD(src, std::string);
};

CURRENT_VARIANT(MaroonIRVar, MaroonIRVarRegular, MaroonIRVarFunctionArg, MaroonIRVarEnumCaseCapture);

CURRENT_FORWARD_DECLARE_STRUCT(MaroonIRStmt);
CURRENT_FORWARD_DECLARE_STRUCT(MaroonIRIf);
CURRENT_FORWARD_DECLARE_STRUCT(MaroonIRBlock);
CURRENT_FORWARD_DECLARE_STRUCT(MaroonIRMatchEnumStmt);

// TODO(dkorolev): Refactor to remove this one.
CURRENT_STRUCT(MaroonIRBlockPlaceholder) {  // NOTE(dkorolev): To avoid pointers.
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(_idx, uint32_t);
};
CURRENT_VARIANT(
    MaroonIRStmtOrBlock, MaroonIRStmt, MaroonIRIf, MaroonIRBlock, MaroonIRMatchEnumStmt, MaroonIRBlockPlaceholder);

// A piece of "O(1)" code to execute.
// TODO(dkorolev): Handle the `AWAIT`-condition separately here, on the type system level.
// TODO(dkorolev): As in, add fields for `await`, a variant of `await / next / done`.
CURRENT_STRUCT(MaroonIRStmt) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(stmt, std::string);
};

CURRENT_STRUCT(MaroonIRIf) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(cond, std::string);
  CURRENT_FIELD(yes, MaroonIRStmtOrBlock);
  CURRENT_FIELD(no, MaroonIRStmtOrBlock);
};

// A set of variables plus the sequence of statements, possibly nested.
// TODO(dkorolev): We now have hoisting, like in the 1st version of JavaScript, lolwut! Fix this.
CURRENT_STRUCT(MaroonIRBlock) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(vars, std::vector<MaroonIRVar>);
  CURRENT_FIELD(code, std::vector<MaroonIRStmtOrBlock>);
};

// TODO(dkorolev): Think if this IR should think of mutability / immutability of enum cases.
CURRENT_STRUCT(MaroonIRMatchEnumStmtArm) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS

  // NOTE(dkorolev): This JSON construct creates indirect dependencies:
  // 1) At most one default arm.
  // 2) All arms of valid types.
  // 3) No multiple arms for the same case.
  CURRENT_FIELD(key, Optional<std::string>);  // Which enum case should match. Unset for default arm.

  // NOTE(dkorolev): Another indirect dependency: var names should match, here and in the block.
  // NOTE(dkorolev): And another indirect dependency: with no `key` there should be no `var`.
  CURRENT_FIELD(capture, Optional<std::string>);  // If the value should be captured, what name to capture it under.

  CURRENT_FIELD(code, MaroonIRBlock);
};

CURRENT_STRUCT(MaroonIRMatchEnumStmt) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(var, std::string);
  CURRENT_FIELD(arms, std::vector<MaroonIRMatchEnumStmtArm>);
};

CURRENT_STRUCT(MaroonIRFunction) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS

  // The return type.
  CURRENT_FIELD(ret, Optional<std::string>);

  // NOTE(dkorolev): The first `args.length` vars of the top-level IR block of `body` are the args.
  // NOTE(dkorolev): Note that the top-level block of `body` can have more vars.
  // NOTE(dkorolev): In this case, the extra vars would need to have init values, while args do not.
  CURRENT_FIELD(args, std::vector<std::string>);

  CURRENT_FIELD(body, MaroonIRBlock);
};

CURRENT_STRUCT(MaroonIRFiber) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  // TODO(dkorolev): Heap type.
  CURRENT_FIELD(functions, (std::map<std::string, MaroonIRFunction>));
};

CURRENT_STRUCT(MaroonIRTypeDefStructField) {
  CURRENT_FIELD(name, std::string);
  CURRENT_FIELD(type, std::string);
};

CURRENT_STRUCT(MaroonIRTypeDefStruct) { CURRENT_FIELD(fields, std::vector<MaroonIRTypeDefStructField>); };

CURRENT_STRUCT(MaroonIRTypeDefEnumCase) {
  CURRENT_FIELD(key, std::string);
  CURRENT_FIELD(type, std::string);
};

CURRENT_STRUCT(MaroonIRTypeDefEnum) { CURRENT_FIELD(cases, std::vector<MaroonIRTypeDefEnumCase>); };

CURRENT_STRUCT(MaroonIRTypeDefOptional) { CURRENT_FIELD(type, std::string); };

CURRENT_VARIANT(MaroonIRTypeDef, MaroonIRTypeDefStruct, MaroonIRTypeDefEnum, MaroonIRTypeDefOptional);

CURRENT_STRUCT(MaroonIRType) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(def, MaroonIRTypeDef);
};

CURRENT_STRUCT(MaroonIRNamespace) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  // TODO(dkorolev): Support types, heaps, etc.
  // CURRENT_FIELD(types, ...);
  // NOTE(dkorolev): The `global` fiber should absolutely exist, others optional.
  CURRENT_FIELD(fibers, (std::map<std::string, MaroonIRFiber>));
  CURRENT_FIELD(types, (std::map<std::string, MaroonIRType>));
};

CURRENT_STRUCT(MaroonTestCaseRunFiber) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(maroon, std::string);
  CURRENT_FIELD(fiber, std::string);
  CURRENT_FIELD(golden_output, std::vector<std::string>);
};

CURRENT_STRUCT(MaroonTestCaseFiberShouldThrow) {
#ifndef NO_LINE_NUMBERS
  CURRENT_FIELD(line, uint32_t);
  void SetLine(uint32_t l) { line = l; }
#else   // NO_LINE_NUMBERS
  void SetLine(uint32_t) {}
#endif  // NO_LINE_NUMBERS
  CURRENT_FIELD(maroon, std::string);
  CURRENT_FIELD(fiber, std::string);
  CURRENT_FIELD(error, std::string);
};

CURRENT_VARIANT(MaroonTestCase, MaroonTestCaseRunFiber, MaroonTestCaseFiberShouldThrow);

CURRENT_STRUCT(MaroonIRScenarios) {
  CURRENT_FIELD(src, std::string);
  CURRENT_FIELD_DESCRIPTION(src, "The source `.mrn` file.");
  CURRENT_FIELD(maroon, (std::map<std::string, MaroonIRNamespace>));
  CURRENT_FIELD(tests, std::vector<MaroonTestCase>);
};

using MaroonIRTopLevel = MaroonIRScenarios;
