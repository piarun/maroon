// NOTE(dkorolev): This is suboptimal, but it ensures the code builds just with `g++ src.cc`, w/o `-std=c++17`.
#define CURRENT_FOR_CPP14

#include <iostream>
#include <fstream>

#include "../current/bricks/dflags/dflags.h"
#include "../current/bricks/file/file.h"
#include "../current/typesystem/schema/schema.h"

#include "ir.h"

DEFINE_string(out, "/dev/stdout", "The output file to dump the schema of the IR into.");
DEFINE_bool(rust, false, "Set to true to output Rust schema, keep at false to output Markdown schema..");

int main(int argc, char** argv) {
  ParseDFlags(&argc, &argv);

  using current::reflection::Language;
  using current::reflection::SchemaInfo;
  using current::reflection::StructSchema;

  StructSchema struct_schema;
  struct_schema.AddType<MaroonIRTopLevel>();
  const SchemaInfo schema = struct_schema.GetSchemaInfo();

  if (!FLAGS_rust) {
    current::FileSystem::WriteStringToFile(schema.Describe<Language::Markdown>(), FLAGS_out.c_str());
  } else {
    current::FileSystem::WriteStringToFile(schema.Describe<Language::Rust>(), FLAGS_out.c_str());
  }
}
