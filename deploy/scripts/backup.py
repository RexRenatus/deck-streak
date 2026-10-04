#!/usr/bin/env python3
"""DeckStreak's daily backup of its database (SPEC-064 R4; ADR-064).

Copies the live database with SQLite's online backup API into a temporary file beside the backups
directory, runs `PRAGMA integrity_check` on the copy, renames it into place and keeps the newest
KEEP copies. It exits non-zero on any failed step and leaves the copies that were there untouched:
a copy is pruned only after a new one is in place. It copies the database and not the ingest's
collection copy, which the day's sync downloads again (ADR-037), and not a credential.

It also snapshots the sync server's store (SPEC-337 R5; ADR-347 D5, D12), in two parts, each run
by a unit of the sync family's own (SPEC-340 R3; ADR-351 D1):

- `--sync-window` is the copy, run by deck-streak-sync-snapshot.service while the server is
  stopped: each user's collection and media index by the online backup, refused at once when a
  running server holds either, and the media files, as one generation that is published by a rename
  only when whole. The unit starts the server again whether this exits 0 or 1.
- `--sync-archive` is the archive, run by deck-streak-sync-archive.service after the server is
  started again: it checks the newest generation's databases with `PRAGMA integrity_check`, writes
  a manifest of sha256 digests, renames the generation into an archive, seals both to the owner's
  offline key by the command the settings name (SPEC-340 R12; ADR-351 D2), copies only the sealed
  files offsite by the command and bucket the settings name (each with arguments, no shell),
  removes the sealed files and keeps the newest KEEP archives. The database's daily run archives
  nothing.

Standard library only, so the unit needs no interpreter of its own.
"""

import argparse
import hashlib
import os
import re
import shlex
import sqlite3
import subprocess
import sys
import tarfile
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
# The sync server's store and what the window copies of each user's folder (the engine's layout).
SYNC_BASE = "/var/lib/deck-streak-sync-server"
SNAPSHOTS = "sync-snapshots"
SNAPSHOT_DATABASES = ("collection.anki2", "media.db")
GENERATION_NAME = re.compile(r"^gen-\d{8}T\d{6}Z\.tar$")
ARCHIVE_NAME = re.compile(r"^sync-\d{8}T\d{6}Z\.tar$")
MANIFEST_NAME = re.compile(r"^sync-\d{8}T\d{6}Z\.sha256$")
# Fixed names, each rewritten in place by the next run, so a failed run leaves nothing to collect.
WINDOW_PARTIAL = ".window.tar.tmp"
WINDOW_COPY = ".window.db"
CHECK_COPY = ".check.db"
MANIFEST_PARTIAL = ".manifest.tmp"
# The settings that name the offsite copy (deploy/deck-streak.env.example).
SNAPSHOT_COPY = "DECKSTREAK_SNAPSHOT_COPY"
SNAPSHOT_BUCKET = "DECKSTREAK_SNAPSHOT_BUCKET"
# The command that seals the archive and its manifest to the owner's offline public key before any
# copy (SPEC-340 R12; ADR-351 D2), and the first line every file it writes begins with: the age
# format's version line.
SNAPSHOT_SEAL = "DECKSTREAK_SNAPSHOT_SEAL"
SEAL_HEADER = b"age-encryption.org/v1\n"
CHUNK = 1 << 20


class Held(Exception):
    """A running server holds the database: the window copies only a stopped one."""


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


def prune(backups, keep, pattern=COPY_NAME):
    """Remove every file `pattern` names but the newest `keep`, newest by the instant its name
    carries."""
    names = sorted(p.name for p in backups.iterdir() if pattern.match(p.name))
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


def refuse_a_holder(status, remaining, total):
    """The online backup's progress callback: a busy or locked step means a running server holds
    the file, and raising ends the backup at once instead of retrying (ADR-347 D12)."""
    if status in (sqlite3.SQLITE_BUSY, sqlite3.SQLITE_LOCKED):
        raise Held("a running server holds the database; the window copies only a stopped one")


