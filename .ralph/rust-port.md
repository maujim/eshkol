# Port Eshkol codebase to Rust

Incrementally port the existing C/C++ Eshkol compiler/runtime to Rust while preserving language behavior, test coverage, and developer workflows.

## Goals
- Build a Rust workspace that mirrors the current architecture (frontend, core runtime, backend, CLI).
- Port subsystems in small verified slices with parity tests against current behavior.
- Keep the existing C/C++ implementation runnable during transition.

## Checklist
- [x] Create migration inventory: current modules, ownership, and crate mapping.
- [x] Bootstrap Rust workspace + crates with CI-friendly commands.
- [x] Port first runtime slice (exact rational arithmetic) with tests.
- [x] Add compatibility test harness strategy (Rust vs existing behavior).
- [x] Port tagged value model + numeric dispatch.
- [x] Port parser + AST pipeline.
- [x] Port VM/backend execution path.
- [x] Port CLI/repl entry points.
- [x] Reach feature-parity milestone for a representative program set.

## Phase 2 Checklist (ongoing)
- [x] Expand evaluator coverage to core forms (`define`, `if`, `let`, `lambda`, `begin`).
- [x] Add file-based parity runner against selected existing `tests/**/*.esk` fixtures.
- [x] Integrate parity harness into CI for Rust-port gating.
- [x] Start runtime interop layer (arena/object headers) for C ABI compatibility.

## Phase 3 Checklist (ongoing)
- [x] Add exported tagged-value C ABI helper functions used by current runtime call sites.
- [x] Build a C/C++ smoke harness that links against Rust interop symbols.
- [x] Port one existing runtime allocation path to Rust interop (pilot replacement).

## Phase 4 Checklist (ongoing)
- [x] Expand interop helper coverage for heap-pointer tagged values (rational/string pointers).
- [x] Add C++ smoke harness variant and wire both C + C++ smoke checks.
- [x] Pilot second runtime allocation path through Rust interop.

## Phase 5 Checklist (ongoing)
- [x] Add exported helpers for heap-pointer metadata mutation (flags/refcount) with safety guards.
- [x] Add mixed-language smoke binary (single executable linking C + C++ objects to Rust interop lib).
- [x] Pilot opt-in C++ production call-site wiring to `eshkol_rust_*` symbols.

## Phase 6 Checklist (ongoing)
- [x] Add subtype-specific policy guards for metadata mutation helpers.
- [x] Add CI lane with `-DESHKOL_RUST_INTEROP_PILOT=ON` and opt-in env-path execution.
- [x] Expand production pilot to second guarded call site.

## Phase 7 Checklist (ongoing)
- [ ] Add pilot-mode diagnostics for blocked subtype-policy metadata mutations.
- [ ] Run a focused parity subset under pilot-enabled CI.
- [ ] Expand production pilot to a non-rational guarded call site.

## Verification
- Created migration inventory: `docs/rust-port/migration-inventory.md`.
- Added explicit compatibility harness strategy: `docs/rust-port/compatibility-harness.md`.
- Created Rust workspace and crate scaffolding:
  - `rust/Cargo.toml`
  - `rust/crates/eshkol-core/*`
  - `rust/crates/eshkol-runtime/*`
  - `rust/crates/eshkol-frontend/*`
  - `rust/crates/eshkol-backend/*`
  - `rust/crates/eshkol-cli/*`
- Ported exact rational arithmetic into `rust/crates/eshkol-core/src/rational.rs`.
- Added tagged-value numeric model and dispatch in `rust/crates/eshkol-core/src/tagged_value.rs`.
- Added parser/AST pipeline in:
  - `rust/crates/eshkol-frontend/src/ast.rs`
  - `rust/crates/eshkol-frontend/src/parser.rs`
- Added backend execution slice in `rust/crates/eshkol-backend/src/vm.rs` and wired exports in `src/lib.rs`.
- Added CLI prototype entry points in `rust/crates/eshkol-cli/src/main.rs`:
  - `-e/--eval`
  - file execution
  - `--repl`
