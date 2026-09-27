#!/usr/bin/env python3
"""vendor-packs: re-vendor the pack probes from a phoenix-v2 checkout (SPEC-037).

    python3 scripts/vendor-packs.py --source DIR [--root ROOT] [--deny-list FILE]

The red-first stub: the old vendoring. It copies every file `.packs/VENDORED.json` lists and every
new file under a pack directory already vendored, and it applies no exclusion and no scan.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--source", required=True)
    parser.add_argument("--root", default=str(REPO))
    parser.add_argument("--deny-list", default=os.environ.get("PERSONA_CORE_DENY_LIST"))
    args = parser.parse_args()
    source, root = Path(args.source).resolve(), Path(args.root).resolve()
    manifest_path = root / ".packs" / "VENDORED.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    wanted = {entry["from"]: entry["path"] for entry in manifest["files"]}
    packs = {"/".join(src.split("/")[:3]) for src in wanted if src.startswith("skills/packs/")}
    for pack in sorted(packs):
        for path in sorted((source / pack).rglob("*")):
            rel = path.relative_to(source).as_posix()
            if path.is_file() and rel not in wanted:
                wanted[rel] = ".packs/" + rel
    files = []
    for src, dst in sorted(wanted.items()):
        if not (source / src).is_file():
            continue
        (root / dst).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / src, root / dst)
        digest = hashlib.sha256((root / dst).read_bytes()).hexdigest()
        files.append({"path": dst, "from": src, "sha256": digest})
    manifest["files"] = sorted(files, key=lambda entry: entry["path"])
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(len(files), "files vendored")
    return 0


if __name__ == "__main__":
    sys.exit(main())
