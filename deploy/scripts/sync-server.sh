#!/bin/bash
# DeckStreak's sync server launcher (SPEC-337 R2; ADR-347 D2, D3): start the engine's own sync
# server, which the release ships as bin/anki-sync-server, with its two users read from the unit's
# credentials.
#
#   sync-server.sh
#
# deck-streak-sync-server.service runs it. The server reads its users only from its environment, as
# SYNC_USER1, SYNC_USER2 and so on, each `name:password`, or `name:<PHC hash>` when PASSWORDS_HASHED
# is set. So this script reads the owner's user and the staging user (ADR-344) from
# $CREDENTIALS_DIRECTORY, each `name:<pbkdf2-sha256 hash>` in the PHC form, refuses an entry of any
# other shape, and execs the server with the two users, PASSWORDS_HASHED, the unit's state directory
# as its data directory, and the loopback address DECKSTREAK_SYNC_SERVER_LISTEN names. The users never
# reach a unit file, a command line or a settings file, and a password never reaches the host: the
# server receives hashes only. A malformed hash would fail only at the first login, inside the
# server, so the shape is checked here and a bad entry fails the unit at start, which pages through
# its OnFailure=. A refusal names the credential's id and never prints what it read; it exits 1
# before the server runs.
set -euo pipefail

# The ids of the two credentials the unit loads (the deploy-template tests read them here).
readonly SYNC_SERVER_OWNER=sync-server-owner
readonly SYNC_SERVER_STAGING=sync-server-staging

# A user name, which the server makes a directory of under its data directory, so it cannot start
# with a dot or hold a slash; then the PHC form of a pbkdf2-sha256 hash in the house's shape
# (SPEC-340 R10; ADR-351 D8): 600000 to 999999 rounds, which bounds the time one check takes from
# both sides, an optional length of 32, a 16-byte salt and a 32-byte digest, each in canonical
# unpadded standard base64, whose last character carries no spare bit.
readonly ENTRY='^[A-Za-z0-9_-][A-Za-z0-9._-]*:\$pbkdf2-sha256\$i=[6-9][0-9]{5}(,l=32)?\$[A-Za-z0-9+/]{21}[AQgw]\$[A-Za-z0-9+/]{42}[AEIMQUYcgkosw048]$'
# A loopback IPv4 address and a port.
readonly LOOPBACK='^127\.([0-9]{1,3})\.([0-9]{1,3})\.([0-9]{1,3}):([0-9]{1,5})$'

refuse() {
  printf 'sync-server: %s\n' "$1" >&2
  exit 1
}

# The entry the credential `$1` holds, its trailing newlines removed, checked for its shape.
entry() {
  local path="$CREDENTIALS_DIRECTORY/$1" value
  [ -f "$path" ] || refuse "the credential $1 is missing"
  value="$(cat -- "$path")"
  [ -n "$value" ] || refuse "the credential $1 is empty"
  [[ "$value" =~ $ENTRY ]] ||
    refuse "the credential $1 is not a user name and a pbkdf2-sha256 hash of the house's shape"
  printf '%s' "$value"
}

[ -n "${CREDENTIALS_DIRECTORY:-}" ] || refuse "no credentials directory"
[ -n "${STATE_DIRECTORY:-}" ] || refuse "no state directory"
owner="$(entry "$SYNC_SERVER_OWNER")"
staging="$(entry "$SYNC_SERVER_STAGING")"
# Two users of one name would share one directory, and the server would keep only the last.
[ "${owner%%:*}" != "${staging%%:*}" ] || refuse "both users have the same name"

listen="${DECKSTREAK_SYNC_SERVER_LISTEN:-}"
unreachable="DECKSTREAK_SYNC_SERVER_LISTEN is not a loopback address with a port"
[[ "$listen" =~ $LOOPBACK ]] || refuse "$unreachable"
octets=("${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}" "${BASH_REMATCH[3]}")
port=$((10#${BASH_REMATCH[4]}))
for octet in "${octets[@]}"; do
  ((10#$octet <= 255)) || refuse "$unreachable"
done
((port >= 1 && port <= 65535)) || refuse "$unreachable"
host="127.$((10#${octets[0]})).$((10#${octets[1]})).$((10#${octets[2]}))"

# The server reads nothing this script did not set: every SYNC_ variable, PASSWORDS_HASHED and the
# payload limit are cleared first, so it reads two users, this directory and this address, and its
# payload limit stays its own default, the bound the Caddy block holds the request body to.
for name in $(compgen -e); do
  case "$name" in
    SYNC_* | PASSWORDS_HASHED | MAX_SYNC_PAYLOAD_MEGS) unset "$name" ;;
  esac
done

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export SYNC_HOST="$host" SYNC_PORT="$port" SYNC_BASE="$STATE_DIRECTORY"
export SYNC_USER1="$owner" SYNC_USER2="$staging" PASSWORDS_HASHED=1
exec "$here/../../bin/anki-sync-server"
