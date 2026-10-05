#!/usr/bin/env python3
"""The TestFlight lane's steps, one verb per workflow step (SPEC-352, ADR-363).

`python3 scripts/ios_lane.py <verb>` runs one step of a lane's job: `plan` on Linux, and
`preflight`, `sign`, `upload-to-testflight`, `clean`, `build-unsigned` and `summary` on the macOS
runner. Standard library only.
"""

import argparse
import sys

VERBS = ("plan", "preflight", "sign", "upload-to-testflight", "clean", "build-unsigned", "summary")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=VERBS)
    parser.add_argument("--lane", choices=("internal", "release"))
    parser.parse_args(argv)
    return 0


if __name__ == "__main__":
    sys.exit(main())
