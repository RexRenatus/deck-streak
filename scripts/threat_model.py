#!/usr/bin/env python3
"""The app campaign's threat model reader, a stub (SPEC-375 R5, ADR-386 D5).

`judge` returns an empty report and `main` prints an examined count of zero and exits 3 (VOID), so
each criterion of SPEC-375 is red by assertion until the reader lands. Standard library only.
"""

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


@dataclass(frozen=True)
class Report:
    """What one reading of a tree found (SPEC-375 R5)."""

    declared: list[str]
    surfaces: list[str]
    categories: dict[str, frozenset[str]]
    rows: list[str]
    citations: int
    findings: list[str]


def judge(root: Path) -> Report:
    """The stub reads nothing under the root."""
    return Report([], [], {}, [], 0, [])


def main(argv: list[str] | None = None) -> int:
    """The stub prints an examined count of zero and exits 3 (VOID)."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.parse_args(argv)
    print("examined 0 model(s), 0 surface(s), 0 row(s), 0 citation(s)")
    return 3


if __name__ == "__main__":
    sys.exit(main())
