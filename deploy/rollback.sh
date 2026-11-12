#!/usr/bin/env bash
# rollback.sh: make an earlier release current again in one command (SPEC-062 R6; ADR-062).
#
#     deploy/rollback.sh TAG        the tag must be SemVer, annotated and on origin's main
#     deploy/rollback.sh caddy-remove   take the Caddy block out again
#
# A release the host still keeps is switched to and restarted with no fetch; one it no longer
# keeps is deployed anew through the same verification a deploy takes. The work is deploy.sh's, so
# a rollback can never skip a check a deploy makes. Configured by the same DECKSTREAK_DEPLOY_*
# variables.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
case "${1:-}" in
caddy-remove) exec "$here/deploy.sh" caddy-remove ;;
"") echo "usage: rollback.sh TAG | caddy-remove" >&2; exit 1 ;;
*) [ $# -eq 1 ] || { echo "usage: rollback.sh TAG" >&2; exit 1; }
   exec "$here/deploy.sh" --rollback "$1" ;;
esac
