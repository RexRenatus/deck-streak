#!/usr/bin/env bash
# One Rust leg's unmutated baseline, bounded by its budget (SPEC-480 R11): the packages of this
# shard's mutants, dealt round-robin over the plan's listing as cargo-mutants deals them, built and
# tested by one nextest run of the mutants profile in cargo-mutants' own baseline environment. It
# exits 124 when the baseline reached its budget, else the run's own exit, and 2 when the shard is
# dealt no package, so the verdict reads every red or unbounded baseline VOID with its reason.
# usage: mutation-baseline.sh <listed.json> <shard> <shards> <log>
set -u
budget=3845
listed=$1
shard=$2
shards=$3
log=$4
packages=$(jq -r --argjson k "$shard" --argjson n "$shards" '[to_entries[] | select(.key % $n == $k) | .value.package] | unique | .[] | "--package=" + .' "$listed") || exit 2
if [ -z "$packages" ]; then
    echo "mutation-baseline: shard $shard of $shards is dealt no package by $listed"
    exit 2
fi
start=$SECONDS
rc=0
# One word per package: the deal's names hold no space.
# shellcheck disable=SC2086
INSTA_UPDATE=no INSTA_FORCE_PASS=0 timeout --kill-after=60 "$budget" cargo nextest run --verbose $packages --profile mutants > "$log" 2>&1 || rc=$?
elapsed=$((SECONDS - start))
# timeout exits 124, or 137 when its kill follows: either, at the budget, is the budget's exit.
if [ "$rc" -ne 0 ] && [ "$elapsed" -ge "$budget" ]; then rc=124; fi
echo "mutation-baseline: shard $shard of $shards: exit $rc after $elapsed s of its $budget s budget"
exit "$rc"
