#!/usr/bin/env bash
# The Rust legs' Timeouts by package, printed beside the verdict so that none counts silently as a
# kill (SPEC-480 R16). For each package the legs' reports name, it prints how many of its mutants
# read Timeout and its baseline tests' seconds: the sum of the times nextest gives that package's
# tests in a leg's green baseline log, the largest over the legs whose baseline tested it. A package
# with a Timeout whose baseline tests, times the margin, pass the per-mutant timeout, or that no log
# times, is marked READ BY NAME, and each of its Timeouts is printed by leg and name.
# usage: mutation-timeouts.sh <the verdict job's reports directory>
set -u -o pipefail
reports=$1
timeout=2300
margin=1.5
legs=0
logs=0
stream=""
for dir in "$reports"/mutation-rust-shard-*; do
    [ -d "$dir" ] || continue
    leg=${dir##*/}
    if [ -f "$dir/mutants.out/outcomes.json" ]; then
        found=$(jq -r --arg leg "$leg" '.outcomes[] | select(.scenario | type == "object") | ["O", .scenario.Mutant.package, .summary, $leg, .scenario.Mutant.name] | @tsv' "$dir/mutants.out/outcomes.json") || exit 2
        stream+="$found"$'\n'
        legs=$((legs + 1))
    fi
    code=""
    if [ -f "$dir/baseline/baseline.exit" ]; then read -r code < "$dir/baseline/baseline.exit"; fi
    if [ "$code" = 0 ] && [ -f "$dir/baseline/baseline.log" ]; then
        found=$(awk '
            match($0, /^[[:space:]]*[A-Z]+[[:space:]]+\[[[:space:]]*[0-9.]+s\][[:space:]]+/) {
                head = substr($0, 1, RLENGTH)
                rest = substr($0, RLENGTH + 1)
                sub(/^\([[:space:]]*[0-9]+\/[0-9]+\)[[:space:]]+/, "", rest)
                split(rest, word, /[[:space:]]+/)
                package = word[1]
                sub(/::.*/, "", package)
                sub(/^[[:space:]]*[A-Z]+[[:space:]]+\[[[:space:]]*/, "", head)
                sub(/s\].*/, "", head)
                sum[package] += head
            }
            END { for (package in sum) printf "T\t%s\t%.3f\n", package, sum[package] }
        ' "$dir/baseline/baseline.log")
        stream="$found"$'\n'"$stream"
        logs=$((logs + 1))
    fi
done
echo "Timeouts by package, over $legs leg report(s) and $logs green baseline log(s): READ BY NAME marks a package with a Timeout whose baseline tests, times $margin, pass the $timeout s per-mutant timeout, or that no log times"
printf '%s' "$stream" | awk -F '\t' -v timeout="$timeout" -v margin="$margin" '
    $1 == "T" && (!($2 in most) || $3 + 0 > most[$2]) { most[$2] = $3 + 0 }
    $1 == "O" {
        mutants[$2]++
        if ($3 == "Timeout") { timeouts[$2]++; named[$2] = named[$2] $2 ": READ BY NAME " $4 ": " $5 "\n" }
    }
    END {
        for (package in mutants) {
            seconds = (package in most) ? sprintf("%.1f s", most[package]) : "untimed"
            line = package ": " (timeouts[package] + 0) " Timeout(s) of " mutants[package] " mutant(s), its baseline tests " seconds
            if (timeouts[package] > 0 && (!(package in most) || most[package] * margin > timeout)) {
                print line ": READ BY NAME"
                printf "%s", named[package]
            } else {
                print line
            }
        }
    }' | LC_ALL=C sort
