#!/usr/bin/env bash
# The red stub of SPEC-480 R11's baseline: it keeps every input and runs the base's test command
# unbounded, in the default environment, with no deal, so each criterion of the baseline fails by
# assertion until the script itself replaces this stub.
# usage: mutation-baseline.sh <listed.json> <shard> <shards> <log>
set -u
budget=3845
listed=$1
shard=$2
shards=$3
log=$4
cargo nextest run --verbose --profile mutants
exit 0
