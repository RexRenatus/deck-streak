#!/usr/bin/env bash
# The local gate: the same script builders run and CI runs (CLAUDE.md, ADR-017, ADR-055).
#
#   bash scripts/check.sh            every stage
#   bash scripts/check.sh fmt clippy only the named stages
#
# Every stage runs even when an earlier one failed, so one run names every red. Each stage writes
# its full output to a log under $CHECK_LOG_DIR, prints one summary line, and adds a row to
# timings.tsv there. With no $CHECK_LOG_DIR it makes a fresh temporary directory and names it on
# stderr; stdout, which a reader quotes, never names it (SPEC-056 R4). Each stage first checks the
# tools it runs:
# a missing tool FAILS that stage by name, with its install hint, because a gate that silently
# skipped a stage would report green having examined nothing. CI runs these stages in five parallel
# jobs, each stage in exactly one of them (ADR-055).
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2
if [ -n "${CHECK_LOG_DIR:-}" ]; then
    LOG_DIR="$CHECK_LOG_DIR"
    NAMED_LOG_DIR=1
else
    LOG_DIR="$(mktemp -d -t deckstreak-check.XXXXXX)"
    NAMED_LOG_DIR=0
fi
mkdir -p "$LOG_DIR"
TIMINGS="$LOG_DIR/timings.tsv"

# In CI's order: the rust job, the engine job, the release job, the web job, the hygiene job. The packs are judged
# on the maintainer's box by scripts/box-packs.sh, never here (ADR-069).
STAGES_ALL=(fmt clippy test doctest audit-rust test-engine test-release web audit-web python scrub secrets)
if [ "$#" -gt 0 ]; then STAGES=("$@"); else STAGES=("${STAGES_ALL[@]}"); fi

# The engine set (SPEC-038 R13), defined here and nowhere else: SPEC-022's two test binaries that
# drive Anki's engine end to end, whose tests take between 20 and 83 s each on a CI runner while
# every other test of the workspace takes under 2 s. `test` runs every test outside the set and
# `test-engine` runs the set, with one command that differs only in this filterset, so each test
# runs in exactly one of the two, and a test added to either binary goes with it.
ENGINE_TESTS='binary_id(=deck-streak-ingest::sync) | binary_id(=deck-streak-ingest::engine_budget)'

# The web audit's level (SPEC-058 R2), defined here and nowhere else: pnpm's lowest, so every
# advisory pnpm grades fails the stage, the bar audit-rust holds, where deny.toml makes every
# RustSec advisory an error. It is given on the command line, so no workspace setting raises it.
AUDIT_WEB_LEVEL=low

failed=()

need() {
    # need <tool> <install hint>
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "missing tool: $1 ($2)"
        return 1
    fi
}

need_cargo() { need cargo "rustup, then rustup show in this repository"; }

need_nextest() { need cargo-nextest "https://nexte.st, or taiki-e/install-action in CI"; }

need_node() {
    need node "Node 24, see .nvmrc" || return 1
    node -e 'process.exit(Number(process.versions.node.split(".")[0]) >= 24 ? 0 : 1)' ||
        { echo "node is older than 24 (.nvmrc)"; return 1; }
    need pnpm "pnpm 11, pinned by packageManager in package.json; never corepack"
}

need_protoc() {
    # Anki's engine compiles its protobuf definitions with prost-build, which runs protoc from
    # PROTOC or PATH (ADR-022).
    if [ -n "${PROTOC:-}" ]; then
        [ -x "$PROTOC" ] || { echo "PROTOC names no executable protoc"; return 1; }
    else
        need protoc "protoc 31.1 from github.com/protocolbuffers/protobuf releases, or set PROTOC"
    fi
}

need_python() {
    need python3 "Python 3.11 or later" || return 1
    python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' ||
        { echo "python3 is older than 3.11"; return 1; }
}

stage_fmt() { need_cargo && cargo fmt --all --check; }

