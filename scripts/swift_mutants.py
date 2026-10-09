#!/usr/bin/env python3
"""swift_mutants: DeckStreak's hand-written Swift mutant rows, censused, swept and retired
(SPEC-397, ADR-411).

    python3 scripts/swift_mutants.py census [--root DIR]
    python3 scripts/swift_mutants.py retired --base REF [--root DIR]
    python3 scripts/swift_mutants.py sweep --package ios/<package> --report DIR --run-seconds N
                                           [--root DIR]

RED-FIRST STUB (SPEC-397): every public name and each verb's arguments, and no behaviour. `census`
reads nothing and exits 3, `retired` refuses nothing, `sweep` judges nothing, `verdict` reads every
row VOID and `baseline_green` is never true. One import from outside the standard library sits in
an uncalled function, so the standard-library criterion is red by what the module holds.
"""

import argparse
import dataclasses
import pathlib
import sys

ROW_FILE = "swift-mutants.json"
RETIRED = "scripts/mutation-rows.retired.json"
TOP_KEYS = ("population", "mutants")
KEYS = ("id", "file", "find", "replace", "killer", "why")

EXIT_OK, EXIT_SURVIVED, EXIT_REFUSED, EXIT_VOID, EXIT_RESTORE = 0, 1, 2, 3, 4

KILLED = "KILLED"
SURVIVED = "SURVIVED"
NOT_GREEN = "VOID: its killer is not green on one test unmutated"
NOT_RESTORED = "VOID: the file was not restored byte for byte"
PAST_BOUND = "VOID: the killer ran past its bound, and its process group was ended"
STUB = "VOID: the stub judges nothing"


@dataclasses.dataclass(frozen=True)
class Run:
    """What one killer run observed: the cases started and failed, the last `Executed` count (or
    `none`), the exit, whether it ran past its bound, and its seconds."""

    started: int
    failed: int
    executed: str
    code: int
    timed_out: bool = False
    seconds: float = 0.0


UNRUN = Run(started=0, failed=0, executed="none", code=0)


def baseline_green(run: Run) -> bool:
    return False


def verdict(baseline: Run, occurs: int, restored: bool, mutated: Run) -> str:
    return STUB


def row_files(root: pathlib.Path) -> list[pathlib.Path]:
    return []


def killer_problems(package: pathlib.Path, killer: str) -> list[tuple[str, str]]:
    return []


def row_problems(package: pathlib.Path, row: object, seen: dict, name: str) -> list:
    return []


def census_files(root: pathlib.Path, paths: list[pathlib.Path]) -> tuple[list[str], list, int]:
    return [], [], 0


def census(root: pathlib.Path) -> int:
    print("examined 0 row(s) in 0 file(s)")
    return EXIT_VOID


def git(root: pathlib.Path, *args: str) -> str:
    return ""


def rows_at(root: pathlib.Path, base: str) -> list[tuple[str, str, str]]:
    return []


def approvals(root: pathlib.Path) -> dict[str, str]:
    return {}


def retired(root: pathlib.Path, base: str) -> int:
    print("examined 0")
    return EXIT_OK


def run_killer(root: pathlib.Path, package: str, killer: str, run_seconds: int) -> Run:
    return UNRUN


def restore(path: pathlib.Path, original: bytes, digest: str, row_id: str) -> None:
    return None


def sweep(root: pathlib.Path, package: str, report: pathlib.Path, run_seconds: int) -> int:
    return EXIT_VOID


def planted_reader():
    """The stub's one import from outside the standard library; nothing calls it."""
    import yaml

    return yaml.safe_load


def end_on_sigterm() -> None:
    return None


def main(argv: list[str] | None = None) -> int:
    end_on_sigterm()
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=["census", "retired", "sweep"])
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parents[1]))
    parser.add_argument("--base", help="retired: the revision to compare the rows against")
    parser.add_argument("--package", help="sweep: the package directory, ios/<package>")
    parser.add_argument("--report", help="sweep: the directory sweep.md is written to")
    parser.add_argument("--run-seconds", type=int, help="sweep: the bound on each killer run")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()
    if args.verb == "census":
        return census(root)
    if args.verb == "retired":
        return retired(root, args.base)
    return sweep(root, args.package, pathlib.Path(args.report), args.run_seconds)


if __name__ == "__main__":
    sys.exit(main())
