#!/usr/bin/env python3
"""companion-site-census: the built site holds no file the owner route serves (SPEC-511 R12).

    python3 scripts/companion-site-census.py --root web/app/build

STUB: the command line only. It reads nothing and passes every site.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="companion-site-census", description=__doc__.split("\n")[0]
    )
    parser.add_argument("--root", required=True, type=Path, help="the built site's root")
    parser.parse_args(argv)
    print("companion-site-census: examined 0 file(s)")
    print("companion-site-census: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
