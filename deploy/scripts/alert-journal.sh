#!/bin/sh
# DeckStreak's one alert path (SPEC-031 R3 as SPEC-582 R7 amends it; ADR-601 D4, D9): write one
# line at error priority to the journal saying that a unit failed, naming the unit and its result.
#
#   alert-journal.sh UNIT
#
# deck-streak-alert@.service runs it as `alert-journal.sh %i` for every unit whose OnFailure= names
# it. systemd passes that unit as $MONITOR_UNIT, its result as $MONITOR_SERVICE_RESULT (exit-code,
# oom-kill, watchdog, timeout...) and its exit as $MONITOR_EXIT_STATUS; the argument names the unit
# when systemd passes none.
#
# It reads no credential, makes no request and reads no journal. The line quotes no line of the
# failed run, because error text in an alert can carry user data and the unit's own lines already
# reach the journal. Its standard error reaches the journal under the alert instance's own
# identifier, and the `<3>` prefix gives the line error priority: it is the one record a monitoring
# service off the host matches.
set -eu

unit="${MONITOR_UNIT:-${1:?usage: alert-journal.sh UNIT}}"
result="${MONITOR_SERVICE_RESULT:-unknown}"
status="${MONITOR_EXIT_STATUS:-}"

text="DeckStreak: $unit failed ($result${status:+, status $status})"

# One line at error priority. A line that cannot be written fails this instance, which stays in
# systemctl --failed: the alert template names no OnFailure=, so nothing pages about it.
printf '<3>%s\n' "$text" >&2
