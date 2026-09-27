#!/bin/sh
# Watch each service's cgroup memory (docs.kernel.org, cgroup v2 memory.events). Arguments are
# unit names without `.service`. A new oom_kill or max event pages (exit 1, printed at <3>); a new
# high event, which is MemoryHigh throttling, and usage above 90% of MemoryMax are logged at <4>.
set -eu
state="${STATE_DIRECTORY:-.}"
paged=0
for unit in "$@"; do
  cgroup="/sys/fs/cgroup/system.slice/$unit.service"
  [ -r "$cgroup/memory.events" ] || { echo "<4>$unit: no cgroup at $cgroup"; continue; }
  for counter in oom_kill max high; do
    now="$(awk -v key="$counter" '$1 == key { print $2 }' "$cgroup/memory.events")"
    before="$(cat "$state/$unit.$counter" 2>/dev/null || echo "$now")"
    echo "$now" > "$state/$unit.$counter"
    [ "$now" -gt "$before" ] || continue
    if [ "$counter" = high ]; then
      echo "<4>$unit: MemoryHigh throttled it $((now - before)) more time(s)"
    else
      echo "<3>$unit: memory.events $counter rose from $before to $now"
      paged=1
    fi
  done
  current="$(cat "$cgroup/memory.current")"
  limit="$(cat "$cgroup/memory.max")"
  if [ "$limit" != max ] && [ "$current" -gt $((limit / 10 * 9)) ]; then
    echo "<4>$unit: memory.current $current is above 90% of MemoryMax $limit"
  fi
done
exit "$paged"
