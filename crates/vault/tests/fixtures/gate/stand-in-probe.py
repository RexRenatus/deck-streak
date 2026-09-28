#!/usr/bin/env python3
"""A stand-in for the vault-duties pack's probe, for crates/vault/tests/staged.rs (SPEC-042 A10).

The executor's gate runs the pack's probe once for each blocking class:
`python3 <probe> --root <run> --subject <run> [--vault <vault>] check <class>`. This stand-in
judges one class as the pack does, `note-links`: a staged note that links a note the vault does not
hold is red (exit 1). Every other class is green (exit 0). The pack's own probe runs on the
maintainer's box (ADR-069).
"""

import re
import sys
from pathlib import Path

LINK = re.compile(r"\[\[([^\]|#]+)")


def main(argv):
    root = Path(argv[argv.index("--root") + 1])
    vault = Path(argv[argv.index("--vault") + 1]) if "--vault" in argv else None
    checked = argv[argv.index("check") + 1]
    if checked != "note-links":
        print(f"{checked}: GREEN: examined 1 run")
        return 0
    held = {path.stem for path in vault.rglob("*.md")} if vault else set()
    missing = sorted(
        {
            target.strip()
            for note in root.rglob("*.md")
            for target in LINK.findall(note.read_text(encoding="utf-8"))
            if target.strip() not in held
        }
    )
    if missing:
        print(f"note-links: RED: {len(missing)} link(s) to a note the vault does not hold")
        return 1
    print("note-links: GREEN: every link resolves")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
