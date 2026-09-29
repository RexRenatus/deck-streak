#!/usr/bin/env bash
# DeckStreak's weekly restore drill (SPEC-064 R5; ADR-010, ADR-064).
#
# Restores the replica with `litestream restore -o` to a private temporary path, opens the newest
# daily copy, runs `PRAGMA integrity_check` on both and compares each one's migration version with
# the live database's. Both copies are removed on exit, whatever the outcome, and the daily copy
# itself stays where it is. It exits non-zero on any failed step, which pages through OnFailure=.
# It restores nothing into the live database.
#
# The migration rule: the replica is at the live database's version (it follows every commit), and a
# daily copy is at that version or an earlier one, older after a deploy, never ahead of the live one.
#
# The unit passes no arguments. The flags exist so a test can run it over a synthetic tree.
set -euo pipefail
umask 077

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${STATE_DIRECTORY:-/var/lib/deck-streak}"
state="${state%%:*}"
litestream="litestream"
config="$here/../litestream.yml"
database="$state/deck_streak.db"
backups="$state/backups"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --litestream) litestream="$2" ;;
    --config) config="$2" ;;
    --database) database="$2" ;;
    --backups) backups="$2" ;;
    *) echo "restore-drill: unknown argument $1" >&2; exit 2 ;;
  esac
  shift 2
done

work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
restored="$work/restored.db"
daily="$work/daily.db"

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
