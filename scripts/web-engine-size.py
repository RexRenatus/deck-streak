#!/usr/bin/env python3
"""web-engine-size: the web engine's size gate (SPEC-338 R8, ADR-349). A stub: the tests come first."""

import argparse
import sys


def judge(total):
    """The stub judges nothing."""
    del total
    return 0


def main(argv):
    parser = argparse.ArgumentParser(prog="web-engine-size")
    parser.add_argument("--module", required=True)
    parser.add_argument("--bindings", required=True)
    parser.parse_args(argv)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
