#!/usr/bin/env bash
# The headless runner (SPEC-043 R1 to R5, ADR-043, ADR-015, ADR-038).
#
#   agent/run-headless.sh PROMPT_FILE
#
# It launches one headless Claude Code run behind the loopback proxy, with the prompt on stdin and
# the device key in the child's environment only. Exit codes:
#   0  success; the result JSON is on stdout
#   1  a tool, the input or the key is missing
#   2  a refused shape (a flag, a remote URL)
#   3  the proxy rejected the key
#   4  the roster is exhausted; the retry instant is on the REFUSE line
#   5  the proxy or its tunnel is unreachable, or answered no documented word
#   6  Claude Code failed, ran past its wall clock or returned an error result (the result JSON, if
#      there is one, is on stdout so the caller can read which cap was reached)
# Every refusal is one `REFUSE:` line on stderr, and none names a key.
#
# Configuration comes from the environment, and never from an argument:
#   DECKSTREAK_AGENT_PROXY_URL         a loopback base URL (required)
#   DECKSTREAK_AGENT_CAPACITY_PATH     the capacity endpoint's path (required)
#   DECKSTREAK_AGENT_MAX_TURNS         default 30
#   DECKSTREAK_AGENT_MAX_BUDGET_USD    default 5
#   DECKSTREAK_AGENT_WALL_SECONDS      default 1800
#   CREDENTIALS_DIRECTORY              holds the credential `agent-device-key`
set -euo pipefail

refuse() {
  local code="$1"
  shift
  printf 'REFUSE: %s\n' "$*" >&2
  exit "$code"
}

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
settings="$here/settings.json"

# The shape: exactly one argument, the prompt file. Any flag is refused by name, so no caller can
# widen the launch (`--bare` skips the settings; a bypass flag skips the permission gate).
if [ "$#" -lt 1 ]; then
  refuse 2 "the prompt file is the one argument"
fi
for argument in "$@"; do
  case "$argument" in
    --bare | --dangerously-skip-permissions | --allow-dangerously-skip-permissions)
      refuse 2 "the flag $argument is refused"
      ;;
    --permission-mode | --settings | -p | --print)
      refuse 2 "the flag $argument is set by the runner, never by a caller"
      ;;
    -*)
      refuse 2 "a flag is not a prompt file"
      ;;
  esac
done
[ "$#" -eq 1 ] || refuse 2 "one argument is taken, the prompt file"
prompt="$1"
[ -r "$prompt" ] || refuse 1 "the prompt file is not readable"
[ -r "$settings" ] || refuse 1 "the settings file is missing"

for tool in claude curl timeout mktemp grep sed cut tail; do
  command -v "$tool" >/dev/null 2>&1 || refuse 1 "the tool $tool is missing"
done

url="${DECKSTREAK_AGENT_PROXY_URL:-}"
[ -n "$url" ] || refuse 1 "the proxy URL is not set"
case "$url" in
  http://127.0.0.1 | http://127.0.0.1:* | http://localhost | http://localhost:* | \
    http://\[::1\] | http://\[::1\]:*) ;;
  *) refuse 2 "the proxy URL is not a loopback URL" ;;
esac
capacity_path="${DECKSTREAK_AGENT_CAPACITY_PATH:-}"
[ -n "$capacity_path" ] || refuse 1 "the capacity path is not set"

turns="${DECKSTREAK_AGENT_MAX_TURNS:-30}"
budget="${DECKSTREAK_AGENT_MAX_BUDGET_USD:-5}"
wall="${DECKSTREAK_AGENT_WALL_SECONDS:-1800}"
case "$turns" in '' | *[!0-9]*) refuse 2 "the turn cap is not a whole number" ;; esac
case "$wall" in '' | *[!0-9]*) refuse 2 "the wall clock is not a whole number of seconds" ;; esac
case "$budget" in '' | *[!0-9.]*) refuse 2 "the budget is not a decimal amount" ;; esac

