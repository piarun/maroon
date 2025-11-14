#!/bin/bash

# NOTE(dkorolev): This file is run as the Github check, and it's best to install it as a git hook via:

CMD="""
ln -sf "../../cpp_ir/verify.sh" "../.git/hooks/pre-commit"
"""

set -e -o pipefail

echo 'Running `./verify.sh` ...'

for i in *.mrn ; do
  ./mrn2ir.sh "$i" --verify
done

for i in autogen/*.mrn.json ; do
  MRN="${i#autogen/}"
  MRN="${MRN/%.json}"
  if ! [ -f "$MRN" ] ; then
    echo "Seeing '$i' but no '$MRN', you may have moved or deleted some source without cleaning up 'autogen/'."
    exit 1
  fi
done

if ! [ -f autogen/ir_schema.md ] ; then
  echo 'The `autogen/ir_schema.md` file is missing, regenerate it by running `make`.'
  exit 1
fi

make autogen/output_schema.bin >/dev/null 2>&1
autogen/output_schema.bin >autogen/ir_schema.md.tmp

if ! diff -w autogen/ir_schema.md autogen/ir_schema.md.tmp ; then
  echo 'The `autogen/ir_schema.md` file is not what is should be, regenerate it by running `make`.'
  exit 1
fi

if ! [ -f autogen/ir_schema.rs ] ; then
  echo 'The `autogen/ir_schema.rs` file is missing, regenerate it by running `make`.'
  exit 1
fi

make autogen/output_schema.bin >/dev/null 2>&1
autogen/output_schema.bin --rust >autogen/ir_schema.rs.tmp

if ! diff -w autogen/ir_schema.rs autogen/ir_schema.rs.tmp ; then
  echo 'The `autogen/ir_schema.rs` file is not what is should be, regenerate it by running `make`.'
  exit 1
fi

echo 'Running `./verify.sh` : Success.'
