#!/bin/bash
#
# TODO(dkorolev): Run this automatically as a Github Action!

set -e -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

(cd "$SCRIPT_DIR"; cd ..; make >/dev/null 2>&1)
(cd "$SCRIPT_DIR"; cargo build >/dev/null 2>&1)

for i in $(cd "$SCRIPT_DIR/../autogen"; ls *.json) ; do
  echo "$i"
  "$SCRIPT_DIR/target/debug/maroon_ir_schema_checker" --in="$SCRIPT_DIR/../autogen/$i" --out="$SCRIPT_DIR/../autogen/$i.rust.tmp"
  "$SCRIPT_DIR/../autogen/diff_ir.bin" --a="$SCRIPT_DIR/../autogen/$i" --b="$SCRIPT_DIR/../autogen/$i.rust.tmp"
  echo "$i : OK"
done

echo 'All Rust JSON parsing tests passed.'
