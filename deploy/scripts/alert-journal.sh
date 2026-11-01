#!/bin/sh
# DeckStreak's one alert path (SPEC-031 R3 as SPEC-582 R7 amends it; ADR-601 D4): write one line
# at error priority to the journal saying that a unit failed, naming the unit and its result and
# quoting its last error lines.
#
#   alert-journal.sh UNIT
#
# deck-streak-alert@.service runs it as `alert-journal.sh %i` for every unit whose OnFailure= names
# it. systemd passes that unit as $MONITOR_UNIT, its result as $MONITOR_SERVICE_RESULT (exit-code,
# oom-kill, watchdog, timeout...), its exit as $MONITOR_EXIT_STATUS and its run as
# $MONITOR_INVOCATION_ID; the argument names the unit when systemd passes none.
#
# It reads no credential and makes no request. Its standard error reaches the journal under the
# alert instance's own identifier, and the `<3>` prefix gives the line error priority: it is the
# one record a monitoring service off the host matches. The text is one line of at most 3500
# bytes, cut at a character boundary; the quoted lines are joined by " | ".
set -eu

unit="${MONITOR_UNIT:-${1:?usage: alert-journal.sh UNIT}}"
result="${MONITOR_SERVICE_RESULT:-unknown}"
status="${MONITOR_EXIT_STATUS:-}"

# The failed run's own error lines when systemd names the run, else the unit's latest; at most five.
# A journal that cannot be read quotes nothing, and the line is still written.
if [ -n "${MONITOR_INVOCATION_ID:-}" ]; then
  lines="$(journalctl "_SYSTEMD_INVOCATION_ID=$MONITOR_INVOCATION_ID" --priority=err --lines=5 \
    --output=cat --no-pager --quiet 2>&- | tail -n 5 || true)"
else
  lines="$(journalctl --unit="$unit" --priority=err --lines=5 --output=cat --no-pager --quiet \
    2>&- | tail -n 5 || true)"
fi

text="DeckStreak: $unit failed ($result${status:+, status $status})"
if [ -n "$lines" ]; then
  joined="$(printf '%s\n' "$lines" | tr -d '\r' | awk 'NR > 1 { printf " | " } { printf "%s", $(0) }')"
  text="$text: $joined"
fi
# At most 3500 bytes, and never half a character: a UTF-8 sequence the cut split is dropped whole.
text="$(printf '%s' "$text" | head -c 3500 | LC_ALL=C sed -e \
  '$ s/\([\xC0-\xDF]\|[\xE0-\xEF][\x80-\xBF]\{0,1\}\|[\xF0-\xF7][\x80-\xBF]\{0,2\}\)$//')"

# One line at error priority. A line that cannot be written fails this instance, which stays in
# systemctl --failed: the alert template names no OnFailure=, so nothing pages about it.
printf '<3>%s\n' "$text" >&2
