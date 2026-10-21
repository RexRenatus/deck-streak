#!/usr/bin/env python3
"""A stand-in for the vault-duties pack's probe, for crates/vault/tests/staged.rs (SPEC-042 R4).

The gate runs the pack's probe once for each blocking class:
`python3 <probe> --root <run> --subject <run> [--vault <vault>] check <class>`. A class that
examined nothing of a run exits 3 and prints `<class>: VOID: examined nothing`, and the gate passes
over it; any other ending but 0 and 1 fails closed. This stand-in reads what to do from the run's
`probe-mode` file:

- `examined-nothing`: `note-links` is green (exit 0), and every other class examined nothing.
- `no-verdict`: `note-links` is green, and every other class exits 3 with another line.
- `nothing-judged`: every class examined nothing.
"""

import sys
from pathlib import Path


def main(argv):
    root = Path(argv[argv.index("--root") + 1])
    checked = argv[argv.index("check") + 1]
    mode = (root / "probe-mode").read_text(encoding="utf-8").strip()
    if checked == "note-links" and mode != "nothing-judged":
        print("note-links: GREEN: examined 1 run")
        return 0
    if mode == "no-verdict":
        print(f"{checked}: VOID: the run could not be read")
        return 3
    print(f"{checked}: VOID: examined nothing")
    return 3


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