stage_clippy() {
    need_cargo && need_protoc &&
        cargo clippy --workspace --all-targets --locked -- -D warnings
}

stage_test() {
    need_cargo && need_nextest && need_protoc &&
        cargo nextest run --workspace --locked --no-fail-fast -E "not ($ENGINE_TESTS)"
}

stage_test_engine() {
    # The engine set alone, with test's own command: CI runs it in a job beside rust (R13, R14),
    # one slice per runner when ENGINE_SLICE names one, m/n (R16). Unset, it runs the whole set.
    local slice=()
    if [ -n "${ENGINE_SLICE:-}" ]; then
        if [[ ! "$ENGINE_SLICE" =~ ^([1-9][0-9]*)/([1-9][0-9]*)$ ]] ||
            [ "${BASH_REMATCH[1]}" -gt "${BASH_REMATCH[2]}" ]; then
            echo "ENGINE_SLICE names no slice m/n of n: '$ENGINE_SLICE'"
            return 1
        fi
        slice=(--partition "slice:$ENGINE_SLICE")
    fi
    # One --test <target> per whole binary the set names, read from ENGINE_TESTS and written
    # nowhere else (R13 as amended), so cargo builds those test targets and no other. --workspace
    # stays: the package selection is what fixes the feature resolution the cache was built under.
    local targets=() rest="$ENGINE_TESTS"
    local named='binary_id\(=[a-z0-9-]+::([a-z0-9_]+)\)'
    while [[ "$rest" =~ $named ]]; do
        targets+=(--test "${BASH_REMATCH[1]}")
        rest="${rest#*"${BASH_REMATCH[0]}"}"
    done
    need_cargo && need_nextest && need_protoc &&
        cargo nextest run --workspace --locked --no-fail-fast -E "$ENGINE_TESTS" \
            ${targets[@]+"${targets[@]}"} ${slice[@]+"${slice[@]}"}
}

stage_test_release() {
    # The workspace's tests in the profile the daemon ships from (release.yml builds with
    # --release; the workspace sets no [profile.release]), so a rule that holds only with debug
    # assertions on fails here (SPEC-330, ADR-330). Every test `test` runs; the engine set stays
    # with test-engine, and engine_budget has its own release run in engine-measure.yml.
    need_cargo && need_nextest && need_protoc &&
        cargo nextest run --workspace --locked --no-fail-fast --release -E "not ($ENGINE_TESTS)"
}

stage_doctest() { need_cargo && need_protoc && cargo test --doc --workspace --locked; }

stage_audit_rust() {
    need_cargo &&
        need cargo-deny "https://github.com/EmbarkStudios/cargo-deny releases, or taiki-e/install-action" &&
        need git "https://git-scm.com/downloads" &&
        cargo deny --locked check advisories bans licenses sources &&
        lock_is_canonical
}

lock_is_canonical() {
    # --locked does not prove a lock canonical. A lock cargo may not write is compared by the
    # resolve it encodes, not by its text, so a text merge's leftover (a version qualifier, #246)
    # passes every --locked stage, while an unlocked resolve, as cargo-mutants runs one, rewrites
    # it. The resolve below writes cargo's own form, and a lock it rewrote fails by name (SPEC-057
    # R18). It judges the committed lock, so an uncommitted edit of it is named first.
    git diff --quiet -- Cargo.lock || {
        echo "audit-rust: Cargo.lock differs from its committed form; commit it, then run the stage"
        return 1
    }
    cargo metadata --format-version 1 >/dev/null || return 1
    git diff --exit-code -- Cargo.lock || {
        echo "audit-rust: an unlocked resolve rewrote Cargo.lock, so it is not in cargo's canonical form; commit the rewritten lock"
        return 1
    }
}

stage_web() {
    need_node &&
        pnpm install --frozen-lockfile &&
        pnpm -r check &&
        pnpm -r test &&
        pnpm -r build &&
        pnpm -r test:e2e
}