- Added representative parity corpus and executable parity harness:
  - `docs/rust-port/representative-corpus.txt`
  - `scripts/rust_port_parity.py`
- Added runtime interop layer bootstrap:
  - `rust/crates/eshkol-runtime/src/interop.rs`
  - `rust/crates/eshkol-core/src/c_abi.rs`
  - `docs/rust-port/runtime-interop.md`
- Added exported tagged-value C ABI helper functions in runtime interop (`eshkol_rust_tv_make_*`, `eshkol_rust_tv_*` accessors).
- Added guarded heap-pointer metadata mutation helpers:
  - `eshkol_rust_header_validate`
  - `eshkol_rust_header_ref_count`
  - `eshkol_rust_header_set_ref_count`
  - `eshkol_rust_header_inc_ref_count`
  - `eshkol_rust_header_dec_ref_count`
  - `eshkol_rust_header_set_flags`
  - `eshkol_rust_header_add_flags`
  - `eshkol_rust_header_clear_flags`
- Added subtype-specific policy guards for metadata mutation helpers in `rust/crates/eshkol-runtime/src/interop.rs`:
  - refcount mutations allowed only for selected shareable subtypes
  - flag masks vary by subtype (rational vs string/symbol vs default)
- Expanded heap-pointer tagged-value helper coverage:
  - `eshkol_rust_tv_make_heap_ptr`
  - `eshkol_rust_tv_make_rational`
  - `eshkol_rust_tv_make_string`
  - `eshkol_rust_tv_make_symbol`
  - `eshkol_rust_tv_ptr`
  - `eshkol_rust_tv_heap_subtype`
- Added pilot rational allocation path in Rust interop: `eshkol_rust_rational_create` (normalization + header subtype).
- Added second pilot allocation path in Rust interop:
  - `eshkol_rust_string_create`
  - `eshkol_rust_symbol_create`
- Added native interop smoke harnesses:
  - `tools/rust-interop-smoke/smoke.c`
  - `tools/rust-interop-smoke/smoke.cpp`
  - `tools/rust-interop-smoke/mixed_c_part.c`
  - `tools/rust-interop-smoke/mixed_cpp_main.cpp`
  - `scripts/run_rust_interop_smoke.sh`
- Expanded backend evaluator semantics in `rust/crates/eshkol-backend/src/vm.rs` to support:
  - special forms: `define`, `if`, `let`, `lambda`, `begin`
  - lexical environments and closure application
  - recursive function definitions in the prototype VM
- Added file-based fixture parity assets:
  - `docs/rust-port/file-fixture-corpus.txt`
  - `scripts/rust_port_file_parity.py`
- Integrated parity + interop smoke checks into CI:
  - `.github/workflows/ci.yml` (`rust-port-parity` job; includes `g++` for C++/mixed smoke builds)
- Added dedicated pilot CI lane in `.github/workflows/ci.yml`:
  - `rust-port-pilot` job configures with `-DESHKOL_RUST_INTEROP_PILOT=ON`, builds `eshkol-run`, builds Rust runtime, and executes opt-in env-path validation.
- Added opt-in production pilot call-site wiring in C++ runtime path:
  - `CMakeLists.txt` option `ESHKOL_RUST_INTEROP_PILOT`
  - `lib/core/rational.cpp` resolves and calls `eshkol_rust_rational_normalize_into` when opt-in env vars are set
  - Expanded from first guarded call-site (`eshkol_rational_create`) to second guarded call-site (`eshkol_rational_compare`).
- Command: `cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DESHKOL_BUILD_TESTS=OFF -DESHKOL_BUILD_INTEGRATION_TESTS=OFF -DESHKOL_REQUIRED_LLVM_MAJOR=22`
  - Result: PASS (configured C++ oracle build)
- Command: `cmake --build build --target eshkol-run -j6`
  - Result: PASS (`build/eshkol-run` produced)
- Command: `cd rust && cargo test --workspace`
  - Result: PASS
  - Totals: `eshkol-core` 19 tests, `eshkol-runtime` 11 tests, `eshkol-frontend` 6 tests, `eshkol-backend` 10 tests.
