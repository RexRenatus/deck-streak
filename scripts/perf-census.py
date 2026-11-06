#!/usr/bin/env python3
"""The bench census (SPEC-630, ADR-652): the stub of the red commit.

    perf-census.py derive --listing FILE --share S --source-root DIR --workload PATH
                          --workload-sha256 HEX --derived-at SHA --release TAG --toolchain T
                          --valgrind V --hot-set OUT --bench-reach OUT
                          [--reach BENCH=FILE]... [--untoggled FILE]...
    perf-census.py check --root DIR [--enforce]

It keeps every argument and lacks each behaviour: `derive` writes both documents with no member,
no bench and no file and accepts every listing, and `check` prints only its summary line and
exits 0. Standard library only.
"""

import argparse
import json
import sys
from fractions import Fraction
from pathlib import Path

HOT_SCHEMA = "deckstreak.perf-hot-set.v1"
REACH_SCHEMA = "deckstreak.perf-bench-reach.v1"
REQUIRED = (
    "--listing",
    "--share",
    "--source-root",
    "--workload",
    "--workload-sha256",
    "--derived-at",
    "--release",
    "--toolchain",
    "--valgrind",
    "--hot-set",
    "--bench-reach",
)


def write(path, doc):
    Path(path).write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")


def derive(args):
    share = Fraction(args.share)
    write(
        args.hot_set,
        {
            "schema": HOT_SCHEMA,
            "share": float(share),
            "workload": {"path": args.workload, "sha256": args.workload_sha256},
            "derived_at": args.derived_at,
            "release": args.release,
            "toolchain": args.toolchain,
            "valgrind": args.valgrind,
            "total_ir": 0,
            "members": [],
        },
    )
    write(
        args.bench_reach,
        {"schema": REACH_SCHEMA, "derived_at": args.derived_at, "benches": {}, "files": []},
    )
    print(f"perf-census: derive wrote 0 members at {float(share):g}%, 0 benches, 0 files")
    return 0


def check(args):
    mode = "enforced" if args.enforce else "report-only"
    print(
        "perf-census: check examined 0 members, 0 defined benches, 0 release tags; "
        f"0 refused ({mode})"
    )
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(required=True)
    derived = modes.add_parser("derive")
    for name in REQUIRED:
        derived.add_argument(name, required=True)
    derived.add_argument("--reach", action="append", default=[])
    derived.add_argument("--untoggled", action="append", default=[])
    derived.set_defaults(run=derive)
    checked = modes.add_parser("check")
    checked.add_argument("--root", required=True)
    checked.add_argument("--enforce", action="store_true")
    checked.set_defaults(run=check)
    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
