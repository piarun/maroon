#!/bin/bash

set -e -o pipefail

VERIFY=0
if [ "$2" == "--verify" ] ; then
  make autogen/diff_ir.bin >/dev/null
  VERIFY=1
fi

JQ="cat"
if jq --version >/dev/null 2>&1 ; then
  JQ="jq ."
fi

if [ "$1" == "" ] ; then
  echo 'Need an argument, the `.mrn` file in this directory.'
  exit 1
fi

IN="${1%.mrn}"

if ! [ -f "$IN.mrn" ] ; then
  echo "The $IN.mrn file should exist."
  exit 1
fi

mkdir -p autogen

# Start preparing the IR-generating code.
cp src/boilerplate/dsl.prefix.h autogen/"$IN.mrn.cc"

# Run the preprocessor on this IR-generating code to turn it into what will build to ultimately produce the JSON IR.
echo '#include "../src/boilerplate/dsl.spec.h"' >"autogen/$IN.mrn.h"
echo 'MAROON_SOURCE("'$IN.mrn'");' >>"autogen/$IN.mrn.h"
echo '#line 1' >>"autogen/$IN.mrn.h"
cat "$IN.mrn" >>"autogen/$IN.mrn.h"
g++ -E "autogen/$IN.mrn.h" 2>/dev/null | grep -v '^#' | grep -v '^$' >>autogen/"$IN.mrn.cc"
echo -e "  ;\n  ctx.Finalize(); std::cout << JSON<JSONFormat::Minimalistic>(ctx.out) << std::endl;\n}" >>autogen/"$IN.mrn.cc"

# Build and run the source file that was just put together to generate the JSON IR.
if ! g++ autogen/"$IN.mrn.cc" -o autogen/"$IN.mrn.bin" ; then
  echo "Failed to build."
  exit 1
fi

autogen/"$IN.mrn.bin" | $JQ > autogen/"$IN.mrn.json.tmp"

if [ $VERIFY -eq 1 ] ; then
  if ! autogen/diff_ir.bin --a autogen/"$IN.mrn.json.tmp" -b autogen/"$IN.mrn.json" ; then
    echo "With $IN.mrn:"
    echo
    echo "=== GENERATED ==="
    cat autogen/"$IN.mrn.json.tmp"
    echo
    echo "=== IN THE REPO ==="
    cat autogen/"$IN.mrn.json"
    echo
    echo "=== INTERMEDIATE ==="
    cat autogen/"$IN.mrn.h"
    echo
    echo "=== DEBUG 1 ==="
    g++ -E "autogen/$IN.mrn.h" 2>/dev/null | grep -v '^#' | grep -v '^$'
    echo
    echo "=== DEBUG 2 ==="
    echo '#include "../src/boilerplate/dsl.spec.h"' >"autogen/$IN.mrn.h"
    echo 'MAROON_SOURCE("'$IN.mrn'");' >>"autogen/$IN.mrn.h"
    echo '#line 1' >>"autogen/$IN.mrn.h"
    cat "$IN.mrn" >>"autogen/$IN.mrn.h"
    cp src/boilerplate/dsl.prefix.h autogen/"$IN.mrn.cc"
    g++ -E "autogen/$IN.mrn.h" 2>/dev/null | grep -v '^#' | grep -v '^$' >>autogen/"$IN.mrn.cc"
    echo -e "  ;\n  std::cout << JSON<JSONFormat::Minimalistic>(ctx.out) << std::endl;\n}" >>autogen/"$IN.mrn.cc"
    cat autogen/"$IN.mrn.cc"
    echo
    echo "=== DEBUG 3 ==="
    CLANG_FORMAT=""
    if clang-format --version >/dev/null 2>&1 ; then
      CLANG_FORMAT="clang-format"
    elif clang-format-10 --version >/dev/null 2>&1 ; then
      CLANG_FORMAT="clang-format-10"
    fi
    if [ "$CLANG_FORMAT" != "" ] ; then
      $CLANG_FORMAT -i autogen/"$IN.mrn.cc"
    fi
    cat autogen/"$IN.mrn.cc"
    echo
    echo "=== DEBUG 4 ==="
    g++ autogen/"$IN.mrn.cc" -o autogen/"$IN.mrn.tmp.bin" && echo "Build successful."
    echo
    echo "=== RE-GENERATED ==="
    autogen/"$IN.mrn.tmp.bin"
    echo
    echo "=== RE-GENERATED II ==="
    autogen/"$IN.mrn.tmp.bin" | $JQ
    echo
    exit 1
  fi
else
  mv autogen/"$IN.mrn.json.tmp" autogen/"$IN.mrn.json"
fi

# Remove the now-unneeded "original" files.
rm -f "autogen/$IN.mrn.bin"

# NOTE(dkorolev): Examinine these now.
# TODO(dkorolev): Remove them later.
# rm -f "autogen/$IN.mrn.gen.cc"
# rm -f "autogen/$IN.mrn.gen.h"