def copy_stopped(source, target):
    """The online backup of `source` into `target`, refused at once when another process holds
    `source`: no busy wait, and a busy step ends the copy."""
    origin = sqlite3.connect(f"file:{source}?mode=ro", uri=True, timeout=0)
    try:
        destination = sqlite3.connect(target)
        try:
            origin.backup(destination, progress=refuse_a_holder)
            destination.execute("PRAGMA journal_mode=DELETE")
        finally:
            destination.close()
    finally:
        origin.close()


def emptied(path):
    """Truncate a fixed working file to nothing, so no copy of a user's data rests in it."""
    if path.exists():
        os.truncate(path, 0)


def window(base, snapshots, now):
    """The stopped-server window's copy (ADR-347 D12): 0 when the generation is whole and
    published, 1 otherwise. It only copies; every check runs after the server is up again."""
    base = Path(base)
    snapshots = Path(snapshots)
    scratch = snapshots / WINDOW_COPY
    try:
        snapshots.mkdir(parents=True, exist_ok=True)
        os.chmod(snapshots, 0o700)
        users = sorted(p for p in base.iterdir() if p.is_dir())
        if not users:
            print("backup: the sync server's store holds no user", file=sys.stderr)
            return 1
        partial = snapshots / WINDOW_PARTIAL
        with open(partial, "wb") as handle:
            os.fchmod(handle.fileno(), 0o600)
            with tarfile.open(fileobj=handle, mode="w") as generation:
                for user in users:
                    for name in SNAPSHOT_DATABASES:
                        if (user / name).is_file():
                            copy_stopped(user / name, scratch)
                            generation.add(scratch, arcname=f"{user.name}/{name}")
                    media = user / "media"
                    files = sorted(media.iterdir()) if media.is_dir() else []
                    for item in files:
                        if item.is_file():
                            generation.add(item, arcname=f"{user.name}/media/{item.name}")
            handle.flush()
            os.fsync(handle.fileno())
        stamp = now.astimezone(timezone.utc).strftime(STAMP_FORMAT)
        os.replace(partial, snapshots / f"gen-{stamp}.tar")
        return 0
    except (OSError, sqlite3.Error, tarfile.TarError, Held) as error:
        print(f"backup: the window's copy failed: {error}", file=sys.stderr)
        return 1
    finally:
        emptied(scratch)


def checked_manifest(generation, scratch):
    """The manifest lines of a generation's members, or None when a database fails its check."""
    lines = []
    with tarfile.open(generation) as archive:
        for member in archive.getmembers():
            if not member.isfile():
                print(f"backup: {member.name} is not a file", file=sys.stderr)
                return None
            digest = hashlib.sha256()
            source = archive.extractfile(member)
            is_database = member.name.rsplit("/", 1)[-1] in SNAPSHOT_DATABASES
            with open(scratch, "wb") as target:
                os.fchmod(target.fileno(), 0o600)
                for chunk in iter(lambda: source.read(CHUNK), b""):
                    digest.update(chunk)
                    if is_database:
                        target.write(chunk)
            if is_database and not integrity_ok(scratch):
                print(f"backup: {member.name} failed its integrity check", file=sys.stderr)
                return None
            lines.append(f"{digest.hexdigest()}  {member.name}\n")
    return lines


def sealed_copy(seal, plain):
    """`plain` sealed by the seal command into `plain.age`, run as `[*seal, "--output", sealed,
    plain]` with no shell (SPEC-340 R12; ADR-351 D2): the sealed file, or None when the command
    fails or the file it wrote does not begin with the format's header."""
    sealed = Path(f"{plain}.age")
    done = subprocess.run([*seal, "--output", str(sealed), str(plain)], check=False)
    if done.returncode != 0:
        print(f"backup: the seal of {plain.name} exited {done.returncode}", file=sys.stderr)
        return None
    with open(sealed, "rb") as handle:
        if handle.read(len(SEAL_HEADER)) != SEAL_HEADER:
            print(f"backup: the seal of {plain.name} wrote no age header", file=sys.stderr)
            return None
    return sealed


