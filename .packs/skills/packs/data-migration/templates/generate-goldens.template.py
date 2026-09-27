#!/usr/bin/env python3
"""Write the parity oracle's goldens by calling the old system's own functions.

Run it where the old system's source lives, never in the public repository's CI:

    python3 tools/parity-oracle/generate.py --source-checkout ../v9 --out crates/v9-import/tests/golden

Each golden records the old system's commit, this file's path and sha256, the seed, and
`inputs: synthetic`. The inputs are drawn from a seeded generator, never read from a database, so a
public tree carries no production row. Commit this file and the goldens together: a changed
generator whose goldens were not regenerated reads red in data-migration's provenance row.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import random
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve()
SEED = 20
#: The function each golden proves, and how to build its synthetic inputs, including the classes
#: a port gets wrong: a half-way value that Python rounds to even, a negative operand for floor
#: division and modulo, and the study day's rollover boundary.
FUNCTIONS = {
    "v9.xp.review_xp": "review_xp_cases",
}


def review_xp_cases(rng: random.Random) -> list[tuple[str | None, dict]]:
    cases: list[tuple[str | None, dict]] = [
        ("tie", {"ease": 3, "maturity": 1.25, "tier": 1}),
        ("negative", {"delta": -7, "per": 25}),
        ("rollover", {"answered_at": "04:00", "rollover_hour": 4}),
    ]
    for _ in range(50):
        cases.append((None, {"ease": rng.randint(1, 4), "maturity": rng.choice([1.0, 1.3, 2.0]), "tier": 1}))
    return cases


def call(function: str, arguments: dict) -> object:
    module_name, _, name = function.rpartition(".")
    return getattr(importlib.import_module(module_name), name)(**arguments)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-checkout", required=True, help="the old system's checkout")
    parser.add_argument("--out", required=True, help="the goldens directory")
    args = parser.parse_args()
    checkout = Path(args.source_checkout).resolve()
    sys.path.insert(0, str(checkout))
    commit = subprocess.run(
        ["git", "-C", str(checkout), "rev-parse", "HEAD"], capture_output=True, text=True, check=True
    ).stdout.strip()
    digest = hashlib.sha256(HERE.read_bytes()).hexdigest()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for function, builder in FUNCTIONS.items():
        rng = random.Random(SEED)
        cases = []
        for edge, arguments in globals()[builder](rng):
            case = {"input": arguments, "output": call(function, arguments)}
            if edge:
                case["class"] = edge
            cases.append(case)
        golden = {
            "schema": "phx.parity-golden.v1",
            "function": function,
            "source_commit": commit,
            "generator": "tools/parity-oracle/generate.py",
            "generator_sha256": digest,
            "inputs": "synthetic",
            "seed": SEED,
            "cases": cases,
        }
        text = json.dumps(golden, indent=2, sort_keys=True, allow_nan=False) + "\n"
        (out / f"{function.rsplit('.', 1)[-1]}.json").write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
