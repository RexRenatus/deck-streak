#!/usr/bin/env bash
# The sync server's dependency audit (SPEC-340 R7; ADR-351 D5): the RustSec advisory check over the
# graph the release builds the sync server from. It reads the fork and the commit from the engine's
# patch entry in Cargo.toml, as the release's build step does, refuses a commit that is not a full
# one before it fetches anything, fetches that commit into a scratch directory, and runs cargo-deny's
# advisory check on the server's own manifest with the house deny.toml, under the fork's lockfile.
# An advisory the check reports fails the audit, and the release job, which needs this job, never
# builds.
#
#   bash scripts/audit-sync-server.sh --work DIR [--manifest FILE] [--deny FILE]
#
# --work is the scratch directory the commit is checked out in; --manifest and --deny default to
# this repository's Cargo.toml and deny.toml.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/Cargo.toml"
deny="$root/deny.toml"
work=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --manifest) manifest="$2"; shift 2 ;;
        --deny) deny="$2"; shift 2 ;;
        --work) work="$2"; shift 2 ;;
        *) echo "audit-sync-server: unknown argument $1" >&2; exit 2 ;;
    esac
done
if [ -z "$work" ]; then
    echo "audit-sync-server: --work names the scratch directory" >&2
    exit 2
fi
if [ ! -f "$deny" ]; then
    echo "audit-sync-server: no house config at $deny" >&2
    exit 1
fi

pin="$(python3 -c 'import sys, tomllib; entry = tomllib.load(open(sys.argv[1], "rb"))["patch"]["https://github.com/ankitects/anki.git"]["anki"]; print(entry["git"], entry["rev"])' "$manifest")"
read -r fork rev <<< "$pin"
if ! grep -Eqx '[0-9a-f]{40}' <<< "$rev"; then
    echo "audit-sync-server: the engine's patch entry pins $rev, not a full commit" >&2
    exit 1
fi

mkdir -p "$work"
git -C "$work" init -q
git -C "$work" fetch -q --depth 1 "$fork" "$rev"
git -C "$work" checkout -q FETCH_HEAD
# cargo-deny takes --manifest-path, --locked and --config before its subcommand, and the check's
# name after it.
cargo deny --manifest-path "$work/rslib/sync/Cargo.toml" --locked --config "$deny" check advisories
