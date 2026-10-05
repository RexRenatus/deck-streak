#!/usr/bin/env bash
# The web engine's stage (SPEC-350 R13, ADR-361). Red stub: the module alone, unchecked.
set -euo pipefail

mkdir -p web/app/build/engine
cp target/web-engine/deck_streak_web_engine_bg.wasm web/app/build/engine/
