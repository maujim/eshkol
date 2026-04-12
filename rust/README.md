# Eshkol Rust Port Workspace

This directory contains an incremental Rust port of the Eshkol compiler/runtime.

## Current crates

- `eshkol-core`: rationals + tagged-value numeric dispatch
- `eshkol-runtime`: runtime coordination + C-ABI interop bootstrap (arena/header layer)
- `eshkol-frontend`: AST + parser (lists/atoms/quote/comments/strings)
- `eshkol-backend`: prototype VM execution path (numeric forms + `define/if/let/lambda/begin`)
- `eshkol-cli`: `eshkol-rs` prototype with eval/file/repl entrypoints

## Commands

```bash
cd rust
cargo test --workspace
cargo run -p eshkol-cli -- -e "(+ 1 2 (* 3 4))"
cargo run -p eshkol-cli -- --repl
```

## Cross-implementation parity check (prototype)

```bash
# Build C++ oracle once (this environment uses LLVM 22)
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DESHKOL_REQUIRED_LLVM_MAJOR=22
cmake --build build --target eshkol-run

# Build Rust candidate
cd rust && cargo build -p eshkol-cli && cd ..

# Run representative expression parity
python3 scripts/rust_port_parity.py

# Run selected file-fixture parity (tests/repl subset)
python3 scripts/rust_port_file_parity.py

# Run native C/C++/mixed smoke harnesses against Rust interop symbols
./scripts/run_rust_interop_smoke.sh
```

CI integration: `.github/workflows/ci.yml` includes `rust-port-parity` job that builds both implementations and runs these parity checks + interop smoke test.

Runtime interop notes: `docs/rust-port/runtime-interop.md`
