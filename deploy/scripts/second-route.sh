#!/bin/sh
# DeckStreak's second route (SPEC-396; ADR-410): tells the owner when the alert sender fails or goes
# silent, through a process and two credentials of its own.
#
#   second-route.sh
#
# deck-streak-second-route.timer runs it every ten minutes as deck-streak-second-route.service. It
# reads the alert path from the service manager (`systemctl` with list-units, list-unit-files and
# show, and nothing else) and never runs the alert script:
#   - each failed alert instance, keyed `failed <instance> <invocation id>`;
#   - the alert template's unit-file state, keyed `template <state>` unless it is static (absent
#     and masked included);
#   - `unreadable` when the service manager answers out of shape or fails.
# The keys not yet in $STATE_DIRECTORY/reported are sent in one report to the report address, and
# only after that request is delivered are the keys of this read recorded, so a key whose instance
# is gone is dropped and a later failure under the same name is new. Then one request with no body
# goes to the check-in address, unless the read was unreadable. The receiver, off the host, tells
# the owner of a report and of check-ins that stop.
#
# Its own failure is an episode (an empty credential, an address that is not https, a request not
# delivered): the first run of an episode prints one line at priority 3 and exits 1, so OnFailure=
# pages through the alert sender; a later run prints its line at priority 4 and exits 0; a run that
# delivers every request it makes ends the episode. The episode is a marker file in
# $STATE_DIRECTORY, open while it holds text and emptied, never removed, as the memory watch keeps
# its own. No line names a credential's value; each request's address and body go to curl on its
# standard input (`--config -`), never on a command line.
set -eu
LC_ALL=C
export LC_ALL

readonly SECOND_ROUTE_CHECK_IN=second-route-check-in
readonly SECOND_ROUTE_REPORT=second-route-report
readonly HEADER='DeckStreak: the alert sender cannot page the owner.'
# The report fits 3500 bytes in whole lines; this much is kept back for the count line.
readonly REPORT_BYTES=3500
readonly COUNT_LINE_BYTES=12

state="${STATE_DIRECTORY:?the unit sets StateDirectory=}"
credentials="${CREDENTIALS_DIRECTORY:?the unit loads its credentials}"
keys=''
unreadable=0

# Its own failure. The first run of an episode: $1 at priority 3, exit 1. A later run: $2, or $1
# again, at priority 4, exit 0.
own_failure() {
  if [ -s "$state/episode" ]; then
    echo "<4>${2:-$1}" >&2
    exit 0
  fi
  echo open >"$state/episode"
  echo "<3>$1" >&2
  exit 1
}

# A value of curl's configuration: each backslash, double quote and carriage return escaped, and each
# line break written \n, since a value must stay on its one line (curl's --config).
quote() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\r/\\r/g' |
    awk 'NR > 1 { printf "\\n" } { printf "%s", $0 }'
}

# One request: the address $1, and the body $2 when there is one. Curl's own words are dropped, since
# one may name the host.
post() {
  {
    printf 'url = "%s"\n' "$(quote "$1")"
    if [ "$#" -gt 1 ]; then
      printf 'data-binary = "%s"\n' "$(quote "$2")"
    fi
  } | curl --fail --silent --show-error --max-time 10 --retry 3 --retry-delay 2 --config - \
    >/dev/null 2>&1
}

# A credential judged by its id: empty, or not an https address, fails the run before anything is
# read or sent.
judge() {
  case $2 in
    '') own_failure "the credential $1 is empty in the credentials directory: no request is sent" ;;
    https://*) ;;
    *) own_failure "the credential $1 is not an https address: no request is sent" ;;
  esac
}

# Adds one key to the read.
add_key() {
  keys="${keys:+$keys
}$1"
}

