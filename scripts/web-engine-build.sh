#!/usr/bin/env bash
# The web engine's build (SPEC-338 R8, ADR-348, ADR-349): the module a browser loads and its JS
# bindings, written to one directory, which scripts/web-engine-size.py then measures.
#
#   bash scripts/web-engine-build.sh --cc clang-18 --ar llvm-ar-18 [--out target/web-engine]
#
# 1. `cargo build --release --target wasm32-unknown-unknown -p deck-streak-web-engine`. SQLite's C
#    source compiles for wasm32 with the C compiler and archiver named here, and only here: they
#    are set on this one command line, never in a file the native workspace reads (ADR-348).
#    getrandom selects its browser backend by the feature the fork's patch turns on, so no flag
#    is set at all.
# 2. `wasm-bindgen --target web`, at the crate's exact version, which CI installs.
# 3. `wasm-opt -Oz` with the post-MVP features the module uses, so the size pass keeps them.
#
# wasm-bindgen and wasm-opt are read from PATH. The two files the size gate reads are
# deck_streak_web_engine_bg.wasm and deck_streak_web_engine.js, in --out.
set -euo pipefail

cc=clang
ar=llvm-ar
out=target/web-engine
while [ "$#" -gt 0 ]; do
    case "$1" in
        --cc) cc=$2; shift 2 ;;
        --ar) ar=$2; shift 2 ;;
        --out) out=$2; shift 2 ;;
        *) echo "web-engine-build: unknown argument $1" >&2; exit 2 ;;
    esac
done

target_dir=${CARGO_TARGET_DIR:-target}
CC_wasm32_unknown_unknown=$cc AR_wasm32_unknown_unknown=$ar \
    cargo build --release --locked --target wasm32-unknown-unknown -p deck-streak-web-engine
raw="$target_dir/wasm32-unknown-unknown/release/deck_streak_web_engine.wasm"

mkdir -p "$out"
wasm-bindgen --target web --out-dir "$out" "$raw"
module="$out/deck_streak_web_engine_bg.wasm"
wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    --enable-mutable-globals --enable-reference-types --enable-multivalue \
    "$module" -o "$module"
echo "web-engine-build: wrote $module and $out/deck_streak_web_engine.js"
