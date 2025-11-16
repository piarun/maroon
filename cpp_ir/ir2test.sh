#!/bin/bash

set -e -o pipefail

if [ "$1" == "" ] ; then
  echo 'Need an argument, the `.mrn` file in this directory that has had `mrn2ir.sh` run on already.'
  exit 1
fi

IN="${1%.mrn}"

if ! [ -f "autogen/$IN.mrn.json" ] ; then
  echo "The autogen/$IN.mrn.json file should exist."
  exit 1
fi

./autogen/gen_test.bin --in "autogen/$IN.mrn.json" --name "$IN" --out "autogen/$IN.mrn.test.h"

CLANG_FORMAT=""
if clang-format --version >/dev/null 2>&1 ; then
  CLANG_FORMAT="clang-format"
elif clang-format-10 --version >/dev/null 2>&1 ; then
  CLANG_FORMAT="clang-format-10"
fi

if [ "$CLANG_FORMAT" != "" ] ; then
  $CLANG_FORMAT -i autogen/"$IN.mrn.test.h"
fi
