# Rust Port Compatibility Harness Strategy

## Objective

Continuously validate that Rust-ported behavior matches the current C/C++ implementation during migration.

## Oracle Model

- **Primary oracle:** existing C/C++ `eshkol-run` behavior.
- **Candidate under test:** Rust implementation (`eshkol-rs`, eventually command-compatible with `eshkol-run`).

## Initial Parity Corpus (Phase 1)

Prototype corpora currently exercised by scripts:

- Expression corpus: `docs/rust-port/representative-corpus.txt`
- File fixture corpus: `docs/rust-port/file-fixture-corpus.txt`
  - Includes selected existing fixtures under `tests/repl/*.esk`

## Comparison Contract

For each test case:

1. Run C/C++ oracle and capture `(exit_code, stdout, stderr)`.
2. Run Rust candidate and capture `(exit_code, stdout, stderr)`.
3. Compare:
   - Exit codes must match.
   - `stdout` must match exactly for deterministic tests.
   - `stderr` compared by normalized category when formatting diverges.

## Execution Plan

1. Keep Rust unit tests (`cargo test`) for internal module verification.
2. Run expression parity script:
   - `python3 scripts/rust_port_parity.py`
3. Run file-based fixture parity script:
   - `python3 scripts/rust_port_file_parity.py`
4. Gate incremental subsystem cutovers on parity corpus pass.

## CI Gating

- GitHub Actions workflow `ci.yml` now contains job `rust-port-parity`.
- The job builds:
  - C++ oracle binary (`eshkol-run`)
  - Rust candidate binary (`eshkol-rs`)
- It executes both parity scripts as required checks for Rust-port changes.

## Milestone Gates

- **M1:** Rational and exact numeric parity. ✅ (prototype subset)
- **M2:** Tagged value and mixed numeric dispatch parity. ✅ (prototype subset)
- **M3:** Parser/AST parity on selected fixture programs. ✅ (`tests/repl` subset via file parity script)
- **M4:** End-to-end command parity for `eshkol-run` subset.
