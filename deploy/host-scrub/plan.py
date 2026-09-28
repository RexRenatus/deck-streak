#!/usr/bin/env python3
"""The deletion list (SPEC-060 R4), as a red-first stub: a checklist of every file under every
root, with no rule, no reason and no digest."""

import argparse
import hashlib
import json
import os


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("inventory")
    parser.add_argument("rules")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    with open(args.inventory, encoding="utf-8") as handle:
        inventory = json.load(handle)
    with open(args.rules, encoding="utf-8") as handle:
        rules = json.load(handle)
    items = []
    for root in rules["roots"]:
        for directory, _dirs, files in os.walk(root["path"]):
            for name in sorted(files):
                path = os.path.join(directory, name)
                item = {"class": "file", "rule": "", "reason": "", "path": path, "bytes": 0}
                items.append({"id": f"i{len(items) + 1:03d}", **item, "digest": ""})
    listing = {"inventory": {"taken_at": inventory["taken_at"]}, "items": items, "bytes": 0}
    text = json.dumps(listing, sort_keys=True, separators=(",", ":"))
    listing["digest"] = hashlib.sha256(text.encode()).hexdigest()
    with open(args.out, "w", encoding="utf-8") as handle:
        json.dump(listing, handle)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
