#!/usr/bin/env python3
"""The regression record checker (SPEC-618, ADR-640): a stub.

It keeps every argument of the checker and refuses nothing: every folder reads as one that holds
no record.
"""

import argparse
import sys


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check",))
    parser.add_argument("--dir", required=True)
    parser.add_argument("--branch", required=True)
    parser.add_argument("--benchmarks", required=True)
    parser.parse_args(argv)
    print("perf-record: examined 0 records, 0 refused")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
