#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$ROOT_DIR/rust"
cargo build -p eshkol-runtime
cd "$ROOT_DIR"

mkdir -p build-rust-interop

CC_BIN="${CC:-cc}"
CXX_BIN="${CXX:-c++}"
C_OUT_BIN="$ROOT_DIR/build-rust-interop/rust-interop-smoke-c"
CPP_OUT_BIN="$ROOT_DIR/build-rust-interop/rust-interop-smoke-cpp"
MIXED_OUT_BIN="$ROOT_DIR/build-rust-interop/rust-interop-smoke-mixed"
MIXED_C_OBJ="$ROOT_DIR/build-rust-interop/mixed_c_part.o"

if [[ "$OSTYPE" == darwin* ]]; then
  LIB_EXT="dylib"
  RPATH_FLAG="-Wl,-rpath,$ROOT_DIR/rust/target/debug"
else
  LIB_EXT="so"
  RPATH_FLAG="-Wl,-rpath,$ROOT_DIR/rust/target/debug"
fi

LIB_FILE="$ROOT_DIR/rust/target/debug/libeshkol_runtime.$LIB_EXT"
if [[ ! -f "$LIB_FILE" ]]; then
  echo "Expected runtime library not found: $LIB_FILE" >&2
  exit 1
fi

"$CC_BIN" "$ROOT_DIR/tools/rust-interop-smoke/smoke.c" \
  -L"$ROOT_DIR/rust/target/debug" -leshkol_runtime \
  "$RPATH_FLAG" -lm -o "$C_OUT_BIN"

"$CXX_BIN" "$ROOT_DIR/tools/rust-interop-smoke/smoke.cpp" \
  -std=c++17 \
  -L"$ROOT_DIR/rust/target/debug" -leshkol_runtime \
  "$RPATH_FLAG" -lm -o "$CPP_OUT_BIN"

"$CC_BIN" -c "$ROOT_DIR/tools/rust-interop-smoke/mixed_c_part.c" -o "$MIXED_C_OBJ"

"$CXX_BIN" "$ROOT_DIR/tools/rust-interop-smoke/mixed_cpp_main.cpp" "$MIXED_C_OBJ" \
  -std=c++17 \
  -L"$ROOT_DIR/rust/target/debug" -leshkol_runtime \
  "$RPATH_FLAG" -lm -o "$MIXED_OUT_BIN"

"$C_OUT_BIN"
"$CPP_OUT_BIN"
"$MIXED_OUT_BIN"