# The alert path as the service manager holds it now: the keys of this read.
read_alert_path() {
  set -f
  if listed="$(systemctl list-units --state=failed --plain --no-legend --no-pager --full \
    'deck-streak-alert@*.service')"; then
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      # shellcheck disable=SC2086
      set -- $line
      name=$1
      case $name in
        deck-streak-alert@?*.service) ;;
        *)
          unreadable=1
          continue
          ;;
      esac
      case $name in
        *[!A-Za-z0-9:_.@\\-]*)
          unreadable=1
          continue
          ;;
      esac
      if invocation="$(systemctl show --property=InvocationID --value "$name")"; then
        case $invocation in
          *[!0-9a-f]*) unreadable=1 ;;
          ????????????????????????????????) add_key "failed $name $invocation" ;;
          *) unreadable=1 ;;
        esac
      else
        unreadable=1
      fi
    done <<EOF_LISTED
$listed
EOF_LISTED
  else
    unreadable=1
  fi
  if files="$(systemctl list-unit-files --no-legend --no-pager --full deck-streak-alert@.service)"; then
    files_status=0
  else
    files_status=$?
  fi
  case $files in
    '') add_key 'template absent' ;;
    *)
      if [ "$files_status" -ne 0 ]; then
        unreadable=1
      else
        # shellcheck disable=SC2086
        set -- $files
        case $files in
          *'
'*) unreadable=1 ;;
          *)
            if [ "$#" -lt 2 ]; then
              unreadable=1
            elif [ "$2" != static ]; then
              add_key "template $2"
            fi
            ;;
        esac
      fi
      ;;
  esac
  if [ "$unreadable" -eq 1 ]; then
    add_key unreadable
  fi
  set +f
}

# Whether the key $1 is among the reported ones.
is_reported() {
  case "
$reported
" in
    *"
$1
"*) return 0 ;;
  esac
  return 1
}

# The report of the keys not yet reported, in whole lines within the bound, the rest counted; the
# keys of this read are recorded only once the request is delivered.
tell_owner() {
  reported="$(cat "$state/reported" 2>/dev/null || true)"
  new=''
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    if ! is_reported "$line"; then
      new="${new:+$new
}$line"
    fi
  done <<EOF_KEYS
$keys
EOF_KEYS
  if [ -n "$new" ]; then
    body="$HEADER"
    left=0
    while IFS= read -r line; do
      size="$(printf '%s\n%s' "$body" "$line" | wc -c)"
      if [ "$left" -eq 0 ] && [ "$size" -le $((REPORT_BYTES - COUNT_LINE_BYTES)) ]; then
        body="$body
$line"
      else
        left=$((left + 1))
      fi
    done <<EOF_NEW
$new
EOF_NEW
    if [ "$left" -gt 0 ]; then
      body="$body
and $left more"
    fi
    if ! post "$report_address" "$body"; then
      own_failure "the request to the credential $SECOND_ROUTE_REPORT was not delivered" \
        "the request to the credential $SECOND_ROUTE_REPORT is still not delivered"
    fi
  fi
  if [ -n "$keys" ]; then
    printf '%s\n' "$keys" >"$state/reported.new"
  else
    : >"$state/reported.new"
  fi
  mv "$state/reported.new" "$state/reported"
}

# The check-in, withheld after an unreadable read.
check_in() {
  if [ "$unreadable" -eq 1 ]; then
    return 0
  fi
  if ! post "$check_in_address"; then
    own_failure "the request to the credential $SECOND_ROUTE_CHECK_IN was not delivered" \
      "the request to the credential $SECOND_ROUTE_CHECK_IN is still not delivered"
  fi
}

main() {
  check_in_address="$(cat "$credentials/$SECOND_ROUTE_CHECK_IN" 2>/dev/null || true)"
  report_address="$(cat "$credentials/$SECOND_ROUTE_REPORT" 2>/dev/null || true)"
  judge "$SECOND_ROUTE_CHECK_IN" "$check_in_address"
  judge "$SECOND_ROUTE_REPORT" "$report_address"
  read_alert_path
  tell_owner
  check_in
  # Every request this run made was delivered, so the episode ends.
  : >"$state/episode"
}

main
