#!/usr/bin/env python3
"""The bench plan of one ci lane (SPEC-618, ADR-640): a stub.

It keeps every argument of the plan and decides nothing: every run reads applicable and upload,
and nothing is written to the output files.
"""

import argparse
import sys

LANES = ("rust", "web")


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lane", required=True, choices=LANES)
    parser.add_argument("--event", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--head-repo", required=True)
    parser.add_argument("--changed")
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--metadata")
    parser.add_argument("--github-output")
    parser.add_argument("--step-summary")
    args = parser.parse_args(argv)
    print("applicable=true\nupload=true\nreason=the plan is a stub")
    print("bench-%s: applicable: the plan is a stub" % args.lane)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
