#!/usr/bin/env bash
# A process name is not a command line; a pgrep -f deck-api in a comment is never read.
pgrep -x -u "$USER" deck-api
pgrep deck-api
pkill -TERM -x deck-api
pidof deck-api
ps -o pid= -C deck-api
