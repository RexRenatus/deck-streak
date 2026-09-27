#!/usr/bin/env python3
"""Write the parity oracle's goldens by calling v9's own functions (ADR-012).

Run it on the owner's private checkout of v9, never in this public repository's CI:

    python3 tools/parity-oracle/generate.py --source-checkout PATH/TO/v9 --out tools/parity-oracle/goldens

Each golden records v9's commit, this file's path and sha256, the seed and `inputs: synthetic`.
Inputs come from a seeded generator, never from a database, so the public tree carries no
production row. Commit this file and the goldens together: a golden whose recorded generator
hash no longer matches this file reads red (tools/parity-oracle/test_goldens.py).

Each wave registers the v9 functions it ports in FUNCTIONS, with a case builder that covers the
classes a port gets wrong: a tie that Python rounds to even, a negative operand for floor division
and modulo, and the study day's rollover boundary.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import random
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

HERE = Path(__file__).resolve()
SEED = 20
SCHEMA = "phx.parity-golden.v1"
GENERATOR = "tools/parity-oracle/generate.py"

#: v9 function (dotted path) -> case builder. A builder returns (edge class or None, kwargs).
FUNCTIONS: dict[str, Callable[[random.Random], list[tuple[str | None, dict]]]] = {}


def call(function: str, arguments: dict) -> object:
    module_name, _, name = function.rpartition(".")
    return getattr(importlib.import_module(module_name), name)(**arguments)


def golden_for(function: str, builder, commit: str, digest: str) -> dict:
    rng = random.Random(SEED)
    cases = []
    for edge, arguments in builder(rng):
        case = {"input": arguments, "output": call(function, arguments)}
        if edge:
            case["class"] = edge
        cases.append(case)
    return {
        "schema": SCHEMA,
        "function": function,
        "source_commit": commit,
        "generator": GENERATOR,
        "generator_sha256": digest,
        "inputs": "synthetic",
        "seed": SEED,
        "cases": cases,
    }


def write(golden: dict, out: Path) -> Path:
    text = json.dumps(golden, indent=2, sort_keys=True, allow_nan=False) + "\n"
    path = out / f"{golden['function'].rsplit('.', 1)[-1]}.json"
    path.write_text(text, encoding="utf-8")
    return path


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--source-checkout", required=True, help="v9's private checkout"
    )
    parser.add_argument("--package", help="the predecessor's top-level package")
    parser.add_argument("--registry", help="the registry directory")
    parser.add_argument("--out", required=True, help="the goldens directory")
    args = parser.parse_args(argv)
    checkout = Path(args.source_checkout).resolve()
    sys.path.insert(0, str(checkout))
    commit = subprocess.run(
        ["git", "-C", str(checkout), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    digest = hashlib.sha256(HERE.read_bytes()).hexdigest()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for function, builder in FUNCTIONS.items():
        print(write(golden_for(function, builder, commit, digest), out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
