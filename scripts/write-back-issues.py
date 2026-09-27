#!/usr/bin/env python3
"""write-back-issues: replace `{{issue:KEY}}` tokens with issue numbers from the manifest.

    python3 scripts/write-back-issues.py FILE [FILE ...]

A planned SPEC, a wiring entry or a brief can name the issue that owns a piece of work before
the issue exists, as `{{issue:<key>}}`, where the key is the manifest's (a work unit like
`DS-W1-07`, a predecessor feature id, an epic `EPIC-W3` or an owner item `OWN-G4`). Once the
issues are created, this replaces each token with `#<number>`. An unknown key is refused by name
and nothing is written.
"""

import json
import re
import sys
from pathlib import Path

MANIFEST = Path(__file__).resolve().parents[1] / "docs" / "issues-manifest.json"
TOKEN = re.compile(r"\{\{issue:([A-Za-z0-9._-]+)\}\}")


def main(paths: list[str]) -> int:
    issues = json.loads(MANIFEST.read_text(encoding="utf-8"))["issues"]
    changed = {}
    missing = set()
    for name in paths:
        path = Path(name)
        text = path.read_text(encoding="utf-8")
        for key in TOKEN.findall(text):
            if key not in issues:
                missing.add(key)
        changed[path] = TOKEN.sub(lambda m: f"#{issues[m.group(1)]['number']}" if m.group(1) in issues else m.group(0), text)
    if missing:
        print(f"write-back-issues: no issue for {sorted(missing)}; nothing written")
        return 1
    for path, text in changed.items():
        path.write_text(text, encoding="utf-8")
    print(f"write-back-issues: {len(changed)} file(s) written")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
