#!/usr/bin/env bash
# The web engine's stage (SPEC-350 R13, A22; ADR-361): puts the module and its bindings that
# scripts/web-engine-build.sh wrote beside the Mini App's build, under `engine/`, where the app's
# Worker loads them from `/engine/`. CI's web-engine job runs it from the repository's root after
# the app's build, and the study suite serves the result.
#
# It checks both files before it creates anything, the bindings first, and refuses, naming the
# file, when either is missing: a stage that copied one file would leave a build whose Worker
# cannot start.
#
# Usage: scripts/web-engine-stage.sh [--from DIR] [--to DIR]   (each relative to the cwd)
set -euo pipefail

from=target/web-engine
to=web/app/build/engine

while [ "$#" -gt 0 ]; do
    case "$1" in
    --from)
        from=$2
        shift 2
        ;;
    --to)
        to=$2
        shift 2
        ;;
    *)
        echo "web-engine-stage: unknown argument $1" >&2
        exit 2
        ;;
    esac
done

files=(deck_streak_web_engine.js deck_streak_web_engine_bg.wasm)

for name in "${files[@]}"; do
    if [ ! -f "$from/$name" ]; then
        echo "web-engine-stage: $from/$name is missing: run scripts/web-engine-build.sh first" >&2
        exit 1
    fi
done

mkdir -p "$to"
cp "$from/deck_streak_web_engine.js" "$to/deck_streak_web_engine.js"
cp "$from/deck_streak_web_engine_bg.wasm" "$to/deck_streak_web_engine_bg.wasm"
echo "web-engine-stage: staged ${#files[@]} files in $to"
