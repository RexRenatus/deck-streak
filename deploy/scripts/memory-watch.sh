#!/bin/sh
# DeckStreak's memory watch (SPEC-031 R5; ADR-031, ADR-032): each DeckStreak unit's memory
# accounting, from the kernel's cgroup v2 files memory.events, memory.current and memory.max.
#
#   memory-watch.sh [--cgroup-root DIR]
#
# deck-streak-memory-watch.timer runs it every minute as deck-streak-memory-watch.service. It reads
# every cgroup named deck-streak-*.service under the system slice, a template's instances in their
# template's slice included:
#   - a new oom_kill or max event pages: printed at priority 3, and the script exits 1, so its unit
#     fails and OnFailure= pages through the alert template unit, which quotes these lines;
#   - a new high event, MemoryHigh throttling the unit, is logged at priority 4;
#   - usage past 90% of memory.max, the unit's MemoryMax= from deploy/host-budget.json, is logged
#     at priority 4 when it crosses, and again only after it has fallen back.
# Each unit's counters are remembered in $STATE_DIRECTORY with its cgroup's identity, the
# directory's inode, which the kernel gives no later cgroup, so each event pages once. A unit's first
# sight is its baseline; a cgroup made since the last run, by a restart or a oneshot's next run,
# counts every event it holds as new. --cgroup-root replaces /sys/fs/cgroup for a test's tree.
set -eu

root=/sys/fs/cgroup
if [ "$#" -eq 2 ] && [ "$1" = --cgroup-root ]; then
  root=$2
elif [ "$#" -ne 0 ]; then
  echo "<3>usage: memory-watch.sh [--cgroup-root DIR]"
  exit 2
fi
state="${STATE_DIRECTORY:?the unit sets StateDirectory=}"

# One counter of a memory.events file, 0 when the kernel does not report it.
counter() {
  awk -v key="$1" '$1 == key { value = $2 } END { print value + 0 }' "$2"
}

# A number read from a file, or 0 when the file holds none.
number() {
  value=$(cat "$1" 2>/dev/null || true)
  case $value in
    '' | *[!0-9]*) echo 0 ;;
    *) echo "$value" ;;
  esac
}

set -f
units=$(find "$root/system.slice" -type d -name 'deck-streak-*.service' 2>/dev/null | sort)
if [ -z "$units" ]; then
  # A watch that sees nothing pages once, and again only after it has seen a unit.
  if [ -e "$state/.blind" ]; then
    echo "<4>still no DeckStreak unit's cgroup under $root/system.slice"
    exit 0
  fi
  : >"$state/.blind"
  echo "<3>no DeckStreak unit's cgroup under $root/system.slice: the watch sees nothing"
  exit 1
fi
rm -f "$state/.blind"

paged=0
for cgroup in $units; do
  unit=${cgroup##*/}
  if [ ! -r "$cgroup/memory.events" ]; then
    echo "<4>$unit: its memory.events cannot be read"
    continue
  fi
  id=$(stat -c %i "$cgroup")
  oom_kill=$(counter oom_kill "$cgroup/memory.events")
  max=$(counter max "$cgroup/memory.events")
  high=$(counter high "$cgroup/memory.events")
  current=$(number "$cgroup/memory.current")
  limit=$(cat "$cgroup/memory.max" 2>/dev/null || echo max)
  record="$state/$unit"
  was_id='' was_oom_kill='' was_max='' was_high='' was_over=''
  if [ -r "$record" ]; then
    read -r was_id was_oom_kill was_max was_high was_over <"$record" || true
    case "$was_oom_kill$was_max$was_high$was_over" in
      '' | *[!0-9]*) was_id='' ;;
    esac
    if [ "$was_id" != "$id" ]; then
      # A cgroup made since the last run: every event it counts happened after that run.
      was_oom_kill=0 was_max=0 was_high=0 was_over=0
    fi
  else
    # The first sight of a unit: what it already counts is not new.
    was_oom_kill=$oom_kill was_max=$max was_high=$high was_over=0
  fi
  if [ "$oom_kill" -gt "$was_oom_kill" ]; then
    echo "<3>$unit: the kernel's OOM killer killed in it: memory.events oom_kill rose from $was_oom_kill to $oom_kill"
    paged=1
  fi
  if [ "$max" -gt "$was_max" ]; then
    echo "<3>$unit: it reached its MemoryMax: memory.events max rose from $was_max to $max"
    paged=1
  fi
  if [ "$high" -gt "$was_high" ]; then
    echo "<4>$unit: MemoryHigh throttled it: memory.events high rose from $was_high to $high"
  fi
  over=0
  case $limit in
    '' | *[!0-9]*) ;;
    *)
      if [ "$current" -gt $((limit * 9 / 10)) ]; then
        over=1
        if [ "$was_over" -eq 0 ]; then
          echo "<4>$unit: memory.current $current is past 90% of its MemoryMax, $limit"
        fi
      fi
      ;;
  esac
  printf '%s %s %s %s %s\n' "$id" "$oom_kill" "$max" "$high" "$over" >"$record.new"
  mv "$record.new" "$record"
done
exit "$paged"
