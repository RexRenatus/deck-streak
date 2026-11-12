#!/usr/bin/env bash
# The bracket form cannot match the grep that holds it.
ps -eo pid,args | grep -v grep | grep -e '[d]eck-api'
