"""Stub: the copy before SPEC-064 (no integrity check, no temporary file, no pruning)."""

import shutil
from datetime import datetime, timezone
from pathlib import Path

KEEP = 1


def parse_stamp(text):
    return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)


def integrity_ok(path):
    return True


def main(argv=None, now=None):
    args = dict(zip(argv[::2], argv[1::2]))
    backups = Path(args["--backups"])
    backups.mkdir(parents=True, exist_ok=True)
    stamp = now.strftime("%Y%m%dT%H%M%SZ")
    try:
        shutil.copyfile(args["--database"], backups / f"deck_streak-{stamp}.db")
    except OSError:
        return 1
    return 0
