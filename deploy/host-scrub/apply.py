#!/usr/bin/env python3
"""The approval-gated delete (SPEC-060 R6, R7), as a red-first stub: it deletes every item the list
holds, as a hand checklist would, with nothing tying the owner's approval to what goes."""

import argparse
import json
import os


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("list")
    parser.add_argument("approval")
    parser.add_argument("--rules", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    with open(args.list, encoding="utf-8") as handle:
        listing = json.load(handle)
    deleted = []
    for item in listing["items"]:
        path = item.get("path")
        if path and os.path.lexists(path):
            os.unlink(path)
            deleted.append({"id": item["id"], "path": path})
    with open(args.log, "w", encoding="utf-8") as handle:
        json.dump({"deleted": deleted, "refused": None}, handle)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
