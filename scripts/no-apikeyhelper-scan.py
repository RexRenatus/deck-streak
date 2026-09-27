#!/usr/bin/env python3
"""Tree-scope probe: no committed Claude Code settings file carries the credential shape that
killed three subscription-proxy accounts in 5.6s on 2026-09-18 (SPEC-V2-1678 SS1; packs/subscription-proxy
`no-apikeyhelper`, delivery d2160).

WHY THIS EXISTS. `tools/scripts/claude-via-proxy.sh` already refuses to LAUNCH a session whose
settings carry an `apiKeyHelper` or an `ANTHROPIC_*` credential entry, because the CLI sends an
`apiKeyHelper` value as both `x-api-key` and `Authorization: Bearer`; the proxy's own
`CLIENT_CREDENTIALS` strip (`crates/phx-subscription-proxy/src/serve.rs`) protects a request that
actually reaches it, but neither one helps a settings file that nobody has launched through yet.
This script is the static, tree-scope half: it proves no COMMITTED settings file in a checkout
carries that shape, independently of any launch.

THE PREDICATE IS A PORT, NOT A GUESS. It reproduces `claude-via-proxy.sh`'s own
`credential_shape()` jq expression (that script's lines 52-53) byte-for-byte:

    jq -e '(.apiKeyHelper // empty) as $h | ($h != null and $h != "")
           or ((.env // {}) | has("ANTHROPIC_API_KEY") or has("ANTHROPIC_AUTH_TOKEN") or has("ANTHROPIC_BASE_URL"))'

Two consequences that a naive "is this field truthy" rewrite would get wrong:
  - `apiKeyHelper: ""` is excluded (jq's `$h != ""`), so an empty string is clean, not a finding.
  - `apiKeyHelper: 0` (a JSON number) is a finding, because jq's `//` operator treats only `false`
    and `null` as absent, and 0 is neither -- unlike a plain Python `if value:` check, which would
    treat 0 as falsy and miss it.
The env check matches exactly three key names (`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`,
`ANTHROPIC_BASE_URL`); it is not a `ANTHROPIC_` prefix wildcard, because the wrapper's own check
is not one either.

WHAT IT SCANS. Every `.claude/settings.json` and `.claude/settings.local.json` anywhere under
`--root`. The wrapper itself checks only one pair per launch (`$PWD`'s settings.json and
settings.local.json, plus the user-global `$HOME/.claude/settings.json`, which is outside any
repository tree and outside this probe's scope). A repository can be launched from more than one
directory inside it -- a worktree, a nested project -- so this tree probe generalizes across every
directory rather than modeling one single launch site.

Unlike the wrapper's shell check, which silently no-ops when `jq` is absent
(`command -v jq >/dev/null || return 1`), this probe's `json.loads` parse always runs, and a file
that fails to parse is a VOID finding of its own (exit 2), never a silent skip.

Exit 0: clean, and the scanned file count is printed. Exit 1: at least one finding, each named by
path and the exact key. Exit 2: VOID -- a scanned file is not valid JSON or not a JSON object, the
root is not a directory, or no settings file was found at all.

d2196 (SPEC-V2-2196 R6) fixed four false verdicts here, each proven by `FoundingScanDefects`:
- A missing root, and a tree with zero settings files, read GREEN. A scan that examined nothing
  cannot say that nothing carries the shape, so both are VOID now.
- A `managed-settings.json`, or a settings TEMPLATE declaring the published `$schema`, was never
  scanned. A consumer ships its agent's settings exactly that way (the pack's reference client
  does), so both join the population.
- A settings file whose JSON is not an object read GREEN. Claude Code refuses such a file, so it
  is VOID here, like invalid JSON.
The predicate itself -- the byte-for-byte port of `credential_shape()` -- did not change.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import sys

# Exactly the three keys `claude-via-proxy.sh` checks (its line 53) -- not an ANTHROPIC_ prefix.
FORBIDDEN_ENV_KEYS = ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_BASE_URL")

# A settings template declares itself with the published schema (code.claude.com/docs/en/settings).
SCHEMA_MARK = re.compile(r'"\$schema"\s*:\s*"[^"]*claude-code-settings\.json"')
PRUNED_DIRS = frozenset(
    {".git", "target", "node_modules", ".venv", "venv", "__pycache__"}
)


def settings_files(root: pathlib.Path) -> list[pathlib.Path]:
    """Every settings file Claude Code loads at project scope, anywhere under `root`, sorted for
    a stable report. `.claude/settings.json` and `.claude/settings.local.json`, at any depth."""
    found: list[pathlib.Path] = []
    for name in ("settings.json", "settings.local.json"):
        found.extend(root.rglob(f".claude/{name}"))
    found.extend(declared_settings(root))
    return sorted(set(found))


def declared_settings(root: pathlib.Path) -> list[pathlib.Path]:
    """Settings documents outside `.claude/`: every `managed-settings.json`, and every JSON file
    whose first 4 KiB declare the published `claude-code-settings.json` schema."""
    declared: list[pathlib.Path] = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames[:] = [d for d in dirnames if d not in PRUNED_DIRS]
        for name in filenames:
            if not name.endswith(".json"):
                continue
            path = pathlib.Path(dirpath) / name
            if name == "managed-settings.json":
                declared.append(path)
                continue
            try:
                with path.open("rb") as handle:
                    head = handle.read(4096).decode("utf-8", errors="replace")
            except OSError:
                continue
            if SCHEMA_MARK.search(head):
                declared.append(path)
    return declared


def findings(path: pathlib.Path) -> list[str]:
    """The forbidden shapes in one parsed settings document, ported from
    `credential_shape()`'s jq expression. Returns one message per finding, empty when clean."""
    text = path.read_text(encoding="utf-8")
    try:
        doc = json.loads(text)
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: not valid JSON: {error}") from error
    if not isinstance(doc, dict):
        raise ValueError(
            f"{path}: not a JSON object, which Claude Code refuses as settings"
        )

    hits: list[str] = []

    # jq: (.apiKeyHelper // empty) as $h | ($h != null and $h != "")
    value = doc.get("apiKeyHelper") if isinstance(doc, dict) else None
    if value is not None and value is not False and value != "":
        hits.append(f"{path}: apiKeyHelper is set")

    env = doc.get("env") if isinstance(doc, dict) else None
    if isinstance(env, dict):
        for key in FORBIDDEN_ENV_KEYS:
            if key in env:
                hits.append(f"{path}: env.{key} is set")

    return hits


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=pathlib.Path,
        default=pathlib.Path(__file__).resolve().parents[1],
        help="work tree root to scan (default: the repository this script lives in)",
    )
    args = parser.parse_args(argv)
    root = args.root.resolve()
    if not root.is_dir():
        print(
            f"NO-APIKEYHELPER VOID root {args.root} is not a directory", file=sys.stderr
        )
        return 2

    files = settings_files(root)
    if not files:
        print(
            f"NO-APIKEYHELPER VOID 0 settings file(s) under {root}: a scan that examined "
            "nothing cannot say nothing carries the shape",
            file=sys.stderr,
        )
        return 2
    all_hits: list[str] = []
    for path in files:
        try:
            all_hits.extend(findings(path))
        except ValueError as error:
            print(f"NO-APIKEYHELPER VOID {error}", file=sys.stderr)
            return 2

    if all_hits:
        print(
            f"NO-APIKEYHELPER RED {len(all_hits)} finding(s) in {len(files)} file(s):"
        )
        for hit in all_hits:
            print(f"  {hit}")
        return 1

    print(
        f"NO-APIKEYHELPER GREEN {len(files)} settings file(s) scanned, none carries the shape"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
