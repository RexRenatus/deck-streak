#!/usr/bin/env bash
# The local gate: the same script builders run and CI runs (CLAUDE.md, ADR-017, ADR-055).
#
#   bash scripts/check.sh            every stage
#   bash scripts/check.sh fmt clippy only the named stages
#
# Every stage runs even when an earlier one failed, so one run names every red. Each stage writes
# its full output to a log under $CHECK_LOG_DIR (a fresh temporary directory by default), prints
# one summary line, and adds a row to timings.tsv there. Each stage first checks the tools it runs:
# a missing tool FAILS that stage by name, with its install hint, because a gate that silently
# skipped a stage would report green having examined nothing. CI runs these stages in four parallel
# jobs, each stage in exactly one of them (ADR-055).
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2
LOG_DIR="${CHECK_LOG_DIR:-$(mktemp -d -t deckstreak-check.XXXXXX)}"
mkdir -p "$LOG_DIR"
TIMINGS="$LOG_DIR/timings.tsv"

# In CI's order: the rust job, the web job, the packs job, the hygiene job.
STAGES_ALL=(fmt clippy test doctest audit-rust web audit-web packs python scrub secrets)
if [ "$#" -gt 0 ]; then STAGES=("$@"); else STAGES=("${STAGES_ALL[@]}"); fi

failed=()

need() {
    # need <tool> <install hint>
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "missing tool: $1 ($2)"
        return 1
    fi
}

need_cargo() { need cargo "rustup, then rustup show in this repository"; }

need_node() {
    need node "Node 24, see .nvmrc" || return 1
    node -e 'process.exit(Number(process.versions.node.split(".")[0]) >= 24 ? 0 : 1)' ||
        { echo "node is older than 24 (.nvmrc)"; return 1; }
    need pnpm "pnpm 11, pinned by packageManager in package.json; never corepack"
}

need_python() {
    need python3 "Python 3.11 or later" || return 1
    python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' ||
        { echo "python3 is older than 3.11"; return 1; }
}

stage_fmt() { need_cargo && cargo fmt --all --check; }

stage_clippy() { need_cargo && cargo clippy --workspace --all-targets --locked -- -D warnings; }

stage_test() {
    need_cargo && need cargo-nextest "https://nexte.st, or taiki-e/install-action in CI" &&
        cargo nextest run --workspace --locked --no-fail-fast
}

stage_doctest() { need_cargo && cargo test --doc --workspace --locked; }

stage_audit_rust() {
    need_cargo &&
        need cargo-deny "https://github.com/EmbarkStudios/cargo-deny releases, or taiki-e/install-action" &&
        cargo deny --locked check advisories bans licenses sources
}

stage_web() {
    need_node &&
        pnpm install --frozen-lockfile &&
        pnpm -r check &&
        pnpm -r test &&
        pnpm -r build &&
        pnpm -r test:e2e
}

stage_audit_web() { need_node && pnpm audit --prod; }

stage_packs() { need_python && python3 scripts/pack-rows.py; }

stage_python() {
    # cargo too: a guard test builds a Rust example (SPEC-042's rails rows).
    need_python && need_cargo &&
        python3 -m unittest discover -s scripts/tests -p 'test_*.py' &&
        python3 -m unittest discover -s tools/parity-oracle -p 'test_*.py'
}

stage_scrub() {
    need_python || return 1
    # The tree and every blob reachable from HEAD: a value that only history or a binary holds
    # is still published (SPEC-033).
    python3 scripts/public-scrub.py --root . --history || return 1
    # No apiKeyHelper in any Claude Code settings file (the subscription-proxy pack's rule). The
    # scan is VOID with no settings file, so until the agent's settings template lands (SPEC-043)
    # it reports pending by name rather than passing over nothing.
    if git ls-files | grep -Eq '(^|/)\.claude/settings[^/]*\.json$|^agent/([^/]+/)*settings[^/]*\.json$|(^|/)managed-settings\.json$'; then
        python3 scripts/no-apikeyhelper-scan.py --root .
    else
        echo "no-apikeyhelper: pending until the agent's settings template lands (SPEC-043)"
    fi
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

echo "logs: $LOG_DIR"
if [ "${#failed[@]}" -gt 0 ]; then
    echo "CHECK FAILED: ${failed[*]}"
    exit 1
fi
echo "CHECK OK: ${#STAGES[@]} stage(s)"
