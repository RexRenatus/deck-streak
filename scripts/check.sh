#!/usr/bin/env bash
# The local gate: the same script builders run and CI runs (CLAUDE.md, ADR-017).
#
#   bash scripts/check.sh            every stage
#   bash scripts/check.sh fmt clippy only the named stages
#
# Every stage runs even when an earlier one failed, so one run names every red. Each stage writes
# its full output to a log under $CHECK_LOG_DIR (a fresh temporary directory by default) and
# prints one summary line. A stage whose tool is missing FAILS with the install hint: a gate that
# silently skipped a stage would report green having examined nothing.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2
LOG_DIR="${CHECK_LOG_DIR:-$(mktemp -d -t deckstreak-check.XXXXXX)}"
mkdir -p "$LOG_DIR"

STAGES_ALL=(toolchain fmt clippy test doctest web python packs scrub audit secrets)
if [ "$#" -gt 0 ]; then STAGES=("$@"); else STAGES=("${STAGES_ALL[@]}"); fi

failed=()

need() {
    # need <tool> <install hint>
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "missing tool: $1 ($2)"
        return 1
    fi
}

stage_toolchain() {
    local ok=0
    need cargo "rustup, then rustup show in this repository" || ok=1
    need cargo-nextest "https://nexte.st, or taiki-e/install-action in CI" || ok=1
    need node "Node 24, see .nvmrc" || ok=1
    need pnpm "pnpm 11, pinned by packageManager in package.json; never corepack" || ok=1
    need python3 "Python 3.11 or later" || ok=1
    need gitleaks "https://github.com/gitleaks/gitleaks releases" || ok=1
    need cargo-deny "https://github.com/EmbarkStudios/cargo-deny releases, or taiki-e/install-action" || ok=1
    python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' ||
        { echo "python3 is older than 3.11"; ok=1; }
    node -e 'process.exit(Number(process.versions.node.split(".")[0]) >= 24 ? 0 : 1)' ||
        { echo "node is older than 24 (.nvmrc)"; ok=1; }
    return "$ok"
}

stage_fmt() { cargo fmt --all --check; }

stage_clippy() { cargo clippy --workspace --all-targets --locked -- -D warnings; }

stage_test() { cargo nextest run --workspace --locked --no-fail-fast; }

stage_doctest() { cargo test --doc --workspace --locked; }

stage_web() {
    pnpm install --frozen-lockfile &&
        pnpm -r check &&
        pnpm -r test &&
        pnpm -r build &&
        pnpm -r test:e2e
}

stage_python() {
    python3 -m unittest discover -s scripts/tests -p 'test_*.py' &&
        python3 -m unittest discover -s tools/parity-oracle -p 'test_*.py'
}

stage_packs() { python3 scripts/pack-rows.py; }

stage_scrub() {
    python3 scripts/public-scrub.py --root . || return 1
    # No apiKeyHelper in any Claude Code settings file (the subscription-proxy pack's rule). The
    # scan is VOID with no settings file, so until the agent's settings template lands (SPEC-043)
    # it reports pending by name rather than passing over nothing.
    if git ls-files | grep -Eq '(^|/)\.claude/settings[^/]*\.json$|^agent/([^/]+/)*settings[^/]*\.json$|(^|/)managed-settings\.json$'; then
        python3 scripts/no-apikeyhelper-scan.py --root .
    else
        echo "no-apikeyhelper: pending until the agent's settings template lands (SPEC-043)"
    fi
}

stage_audit() {
    cargo deny --locked check advisories bans licenses sources &&
        pnpm audit --prod
}

stage_secrets() {
    # The working tree here; CI also scans the whole history (fetch-depth: 0).
    gitleaks dir --no-banner --redact --config .gitleaks.toml . &&
        if [ "${CHECK_HISTORY:-0}" = "1" ]; then gitleaks git --no-banner --redact --config .gitleaks.toml .; fi
}

for stage in "${STAGES[@]}"; do
    fn="stage_${stage}"
    if ! declare -F "$fn" >/dev/null; then
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
    else
        failed+=("$stage")
        last=$(grep -v '^[[:space:]]*$' "$log" | tail -n 1)
        printf 'FAILED %-10s %4ss exit %s: %s\n' "$stage" "$seconds" "$code" "${last:0:160}"
    fi
done

echo "logs: $LOG_DIR"
if [ "${#failed[@]}" -gt 0 ]; then
    echo "CHECK FAILED: ${failed[*]}"
    exit 1
fi
echo "CHECK OK: ${#STAGES[@]} stage(s)"
