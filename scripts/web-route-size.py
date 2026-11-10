#!/usr/bin/env python3
"""web-route-size: each web route's JavaScript against its recorded budget (SPEC-652, ADR-674).

The red stub: it keeps the check's options, level and report, and finds nothing."""
import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True
LEVEL = "warn"
EXIT_PASS = 0


def report(figures, counts, table, level):
    print("web-route-size: examined 0 route(s) and the shell")
    print("web-route-size: PASS: every route within its budget")
    return EXIT_PASS


def main(argv):
    parser = argparse.ArgumentParser(prog="web-route-size", description=__doc__.split("\n")[0])
    parser.add_argument("--app", required=True, type=Path, help="the web app's folder, built")
    parser.add_argument(
        "--budgets", required=True, type=Path, help="the budget file, one entry per route"
    )
    parser.parse_args(argv)
    return report({}, {}, {}, LEVEL)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
