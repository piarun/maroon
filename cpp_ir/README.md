IR Schema: [human-readable](https://github.com/dimacurrentai/mrn-dsl-cpp-eval/blob/main/autogen/ir_schema.md), [`ir_schema.rs`](https://github.com/dimacurrentai/mrn-dsl-cpp-eval/blob/main/autogen/ir_schema.rs).

NOTE TO ADD:

There is an implicit invariant in the IR schema that the `init` value for `Block`-scoped variables should be missing only for the top-level block of the function, and only for the first N variables, where N is the number of elements in the `types` of this function.

Moreover, these types should match, since `args` get passed as the first N top-most "in-function" on-stack vars.

ANOTHER NOTE:

We're about to have destructors (or the `Drop` "trait") to confirm the variables are deleted upon leaving the scope, not upon entering the "next" one.

TODO BEFORE CHECKING IN

- protect the branch

ADD THE NOTE THAT:

- obviously the `.mrn` files should be renamed to some `.mrn.h`, since they will eventually be replaced.
- however, the IR represented as the JSON — with the instructions on what is the test to be run, right within the JSON — should be successfully parsed, executed, and test-passed by the C++ code too!
- TODO(dkorolev): and run different maroon topologies in different threads, for proper performance testing
- TODO(dkorolev): Add the pure C version one day! =)

# `maroon-dsl`

So I figured I'll sketch up the JSON version of the IR to be ultimately interpreted and/or transpiled into C++ (or eventually C / LLVM) for performance.

## Goals

The goals, in the following order of importance, are:

1. **Have the JSON schema for the IR that we can work with, or iterate from.** This is important since we're getting into the weeds now: types of `await` / `select`, batch and grouped side effects, futures and their types and lifetimes, exceptions, etc. Also, I want to extend the "JSON IR DSL" further, to allow externally scheduled "events", multiple maroon programs to talk to each other — for self-tests.
2. **Begin implementing the C/C++ "target", so that we can measure CPU utilization and performance.** This will come in handy when we'll need to a) test against Rust, b) test transpilation vs. interpreteation, c) test WASM, and d) ultimately, justify the 100M+ transactions per second figure.
3. **Have a quick way to "write the DSL" before we have the, well, DSL.** Ultimately, we will have a DSL — likely as a Pest grammar, plus a Rust-built "frontend compiler", to convert it into the, well, IR (JSON), and packaged nicely in various ways: Docker container, WASM / static HTML+JS page, etc. But for now I want a quick&ugly way for our prototypes.

## Plan

I plan to start from (3) above, introducing a, well, duh, the "DSL" based on the C preprocessor. And I'll write plenty of test cases in this "DSL" until we ultimately convert them to the Pest-based grammar with our own syntax.

Also, the inner code blocks will stay just that: code blocks. As we agreed, we'll manage the "inner-", "statement-level" Pest grammar later on. For now it's just for us, so we can trust our own tests to be free from exploits.

Perhaps if I do this part of the job well enough, these tests will live on for longer, or even forever, as Github actions, to confirm that the IR JSON schema is what it should be, and that it is executed 100% as expected.

## Features

To be feature-complete I'll need to support:

- [x] Basic trivial syntax for "code blocks".
- [x] Stack variables.
- [x] Code blocks, sequential and hierarchical.
- [ ] Heap variables.
- [ ] Futures, of various types.
- [ ] Timeouts on `await`-s and `select`-s (or perhaps I'll group them into one "construct").
- [ ] Variant/sum/enum types and the "match" construct.
- [ ] The means to register "endpoints" and "call" them — from the outside.
- [ ] The "external side effects", and their management, across Maroon environments and outside.

## Less structured TODOs

- have the future declared on the stack
  - perhaps have some debug_futures on/off, as part of full dump
- do not forget ticks as part of setting timeouts / quantization

- dump the schema and have a test for that! (Via diff -w)

- move to `size_t` when we're dealing with indexes, there's a discrepancy now, and sometimes it's `uint32_t`
