#!/usr/bin/env bash
# The packs that cannot run in public CI, run on the maintainer's box against a DeckStreak checkout
# (ADR-004): the ten packs built into phxd, and the subscription-proxy client scan, whose probe
# names a private secret and so is not vendored. Their verdicts are posted on the pull request.
#
#   PHOENIX=/path/to/a/phoenix-v2/checkout PHXD=/path/to/phxd bash scripts/box-packs.sh [ROOT]
#
# PHXD must be built from the same phoenix-v2 commit `.packs/VENDORED.json` names, because a
# prebuilt phxd embeds an older catalog. The script writes one JSON card per pack under
# $BOX_PACKS_OUT and exits non-zero if any card is red.
set -uo pipefail

ROOT="$(cd "${1:-$(dirname "${BASH_SOURCE[0]}")/..}" && pwd)"
: "${PHOENIX:?set PHOENIX to a phoenix-v2 checkout at the vendored commit}"
: "${PHXD:?set PHXD to a phxd built from that checkout}"
OUT="${BOX_PACKS_OUT:-$(mktemp -d -t deckstreak-box-packs.XXXXXX)}"
mkdir -p "$OUT"
want=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["vendored_from"])' "$ROOT/.packs/VENDORED.json")
have=$(git -C "$PHOENIX" rev-parse HEAD)
if [ "$want" != "$have" ]; then
    echo "box-packs: phoenix-v2 is at $have, the vendored copy names $want; re-pin one of them"
    exit 2
fi

red=0
run() {
    # run <name> <command...>
    local name="$1"
    shift
    if "$@" >"$OUT/$name.json" 2>"$OUT/$name.err"; then
        echo "ok      $name"
    else
        echo "RED     $name (exit $?): $OUT/$name.json"
        red=1
    fi
}

for pack in greenfield web-launch vibecode-polish auth; do
    run "$pack" "$PHXD" pack probe --pack "$pack" --root "$ROOT" --format json
done
run ux-laws "$PHXD" pack probe --pack ux-laws --root "$ROOT" --format json
run ui-styles "$PHXD" pack probe --pack ui-styles --root "$ROOT" --format json
run web-security "$PHXD" pack probe --pack web-security --root "$ROOT" --format json
run cyber-pipeline "$PHXD" pack probe --pack cyber-pipeline --root "$ROOT" --format json
run ledger-sqlite "$PHXD" pack probe --pack ledger-sqlite --root "$ROOT" --format json
if [ -d "$ROOT/web/site/dist" ]; then
    run seo-pipeline "$PHXD" verify seo-pipeline --subject "$ROOT/web/site/dist" --format json
else
    echo "pending seo-pipeline: the landing page is not built yet (W7)"
fi
run subscription-proxy-client python3 "$PHOENIX/scripts/proxy-client-scan.py" --root "$ROOT" check all

echo "cards: $OUT"
exit "$red"
