#!/usr/bin/env bash
# Patterns that are not in the command line of the script that runs them.
until pgrep -f deck-api; do sleep 1; done
printf '%s\n' "$(pgrep -n -f deck-api)"
pkill --signal TERM --full -- deck-worker
pgrep -f "$SERVICE"
