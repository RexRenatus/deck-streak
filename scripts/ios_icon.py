#!/usr/bin/env python3
"""The harness's app icon, written at build time (SPEC-352 R19, ADR-363).

`python3 scripts/ios_icon.py write <path>` is the one command line; any other is refused with the
usage line and exit 1. This is the stub R4 commits: it writes nothing yet. Standard library only.
"""

import sys


def main(argv):
    """Refuse any command line but `write <path>`; the stub writes nothing to the path."""
    if len(argv) != 2 or argv[0] != "write":
        sys.exit("usage: ios_icon.py write <path>")


if __name__ == "__main__":
    main(sys.argv[1:])