def archive(snapshots, environ):
    """The archive unit's run (`--sync-archive`), after the window: 0 when there is no generation
    or the newest is checked, archived, sealed and copied offsite; 1 on any failed step. Only the
    sealed files are copied, and they are removed whatever the copy does."""
    snapshots = Path(snapshots)
    if not snapshots.is_dir():
        return 0
    generations = sorted(p.name for p in snapshots.iterdir() if GENERATION_NAME.match(p.name))
    if not generations:
        return 0
    command = shlex.split(environ.get(SNAPSHOT_COPY, ""))
    bucket = environ.get(SNAPSHOT_BUCKET, "")
    if not command or not bucket:
        print(
            f"backup: {SNAPSHOT_COPY} and {SNAPSHOT_BUCKET} name the offsite copy", file=sys.stderr
        )
        return 1
    seal = shlex.split(environ.get(SNAPSHOT_SEAL, ""))
    if not seal:
        print(f"backup: {SNAPSHOT_SEAL} names the seal of the offsite copy", file=sys.stderr)
        return 1
    newest = generations[-1]
    stamp = newest[len("gen-") : -len(".tar")]
    scratch = snapshots / CHECK_COPY
    manifest = snapshots / f"sync-{stamp}.sha256"
    tar = snapshots / f"sync-{stamp}.tar"
    try:
        lines = checked_manifest(snapshots / newest, scratch)
        if lines is None:
            return 1
        partial = snapshots / MANIFEST_PARTIAL
        with open(partial, "w", encoding="utf-8") as handle:
            os.fchmod(handle.fileno(), 0o600)
            handle.writelines(lines)
        os.replace(partial, manifest)
        os.replace(snapshots / newest, tar)
        sealed = [sealed_copy(seal, plain) for plain in (tar, manifest)]
        if None in sealed:
            return 1
        done = subprocess.run([*command, *map(str, sealed), bucket], check=False)
        if done.returncode != 0:
            print(f"backup: the offsite copy exited {done.returncode}", file=sys.stderr)
            return 1
        for pattern in (ARCHIVE_NAME, MANIFEST_NAME, GENERATION_NAME):
            prune(snapshots, KEEP, pattern)
        return 0
    except (OSError, sqlite3.Error, tarfile.TarError) as error:
        print(f"backup: the snapshot's archive failed: {error}", file=sys.stderr)
        return 1
    finally:
        emptied(scratch)
        for plain in (tar, manifest):
            Path(f"{plain}.age").unlink(missing_ok=True)


def main(argv=None, now=None):
    states = os.environ.get("STATE_DIRECTORY", "/var/lib/deck-streak").split(":")
    state = states[0]
    parser = argparse.ArgumentParser(description="DeckStreak's daily database backup")
    parser.add_argument("--database", default=str(Path(state) / DATABASE_NAME))
    parser.add_argument("--backups", default=str(Path(state) / "backups"))
    parser.add_argument("--keep", type=int, default=KEEP)
    parser.add_argument("--sync-window", action="store_true")
    parser.add_argument("--sync-archive", action="store_true")
    parser.add_argument("--sync-base", default=states[1] if len(states) > 1 else SYNC_BASE)
    parser.add_argument("--snapshots", default=str(Path(state) / SNAPSHOTS))
    args = parser.parse_args(argv)
    now = now or datetime.now(timezone.utc)
    if args.sync_window:
        return window(args.sync_base, args.snapshots, now)
    if args.sync_archive:
        return archive(args.snapshots, os.environ)
    if args.keep < 1:
        print("backup: --keep must be at least 1", file=sys.stderr)
        return 1
    return run(args.database, args.backups, args.keep, now)


if __name__ == "__main__":
    sys.exit(main())
