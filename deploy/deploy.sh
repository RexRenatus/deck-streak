#!/usr/bin/env bash
# deploy.sh: install one release tag on the host, switch to it, and prove it ready (SPEC-062;
# ADR-062, ADR-010, ADR-017). It runs on the maintainer's machine and reaches the host only through
# the host command it is given.
#
#     deploy/deploy.sh TAG              verify the tag and the release, then install and switch
#     deploy/deploy.sh --rollback TAG   make a kept release current again, or deploy it anew
#     deploy/deploy.sh caddy-install TAG   render the Caddy block, validate a copy, then reload
#     deploy/deploy.sh caddy-remove     take the block out again
#
# The order is fixed and every step before the host refuses with the host untouched: the tag is
# SemVer, annotated, and its commit is on origin's main; the tarball's build provenance verifies
# against this repository's release workflow; the tarball's digest matches SHA256SUMS. Only then
# does the tarball reach the host, unpacked beside the running release, and current is replaced by
# one rename. A release that does not become ready is switched back from.
#
# Configuration (environment; only the first two are needed on the maintainer's machine):
#   DECKSTREAK_DEPLOY_REPO           owner/name of the repository whose release is installed
#   DECKSTREAK_DEPLOY_HOST           a command that runs its argv on the host as given (the private rail's host command)
#   DECKSTREAK_DEPLOY_ELEVATE        the host's privilege command, default sudo (empty for none)
#   DECKSTREAK_DEPLOY_CHECKOUT       the checkout whose tags are read, default this script's tree
#   DECKSTREAK_DEPLOY_ROOT, _UNIT_DIR, _ENV_FILE, _CADDY_DIR, _CADDYFILE, _CADDY_CONFIG,
#   _READY_SECONDS, _READY_POLL, _KEEP   the host's paths and the readiness and keep settings
set -euo pipefail

# Every setting the script reads, named once (ADR-198). A Caddy step refuses any other deploy
# setting in its environment before it reads or writes anything, so a name it does not list can
# never choose where it writes.
SETTINGS='
DECKSTREAK_DEPLOY_REPO DECKSTREAK_DEPLOY_HOST DECKSTREAK_DEPLOY_ELEVATE DECKSTREAK_DEPLOY_CHECKOUT
DECKSTREAK_DEPLOY_ROOT DECKSTREAK_DEPLOY_UNIT_DIR DECKSTREAK_DEPLOY_ENV_FILE
DECKSTREAK_DEPLOY_CADDY_DIR DECKSTREAK_DEPLOY_CADDYFILE DECKSTREAK_DEPLOY_CADDY_CONFIG
DECKSTREAK_DEPLOY_READY_SECONDS DECKSTREAK_DEPLOY_READY_POLL DECKSTREAK_DEPLOY_KEEP
'
case "${1:-}" in
caddy-install | caddy-remove)
    for name in ${!DECKSTREAK_DEPLOY_@}; do
        named=
        for setting in $SETTINGS; do [ "$name" != "$setting" ] || named=1; done
        [ -n "$named" ] || { echo "deploy: $name is not a setting of deploy.sh" >&2; exit 1; }
    done
    ;;
esac

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=${DECKSTREAK_DEPLOY_ROOT:-/usr/local/lib/deck-streak}
UNIT_DIR=${DECKSTREAK_DEPLOY_UNIT_DIR:-/etc/systemd/system}
ENV_FILE=${DECKSTREAK_DEPLOY_ENV_FILE:-/etc/deck-streak/deck-streak.env}
CADDY_DIR=${DECKSTREAK_DEPLOY_CADDY_DIR:-/etc/caddy}
CADDYFILE=${DECKSTREAK_DEPLOY_CADDYFILE:-/etc/caddy/Caddyfile}
READY_SECONDS=${DECKSTREAK_DEPLOY_READY_SECONDS:-180}
READY_POLL=${DECKSTREAK_DEPLOY_READY_POLL:-2}
KEEP=${DECKSTREAK_DEPLOY_KEEP:-3}
IMPORT_LINE='import deck-streak.caddy'

