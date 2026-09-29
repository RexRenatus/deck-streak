#!/usr/bin/env python3
"""DeckStreak's daily backup of its database (SPEC-064 R4; ADR-064).

Copies the live database with SQLite's online backup API into a temporary file beside the backups
directory, runs `PRAGMA integrity_check` on the copy, renames it into place and keeps the newest
KEEP copies. It exits non-zero on any failed step and leaves the copies that were there untouched:
a copy is pruned only after a new one is in place. It copies the database and nothing else: not the
collection copy, which the day's sync downloads again (ADR-037), and not a credential.

Standard library only, so the unit needs no interpreter of its own.
"""

import argparse
import os
import re
import sqlite3
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

# How many daily copies stay on the host (SPEC-064 R9): with one copy a day, the oldest is KEEP days
# old, inside the declared backups window.
KEEP = 3
DATABASE_NAME = "deck_streak.db"
COPY_PREFIX = "deck_streak-"
COPY_SUFFIX = ".db"
STAMP_FORMAT = "%Y%m%dT%H%M%SZ"
COPY_NAME = re.compile(r"^deck_streak-\d{8}T\d{6}Z\.db$")


def parse_stamp(text):
    """A UTC instant from `YYYY-MM-DDTHH:MM:SSZ`, for a caller that fixes the clock."""
    return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)


def integrity_ok(path):
    """True when `PRAGMA integrity_check` on the file at `path` answers exactly `ok`."""
    try:
        connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        try:
            rows = connection.execute("PRAGMA integrity_check").fetchall()
        finally:
            connection.close()
    except sqlite3.Error:
        return False
    return rows == [("ok",)]


def copy_database(source, target):
    """The online backup of `source` into a new file at `target`, which stays private to its user."""
    origin = sqlite3.connect(f"file:{source}?mode=ro", uri=True, timeout=30)
    try:
        destination = sqlite3.connect(target)
        try:
            origin.backup(destination)
            # A copy is one file: the source's WAL mode would leave a -wal and a -shm beside it.
            destination.execute("PRAGMA journal_mode=DELETE")
        finally:
            destination.close()
    finally:
        origin.close()


def prune(backups, keep):
    """Remove every copy but the newest `keep`, newest by the instant its name carries."""
    names = sorted(p.name for p in backups.iterdir() if COPY_NAME.match(p.name))
    for name in names[: max(len(names) - keep, 0)]:
        (backups / name).unlink()


def run(database, backups, keep, now):
    """One backup; 0 when a checked copy is in place, 1 on any failed step."""
    database = Path(database)
    backups = Path(backups)
    scratch = None
    try:
        backups.mkdir(parents=True, exist_ok=True)
        os.chmod(backups, 0o700)
        handle, scratch = tempfile.mkstemp(prefix=".backup-", suffix=".tmp", dir=backups.parent)
        os.close(handle)
        os.chmod(scratch, 0o600)
        copy_database(database, scratch)
        if not integrity_ok(scratch):
            print("backup: the copy failed its integrity check", file=sys.stderr)
            return 1
        name = f"{COPY_PREFIX}{now.astimezone(timezone.utc).strftime(STAMP_FORMAT)}{COPY_SUFFIX}"
        os.replace(scratch, backups / name)
        scratch = None
        prune(backups, keep)
        return 0
    except (OSError, sqlite3.Error) as error:
        print(f"backup: {error}", file=sys.stderr)
        return 1
    finally:
        if scratch is not None and os.path.exists(scratch):
            os.unlink(scratch)


def main(argv=None, now=None):
    state = os.environ.get("STATE_DIRECTORY", "/var/lib/deck-streak").split(":")[0]
    parser = argparse.ArgumentParser(description="DeckStreak's daily database backup")
    parser.add_argument("--database", default=str(Path(state) / DATABASE_NAME))
    parser.add_argument("--backups", default=str(Path(state) / "backups"))
    parser.add_argument("--keep", type=int, default=KEEP)
    args = parser.parse_args(argv)
    if args.keep < 1:
        print("backup: --keep must be at least 1", file=sys.stderr)
        return 1
    return run(args.database, args.backups, args.keep, now or datetime.now(timezone.utc))


if __name__ == "__main__":
    sys.exit(main())
