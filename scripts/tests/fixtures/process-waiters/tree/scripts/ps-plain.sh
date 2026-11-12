#!/usr/bin/env bash
# Without the bracket, the command line of the grep matches its own pattern.
ps -eo pid,args | grep deck-api
