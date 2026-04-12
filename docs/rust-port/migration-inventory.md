# Rust Port Migration Inventory

Snapshot generated during initial Rust-port bootstrap.

## Codebase Snapshot (2026-04-12)

- `lib/`: 178 files (124 C/C++/headers, 52 `.esk`)
- `inc/`: 56 public headers
- `exe/`: 3 CLI entry points (`eshkol-run`, `eshkol-repl`, `eshkol-server`)
- `tests/`: 443 files (431 `.esk` test programs)
- Top extensions: `.esk` (494), `.cpp` (71), `.h` (69), `.c` (53)

## Proposed Rust Workspace Layout

| Existing area | Rust crate | Notes |
|---|---|---|
| `lib/core`, `inc/eshkol/core` | `eshkol-core` | Value model, exact numerics, utility/runtime primitives |
| `lib/frontend`, parser/macro expander | `eshkol-frontend` | Reader/parser/AST/macro pipeline |
| `lib/backend`, `lib/bridge` | `eshkol-backend` | Bytecode + native backends, codegen adapters |
| runtime glue across `lib/core` + `lib/backend` | `eshkol-runtime` | Execution state, memory regions, runtime services |
| `exe/*` | `eshkol-cli` | `eshkol-rs` entrypoint first; map to run/repl/server later |

## Initial Priorities

1. Port exact rational arithmetic (lowest-risk runtime wedge).
2. Port tagged value representation and dispatch on top of rationals.
3. Stand up parser AST types + golden parsing tests.
4. Add compatibility harness to run selected `.esk` programs across both implementations.

## Compatibility Strategy

- Keep C/C++ build as baseline oracle during migration.
- Add curated parity suite (numeric semantics, parser behavior, selected compiler output).
- Only replace production paths after parity gates pass.
