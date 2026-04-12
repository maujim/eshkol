#!/usr/bin/env python3
"""Run Rust-vs-C++ parity across selected tests/**/*.esk fixtures.

Mode: whole-file
- Wrap each fixture source as `(begin ...source...)`
- C++ oracle evaluates `(display <wrapped>)(newline)`
- Rust candidate evaluates `<wrapped>`
- Compare exit code + normalized stdout
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FIXTURES = ROOT / "docs" / "rust-port" / "file-fixture-corpus.txt"
DEFAULT_CPP_BIN = ROOT / "build" / "eshkol-run"
DEFAULT_RS_BIN = ROOT / "rust" / "target" / "debug" / "eshkol-rs"


@dataclass
class RunResult:
    code: int
    stdout: str
    stderr: str


def normalize_output(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n").strip()


def run_cmd(cmd: list[str], cwd: Path) -> RunResult:
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    return RunResult(code=proc.returncode, stdout=proc.stdout, stderr=proc.stderr)


def color(enabled: bool, code: str, text: str) -> str:
    if not enabled:
        return text
    return f"\033[{code}m{text}\033[0m"


def check_binary(path: Path, hint: str) -> None:
    if not path.exists():
        raise FileNotFoundError(f"{path} not found. {hint}")
    if not os.access(path, os.X_OK):
        raise PermissionError(f"{path} is not executable")


def load_fixture_list(path: Path) -> list[Path]:
    fixtures: list[Path] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fixture = Path(line)
        if not fixture.is_absolute():
            fixture = ROOT / fixture
        fixtures.append(fixture)
    return fixtures


def wrap_fixture_as_begin(source: str) -> str:
    return f"(begin\n{source}\n)"


def main() -> int:
    parser = argparse.ArgumentParser(description="Run file-based Rust-vs-C++ parity fixtures")
    parser.add_argument("--fixtures", default=str(DEFAULT_FIXTURES), help="Path to fixture list file")
    parser.add_argument("--cpp-bin", default=str(DEFAULT_CPP_BIN), help="Path to C++ oracle binary")
    parser.add_argument("--rs-bin", default=str(DEFAULT_RS_BIN), help="Path to Rust candidate binary")
    parser.add_argument("--no-color", action="store_true", help="Disable ANSI color output")
    args = parser.parse_args()

    use_color = not args.no_color
    fixture_list = Path(args.fixtures)
    cpp_bin = Path(args.cpp_bin)
    rs_bin = Path(args.rs_bin)

    try:
        if not fixture_list.exists():
            raise FileNotFoundError(f"Fixture list not found: {fixture_list}")

        check_binary(cpp_bin, "Build C++ oracle with: cmake --build build --target eshkol-run")
        check_binary(rs_bin, "Build Rust candidate with: cd rust && cargo build -p eshkol-cli")

        fixtures = load_fixture_list(fixture_list)
        if not fixtures:
            raise ValueError(f"No fixtures found in: {fixture_list}")

        print(color(use_color, "1;34", f"Running file parity fixtures: {fixture_list}"))
        print(f"Oracle   : {cpp_bin}")
        print(f"Candidate: {rs_bin}")
        print(f"Fixtures : {len(fixtures)}")
        print()

        failures = 0

        for fixture in fixtures:
            if not fixture.exists():
                print(color(use_color, "31", f"[FAIL] missing fixture: {fixture}"))
                failures += 1
                continue

            source = fixture.read_text(encoding="utf-8")
            wrapped = wrap_fixture_as_begin(source)

            cpp_expr = f"(display {wrapped})(newline)"
            cpp = run_cmd([str(cpp_bin), "-e", cpp_expr], cwd=ROOT)
            rs = run_cmd([str(rs_bin), "-e", wrapped], cwd=ROOT)

            cpp_out = normalize_output(cpp.stdout)
            rs_out = normalize_output(rs.stdout)

            ok = (cpp.code == rs.code) and (cpp_out == rs_out)
            rel = fixture.relative_to(ROOT)
            marker = color(use_color, "32", "PASS") if ok else color(use_color, "31", "FAIL")
            print(f"[{marker}] {rel}")

            if not ok:
                failures += 1
                print(f"  oracle: exit={cpp.code} stdout={cpp_out!r}")
                if cpp.stderr.strip():
                    print(f"          stderr={normalize_output(cpp.stderr)!r}")
                print(f"  rust  : exit={rs.code} stdout={rs_out!r}")
                if rs.stderr.strip():
                    print(f"          stderr={normalize_output(rs.stderr)!r}")

        print()
        if failures == 0:
            print(color(use_color, "1;32", f"File parity PASSED ({len(fixtures)}/{len(fixtures)})"))
            return 0

        print(color(use_color, "1;31", f"File parity FAILED ({len(fixtures) - failures}/{len(fixtures)} passed)"))
        return 1

    except Exception as exc:  # pragma: no cover
        print(color(use_color, "1;31", f"error: {exc}"), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
