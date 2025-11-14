// NOTE(dkorolev): This is suboptimal, but it ensures the code builds just with `g++ src.cc`, w/o `-std=c++17`.
#define CURRENT_FOR_CPP14

#include <iostream>
#include <fstream>

#include "../current/bricks/dflags/dflags.h"
#include "../current/bricks/file/file.h"
#include "../current/typesystem/schema/schema.h"

#include "ir.h"

DEFINE_string(out, "/dev/stdout", "The output file to dump the Markdown schema of the IR into.");

int main(int argc, char** argv) {
  ParseDFlags(&argc, &argv);

  using current::reflection::Language;
  using current::reflection::SchemaInfo;
  using current::reflection::StructSchema;

  StructSchema struct_schema;
  struct_schema.AddType<MaroonIRTopLevel>();
  const SchemaInfo schema = struct_schema.GetSchemaInfo();

  current::FileSystem::WriteStringToFile(schema.Describe<Language::Markdown>(), FLAGS_out.c_str());
}
