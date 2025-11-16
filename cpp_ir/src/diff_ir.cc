// NOTE(dkorolev): This is suboptimal, but it ensures the code builds just with `g++ src.cc`, w/o `-std=c++17`.
#define CURRENT_FOR_CPP14

#include <iostream>
#include <fstream>

#include "../current/bricks/dflags/dflags.h"
#include "../current/bricks/file/file.h"
#include "../current/typesystem/serialization/json.h"

#define NO_LINE_NUMBERS
#include "ir.h"

DEFINE_string(a, "", "One IR file as JSON.");
DEFINE_string(b, "", "Another IR file as JSON.");
DEFINE_bool(verbose, false, "Actually dump post-line-nullified JSONs.");

int main(int argc, char** argv) {
  ParseDFlags(&argc, &argv);

  if (FLAGS_a.empty() || FLAGS_b.empty()) {
    std::cerr << "The `--a` and `--b` parameters are required." << std::endl;
    std::exit(1);
  }

  MaroonIRScenarios a;
  MaroonIRScenarios b;
  using T = decltype(a);

  try {
    a = ParseJSON<T, JSONFormat::Minimalistic>(current::FileSystem::ReadFileAsString(FLAGS_a));
  } catch (current::Exception const&) {
    std::cerr << "Failed to read and parse the IR JSON from `" << FLAGS_a << "`." << std::endl;
    std::exit(1);
  }

  try {
    b = ParseJSON<T, JSONFormat::Minimalistic>(current::FileSystem::ReadFileAsString(FLAGS_b));
  } catch (current::Exception const&) {
    std::cerr << "Failed to read and parse the IR JSON from `" << FLAGS_b << "`." << std::endl;
    std::exit(1);
  }

  // Poor man's comparison.
  std::string const sa = JSON<JSONFormat::Minimalistic>(a);
  std::string const sb = JSON<JSONFormat::Minimalistic>(b);
  if (sa != sb) {
    std::cout << "The IR JSONs are not identical." << std::endl;
    if (FLAGS_verbose) {
      std::cout << std::endl << sa << std::endl << sb << std::endl << std::endl;
    }
    std::exit(1);
  }
}
