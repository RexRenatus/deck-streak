#!/usr/bin/env python3
"""The host inventory (SPEC-060 R1, R2), as a red-first stub: it runs its reads and the health
checks as they come, with no allow list and no nice, and measures nothing of the disk."""

import argparse
import json
import subprocess
from datetime import datetime, timezone

READS = [
    ["systemctl", "list-unit-files", "--no-legend", "--no-pager"],
    ["systemctl", "list-units", "--all", "--no-legend", "--no-pager", "--plain"],
    ["dpkg-query", "-W"],
    ["ps", "-eo", "pid=,uid=,rss=,comm="],
]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("rules")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    with open(args.rules, encoding="utf-8") as handle:
        rules = json.load(handle)
    commands = []
    for argv in READS + [check["argv"] for check in rules.get("health", []) if "argv" in check]:
        done = subprocess.run(argv, capture_output=True, text=True, check=False)
        commands.append({"argv": argv, "exit": done.returncode})
    roots = [
        {"path": root["path"], "space": {"total": 0, "free": 0}, "size": 0}
        for root in rules["roots"]
    ]
    record = {
        "taken_at": datetime.now(timezone.utc).isoformat(),
        "roots": roots,
        "mounts": [],
        "entries": [],
        "backups": [],
        "venvs": [],
        "worktrees": [],
        "packages": [],
        "units": [],
        "memory": [],
        "commands": commands,
        "health_before": [],
    }
    with open(args.out, "w", encoding="utf-8") as handle:
        json.dump(record, handle)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
