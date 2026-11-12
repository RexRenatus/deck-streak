#!/usr/bin/env bash
# A waiter whose pattern is in the command line of the shell that runs it.
timeout 30 bash -c 'until pgrep -f deck-api; do sleep 1; done'
sh -c 'pgrep -fx deck-api'