stage_audit_web() {
    # Every package the lockfile resolves, development ones included: the Mini App ships as a
    # static build, so its packages are all development dependencies, and no flag narrows the
    # classes pnpm audits. The verdict reads pnpm's report, prints how many packages it examined,
    # and reads a report that examined none as VOID (SPEC-058).
    need_node && need_python || return 1
    local report code
    report=$(pnpm audit --json --audit-level "$AUDIT_WEB_LEVEL")
    code=$?
    python3 scripts/audit-web-verdict.py --level "$AUDIT_WEB_LEVEL" --pnpm-exit "$code" \
        <<<"$report"
}

stage_python() {
    # cargo too: guard tests build and audit the workspace (SPEC-055's engine pin).
    need_python && need_cargo || return 1
    # Both suites run whatever the first found, and the last line gives each one's tests and exit,
    # so a red suite never hides the other's result; a suite that ran no test fails (SPEC-054 R7).
    local suite output code ran failed=0 verdicts=""
    for suite in scripts/tests tools/parity-oracle agent/tests; do
        output=$(python3 -m unittest discover -s "$suite" -p 'test_*.py' 2>&1)
        code=$?
        printf '%s\n' "$output"
        ran=$(printf '%s\n' "$output" | grep -Eo '^Ran [0-9]+' | tail -n 1)
        ran=${ran#Ran }
        if [ "$code" -ne 0 ] || [ "${ran:-0}" -eq 0 ]; then failed=1; fi
        verdicts+="${verdicts:+; }$suite ran ${ran:-0} test(s), exit $code"
    done
    echo "python: $verdicts"
    return "$failed"
}

stage_scrub() {
    need_python || return 1
    # The tree and every blob reachable from HEAD: a value that only history or a binary holds
    # is still published (SPEC-033).
    python3 scripts/public-scrub.py --root . --history
}

stage_secrets() {
    # The working tree here; CI also scans the whole history (fetch-depth: 0).
    need gitleaks "https://github.com/gitleaks/gitleaks releases" &&
        gitleaks dir --no-banner --redact --config .gitleaks.toml . &&
        if [ "${CHECK_HISTORY:-0}" = "1" ]; then gitleaks git --no-banner --redact --config .gitleaks.toml .; fi
}

printf 'stage\tseconds\tverdict\texit\n' >"$TIMINGS"
for stage in "${STAGES[@]}"; do
    fn="stage_${stage//-/_}"
    if [[ ! "$stage" =~ ^[a-z][a-z0-9-]*$ ]] || ! declare -F "$fn" >/dev/null; then
        echo "check.sh: unknown stage '$stage' (known: ${STAGES_ALL[*]})"
        exit 2
    fi
    log="$LOG_DIR/$stage.log"
    started=$(date +%s)
    "$fn" >"$log" 2>&1
    code=$?
    seconds=$(($(date +%s) - started))
    if [ "$code" -eq 0 ]; then
        printf 'ok     %-10s %4ss\n' "$stage" "$seconds"
        printf '%s\t%s\tok\t0\n' "$stage" "$seconds" >>"$TIMINGS"
    else
        failed+=("$stage")
        last=$(grep -v '^[[:space:]]*$' "$log" | tail -n 1)
        printf 'FAILED %-10s %4ss exit %s: %s\n' "$stage" "$seconds" "$code" "${last:0:160}"
        printf '%s\t%s\tFAILED\t%s\n' "$stage" "$seconds" "$code" >>"$TIMINGS"
    fi
done

# The directory is named only when the caller did not name it, and on stderr (SPEC-056 R4).
if [ "$NAMED_LOG_DIR" = 0 ]; then echo "logs: $LOG_DIR" >&2; fi
if [ "${#failed[@]}" -gt 0 ]; then
    echo "CHECK FAILED: ${failed[*]}"
    exit 1
fi
echo "CHECK OK: ${#STAGES[@]} stage(s)"
