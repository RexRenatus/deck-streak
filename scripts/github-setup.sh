#!/usr/bin/env bash
# Idempotent GitHub settings for deck-streak (ADR-017). Each step reports ok or its refusal and the
# script carries on, so it can run while the repository is private (recording what the plan
# refuses) and again after the maintainer makes it public. It needs `gh` authenticated as an admin.
#
#   bash scripts/github-setup.sh [labels|settings|security|rulesets|all]
set -uo pipefail

REPO="${REPO:-RexRenatus/deck-streak}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WHAT="${1:-all}"
status=0

step() {
    # step <description> <command...>
    local what="$1"
    shift
    local out
    if out=$("$@" 2>&1); then
        echo "ok      $what"
    else
        echo "REFUSED $what: $(echo "$out" | tr '\n' ' ' | cut -c1-200)"
        status=1
    fi
}

ensure_label() {
    # ensure_label <name> <colour> <description>
    gh label create "$1" --repo "$REPO" --color "$2" --description "$3" --force >/dev/null
}

ensure_milestone() {
    # ensure_milestone <title> <description>
    local exists
    exists=$(gh api "repos/$REPO/milestones?state=all&per_page=100" --jq ".[] | select(.title==\"$1\") | .number")
    if [ -z "$exists" ]; then
        gh api -X POST "repos/$REPO/milestones" -f title="$1" -f description="$2" >/dev/null
    fi
}

labels() {
    for wave in 0 1 2 3 4 5 6 7 8 9; do
        step "label wave:$wave" ensure_label "wave:$wave" "0E8A16" "Delivered in wave W$wave"
    done
    python3 - "$ROOT/docs/CONTEXT-MAP.md" <<'PY' | while read -r context; do step "label context:$context" ensure_label "context:$context" "1D76DB" "The $context bounded context"; done
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
fence = re.search(r"```context-map\n(.*?)```", text, re.S).group(1)
for line in fence.splitlines():
    if line.strip():
        print(line.split()[0].removeprefix("deck-streak-"))
for extra in ("deploy", "repo"):
    print(extra)
PY
    step "label type:epic" ensure_label "type:epic" "5319E7" "A wave's epic"
    step "label type:feature" ensure_label "type:feature" "FBCA04" "A parity or product feature"
    step "label type:ops" ensure_label "type:ops" "C5DEF5" "Operations and deployment work"
    step "label type:owner" ensure_label "type:owner" "D93F0B" "Needs the owner's decision or action"
    while IFS='|' read -r title description; do
        step "milestone $title" ensure_milestone "$title" "$description"
    done <"$ROOT/docs/github/milestones.txt"
}

settings() {
    step "default branch main, discussions on" gh api -X PATCH "repos/$REPO" \
        -f default_branch=main -F has_discussions=true -F delete_branch_on_merge=true \
        -F allow_merge_commit=true -F allow_squash_merge=true -F allow_rebase_merge=true
}

security() {
    step "secret scanning and push protection" gh api -X PATCH "repos/$REPO" --input - <<'JSON'
{"security_and_analysis": {"secret_scanning": {"status": "enabled"}, "secret_scanning_push_protection": {"status": "enabled"}}}
JSON
    step "Dependabot alerts" gh api -X PUT "repos/$REPO/vulnerability-alerts"
    # Security updates open against the default branch (main), which only a release pull request from
    # dev may change, so they stay OFF; alerts stay on and are triaged into pull requests into dev
    # (ADR-036). Deleting is idempotent, so a re-run never turns them back on.
    step "Dependabot security updates stay off" gh api -X DELETE "repos/$REPO/automated-security-fixes"
    step "private vulnerability reporting" gh api -X PUT "repos/$REPO/private-vulnerability-reporting"
}

apply_ruleset() {
    local file="$1" name id
    name=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["name"])' "$file")
    id=$(gh api "repos/$REPO/rulesets" --jq ".[] | select(.name==\"$name\") | .id")
    if [ -n "$id" ]; then
        gh api -X PUT "repos/$REPO/rulesets/$id" --input "$file" >/dev/null
    else
        gh api -X POST "repos/$REPO/rulesets" --input "$file" >/dev/null
    fi
}

rulesets() {
    for file in "$ROOT"/.github/rulesets/*.json; do
        step "ruleset $(basename "$file")" apply_ruleset "$file"
    done
}

case "$WHAT" in
labels) labels ;;
settings) settings ;;
security) security ;;
rulesets) rulesets ;;
all) labels; settings; security; rulesets ;;
*) echo "usage: $0 [labels|settings|security|rulesets|all]"; exit 2 ;;
esac
exit "$status"
