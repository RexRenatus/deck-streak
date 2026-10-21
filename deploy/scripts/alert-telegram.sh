#!/bin/sh
# DeckStreak's one alert path (SPEC-031 R3; ADR-031, ADR-038): page the owner on Telegram that a unit
# failed, naming the unit and its result and quoting its last error lines.
#
#   alert-telegram.sh UNIT
#
# deck-streak-alert@.service runs it as `alert-telegram.sh %i` for every unit whose OnFailure= names
# it. systemd passes that unit as $MONITOR_UNIT, its result as $MONITOR_SERVICE_RESULT (exit-code,
# oom-kill, watchdog, timeout...), its exit as $MONITOR_EXIT_STATUS and its run as
# $MONITOR_INVOCATION_ID; the argument names the unit when systemd passes none. The bot token and the
# owner's user id, which is the id of the owner's private chat, are credentials in
# $CREDENTIALS_DIRECTORY (LoadCredential=, ADR-038).
#
# The token never reaches a command line or the environment: the request's URL and its fields are
# written to curl's standard input as its configuration (`--config -`), which no process table
# shows. The text is plain, at most 3500 bytes cut at a character boundary, inside Telegram's 4096
# characters.
set -eu

unit="${MONITOR_UNIT:-${1:?usage: alert-telegram.sh UNIT}}"
result="${MONITOR_SERVICE_RESULT:-unknown}"
status="${MONITOR_EXIT_STATUS:-}"
token="$(cat "$CREDENTIALS_DIRECTORY/telegram-bot-token")"
chat="$(cat "$CREDENTIALS_DIRECTORY/owner-user-id")"

# A credential with nothing left once the command substitution has removed its trailing newlines
# refuses the page by its id, before the journal is read or a request is made (SPEC-066 R3;
# ADR-067). The line reaches the journal at error priority, and exit 1 leaves this instance failed,
# in systemctl --failed: the alert unit names no OnFailure=, so nothing pages about it (#285).
refuse_empty() {
  case $2 in
    '')
      echo "<3>the credential $1 is empty in the credentials directory: no page is sent" >&2
      exit 1
      ;;
  esac
}
refuse_empty telegram-bot-token "$token"
refuse_empty owner-user-id "$chat"

# The failed run's own error lines when systemd names the run, else the unit's latest; at most five.
# A journal that cannot be read quotes nothing, and the page still goes.
if [ -n "${MONITOR_INVOCATION_ID:-}" ]; then
  lines="$(journalctl "_SYSTEMD_INVOCATION_ID=$MONITOR_INVOCATION_ID" --priority=err --lines=5 \
    --output=cat --no-pager --quiet 2>/dev/null | tail -n 5 || true)"
else
  lines="$(journalctl --unit="$unit" --priority=err --lines=5 --output=cat --no-pager --quiet \
    2>/dev/null | tail -n 5 || true)"
fi

text="DeckStreak: $unit failed ($result${status:+, status $status})"
if [ -n "$lines" ]; then
  text="$text
$lines"
fi
# At most 3500 bytes, and never half a character: a UTF-8 sequence the cut split is dropped whole.
text="$(printf '%s' "$text" | head -c 3500 | LC_ALL=C sed -e \
  '$ s/\([\xC0-\xDF]\|[\xE0-\xEF][\x80-\xBF]\{0,1\}\|[\xF0-\xF7][\x80-\xBF]\{0,2\}\)$//')"

# A value of curl's configuration: each backslash, double quote and carriage return escaped, and each
# line break written \n, since a value must stay on its one line (curl's --config).
quote() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\r/\\r/g' |
    awk 'NR > 1 { printf "\\n" } { printf "%s", $0 }'
}

{
  printf 'url = "%s"\n' "$(quote "https://api.telegram.org/bot$token/sendMessage")"
  printf 'data-urlencode = "%s"\n' "$(quote "chat_id=$chat")"
  printf 'data-urlencode = "%s"\n' "$(quote "text=$text")"
} | curl --fail --silent --show-error --max-time 10 --retry 3 --retry-delay 2 --config - >/dev/null
