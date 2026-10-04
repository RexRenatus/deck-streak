#!/usr/bin/env bash
# DeckStreak's weekly restore drill (SPEC-064 R5; ADR-010, ADR-064).
#
# Restores the replica with `litestream restore -o` to a private temporary path, opens the newest
# daily copy, runs `PRAGMA integrity_check` on both and compares each one's migration version with
# the live database's. Both copies are removed on exit, whatever the outcome, and the daily copy
# itself stays where it is. It exits non-zero on any failed step, which pages through OnFailure=.
# It restores nothing into the live database.
#
# The sync server's part, `--part sync`, restores the newest snapshot archive into a private
# directory instead, checks every file against the archive's manifest of sha256 digests, runs
# `PRAGMA integrity_check` on each user's collection and media index, and opens each collection to
# count its cards (SPEC-337 R5; ADR-347 D5, D12). It runs as the sync family's own user, in a unit of
# its own (SPEC-340 R5; ADR-351 D1), which is installed only with the server, so a missing snapshot
# directory or one with no archive fails it.
#
# The migration rule: the replica is at the live database's version (it follows every commit), and a
# daily copy is at that version or an earlier one, older after a deploy, never ahead of the live one.
#
# The database's unit passes no arguments, and the part is the database's. The sync family's unit
# passes `--part sync`. The other flags exist so a test can run it over a synthetic tree.
set -euo pipefail
umask 077

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${STATE_DIRECTORY:-/var/lib/deck-streak}"
state="${state%%:*}"
litestream="litestream"
config="$here/../litestream.yml"
database="$state/deck_streak.db"
backups="$state/backups"
snapshots="$state/sync-snapshots"
part="database"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --litestream) litestream="$2" ;;
    --config) config="$2" ;;
    --database) database="$2" ;;
    --backups) backups="$2" ;;
    --snapshots) snapshots="$2" ;;
    --part) part="$2" ;;
    *) echo "restore-drill: unknown argument $1" >&2; exit 2 ;;
  esac
  shift 2
done
case "$part" in
  database|sync) ;;
  *) echo "restore-drill: unknown part $part" >&2; exit 2 ;;
esac

work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
restored="$work/restored.db"
daily="$work/daily.db"

# The database's part. The body stays at the margin, as its here-document's Python must.
if [ "$part" = database ]; then
# The newest daily copy, by the instant its name carries.
newest=""
for candidate in "$backups"/deck_streak-*.db; do
  [ -f "$candidate" ] && newest="$candidate"
done
if [ -z "$newest" ]; then
  echo "restore-drill: no daily copy exists in the backups directory" >&2
  exit 1
fi
cp -- "$newest" "$daily"

"$litestream" restore -config "$config" -o "$restored" "$database"

python3 - "$database" "$restored" "$daily" <<'PY'
import sqlite3
import sys

live_path, restored_path, daily_path = sys.argv[1:4]


def open_read_only(path):
    return sqlite3.connect(f"file:{path}?mode=ro", uri=True)


def integrity(path):
    connection = open_read_only(path)
    try:
        return connection.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
    finally:
        connection.close()


def version(path):
    connection = open_read_only(path)
    try:
        row = connection.execute(
            "SELECT MAX(version) FROM _sqlx_migrations WHERE success = 1"
        ).fetchone()
        return row[0]
    finally:
        connection.close()


failed = []
for name, path in (("replica", restored_path), ("daily copy", daily_path)):
    if not integrity(path):
        failed.append(f"the {name} failed PRAGMA integrity_check")
live = version(live_path)
replica = version(restored_path)
copy = version(daily_path)
if replica != live:
    failed.append(f"the replica is at migration {replica}, the live database at {live}")
if copy is None or live is None or copy > live:
    failed.append(f"the daily copy is at migration {copy}, the live database at {live}")
for line in failed:
    print(f"restore-drill: {line}", file=sys.stderr)
if failed:
    sys.exit(1)
print(f"restore-drill: replica and daily copy are sound at migration {replica} and {copy}")
PY
exit 0
fi

# The sync server's part.
[ -d "$snapshots" ] || { echo "restore-drill: no snapshot directory" >&2; exit 1; }
mkdir -- "$work/sync"
python3 - "$snapshots" "$work/sync" <<'PY'
import hashlib
import re
import sqlite3
import sys
import tarfile
from pathlib import Path

snapshots, target = Path(sys.argv[1]), Path(sys.argv[2])
archive_name = re.compile(r"^sync-\d{8}T\d{6}Z\.tar$")
archives = sorted(p.name for p in snapshots.iterdir() if archive_name.match(p.name))
if not archives:
    print("restore-drill: the snapshot directory holds no snapshot archive", file=sys.stderr)
    sys.exit(1)
newest = snapshots / archives[-1]
manifest = {}
try:
    for line in newest.with_suffix(".sha256").read_text(encoding="utf-8").splitlines():
        digest, _, member = line.partition("  ")
        manifest[member] = digest
    with tarfile.open(newest) as archive:
        archive.extractall(target, filter="data")
except (OSError, tarfile.TarError) as error:
    print(f"restore-drill: the snapshot {newest.name} did not restore: {error}", file=sys.stderr)
    sys.exit(1)


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def integrity(path):
    connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        return connection.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
    except sqlite3.Error:
        return False
    finally:
        connection.close()


failed = []
found = sorted(str(p.relative_to(target)) for p in target.rglob("*") if p.is_file())
if found != sorted(manifest):
    failed.append("the restored files differ from the snapshot's manifest")
for member in found:
    if member in manifest and sha256(target / member) != manifest[member]:
        failed.append(f"{member} does not match its sha256 digest")
counts = []
for user in sorted(p for p in target.iterdir() if p.is_dir()):
    for name in ("collection.anki2", "media.db"):
        if (user / name).is_file() and not integrity(user / name):
            failed.append(f"{user.name}/{name} failed PRAGMA integrity_check")
    collection = user / "collection.anki2"
    if collection.is_file() and not failed:
        connection = sqlite3.connect(f"file:{collection}?mode=ro", uri=True)
        try:
            counts.append((user.name, connection.execute("SELECT count(*) FROM cards").fetchone()[0]))
        except sqlite3.Error as error:
            failed.append(f"{user.name}'s collection did not open: {error}")
        finally:
            connection.close()
for line in failed:
    print(f"restore-drill: {line}", file=sys.stderr)
if failed:
    sys.exit(1)
for user, count in counts:
    print(f"restore-drill: {user}: {count} card(s) in the restored collection")
print(f"restore-drill: the snapshot {newest.name} restores and matches its manifest")
PY