- Command: `python3 scripts/rust_port_parity.py --no-color`
  - Result: PASS
  - Corpus parity: `10/10` expressions matched between `build/eshkol-run` and `rust/target/debug/eshkol-rs`.
- Command: `python3 scripts/rust_port_file_parity.py --no-color`
  - Result: PASS
  - File parity: `6/6` selected fixtures matched (`tests/repl/01,02,05,06,07,08`).
- Command: `./scripts/run_rust_interop_smoke.sh`
  - Result: PASS
  - Output:
    - `rust interop smoke C PASS`
    - `rust interop smoke C++ PASS`
    - `rust interop smoke mixed PASS`
- Command: `cmake -S . -B build-rust-pilot -DCMAKE_BUILD_TYPE=Release -DESHKOL_BUILD_TESTS=OFF -DESHKOL_BUILD_INTEGRATION_TESTS=OFF -DESHKOL_REQUIRED_LLVM_MAJOR=22 -DESHKOL_RUST_INTEROP_PILOT=ON`
  - Result: PASS
- Command: `cmake --build build-rust-pilot --target eshkol-run -j6`
  - Result: PASS
- Command: `ESHKOL_RUST_INTEROP_ENABLED=1 ESHKOL_RUST_INTEROP_LIB=<repo>/rust/target/debug/libeshkol_runtime.dylib ESHKOL_RUST_INTEROP_LOG=1 ./build-rust-pilot/eshkol-run -e "(display (/ 4 6))(newline)"`
  - Result: PASS
  - Output includes: `[eshkol] rust interop pilot: eshkol_rational_create normalized via Rust`
  - Runtime output: `2/3`

## Notes
- Runtime interop now includes namespaced exported symbols (`eshkol_rust_*`) to avoid collisions with existing C runtime symbols.
- Pilot replacement paths are active for rational and string/symbol allocation through Rust interop.
- Metadata mutation helpers are now exported with guard checks and covered by tests.
- Prototype VM covers the initial core-form subset required for broader parity (`define/if/let/lambda/begin`).
- Two parity layers are active: expression corpus and file-fixture corpus, both cross-checking against `eshkol-run`.
- CI includes dedicated Rust-port jobs for parity/smoke and pilot-on execution.
- Opt-in C++ production call-site wiring is in place for rational normalization with two guarded call sites (`create` + `compare`).
- Next iteration should add pilot-mode diagnostics for blocked subtype-policy mutations and run a focused parity subset in pilot CI.

## Reflection (Iteration 6)

1. **What has been accomplished so far?**
   - Delivered a functioning Rust vertical slice (parse → eval → print) with representative parity and selected file-fixture parity against `eshkol-run`.
   - Established interop foundations: C-layout tagged values, object headers, arena-style allocators, namespaced exported symbols, and pilot runtime allocation paths.
   - Added CI gating that validates parity plus native interop smoke.

2. **What’s working well?**
   - Incremental, test-first wedges are keeping risk low and producing verifiable progress each iteration.
   - The two-layer parity harness (expression + fixture) catches regressions quickly.
   - Rust interop exports can be linked from native C/C++ consumers already, validating migration feasibility.

3. **What’s not working / blocking progress?**
   - There is still no production call-site adoption yet; interop is proven in smoke tests but not in live runtime paths.
   - Current fixture parity corpus is intentionally narrow (`tests/repl` subset), so broader language/runtime coverage is still missing.
   - ABI surface is still partial (no full heap-pointer metadata mutation and limited pointer-value helper ergonomics).

4. **Should the approach be adjusted?**
   - Yes: keep the same incremental strategy, but shift from “capability scaffolding” to “controlled adoption” by wiring one guarded production call site to Rust interop and expanding smoke tests to mixed-language linkage.

5. **Next priorities**
   - Implement guarded heap-pointer metadata helper exports.
   - Add mixed-language smoke binary linking both C and C++ objects against Rust runtime.
   - Pilot first opt-in production C++ call-site wiring to `eshkol_rust_*`, then validate with parity/CI gates.
