#!/usr/bin/env python3
"""credential-pairs: the (unit, credential id) pairs the deploy templates declare (SPEC-061 R5).

The red-first stub: it lists no pair, examines nothing and refuses nothing.
"""

import json
import sys


def main(argv=None):
    print(json.dumps({"pairs": [], "optional": [], "examined": {"files": 0, "lines": 0}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
