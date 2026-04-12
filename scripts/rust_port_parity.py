#!/usr/bin/env python3
"""Run a Rust-vs-C++ parity corpus for the incremental Eshkol Rust port.

By default:
- Oracle (C++): ./build/eshkol-run
- Candidate (Rust): ./rust/target/debug/eshkol-rs
- Corpus: docs/rust-port/representative-corpus.txt
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CORPUS = ROOT / "docs" / "rust-port" / "representative-corpus.txt"
DEFAULT_CPP_BIN = ROOT / "build" / "eshkol-run"
DEFAULT_RS_BIN = ROOT / "rust" / "target" / "debug" / "eshkol-rs"


@dataclass
class RunResult:
    code: int
    stdout: str
    stderr: str


def normalize_output(text: str) -> str:
    """Normalize line endings and trailing whitespace/newlines."""
    return text.replace("\r\n", "\n").replace("\r", "\n").strip()


def run_cmd(cmd: list[str], cwd: Path) -> RunResult:
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    return RunResult(code=proc.returncode, stdout=proc.stdout, stderr=proc.stderr)


def load_corpus(path: Path) -> list[str]:
    expressions: list[str] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        expressions.append(line)
    return expressions


def check_binary(path: Path, hint: str) -> None:
    if not path.exists():
        raise FileNotFoundError(f"{path} not found. {hint}")
    if not os.access(path, os.X_OK):
        raise PermissionError(f"{path} is not executable")


def color(enabled: bool, code: str, text: str) -> str:
    if not enabled:
        return text
    return f"\033[{code}m{text}\033[0m"


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Rust-vs-C++ parity corpus")
    parser.add_argument("--corpus", default=str(DEFAULT_CORPUS), help="Path to expression corpus")
    parser.add_argument("--cpp-bin", default=str(DEFAULT_CPP_BIN), help="Path to C++ oracle binary")
    parser.add_argument("--rs-bin", default=str(DEFAULT_RS_BIN), help="Path to Rust candidate binary")
    parser.add_argument("--no-color", action="store_true", help="Disable ANSI color output")
    args = parser.parse_args()

    use_color = not args.no_color

    corpus = Path(args.corpus)
    cpp_bin = Path(args.cpp_bin)
    rs_bin = Path(args.rs_bin)

    try:
        if not corpus.exists():
            raise FileNotFoundError(f"Corpus file not found: {corpus}")

        check_binary(cpp_bin, "Build C++ oracle with: cmake --build build --target eshkol-run")
        check_binary(rs_bin, "Build Rust candidate with: cd rust && cargo build -p eshkol-cli")

        expressions = load_corpus(corpus)
        if not expressions:
            raise ValueError(f"Corpus has no expressions: {corpus}")

        print(color(use_color, "1;34", f"Running parity corpus: {corpus}"))
        print(f"Oracle   : {cpp_bin}")
        print(f"Candidate: {rs_bin}")
        print(f"Cases    : {len(expressions)}")
        print()

        failures = 0

        for idx, expr in enumerate(expressions, start=1):
            oracle_expr = f"(display {expr})(newline)"
            cpp = run_cmd([str(cpp_bin), "-e", oracle_expr], cwd=ROOT)
            rs = run_cmd([str(rs_bin), "-e", expr], cwd=ROOT)

            cpp_out = normalize_output(cpp.stdout)
            rs_out = normalize_output(rs.stdout)

            ok = (cpp.code == rs.code) and (cpp_out == rs_out)
            prefix = color(use_color, "32", "PASS") if ok else color(use_color, "31", "FAIL")
            print(f"[{prefix}] {idx:02d}: {expr}")

            if not ok:
                failures += 1
                print("  oracle:")
                print(f"    exit={cpp.code} stdout={cpp_out!r}")
                if cpp.stderr.strip():
                    print(f"    stderr={normalize_output(cpp.stderr)!r}")
                print("  rust:")
                print(f"    exit={rs.code} stdout={rs_out!r}")
                if rs.stderr.strip():
                    print(f"    stderr={normalize_output(rs.stderr)!r}")
                print()

        print()
        if failures == 0:
            print(color(use_color, "1;32", f"Parity corpus PASSED ({len(expressions)}/{len(expressions)})"))
            return 0

        print(color(use_color, "1;31", f"Parity corpus FAILED ({len(expressions) - failures}/{len(expressions)} passed)"))
        return 1

    except Exception as exc:  # pragma: no cover - defensive top-level handler
        print(color(use_color, "1;31", f"error: {exc}"), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
