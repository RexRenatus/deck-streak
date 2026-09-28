#!/bin/sh
# Page the owner on Telegram that a unit failed, with its result and its last error lines.
# Run by alert@.template.service as `alert-telegram.sh %i`. systemd 251 and later pass the failed
# unit's $MONITOR_UNIT and $MONITOR_SERVICE_RESULT (oom-kill, watchdog, timeout, exit-code...).
set -eu
unit="${MONITOR_UNIT:-$1}"
result="${MONITOR_SERVICE_RESULT:-unknown}"
status="${MONITOR_EXIT_STATUS:-?}"
token="$(cat "$CREDENTIALS_DIRECTORY/telegram-bot-token")"
chat="$(cat "$CREDENTIALS_DIRECTORY/telegram-alert-chat")"
lines="$(journalctl --unit "$unit" --priority err --lines 5 --output cat --no-pager 2>/dev/null || true)"
text="$unit failed: $result ($status)
$lines"
# Telegram's limit is 4096 characters per message; keep well under it.
text="$(printf '%s' "$text" | cut -c1-3500)"
curl --fail --silent --show-error --max-time 10 --retry 3 --retry-delay 2 \
  --data-urlencode "chat_id=$chat" --data-urlencode "text=$text" \
  "https://api.telegram.org/bot$token/sendMessage" > /dev/null