die() {
    echo "deploy: $*" >&2
    exit 1
}

need_host() {
    [ -n "${DECKSTREAK_DEPLOY_HOST:-}" ] || die "DECKSTREAK_DEPLOY_HOST is not set"
    read -r -a HOST_CMD <<<"$DECKSTREAK_DEPLOY_HOST"
    read -r -a ELEVATE_CMD <<<"${DECKSTREAK_DEPLOY_ELEVATE-sudo}"
}

# Run a script on the host, elevated; the caller's stdin is the script's stdin.
on_host() {
    local script=$1
    shift
    "${HOST_CMD[@]}" ${ELEVATE_CMD[@]+"${ELEVATE_CMD[@]}"} bash -c "$script" deck-streak-host "$@"
}

# The tag is SemVer, annotated, and on origin's main. Nothing has been fetched from the release.
# It leaves `checkout` set: the checkout whose tag the Caddy block is later read from.
checkout=
verify_tag() {
    local tag=$1 kind
    [[ $tag =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] ||
        die "$tag is not a SemVer tag (vMAJOR.MINOR.PATCH)"
    checkout=${DECKSTREAK_DEPLOY_CHECKOUT:-$(git -C "$here" rev-parse --show-toplevel)}
    git -C "$checkout" fetch --quiet origin \
        "+refs/heads/main:refs/remotes/origin/main" "+refs/tags/$tag:refs/tags/$tag" ||
        die "$tag could not be fetched from origin with main"
    kind=$(git -C "$checkout" cat-file -t "refs/tags/$tag" 2>/dev/null) || die "$tag does not exist"
    [ "$kind" = tag ] || die "$tag is not an annotated tag"
    git -C "$checkout" merge-base --is-ancestor "refs/tags/$tag^{commit}" refs/remotes/origin/main ||
        die "$tag is not on origin's main"
}

# The host side. Arguments: MODE TAG ROOT UNIT_DIR ENV_FILE SECONDS POLL KEEP. MODE install reads
# the tarball on stdin; MODE switch uses the release the host already keeps.
read -r -d '' HOST_SCRIPT <<'HOSTEOF' || true
set -eu
mode=$1 tag=$2 root=$3 unitdir=$4 envfile=$5 secs=$6 poll=$7 keep=$8
rel=$root/releases/$tag
prev=
[ -L "$root/current" ] && prev=$(readlink "$root/current")
[ "$prev" != "$rel" ] || { echo "deploy: $tag is already current" >&2; exit 1; }
port=$(sed -n 's/^DECKSTREAK_API_LISTEN=.*:\([0-9][0-9]*\)$/\1/p' "$envfile" 2>/dev/null | tail -n 1)
port=${port:-8080}

install_units() {
    local src=$1/deploy/systemd f d n c base
    for f in "$src"/*.service "$src"/*.timer "$src"/*.socket "$src"/*.path; do
        [ -f "$f" ] && install -m 0644 "$f" "$unitdir/$(basename "$f")"
    done
    for d in "$src"/*@*.d; do
        [ -d "$d" ] || continue
        n=$(basename "$d")
        install -d -m 0755 "$unitdir/$n"
        for c in "$d"/*.conf; do
            [ -f "$c" ] && install -m 0644 "$c" "$unitdir/$n/$(basename "$c")"
        done
    done
    for d in "$unitdir"/deck-streak-*@*.d; do
        [ -d "$d" ] || continue
        n=$(basename "$d")
        for c in "$d"/*.conf; do
            [ -f "$c" ] || continue
            base=$(basename "$c")
            [ "$base" = 10-rail.conf ] && continue
            [ -f "$src/$n/$base" ] || find "$c" -delete
        done
    done
}

switch_to() {
    ln -sfn "$1" "$root/.current.$$"
    mv -T "$root/.current.$$" "$root/current"
}

restart() {
    systemctl restart "$1" || { echo "deploy: $1 did not restart" >&2; return 1; }
}

ready() {
    local end=$((SECONDS + secs))
    while :; do
        curl -fsS --max-time 2 "http://127.0.0.1:$port/api/readyz" >/dev/null 2>&1 && return 0
        [ "$SECONDS" -ge "$end" ] && return 1
        sleep "$poll"
    done
}

back() {
    echo "deploy: $1 did not become ready; switching back" >&2
    if [ -n "$prev" ]; then
        switch_to "$prev"
        install_units "$prev"
        systemctl daemon-reload
        restart deck-streak-api.service || true
        restart deck-streak-bot.service || true
    else
        rm -f "$root/current"
    fi
    [ "$mode" = install ] && find "$rel" -delete
    exit 1
}

if [ "$mode" = install ]; then
    [ ! -e "$rel" ] || { echo "deploy: $tag is already installed; use the rollback" >&2; exit 1; }
    mkdir -p "$root/releases"
    [ -e "$rel.partial" ] && find "$rel.partial" -delete
    mkdir "$rel.partial"
    tar -xzf - --no-same-owner -C "$rel.partial"
    (cd "$rel.partial" && sha256sum -c --quiet MANIFEST.sha256) ||
        { echo "deploy: the unpacked release does not match its MANIFEST.sha256" >&2; find "$rel.partial" -delete; exit 1; }
    mv -T "$rel.partial" "$rel"
else
    [ -d "$rel" ] || { echo "deploy: $tag is not kept on the host" >&2; exit 1; }
fi

install_units "$rel"
systemctl daemon-reload
checked=$(mktemp)
for f in "$rel"/deploy/systemd/*.service "$rel"/deploy/systemd/*@*.d; do
    [ -e "$f" ] || continue
    u=$(basename "$f")
    systemctl cat "${u%.d}" >>"$checked" ||
        { echo "deploy: ${u%.d} could not be shown" >&2; : >"$checked"; break; }
done
python3 "$rel/deploy/scripts/effective-check.py" --root "$rel" "$checked" ||
    { echo "deploy: the effective configuration is refused" >&2; find "$checked" -delete
      [ -n "$prev" ] && install_units "$prev"; systemctl daemon-reload
      [ "$mode" = install ] && find "$rel" -delete; exit 1; }
find "$checked" -delete

switch_to "$rel"
restart deck-streak-api.service || back deck-streak-api.service
ready || back deck-streak-api.service
restart deck-streak-bot.service || back deck-streak-bot.service

if [ "$mode" = install ]; then
    prevname=
    [ -n "$prev" ] && prevname=$(basename "$prev")
    spare=$((keep - 1))
    [ -n "$prevname" ] && spare=$((keep - 2))
    doomed=$(find "$root/releases" -mindepth 1 -maxdepth 1 -printf '%f\n' | grep -v '^\.' | grep -v '\.partial$' |
        grep -vxF -e "$tag" -e "${prevname:-$tag}" | sort -V -r | tail -n +"$((spare + 1))")
    for old in $doomed; do
        find "$root/releases/$old" -delete
    done
fi
echo "deploy: $tag is current"
HOSTEOF

# The release for a tag, fetched and verified in a temporary directory; prints its tarball path.
fetch_release() {
    local tag=$1 dir=$2 repo=${DECKSTREAK_DEPLOY_REPO:?DECKSTREAK_DEPLOY_REPO is not set}
    local name="deck-streak-$tag.tar.gz" listed
    gh release download "$tag" --repo "$repo" --dir "$dir" || die "the release $tag could not be downloaded"
    gh attestation verify "$dir/$name" --repo "$repo" \
        --signer-workflow "$repo/.github/workflows/release.yml" >/dev/null ||
        die "the build attestation of $name does not verify"
    [ -f "$dir/SHA256SUMS" ] || die "the release carries no SHA256SUMS"
    listed=$(awk -v n="$name" '$2 == n' "$dir/SHA256SUMS")
    [ -n "$listed" ] || die "SHA256SUMS does not name $name"
    (cd "$dir" && printf '%s\n' "$listed" | sha256sum -c --quiet -) ||
        die "$name does not match SHA256SUMS"
}

release_tmp=
cleanup() {
    [ -n "$release_tmp" ] && [ -d "$release_tmp" ] && find "$release_tmp" -delete
    return 0
}
trap cleanup EXIT

run_host() {
    local mode=$1 tag=$2
    shift 2
    on_host "$HOST_SCRIPT" "$mode" "$tag" "$ROOT" "$UNIT_DIR" "$ENV_FILE" "$READY_SECONDS" \
        "$READY_POLL" "$KEEP" "$@"
}

install_tag() {
    local tag=$1
    release_tmp=$(mktemp -d)
    fetch_release "$tag" "$release_tmp"
    run_host install "$tag" <"$release_tmp/deck-streak-$tag.tar.gz"
}

rollback_tag() {
    local tag=$1
    verify_tag "$tag"
    need_host
    # shellcheck disable=SC2016  # a literal script for the host
    if on_host 'test -d "$1/releases/$2"' "$ROOT" "$tag" </dev/null; then
        run_host switch "$tag" </dev/null
    else
        install_tag "$tag"
    fi
}

caddy_install() {
    local tag=$1 config=${DECKSTREAK_DEPLOY_CADDY_CONFIG:-} block template
    verify_tag "$tag"
    [ -n "$config" ] && [ -f "$config" ] || die "the private Caddy configuration is missing"
    need_host
    release_tmp=$(mktemp -d)
    template=$release_tmp/deck-streak.caddy
    git -C "$checkout" show "refs/tags/$tag:deploy/caddy/deck-streak.caddy" >"$template" ||
        die "$tag holds no Caddy block"
    block=$(python3 "$here/scripts/render-caddy.py" --config "$config" --template "$template") ||
        die "the Caddy block was not rendered"
    # shellcheck disable=SC2016  # a literal script for the host
    printf '%s\n' "$block" | on_host '
set -eu
dir=$1 file=$2 line=$3
block=$dir/deck-streak.caddy
copy=$dir/deck-streak.candidate
kept=$file.previous
had=
put_back_block() {
    if [ -n "$had" ]; then mv -T "$had" "$block"; else [ ! -f "$block" ] || find "$block" -delete; fi
}
undo() {
    [ ! -f "$copy" ] || find "$copy" -delete
    put_back_block
    echo "deploy: the Caddy configuration was refused" >&2
    exit 1
}
[ -w "$dir" ] && [ -f "$file" ] || { echo "deploy: the Caddy configuration was refused" >&2; exit 1; }
[ -w "$(dirname -- "$file")" ] || { echo "deploy: the Caddy configuration was refused" >&2; exit 1; }
for path in "$block" "$block.previous" "$copy" "$kept"; do
    { [ ! -e "$path" ] && [ ! -L "$path" ]; } ||
        [ -z "$(find "$path" -maxdepth 0 \( ! -type f -o -links +1 \) -print)" ] ||
        { echo "deploy: the Caddy configuration was refused" >&2; exit 1; }
done
[ -f "$block" ] && { had=$block.previous; cp -p "$block" "$had" || { echo "deploy: the Caddy configuration was refused" >&2; exit 1; }; }
cat >"$block" || undo
cp -p "$file" "$copy" || undo
grep -qxF "$line" "$copy" || printf "%s\n" "$line" >>"$copy" || undo
caddy validate --adapter caddyfile --config "$copy" || undo
caddy adapt --adapter caddyfile --config "$copy" --validate >/dev/null || undo
cp -p "$file" "$kept" || undo
mv -T "$copy" "$file" || { find "$kept" -delete; undo; }
if ! caddy reload --config "$file"; then
    mv -T "$kept" "$file"
    put_back_block
    echo "deploy: the Caddy reload failed; the previous site file and Caddyfile were restored" >&2
    caddy reload --config "$file" || echo "deploy: the restoring reload also failed" >&2
    exit 1
fi
find "$kept" -delete
[ -n "$had" ] && find "$had" -delete
exit 0
' "$CADDY_DIR" "$CADDYFILE" "$IMPORT_LINE"
}

caddy_remove() {
    need_host
    # shellcheck disable=SC2016  # a literal script for the host
    on_host '
set -eu
dir=$1 file=$2 line=$3
block=$dir/deck-streak.caddy
copy=$dir/deck-streak.candidate
kept=$file.previous
had=
unwritten() {
    [ ! -f "$copy" ] || find "$copy" -delete
    echo "deploy: the candidate Caddyfile could not be written" >&2
    exit 1
}
[ -w "$dir" ] && [ -f "$file" ] || { echo "deploy: the candidate Caddyfile could not be written" >&2; exit 1; }
[ -w "$(dirname -- "$file")" ] || { echo "deploy: the candidate Caddyfile could not be written" >&2; exit 1; }
[ ! -L "$copy" ] || unwritten
[ ! -e "$copy" ] || [ -z "$(find "$copy" -maxdepth 0 \( -type p -o -type s -o -type b -o -type c -o -type f -links +1 \) -print)" ] ||
    { echo "deploy: the candidate Caddyfile could not be written" >&2; exit 1; }
for name in "$block" "$block.previous" "$kept"; do
    if [ -e "$name" ] || [ -L "$name" ]; then
        [ -z "$(find "$name" -maxdepth 0 \( -links +1 -o ! -type f \) -print)" ] ||
            { echo "deploy: the candidate Caddyfile could not be written" >&2; exit 1; }
    fi
done
: >"$copy" || unwritten
grep -vxF "$line" "$file" >"$copy" || [ "$?" -eq 1 ] || unwritten
caddy validate --adapter caddyfile --config "$copy" || { [ ! -f "$copy" ] || find "$copy" -delete; echo "deploy: refused" >&2; exit 1; }
caddy adapt --adapter caddyfile --config "$copy" --validate >/dev/null || { [ ! -f "$copy" ] || find "$copy" -delete; echo "deploy: refused" >&2; exit 1; }
cp -p "$file" "$kept"
mv -T "$copy" "$file"
[ -f "$block" ] && { had=$block.previous; mv -T "$block" "$had"; }
if ! caddy reload --config "$file"; then
    [ -n "$had" ] && mv -T "$had" "$block"
    mv -T "$kept" "$file"
    echo "deploy: the Caddy reload failed; the previous site file and Caddyfile were restored" >&2
    caddy reload --config "$file" || echo "deploy: the restoring reload also failed" >&2
    exit 1
fi
find "$kept" -delete
[ -n "$had" ] && find "$had" -delete
exit 0
' "$CADDY_DIR" "$CADDYFILE" "$IMPORT_LINE" </dev/null
}

case "${1:-}" in
caddy-install) [ $# -eq 2 ] || die "usage: deploy.sh caddy-install TAG"; caddy_install "$2" ;;
caddy-remove) caddy_remove ;;
--rollback) [ $# -eq 2 ] || die "usage: deploy.sh --rollback TAG"; rollback_tag "$2" ;;
"") die "usage: deploy.sh TAG | --rollback TAG | caddy-install TAG | caddy-remove" ;;
*)
    [ $# -eq 1 ] || die "usage: deploy.sh TAG"
    verify_tag "$1"
    need_host
    install_tag "$1"
    ;;
esac