# The device key is a systemd credential (ADR-038): read once, held in an unexported variable.
credential_file="${CREDENTIALS_DIRECTORY:-}/agent-device-key"
if [ -z "${CREDENTIALS_DIRECTORY:-}" ] || [ ! -r "$credential_file" ]; then
  refuse 1 "the device key credential is not readable"
fi
device_key="$(<"$credential_file")"
[ -n "$device_key" ] || refuse 1 "the device key credential is empty"

work="$(mktemp -d)"
cleanup() {
  rm -f "$work/body" "$work/result" "$work/stderr"
  rmdir "$work" 2>/dev/null || true
}
trap cleanup EXIT

# The preflight: the key goes to curl on its stdin (printf is a builtin, so no argv holds it).
http=""
curl_status=0
http="$(printf 'Authorization: Bearer %s\n' "$device_key" |
  curl -sS -m 15 -H @- -o "$work/body" -w '%{http_code}' "$url$capacity_path" 2>/dev/null)" ||
  curl_status=$?
[ "$curl_status" -eq 0 ] || refuse 5 "the proxy could not be reached"
case "$http" in
  401) refuse 3 "the proxy rejected the device key" ;;
  200) ;;
  *) refuse 5 "the proxy answered no documented status" ;;
esac
status="$(grep -o '"status" *: *"[a-z_]*"' "$work/body" | head -n 1 | sed 's/.*: *"\(.*\)"/\1/' || true)"
case "$status" in
  ready) ;;
  exhausted)
    retry="$(grep -o '"retry_after" *: *"[^"]*"' "$work/body" | head -n 1 | sed 's/.*: *"\(.*\)"/\1/' || true)"
    refuse 4 "the roster is exhausted; retry after ${retry:-an unstated instant}"
    ;;
  *) refuse 5 "the proxy answered no documented status word" ;;
esac

# The launch: a scrubbed environment, the key only in the child's, every cap on the command line.
unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN ANTHROPIC_CUSTOM_HEADERS CLAUDE_CONFIG_DIR \
  CLAUDE_CODE_USE_BEDROCK CLAUDE_CODE_USE_VERTEX CLAUDE_CODE_USE_FOUNDRY CLAUDE_CODE_SIMPLE
export ANTHROPIC_BASE_URL="$url"
export CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1
export DISABLE_AUTOUPDATER=1

exit_code=0
CLAUDE_CODE_OAUTH_TOKEN="$device_key" timeout --kill-after=30 "$wall" \
  claude -p \
  --output-format json \
  --max-turns "$turns" \
  --max-budget-usd "$budget" \
  --permission-mode dontAsk \
  --strict-mcp-config \
  --settings "$settings" \
  <"$prompt" >"$work/result" 2>"$work/stderr" || exit_code=$?

tail_line=""
if [ -s "$work/stderr" ]; then
  tail_line="$(tail -n 1 "$work/stderr" | cut -c 1-200)"
  tail_line="${tail_line//"$device_key"/[redacted]}"
fi
unset device_key

if [ "$exit_code" -eq 124 ] || [ "$exit_code" -eq 137 ] || [ "$exit_code" -eq 143 ]; then
  cat "$work/result" 2>/dev/null || true
  refuse 6 "claude ran past its wall clock of ${wall} seconds"
fi
if [ "$exit_code" -ne 0 ]; then
  cat "$work/result" 2>/dev/null || true
  refuse 6 "claude failed with status $exit_code: $tail_line"
fi

# The result's own verdict is read before its text: `subtype` and `is_error` come first in the
# JSON, so the first occurrence of each is the CLI's, never a reply that quotes one.
subtype="$(grep -o '"subtype" *: *"[a-z_]*"' "$work/result" | head -n 1 | sed 's/.*: *"\(.*\)"/\1/' || true)"
is_error="$(grep -o '"is_error" *: *\(true\|false\)' "$work/result" | head -n 1 | sed 's/.*: *//' || true)"
if [ "$subtype" != "success" ] || [ "$is_error" != "false" ]; then
  cat "$work/result"
  refuse 6 "claude returned an error result: ${subtype:-no subtype}"
fi
cat "$work/result"
