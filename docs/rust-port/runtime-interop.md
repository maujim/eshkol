# Runtime Interop Layer (C ABI Compatibility Bootstrap)

This document tracks the initial Rust-side runtime interop surface for incremental replacement of C/C++ runtime components.

## Implemented (Iterations 4–8)

### 1) Object header C layout

Implemented in `rust/crates/eshkol-runtime/src/interop.rs`:

- `#[repr(C)] struct EshkolObjectHeader`
  - `subtype: u8`
  - `flags: u8`
  - `ref_count: u16`
  - `size: u32`
- Verified size contract: **8 bytes**.

### 2) Arena allocator C ABI shim

Implemented exported symbols (Rust `extern "C"`):

- Base-compatible names:
  - `arena_create`
  - `arena_destroy`
  - `arena_allocate`
  - `arena_allocate_with_header`
  - `arena_allocate_with_header_zeroed`
  - `arena_allocate_string_with_header`
  - `arena_get_used_memory`
  - `arena_get_block_count`
  - `arena_get_default_block_size`
- Namespaced wrappers for pilot linking without symbol collisions:
  - `eshkol_rust_arena_create`
  - `eshkol_rust_arena_destroy`
  - `eshkol_rust_arena_allocate`
  - `eshkol_rust_arena_allocate_with_header`
  - `eshkol_rust_arena_allocate_string_with_header`
  - `eshkol_rust_arena_get_used_memory`

Prototype notes:

- Uses owned allocation records and deterministic free on `arena_destroy`.
- Uses zeroed allocations to simplify deterministic behavior for interop bootstrap.
- Supports header-prefixed object allocation for subtype/flags interoperability.

### 3) Header helper ABI functions

- Base-compatible names:
  - `eshkol_get_header_from_data_ptr`
  - `eshkol_get_subtype_from_data_ptr`
  - `eshkol_get_flags_from_data_ptr`
- Namespaced wrappers:
  - `eshkol_rust_get_subtype_from_data_ptr`
  - `eshkol_rust_get_flags_from_data_ptr`

These mirror the C header-macro semantics (`data_ptr - sizeof(header)`).

### 3.1) Guarded header metadata mutation helpers

Added exported helpers for controlled flag/refcount mutation with pointer validation guards and subtype-aware policy checks:

- `eshkol_rust_header_validate`
- `eshkol_rust_header_ref_count`
- `eshkol_rust_header_set_ref_count`
- `eshkol_rust_header_inc_ref_count`
- `eshkol_rust_header_dec_ref_count`
- `eshkol_rust_header_set_flags`
- `eshkol_rust_header_add_flags`
- `eshkol_rust_header_clear_flags`

Subtype policy highlights:

- Refcount mutation allowed only on selected shareable subtypes (string/symbol/vector/hash/record/port/promise).
- Flag mutation uses subtype-specific masks:
  - rationals: linearity lifecycle flags only
  - strings/symbols: sharing/external lifecycle flags
  - others: existing full mask behavior

### 4) Tagged value C layout bridge + exported helpers

Implemented in:

- `rust/crates/eshkol-core/src/c_abi.rs` (layout + conversions)
- `rust/crates/eshkol-runtime/src/interop.rs` (exported helper functions)

Key points:

- `#[repr(C)] union EshkolTaggedData`
- `#[repr(C)] struct EshkolTaggedValue`
- Verified size contract: **16 bytes**.
- Exported helper constructors/accessors for immediate values:
  - `eshkol_rust_tv_make_null`
  - `eshkol_rust_tv_make_int64`
  - `eshkol_rust_tv_make_double`
  - `eshkol_rust_tv_make_bool`
  - `eshkol_rust_tv_type`
  - `eshkol_rust_tv_flags`
  - `eshkol_rust_tv_int64`
  - `eshkol_rust_tv_double`
  - `eshkol_rust_tv_bool`
- Expanded helper coverage for heap-pointer tagged values:
  - `eshkol_rust_tv_make_heap_ptr`
  - `eshkol_rust_tv_make_rational`
  - `eshkol_rust_tv_make_string`
  - `eshkol_rust_tv_make_symbol`
  - `eshkol_rust_tv_ptr`
  - `eshkol_rust_tv_heap_subtype`

### 5) Pilot runtime allocation paths

Ported concrete allocation paths into Rust interop:

- First path: `eshkol_rust_rational_create(arena, numerator, denominator)`
- Second path: `eshkol_rust_string_create(arena, c_str)` and `eshkol_rust_symbol_create(arena, c_str)`

Behavior matches C runtime expectations for these slices:

- rational denominator normalization (always positive)
- rational GCD reduction
- header-subtyped allocation for rational/string/symbol object data
- NUL-terminated payload copy for string/symbol constructors

## Verification

- `cd rust && cargo test --workspace`
  - `eshkol-runtime` interop tests: 11 passed
  - `eshkol-core` c_abi tests: 4 passed
- Native smoke harness:
  - `./scripts/run_rust_interop_smoke.sh` →
    - `rust interop smoke C PASS`
    - `rust interop smoke C++ PASS`
    - `rust interop smoke mixed PASS`
- Existing parity harness checks continue to pass:
  - `python3 scripts/rust_port_parity.py --no-color` → PASS
  - `python3 scripts/rust_port_file_parity.py --no-color` → PASS
- Opt-in production call-site pilot verified:
  - Configure: `cmake -S . -B build-rust-pilot ... -DESHKOL_RUST_INTEROP_PILOT=ON`
  - Run with opt-in env:
    - `ESHKOL_RUST_INTEROP_ENABLED=1 ESHKOL_RUST_INTEROP_LIB=<path>/libeshkol_runtime.dylib ESHKOL_RUST_INTEROP_LOG=1 ./build-rust-pilot/eshkol-run -e "(display (/ 4 6))(newline)"`
    - Observed log: `rust interop pilot: eshkol_rational_create normalized via Rust`
  - Production pilot now touches two guarded C++ call sites:
    - `eshkol_rational_create`
    - `eshkol_rational_compare`

## Smoke Harness

- Sources:
  - `tools/rust-interop-smoke/smoke.c`
  - `tools/rust-interop-smoke/smoke.cpp`
  - `tools/rust-interop-smoke/mixed_c_part.c`
  - `tools/rust-interop-smoke/mixed_cpp_main.cpp`
- Runner: `scripts/run_rust_interop_smoke.sh` (builds/runs C, C++, and mixed-language binaries)
- CI: included in `.github/workflows/ci.yml` under:
  - `rust-port-parity` (parity + smoke)
  - `rust-port-pilot` (pilot-on configure/build + opt-in env execution)

## Next steps

- Add explicit subtype policy diagnostics/telemetry for blocked metadata mutations in pilot mode.
- Expand production pilot beyond rational paths (next candidate: selected string/symbol helper call-site).
- Add small parity subset execution under pilot-enabled CI to guard behavior under interop mode.
