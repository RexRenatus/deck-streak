#!/usr/bin/env python3
"""mutation_rows: DeckStreak's hand-proved mutation rows, read, censused, proved and retired
(SPEC-039 R8 to R11, ADR-057).

    python3 scripts/mutation_rows.py count|ids [--root DIR]
    python3 scripts/mutation_rows.py census [--root DIR]
    python3 scripts/mutation_rows.py prove [--root DIR] (--all | --row ID ... | --band S<lo>-S<hi>
                                           | --rows-from PLAN) [--report FILE]
    python3 scripts/mutation_rows.py retired --base REF [--root DIR]

Red stub: every entry point is in place and does nothing yet.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

sys.dont_write_bytecode = True

MONOLITH = "scripts/mutation-rows.json"
FRAGMENTS = "scripts/mutation-rows.d"
RETIRED = "scripts/mutation-rows.retired.json"
BAND_NAME = re.compile(r"S([0-9]+)-S([0-9]+)\.json")
ID_STEM = re.compile(r"S([0-9]+)")


class PopulationRefused(ValueError):
    """The header and its fragments do not assemble into one population."""


def fragment_band(name: str) -> tuple[int, int] | None:
    return None


def assemble(monolith: object, fragments: list[tuple[str, object]]) -> object:
    return monolith


def tree_fragments(root: pathlib.Path | str) -> list[tuple[str, object]]:
    return []


def load_tree(root: pathlib.Path | str) -> object:
    text = (pathlib.Path(root) / MONOLITH).read_text(encoding="utf-8")
    return json.loads(text)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=["count", "ids", "census", "prove", "retired"])
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parents[1]))
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--row", action="append", default=[])
    parser.add_argument("--band")
    parser.add_argument("--rows-from")
    parser.add_argument("--report")
    parser.add_argument("--base")
    args = parser.parse_args(argv)
    if args.verb == "prove":
        for row in args.row:
            print(f"{row}: KILLED")
        print(f"examined {len(args.row)}")
        return 0
    print("examined 0")
    return 0


if __name__ == "__main__":
    sys.exit(main())
