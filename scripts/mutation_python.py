#!/usr/bin/env python3
"""Mutation testing for the repository's own Python (SPEC-087, ADR-073): the red-first stub.

Every public name the tests use exists and answers inertly: an empty listing, a report with no
mutants and exit 0, so each criterion fails by assertion until the runner is built.
"""

import argparse
import json
import pathlib
import sys

SCHEMA = "deckstreak.mutation-python.v1"


def list_source(source):
    """The stub lists nothing."""
    return []


def judge_mutant(target, original, text, tests):
    """The stub judges nothing."""
    return ("survived", [])


def count(report):
    """The stub counts nothing."""
    return {}


def exit_code(report):
    """The stub is always clean."""
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("verb")
    parser.add_argument("--root")
    parser.add_argument("--plan")
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--file")
    parser.add_argument("--shard")
    parser.add_argument("--failfast", action="store_true")
    parser.add_argument("--report")
    parser.add_argument("--out")
    parser.add_argument("--test-seconds", type=float)
    parser.add_argument("--control-seconds", type=float)
    args, _rest = parser.parse_known_args(argv)
    if args.verb == "census":
        print("examined 0")
    elif args.verb == "list":
        print("mutation-python: listed 0")
    elif args.verb == "run":
        if args.report:
            placeholder = {
                "mutant": "replace return value with return None in double",
                "outcome": "survived",
                "killers": [],
            }
            entry = {
                "path": args.file or "scripts/calc.py",
                "control": {},
                "bound": 0,
                "mutants": [placeholder],
            }
            document = {"schema": SCHEMA, "files": [entry], "counts": {}}
            pathlib.Path(args.report).write_text(json.dumps(document), encoding="utf-8")
        print("examined 0")
    return 0


if __name__ == "__main__":
    sys.exit(main())
