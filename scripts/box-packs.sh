#!/usr/bin/env bash
# The box run: every pack DeckStreak consumes, judged on the maintainer's box against a DeckStreak
# commit, with the maintainer's private checkout of the packs and the runner built from it
# (ADR-069, ADR-030; SPEC-030 R10 to R14, SPEC-054 R4, SPEC-056 R7 to R11).
#
#   PACKS_WIRING=<private file> PACKS_CHECKOUT=<checkout> PACKS_RUNNER=<runner> \
#       bash scripts/box-packs.sh [--rev REV] [--post-status] [ROOT]
#
# * PACKS_WIRING names a private file, schema `deckstreak.box-wiring.v1`, that this repository never
#   holds: the pin (the checkout's commit), the skills directory, the scripts of the sdd, ddd and
#   tdd probes and of the proxy-client and apiKeyHelper scans, the packs and their states, the box
#   section's expectations, each owned file's source, and the variables the runner must not
#   inherit. PACKS_CHECKOUT is the checkout at the pin, and PACKS_RUNNER the runner built from it.
#   A variable that is unset or unusable, a file of another schema or a pin the checkout is not at
#   makes the run VOID by name.
# * It judges the COMMITTED tree at REV (default HEAD) of ROOT (default this repository), exported
#   with `git archive` into a scratch directory outside the repository and the checkout.
# * The packs section: each pack runs through the runner with its catalog's probe verb and
#   `--scope tree`, and each row is judged by its exit: an `enforced` pack fails on a blocking row
#   that is RED, VOID or in ERROR; a `pending` pack reads a blocking VOID row as pending, and is
#   STALE once every blocking row passes; a `deferred` pack runs no row; an excluded row is counted
#   and never judged; a deferred row that passes is STALE; an advisory row never fails. A row the
#   file names that the pack lacks makes the run VOID.
# * The box section: each pack runs with the verb its catalog admits, read from the runner's
#   `pack list` and never written here: a probe card is `pack probe`, a run card is `pack run`
#   against a scratch ledger made with `init` and `project register`, and a seo-pipeline card is
#   `verify seo-pipeline` over `web/site/dist`, once the judged tree holds it (#59). A card is read
#   by the suffix of its schema. A red row the file does not name under `expected_red` fails the run
#   by name, a named row that is no longer red is refused as stale, and an advisory row never fails.
#   A pack that examines nothing reads `pending` with the issue the file names, is VOID without one,
#   and is stale once it examines a row.
# * A packs-section pack whose entry names an `advisory_waivers` lint (durable-services: the lint in
#   the checkout, because the runner's card cuts each row's report short) runs it over the judged
#   tree, `lint --format json`, and reads every advisory finding by unit and reason. Each must be
#   waived in its unit (`X-DurableServices-Waive=<reason> <why>`, a why of more than five words) or
#   wait on an issue the entry names. An unwaived departure or a thin why fails the pack by name; a
#   waiver or waiting entry that matches no finding, or waits on a closed issue, is stale. Other
#   packs' advisory rows never fail (SPEC-056 R15).
# * The sdd, ddd and tdd probes run from the checkout against the judged tree (`check all`): any
#   class that is not OK fails its probe by name. The proxy-client scan is read by its rows, never
#   its exit: any RED fails, and it reads `pending` while it examines no settings document.
# * The apiKeyHelper scan (the subscription-proxy pack's `no-apikeyhelper` rule) is read by its one
#   verdict line and its exit, with the refusals the public gate gave it: a finding fails, and so
#   does a settings file it cannot read; while it finds no settings file it reads `pending` with
#   the issue the box section names, and is VOID without one. A tree holding a file where the
#   removed gate step looked for settings (`.claude/settings*.json`, `agent/**/settings*.json`,
#   `managed-settings.json`) is never pending: a scan that finds none there is VOID.
# * Each owned file, a copy DeckStreak keeps of a pack's data (ADR-069), is compared with its source
#   in the checkout over the fields it keeps: a kept field that differs fails by name, and so does a
#   field the source gained that the copy neither keeps nor drops. A missing source makes the run
#   VOID.
# * Before any pack runs, it reads the state of every issue the box section names (each
#   `expected_red` row's issue, each `pending`, each scan's `pending`, and each advisory waiting
#   entry's issue) once, with
#   `gh issue view <n> --json state` run in ROOT, so gh resolves the repository from ROOT's remotes
#   or from $GH_REPO. An expectation whose issue is CLOSED is stale, and fails its pack by name.
#   When gh is not on the path, is not logged in, cannot reach GitHub or answers no state, the run
#   is VOID with that reason and runs no pack: it never passes on an issue it could not read.
# * With --post-status it posts one commit status on the judged commit, through `gh api` in ROOT:
#   context `box/packs`; state `success`, `failure` or `error` (it could not judge); and a
#   description of a few words that names no row. It is not a required check (ADR-069).
#
# It prints one line per pack, probe, scan and owned file, and a summary, and exits 0 when nothing
# failed, 1 when something did, and 2 when it cannot judge. The scratch directory (the exported tree
# and the ledger) is removed when the run ends; each card is kept under $BOX_PACKS_OUT (a fresh
# temporary directory by default), which it names on stderr.
set -euo pipefail
BOX_PACKS_SELF="${BASH_SOURCE[0]}" exec python3 - "$@" <<'PY'
"""The box driver: this file's opening comment is its documentation (ADR-069, ADR-030)."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

SELF = Path(os.environ["BOX_PACKS_SELF"]).resolve()
SCHEMA = "deckstreak.box-wiring.v1"
# The runner's verbs, by the suffix of the card schema a catalog row declares. Which pack takes
# which verb is read from the runner's `pack list`, never written here.
VERBS = {".pack.probe.v1": "probe", ".pack.run.v1": "run", ".seo-pipeline.v1": "verify"}
SITE = "web/site/dist"
SCAN = "proxy-client-scan"
HELPER = "no-apikeyhelper"
PROBES = ("sdd", "ddd", "tdd")
WIRING_KEYS = {"schema", "pin", "skills", "scripts", "packs", "box", "owned", "unset_env", "note"}
BOX_KEYS = {"packs", SCAN, HELPER, "note"}
PACK_KEYS = {"expected_red", "pending", "note"}
SCAN_KEYS = {"pending", "note"}
ROW_PACK_KEYS = {
    "state", "enforced_by", "excluded_rows", "deferred_rows", "advisory_waivers", "note",
}
WAIVER_KEYS = {"lint", "waiting", "note"}
# How a unit waives an advisory departure (the durable lint's key, which systemd ignores), and the
# unit files and drop-ins it may sit in (SPEC-056 R15).
WAIVE_KEY = "X-DurableServices-Waive"
UNIT_SUFFIXES = (".service", ".timer", ".slice")
STATES = ("enforced", "pending", "deferred")
OWNED_KEYS = {"source", "dropped"}
PIN = re.compile(r"^[0-9a-f]{40}$")
ISSUE = re.compile(r"^#\d+$")
REGISTERED = re.compile(r"registered\s+\D*(\d+)")
SCAN_ROW = re.compile(r"^PROXY-CLIENT (GREEN|RED|ADVISORY|VOID) (\S+): (\d+) (.+?) examined\b")
SCAN_ALL = re.compile(r"^PROXY-CLIENT ALL \w+: blocking (\d+) green, (\d+) red, (\d+) void\b")
SETTINGS = "settings document(s)"
PROBE_LINE = re.compile(r"^([A-Z]+) ([a-z0-9-]+) (OK|REFUSED|VOID)\b")
# The apiKeyHelper scan's verdict line, and the two shapes its rest takes.
HELPER_LINE = re.compile(r"^NO-APIKEYHELPER (GREEN|RED|VOID) (.*)$")
HELPER_FILES = re.compile(r"^(\d+) settings file\(s\)")
HELPER_FINDINGS = re.compile(r"^(\d+) finding\(s\) in (\d+) file\(s\)")
# The paths the removed gate step read as settings files: a tree holding one is never pending.
SETTINGS_PATHS = re.compile(
    r"(^|/)\.claude/settings[^/]*\.json$|^agent/([^/]+/)*settings[^/]*\.json$"
    r"|(^|/)managed-settings\.json$"
)
# The states `gh issue view --json state` answers, and its exit when it is not logged in.
ISSUE_STATES = ("OPEN", "CLOSED")
GH_NOT_LOGGED_IN = 4
CONTEXT = "box/packs"


class Refusal(Exception):
    """The run cannot judge; the message names why (VOID, exit 2)."""


@dataclass
class Card:
    """One pack's card, read by its schema: how many rows it examined, and each row's state."""

    examined: int
    rows: dict[str, str]


@dataclass
class Verdict:
    """One line: `ok`, `pending`, `deferred` or `FAIL`, with what decided it."""

    pack: str
    verb: str
    mark: str = "ok"
    examined: int = 0
    unexpected: list[str] = field(default_factory=list)
    expected: list[str] = field(default_factory=list)
    stale: list[str] = field(default_factory=list)
    detail: str = ""
    # A box pack's and the scan's line counts its unexpected, expected and stale rows.
    counted: bool = True

    def line(self) -> str:
        head = f"{self.mark:8} {self.pack:20} {self.verb:6} examined {self.examined}"
        parts = []
        if self.counted and (self.examined or self.unexpected or self.stale):
            parts.append(
                f"unexpected {len(self.unexpected)}, expected {len(self.expected)}, "
                f"stale {len(self.stale)}"
            )
        if self.unexpected:
            parts.append(f"unexpected red: {', '.join(self.unexpected)}")
        if self.stale:
            parts.append(f"stale: {', '.join(self.stale)}")
        if self.detail:
            parts.append(self.detail)
        return f"{head}: {'; '.join(parts)}" if parts else head


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="box-packs.sh",
        description="judge every pack, probe and owned file against a DeckStreak commit",
    )
    parser.add_argument("--rev", default="HEAD", help="the commit to judge (default HEAD)")
    parser.add_argument(
        "--post-status", action="store_true", help=f"post one {CONTEXT} status on the commit"
    )
    parser.add_argument("root", nargs="?", default=str(SELF.parents[1]), help="the repository")
    args = parser.parse_args(argv)
    # A signal exits through the scratch directory's cleanup, never around it.
    for number in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, lambda signum, _frame: sys.exit(128 + signum))
    root = Path(args.root).resolve()
    try:
        sha = git(root, "rev-parse", "--verify", "--quiet", f"{args.rev}^{{commit}}")
    except Refusal as refusal:
        print(f"box-packs: VOID: {refusal}", flush=True)
        return 2
    try:
        verdicts = run(args, root, sha)
    except Refusal as refusal:
        print(f"box-packs: VOID: {refusal}", flush=True)
        if args.post_status:
            post_status(root, sha, "error", "the box run could not judge this commit")
        return 2
    failed = [verdict.pack for verdict in verdicts if verdict.mark == "FAIL"]
    if failed:
        print(f"BOX PACKS FAILED: {len(failed)} of {len(verdicts)}: {', '.join(failed)}")
        if args.post_status:
            post_status(root, sha, "failure", f"{len(failed)} of {len(verdicts)} checks failed")
        return 1
    pending = sum(verdict.mark in ("pending", "deferred") for verdict in verdicts)
    print(f"BOX PACKS OK: {len(verdicts)} verdict(s), {pending} pending")
    if args.post_status:
        post_status(root, sha, "success", f"{len(verdicts)} checks judged, none failed")
    return 0


def run(args: argparse.Namespace, root: Path, sha: str) -> list[Verdict]:
    wiring_file = required("PACKS_WIRING", "the private wiring file", "file")
    checkout = required("PACKS_CHECKOUT", "the packs checkout at the file's pin", "directory")
    runner = required("PACKS_RUNNER", "the runner built from that checkout", "executable")
    wiring = read_wiring(wiring_file)
    have = git(checkout, "rev-parse", "HEAD")
    if have != wiring["pin"]:
        raise Refusal(
            f"the checkout is at {have[:12]}, and the private file's pin is "
            f"{str(wiring['pin'])[:12]}; re-pin one of them"
        )
    for path, entry in sorted(wiring["owned"].items()):
        if not (checkout / entry["source"]).is_file():
            raise Refusal(
                f"the owned file {path}'s source {entry['source']} is not in the checkout"
            )
    for pack, entry in sorted(wiring["packs"].items()):
        lint = entry.get("advisory_waivers", {}).get("lint")
        if lint and not (checkout / lint).is_file():
            raise Refusal(f"packs.{pack}'s advisory lint {lint} is not in the checkout")
    out = cards_directory()
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
    for name in wiring["unset_env"]:
        env.pop(name, None)
    box = wiring["box"]
    issues = named_issues(box, wiring["packs"])
    states = issue_states(root, issues)
    listed = ", ".join(f"{issue} {states[issue]}" for issue in issues)
    named = f"box-packs: {len(issues)} issue(s) the wiring names"
    print(named + (f": {listed}" if listed else ""), flush=True)
    closed = {issue for issue, state in states.items() if state == "CLOSED"}
    verdicts: list[Verdict] = []
    with tempfile.TemporaryDirectory(prefix="deckstreak-box-packs.") as name:
        scratch = Path(name).resolve()
        for repository, what in ((root, "the DeckStreak repository"), (checkout, "the checkout")):
            if scratch.is_relative_to(repository):
                raise Refusal(f"the scratch directory is inside {what}; set TMPDIR outside both")
        tree = export(root, sha, scratch / "tree")
        print(
            f"box-packs: judging {sha[:12]} ({args.rev}) with the packs checkout at {have[:12]}",
            flush=True,
        )
        driver = Runner(runner, scratch, tree, checkout / wiring["skills"], env)
        catalog = driver.pack_list()

        def report(verdict: Verdict) -> None:
            verdicts.append(verdict)
            print(verdict.line(), flush=True)

        for pack, entry in sorted(wiring["packs"].items()):
            verdict = judge_rows(driver, pack, entry, catalog, out)
            waivers = entry.get("advisory_waivers")
            if waivers and verdict.mark != "deferred":
                lint = checkout / waivers["lint"]
                judge_waivers(verdict, waivers, lint, tree, scratch, out, closed, env)
            report(verdict)
        for pack, expectation in sorted(box["packs"].items()):
            report(judge_pack(driver, pack, expectation, catalog, out, closed))
        for probe in PROBES:
            script = checkout / wiring["scripts"][probe]
            report(judge_probe(probe, script, tree, scratch, out, env))
        scan = checkout / wiring["scripts"][SCAN]
        report(judge_scan(box.get(SCAN, {}), scan, tree, scratch, out, closed, env))
        helper = checkout / wiring["scripts"][HELPER]
        report(judge_helper(box.get(HELPER, {}), helper, tree, scratch, out, closed, env))
        for path, entry in sorted(wiring["owned"].items()):
            report(judge_owned(path, entry, tree, checkout))
    print(f"cards: {out}", file=sys.stderr)
    return verdicts


def required(variable: str, what: str, kind: str) -> Path:
    value = os.environ.get(variable)
    if not value:
        raise Refusal(f"set {variable} to {what}")
    path = Path(value).resolve()
    usable = {
        "file": path.is_file(),
        "directory": path.is_dir(),
        "executable": path.is_file() and os.access(path, os.X_OK),
    }[kind]
    if not usable:
        raise Refusal(f"{variable}={value} is not {what}")
    return path


def read_wiring(path: Path) -> dict:
    """The private file, refused unless every section has its shape (SPEC-056 R7)."""
    try:
        wiring = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise Refusal(f"the private wiring file cannot be read: {error}") from error
    if not isinstance(wiring, dict) or wiring.get("schema") != SCHEMA:
        raise Refusal(f"the private wiring file is not {SCHEMA}")
    unknown = sorted(set(wiring) - WIRING_KEYS)
    if unknown:
        raise Refusal(f"the private wiring file has unknown key(s) {unknown}")
    if not PIN.match(str(wiring.get("pin", ""))):
        raise Refusal("the private wiring file's pin is not a full commit id")
    if not isinstance(wiring.get("skills"), str) or not wiring["skills"]:
        raise Refusal("the private wiring file names no skills directory")
    scripts = wiring.get("scripts")
    for name in (*PROBES, SCAN, HELPER):
        if not isinstance(scripts, dict) or not isinstance(scripts.get(name), str):
            raise Refusal(f"the private wiring file names no script for the {name} run")
    wiring["packs"] = packs_of(wiring.get("packs", {}))
    wiring["box"] = box_of(wiring.get("box"))
    wiring["owned"] = owned_of(wiring.get("owned", {}))
    unset = wiring.get("unset_env", [])
    if not isinstance(unset, list) or not all(isinstance(name, str) for name in unset):
        raise Refusal("the private wiring file's unset_env is not a list of names")
    wiring["unset_env"] = unset
    both = sorted(set(wiring["packs"]) & set(wiring["box"]["packs"]))
    if both:
        raise Refusal(f"the private wiring file judges {both} in both sections")
    return wiring


def packs_of(packs: object) -> dict:
    """The packs section, in the removed row runner's shape (SPEC-056 R8)."""
    if not isinstance(packs, dict):
        raise Refusal("the private wiring file's packs is not an object")
    for name, entry in packs.items():
        if not isinstance(entry, dict) or set(entry) - ROW_PACK_KEYS:
            raise Refusal(f"packs.{name} takes only {sorted(ROW_PACK_KEYS)}")
        state = entry.get("state")
        if state not in STATES:
            raise Refusal(f"packs.{name}: state {state!r} is not one of {STATES}")
        if state in ("pending", "deferred") and not ISSUE.match(str(entry.get("enforced_by"))):
            raise Refusal(f"packs.{name}: a {state} pack names the issue that enforces it")
        for key in ("excluded_rows", "deferred_rows"):
            rows = entry.get(key, {})
            if not isinstance(rows, dict) or not all(str(why).strip() for why in rows.values()):
                raise Refusal(f"packs.{name}.{key}: every row needs a reason or an issue")
        if "advisory_waivers" in entry:
            waivers_of(name, entry["advisory_waivers"])
    return packs


def waivers_of(name: str, waivers: object) -> None:
    """An `advisory_waivers` entry: its lint's path in the checkout, and each departure that waits
    on an issue, by unit, then reason (SPEC-056 R15)."""
    lint = waivers.get("lint") if isinstance(waivers, dict) else None
    if not isinstance(lint, str) or not lint or set(waivers) - WAIVER_KEYS:
        raise Refusal(f"packs.{name}.advisory_waivers takes a lint and its waiting departures")
    waiting = waivers.get("waiting", {})
    shaped = isinstance(waiting, dict) and all(
        isinstance(reasons, dict) and all(ISSUE.match(str(issue)) for issue in reasons.values())
        for reasons in waiting.values()
    )
    if not shaped:
        raise Refusal(f"packs.{name}.advisory_waivers.waiting maps a unit and a reason to an issue")


def box_of(box: object) -> dict:
    """The box section, refused unless every expectation names an issue (R12, R13)."""
    if not isinstance(box, dict) or not isinstance(box.get("packs"), dict) or not box["packs"]:
        raise Refusal("the private wiring file names no pack under box.packs")
    unknown = sorted(set(box) - BOX_KEYS)
    if unknown:
        raise Refusal(f"box has unknown key(s) {unknown}")
    for pack, entry in box["packs"].items():
        if not isinstance(entry, dict) or set(entry) - PACK_KEYS:
            raise Refusal(f"box.packs.{pack} takes only {sorted(PACK_KEYS)}")
        if "pending" in entry and entry.get("expected_red"):
            raise Refusal(f"box.packs.{pack} is pending, so it expects no red row")
        issues = [entry["pending"]] if "pending" in entry else []
        issues += list(entry.get("expected_red", {}).values())
        for issue in issues:
            if not ISSUE.match(str(issue)):
                raise Refusal(f"box.packs.{pack} waits on {issue!r}, not an issue")
    for name in (SCAN, HELPER):
        scan = box.get(name, {})
        if not isinstance(scan, dict) or set(scan) - SCAN_KEYS:
            raise Refusal(f"box.{name} takes only {sorted(SCAN_KEYS)}")
        if "pending" in scan and not ISSUE.match(str(scan["pending"])):
            raise Refusal(f"box.{name} waits on {scan['pending']!r}, not an issue")
    return box


def owned_of(owned: object) -> dict:
    """Each owned file's source and the fields it drops (SPEC-056 R10)."""
    if not isinstance(owned, dict):
        raise Refusal("the private wiring file's owned is not an object")
    for path, entry in owned.items():
        if not isinstance(entry, dict) or set(entry) - OWNED_KEYS or "source" not in entry:
            raise Refusal(f"owned.{path} takes a source and the fields it drops")
        dropped = entry.setdefault("dropped", [])
        if not isinstance(dropped, list) or not all(isinstance(item, str) for item in dropped):
            raise Refusal(f"owned.{path}.dropped is not a list of fields")
    return owned


def named_issues(box: dict, packs: dict) -> list[str]:
    """Every issue the box section and the advisory waiting entries name, once each, in number
    order (SPEC-054 R4, SPEC-056 R15)."""
    named = set()
    for entry in packs.values():
        for reasons in entry.get("advisory_waivers", {}).get("waiting", {}).values():
            named.update(reasons.values())
    for entry in box["packs"].values():
        named.update(entry.get("expected_red", {}).values())
        if "pending" in entry:
            named.add(entry["pending"])
    for name in (SCAN, HELPER):
        if "pending" in box.get(name, {}):
            named.add(box[name]["pending"])
    return sorted(named, key=lambda issue: int(issue[1:]))


def issue_states(root: Path, issues: list[str]) -> dict[str, str]:
    """Each issue's state, OPEN or CLOSED, as `gh issue view <n> --json state` answers in ROOT.
    An issue gh cannot answer for makes the run VOID: a refusal (exit 2) naming why, because an
    expectation whose issue may be closed cannot be judged (SPEC-054 R4)."""
    if not issues:
        return {}
    gh = shutil.which("gh")
    if gh is None:
        raise Refusal(f"gh is not on the path, so the state of {', '.join(issues)} cannot be read")
    states = {}
    for issue in issues:
        done = subprocess.run(
            [gh, "issue", "view", issue.removeprefix("#"), "--json", "state"],
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if done.returncode == GH_NOT_LOGGED_IN:
            raise Refusal(
                f"gh is not logged in (exit {GH_NOT_LOGGED_IN}), so the state of {issue} "
                "cannot be read"
            )
        if done.returncode != 0:
            raise Refusal(
                f"gh could not read the state of {issue} (exit {done.returncode}): "
                f"{tail(done.stderr)}"
            )
        try:
            state = json.loads(done.stdout).get("state")
        except (json.JSONDecodeError, AttributeError):
            state = None
        if state not in ISSUE_STATES:
            raise Refusal(f"gh answered no state for {issue}: {tail(done.stdout)}")
        states[issue] = state
    return states


def post_status(root: Path, sha: str, state: str, description: str) -> None:
    """One commit status on `sha`, through `gh api` in ROOT (SPEC-056 R11)."""
    gh = shutil.which("gh")
    if gh is None:
        print(f"box-packs: gh is not on the path, so no {CONTEXT} status was posted")
        return
    done = subprocess.run(
        [
            gh, "api", f"repos/{{owner}}/{{repo}}/statuses/{sha}", "--method", "POST",
            "-f", f"state={state}", "-f", f"context={CONTEXT}", "-f", f"description={description}",
        ],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )  # fmt: skip
    if done.returncode == 0:
        print(f"box-packs: posted {CONTEXT} {state} on {sha[:12]}")
    else:
        print(f"box-packs: the {CONTEXT} status was not posted (exit {done.returncode})")


def closed_expectations(expectation: dict, closed: set[str]) -> list[str]:
    """Each expectation whose issue is closed, by its row (or `pending`) and its issue: stale,
    because a closed issue builds nothing more (SPEC-054 R4)."""
    found = [
        f"{row} ({issue} is closed)"
        for row, issue in sorted(expectation.get("expected_red", {}).items())
        if issue in closed
    ]
    if expectation.get("pending") in closed:
        found.append(f"pending ({expectation['pending']} is closed)")
    return found


def git(repository: Path, *words: str) -> str:
    done = subprocess.run(
        ["git", "-C", str(repository), *words], capture_output=True, text=True, check=False
    )
    if done.returncode != 0:
        raise Refusal(f"git {' '.join(words)} failed in {repository}: {tail(done.stderr)}")
    return done.stdout.strip()


def cards_directory() -> Path:
    chosen = os.environ.get("BOX_PACKS_OUT")
    if chosen:
        out = Path(chosen)
        out.mkdir(parents=True, exist_ok=True)
        return out
    return Path(tempfile.mkdtemp(prefix="deckstreak-box-cards."))


def export(root: Path, sha: str, tree: Path) -> Path:
    """The committed tree at `sha`, never the working copy (R11)."""
    tree.mkdir()
    archive = subprocess.run(
        ["git", "-C", str(root), "archive", "--format=tar", sha], capture_output=True, check=False
    )
    if archive.returncode != 0:
        raise Refusal(f"git archive {sha[:12]} failed: {tail(archive.stderr.decode())}")
    unpack = subprocess.run(
        ["tar", "-x", "-C", str(tree)], input=archive.stdout, capture_output=True, check=False
    )
    if unpack.returncode != 0:
        raise Refusal(f"the archive of {sha[:12]} did not unpack: {tail(unpack.stderr.decode())}")
    return tree


def tail(text: str) -> str:
    lines = [line for line in text.strip().splitlines() if line.strip()]
    return lines[-1][:200] if lines else "(nothing printed)"


def verbs_of(declared: list[str]) -> list[str]:
    """The verbs a pack's declared card schemas admit, read by each schema's suffix."""
    return sorted(
        {verb for schema in declared for suffix, verb in VERBS.items() if schema.endswith(suffix)}
    )


def declared_schemas(entry: dict) -> list[str]:
    """Every card schema a catalog entry declares: each of its strings with a known suffix."""
    found = []
    for value in entry.values():
        for item in value if isinstance(value, list) else [value]:
            if isinstance(item, str) and item.endswith(tuple(VERBS)):
                found.append(item)
    return found


class Runner:
    """The runner, called with the scratch directory as its working directory."""

    def __init__(self, exe: Path, scratch: Path, tree: Path, skills: Path, env: dict) -> None:
        self.exe, self.scratch, self.tree, self.skills, self.env = exe, scratch, tree, skills, env
        self.ledger: Path | None = None
        self.project: str | None = None

    def call(self, *words: object) -> subprocess.CompletedProcess:
        return subprocess.run(
            [str(self.exe), *map(str, words)],
            cwd=self.scratch,
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )

    def pack_list(self) -> dict[str, list[str]]:
        """Each catalogued pack's card schemas, as the runner's `pack list` reports them (R10)."""
        done = self.call("pack", "list", "--skills-root", self.skills, "--format", "json")
        try:
            packs = json.loads(done.stdout)["packs"]
            return {
                str(entry["id"]).removeprefix("packs/"): declared_schemas(entry) for entry in packs
            }
        except (json.JSONDecodeError, KeyError, TypeError, AttributeError) as error:
            raise Refusal(
                f"the runner's pack list answered no list (exit {done.returncode}): "
                f"{tail(done.stderr)}"
            ) from error

    def project_id(self) -> tuple[Path, str]:
        """A scratch ledger with the judged tree registered as a project, made once per run."""
        if self.ledger is None or self.project is None:
            ledger = self.scratch / "ledger" / "ledger.db"
            ledger.parent.mkdir()
            made = self.call("--ledger", ledger, "init")
            if made.returncode != 0:
                raise Refusal(f"the runner's init refused the scratch ledger: {tail(made.stderr)}")
            done = self.call(
                "--ledger", ledger, "project", "register",
                "--slug", "deckstreak", "--name", "DeckStreak", "--repo", self.tree,
            )  # fmt: skip
            found = REGISTERED.search(done.stdout)
            if done.returncode != 0 or found is None:
                why = tail(done.stderr or done.stdout)
                raise Refusal(f"the runner's project register failed: {why}")
            self.ledger, self.project = ledger, found.group(1)
        return self.ledger, self.project


def save_card(out: Path, pack: str, done: subprocess.CompletedProcess) -> None:
    (out / f"{pack}.json").write_text(done.stdout, encoding="utf-8")
    (out / f"{pack}.err").write_text(done.stderr, encoding="utf-8")


def judge_row(code: object, severity: str, state: str) -> str:
    """A row's verdict by its exit, as the removed row runner read it: 0 ok, 1 RED, 3 VOID (read
    as pending in a pending pack), anything else ERROR; an advisory row never fails."""
    if code == 0:
        return "ok"
    if severity == "advisory":
        return "advisory"
    if code == 1:
        return "RED"
    if code == 3:
        return "VOID" if state == "enforced" else "pending"
    return "ERROR"


def judge_rows(driver: Runner, pack: str, entry: dict, catalog: dict, out: Path) -> Verdict:
    """A pack of the packs section, judged row by row (SPEC-056 R8)."""
    state = entry["state"]
    if state == "deferred":
        why = f"deferred to {entry['enforced_by']}"
        return Verdict(pack, "-", "deferred", detail=why, counted=False)
    declared = catalog.get(pack)
    if declared is None:
        why = "the runner's pack list names no such pack"
        return Verdict(pack, "?", "FAIL", detail=why, counted=False)
    if verbs_of(declared) != ["probe"]:
        why = f"a packs-section pack is a probe; {declared}"
        return Verdict(pack, "?", "FAIL", detail=why, counted=False)
    done = driver.call(
        "pack", "probe", "--pack", pack, "--root", driver.tree,
        "--skills-root", driver.skills, "--scope", "tree", "--format", "json",
    )  # fmt: skip
    save_card(out, pack, done)
    try:
        card = json.loads(done.stdout)
        rows = card["checks"] if str(card.get("schema", "")).endswith(".pack.probe.v1") else None
    except (json.JSONDecodeError, KeyError, TypeError, AttributeError):
        rows = None
    if not isinstance(rows, list):
        why = f"the runner gave no probe card (exit {done.returncode}): {tail(done.stderr)}"
        return Verdict(pack, "probe", "FAIL", detail=why, counted=False)
    excluded = entry.get("excluded_rows", {})
    deferred = entry.get("deferred_rows", {})
    missing = sorted((set(excluded) | set(deferred)) - {row.get("id") for row in rows})
    if missing:
        raise Refusal(f"packs.{pack} names row(s) {', '.join(missing)}, which the pack lacks")
    counts: Counter[str] = Counter()
    failing, blocking = [], []
    for row in rows:
        ident, code = row.get("id"), row.get("exit")
        severity = "advisory" if row.get("severity") == "advisory" else "block"
        if ident in excluded:
            counts["excluded"] += 1
        elif ident in deferred:
            if code == 0:
                failing.append(f"STALE {ident} (deferred to {deferred[ident]}, and it passes)")
            else:
                counts["deferred"] += 1
        else:
            verdict = judge_row(code, severity, state)
            counts[verdict] += 1
            if verdict in ("RED", "VOID", "ERROR"):
                failing.append(f"{verdict} {ident}")
            if severity == "block":
                blocking.append(code)
    if state == "pending" and blocking and all(code == 0 for code in blocking):
        failing.append(
            f"STALE: pending on {entry['enforced_by']}, but every blocking row ran and passed "
            f"({len(blocking)} row(s))"
        )
    examined = sum(counts[word] for word in ("ok", "advisory", "pending", "RED", "VOID", "ERROR"))
    words = ("ok", "advisory", "pending", "excluded", "deferred")
    parts = [", ".join(f"{word} {counts[word]}" for word in words)]
    if failing:
        parts.append("; ".join(failing))
        mark = "FAIL"
    elif counts["pending"]:
        parts.insert(0, f"pending {entry['enforced_by']}")
        mark = "pending"
    else:
        mark = "ok"
    return Verdict(pack, "probe", mark, examined=examined, detail="; ".join(parts), counted=False)


def judge_pack(
    driver: Runner, pack: str, expectation: dict, catalog: dict, out: Path, closed: set[str]
) -> Verdict:
    """A pack of the box section, with the verb its catalog admits."""
    stale = closed_expectations(expectation, closed)
    declared = catalog.get(pack)
    if declared is None:
        detail = "the runner's pack list names no such pack"
        return Verdict(pack, "?", "FAIL", stale=stale, detail=detail)
    verbs = verbs_of(declared)
    if len(verbs) != 1:
        return Verdict(pack, "?", "FAIL", stale=stale, detail=f"no single verb admits {declared}")
    verb = verbs[0]
    tree = driver.tree
    if verb == "verify":
        site = tree / SITE
        if not site.is_dir():
            why = f"{SITE} is not in the judged tree"
            return judged(pack, verb, expectation, None, why, closed)
        done = driver.call("verify", "seo-pipeline", "--subject", site, "--format", "json")
    elif verb == "probe":
        done = driver.call(
            "pack", "probe", "--pack", pack, "--root", tree,
            "--skills-root", driver.skills, "--format", "json",
        )  # fmt: skip
    else:
        ledger, project = driver.project_id()
        done = driver.call(
            "--ledger", ledger, "pack", "run", "--pack", pack, "--project", project,
            "--root", tree, "--skills-root", driver.skills, "--format", "json",
        )  # fmt: skip
    save_card(out, pack, done)
    card = read_card(verb, done.stdout)
    if card is None:
        why = f"the runner gave no card (exit {done.returncode}): {tail(done.stderr)}"
        return Verdict(pack, verb, "FAIL", stale=stale, detail=why)
    return judged(pack, verb, expectation, card, "the walk examined no row", closed)


def read_card(verb: str, stdout: str) -> Card | None:
    """The card, read by the suffix of the schema its verb stamps; anything else is no card."""
    try:
        card = json.loads(stdout)
    except json.JSONDecodeError:
        return None
    if not isinstance(card, dict):
        return None
    schema = str(card.get("schema", ""))
    if verb == "probe" and schema.endswith(".pack.probe.v1"):
        rows = {row["id"]: state(row.get("color"), row) for row in card.get("checks", [])}
        return Card(int(card.get("examined", len(rows))), rows)
    if verb == "run" and schema.endswith(".pack.run.v1"):
        rows = {row["id"]: state(row.get("verdict"), row) for row in card.get("checks", [])}
        return Card(int(card.get("examined", 0)), rows)
    if verb == "verify" and schema.endswith(".seo-pipeline.v1"):
        stages = card.get("stages", [])
        rows = {stage["id"]: state(stage.get("verdict"), stage) for stage in stages}
        return Card(len(rows), rows)
    return None


def state(word: object, row: dict) -> str:
    """A row's state: `red` only for a blocking row that failed; an advisory row never is."""
    if row.get("severity") == "advisory" and word in ("red", "advisory"):
        return "advisory"
    return {"red": "red", "green": "green", "ok": "green"}.get(str(word), str(word))


def judged(
    pack: str, verb: str, expectation: dict, card: Card | None, void: str, closed: set[str]
) -> Verdict:
    """R12 and R14: unexpected reds, expected reds, stale expectations, and a pending VOID; and an
    expectation whose issue is closed is stale whatever the card says (SPEC-054 R4)."""
    verdict = Verdict(pack, verb, examined=card.examined if card else 0)
    verdict.stale = closed_expectations(expectation, closed)
    expected = expectation.get("expected_red", {})
    pending = expectation.get("pending")
    if card is None or card.examined == 0:
        if pending:
            verdict.mark, verdict.detail = "pending", f"pending {pending}: {void}"
        else:
            verdict.mark, verdict.detail = "FAIL", f"VOID: {void}, and the wiring names no issue"
        if verdict.stale:
            verdict.mark = "FAIL"
        return verdict
    reds = sorted(row for row, word in card.rows.items() if word == "red")
    verdict.unexpected = [row for row in reds if row not in expected]
    verdict.expected = [row for row in reds if row in expected]
    for row, issue in sorted(expected.items()):
        if issue not in closed and card.rows.get(row) != "red":
            verdict.stale.append(f"{row} ({issue})")
    if pending:
        verdict.stale.append(f"pending {pending}, but the pack examined {card.examined} row(s)")
    if verdict.unexpected or verdict.stale:
        verdict.mark = "FAIL"
    return verdict


def judge_waivers(
    verdict: Verdict, waivers: dict, lint: Path, tree: Path, scratch: Path, out: Path,
    closed: set[str], env: dict,
) -> None:
    """R15: every advisory finding of the lint's report, by unit and reason, is waived in its unit
    with a why of more than five words, or waits on an open issue the entry names. An unwaived
    departure or a thin why fails the pack by name; a waiver or waiting entry that matches no
    finding, or waits on a closed issue, is stale. The rows' own verdict stands beside it."""
    done = subprocess.run(
        [sys.executable, str(lint), "lint", "--root", str(tree), "--format", "json"],
        cwd=scratch,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    (out / f"{verdict.pack}.lint.json").write_text(done.stdout + done.stderr, encoding="utf-8")
    try:
        checks = [c for c in json.loads(done.stdout)["checks"] if c["severity"] == "advisory"]
        departures = {(f["unit"], c["reason"]) for c in checks for f in c["findings"]}
        waived = {
            (Path(item["unit"]).name, c["reason"]): str(item["why"])
            for c in checks
            for item in c["waived"]
        }
    except (json.JSONDecodeError, KeyError, TypeError) as error:
        verdict.mark = "FAIL"
        verdict.detail = "; ".join(filter(None, (verdict.detail, (
            f"the advisory lint gave no report it can read (exit {done.returncode}): {error}"
        ))))
        return
    waiting = {
        (unit, reason): issue
        for unit, reasons in waivers.get("waiting", {}).items()
        for reason, issue in reasons.items()
    }
    problems = [f"unwaived {unit} {reason}" for unit, reason in sorted(departures - set(waiting))]
    problems += [
        f"thin why {unit} {reason}" for (unit, reason), why in sorted(waived.items())
        if len(why.split()) <= 5
    ]
    stale = [
        f"waiting {unit} {reason} matches no departure"
        for unit, reason in sorted(set(waiting) - departures)
    ]
    stale += [
        f"waiting {unit} {reason} ({issue} is closed)"
        for (unit, reason), issue in sorted(waiting.items()) if issue in closed
    ]
    stale += [
        f"waiver {unit} {reason} matches no departure"
        for unit, reason in sorted(declared_waivers(tree) - set(waived))
    ]
    parts = [f"waivers: {len(waived)} waived, {len(waiting)} waiting", *problems]
    parts += [f"STALE {entry}" for entry in stale]
    verdict.detail = "; ".join(filter(None, (verdict.detail, *parts)))
    if problems or stale:
        verdict.mark = "FAIL"


def declared_waivers(tree: Path) -> set[tuple[str, str]]:
    """Every waiver with a why that the judged tree's units declare in `[Unit]`, by unit and
    reason; a drop-in's waiver is its unit's (SPEC-056 R15). A waiver is one line."""
    found: set[tuple[str, str]] = set()
    deploy = tree / "deploy"
    for path in sorted(deploy.rglob("*")) if deploy.is_dir() else []:
        folder = path.parent.name
        if path.suffix in UNIT_SUFFIXES and not folder.endswith(".d"):
            unit = path.name
        elif path.suffix == ".conf" and folder[:-2].endswith(UNIT_SUFFIXES) and folder[-2:] == ".d":
            unit = folder[:-2]
        else:
            continue
        section = None
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            line = line.strip()
            if line.startswith("[") and line.endswith("]"):
                section = line[1:-1].strip()
            elif section == "Unit" and line.startswith(WAIVE_KEY + "="):
                reason, _, why = line.split("=", 1)[1].strip().partition(" ")
                if reason and why.strip():
                    found.add((unit, reason))
    return found


def judge_probe(
    name: str, script: Path, tree: Path, scratch: Path, out: Path, env: dict
) -> Verdict:
    """A methodology probe from the checkout, `check all` over the judged tree (SPEC-056 R9)."""
    verdict = Verdict(name, "probe", counted=False)
    if not script.is_file():
        verdict.mark, verdict.detail = "FAIL", f"the checkout has no {script.name}"
        return verdict
    done = subprocess.run(
        [sys.executable, str(script), "--root", str(tree), "check", "all"],
        cwd=scratch,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    (out / f"{name}.txt").write_text(done.stdout + done.stderr, encoding="utf-8")
    found = [line for line in done.stdout.splitlines() if PROBE_LINE.match(line)]
    verdict.examined = len(found)
    classes = [PROBE_LINE.match(line).groups() for line in found]
    refused = [f"{ident} {word}" for _, ident, word in classes if word != "OK"]
    if not found:
        verdict.mark = "FAIL"
        verdict.detail = f"the probe printed no verdict it can read (exit {done.returncode})"
    elif refused or done.returncode != 0:
        verdict.mark = "FAIL"
        verdict.detail = ", ".join(refused) or f"exit {done.returncode}"
    else:
        verdict.detail = f"{len(found)} class(es) OK"
    return verdict


def judge_scan(
    expectation: dict, script: Path, tree: Path, scratch: Path, out: Path, closed: set[str],
    env: dict,
) -> Verdict:
    """R13: the proxy scan, read by its row lines, because `check all` exits VOID over RED. A
    pending issue that is closed fails it (SPEC-054 R4)."""
    verdict = Verdict(SCAN, "scan", stale=closed_expectations(expectation, closed))
    pending = expectation.get("pending")
    if not script.is_file():
        verdict.mark, verdict.detail = "FAIL", f"the checkout has no {script.name}"
        return verdict
    done = subprocess.run(
        [sys.executable, str(script), "--root", str(tree), "check", "all"],
        cwd=scratch,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    (out / f"{SCAN}.txt").write_text(done.stdout + done.stderr, encoding="utf-8")
    rows, settings, summary = {}, None, None
    for line in (done.stdout + "\n" + done.stderr).splitlines():
        row = SCAN_ROW.match(line)
        if row:
            word, ident, count, unit = row.groups()
            rows[ident] = word
            if unit == SETTINGS:
                settings = max(settings or 0, int(count))
            continue
        total = SCAN_ALL.match(line)
        if total:
            summary = tuple(int(number) for number in total.groups())
    verdict.examined = len(rows)
    if not rows or summary is None or settings is None:
        verdict.mark = "FAIL"
        verdict.detail = f"the scan printed no verdict it can read (exit {done.returncode})"
        return verdict
    green, red, void = summary
    counts = f"{settings} settings document(s); blocking {green} green, {red} red, {void} void"
    verdict.unexpected = sorted(ident for ident, word in rows.items() if word == "RED")
    if verdict.unexpected or red:
        verdict.mark, verdict.detail = "FAIL", counts
    elif settings == 0 and pending:
        verdict.mark, verdict.detail = "pending", f"pending {pending}: {counts}"
    elif settings == 0:
        verdict.mark, verdict.detail = "FAIL", f"VOID: {counts}, and the wiring names no issue"
    elif pending:
        verdict.mark, verdict.detail = "FAIL", counts
        verdict.stale.append(f"pending {pending}, but a settings document was examined")
    elif void:
        verdict.mark, verdict.detail = "FAIL", f"VOID: {counts}"
    else:
        verdict.detail = counts
    if verdict.stale:
        verdict.mark = "FAIL"
    return verdict


def judge_helper(
    expectation: dict, script: Path, tree: Path, scratch: Path, out: Path, closed: set[str],
    env: dict,
) -> Verdict:
    """R9: the apiKeyHelper scan, read by its one verdict line and its exit (0 clean, 1 a finding,
    2 VOID). Finding no settings file reads `pending` with the issue the box section names; once a
    file is examined, or the issue closes, that expectation is stale (SPEC-054 R4)."""
    verdict = Verdict(HELPER, "scan", counted=False, stale=closed_expectations(expectation, closed))
    pending = expectation.get("pending")
    if not script.is_file():
        verdict.mark, verdict.detail = "FAIL", f"the checkout has no {script.name}"
        return verdict
    done = subprocess.run(
        [sys.executable, str(script), "--root", str(tree)],
        cwd=scratch,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    (out / f"{HELPER}.txt").write_text(done.stdout + done.stderr, encoding="utf-8")
    lines = (done.stdout + "\n" + done.stderr).splitlines()
    found = [match.groups() for match in map(HELPER_LINE.match, lines) if match]
    if len(found) != 1:
        verdict.mark = "FAIL"
        verdict.detail = f"the scan printed no verdict it can read (exit {done.returncode})"
        return verdict
    word, rest = found[0]
    files, findings = HELPER_FILES.match(rest), HELPER_FINDINGS.match(rest)
    if word == "GREEN" and files and done.returncode == 0:
        verdict.examined = int(files.group(1))
        verdict.detail = f"{verdict.examined} settings file(s), none carries the shape"
        if pending:
            verdict.stale.append(f"pending {pending}, but a settings file was examined")
    elif word == "RED" and findings and done.returncode == 1:
        verdict.examined = int(findings.group(2))
        verdict.mark, verdict.detail = "FAIL", f"{findings.group(1)} finding(s)"
    elif word == "VOID" and files and int(files.group(1)) == 0 and done.returncode == 2:
        listed = settings_paths(tree)
        if listed:
            verdict.mark = "FAIL"
            verdict.detail = f"VOID: the scan found no settings file in {', '.join(listed)}"
        elif pending:
            verdict.mark, verdict.detail = "pending", f"pending {pending}: 0 settings file(s)"
        else:
            verdict.mark = "FAIL"
            verdict.detail = "VOID: 0 settings file(s), and the private file names no issue"
    else:
        verdict.mark, verdict.detail = "FAIL", f"{word} (exit {done.returncode})"
    if verdict.stale:
        verdict.mark = "FAIL"
    return verdict


def settings_paths(tree: Path) -> list[str]:
    """Every file of the judged tree at a path the removed gate step read as a settings file."""
    files = (path.relative_to(tree).as_posix() for path in tree.rglob("*") if path.is_file())
    return sorted(name for name in files if SETTINGS_PATHS.search(name))


def drift(mine: object, theirs: object, dropped: set[str], where: str) -> tuple[str | None, int]:
    """The first difference between an owned document and its source over the fields the owned
    one keeps, or a field the source gained that it neither keeps nor drops; and how many kept
    values were compared. A list index is written `[n]`, and `[]` in a dropped field's name."""
    if isinstance(mine, dict):
        if not isinstance(theirs, dict):
            return f"{where or 'the document'} is no longer an object in the source", 0
        compared = 0
        for key, value in mine.items():
            place = f"{where}.{key}" if where else key
            if key not in theirs:
                return f"{place} is gone from the source", compared
            found, count = drift(value, theirs[key], dropped, place)
            compared += count
            if found:
                return found, compared
        for key in theirs:
            place = f"{where}.{key}" if where else key
            if key not in mine and re.sub(r"\[\d+\]", "[]", place) not in dropped:
                return f"the source has a new field {place}", compared
        return None, compared
    if isinstance(mine, list):
        if not isinstance(theirs, list) or len(theirs) != len(mine):
            size = len(theirs) if isinstance(theirs, list) else "no"
            return f"{where} holds {len(mine)} item(s), and the source {size}", 0
        compared = 0
        for index, (one, other) in enumerate(zip(mine, theirs, strict=True)):
            found, count = drift(one, other, dropped, f"{where}[{index}]")
            compared += count
            if found:
                return found, compared
        return None, compared
    if mine != theirs:
        return f"{where} differs from the source", 1
    return None, 1


def judge_owned(path: str, entry: dict, tree: Path, checkout: Path) -> Verdict:
    """An owned file against its source at the pin, over the fields it keeps (SPEC-056 R10)."""
    verdict = Verdict(path, "drift", counted=False)
    try:
        mine = json.loads((tree / path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        verdict.mark, verdict.detail = "FAIL", f"the judged tree's copy cannot be read: {error}"
        return verdict
    try:
        theirs = json.loads((checkout / entry["source"]).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise Refusal(f"the owned file {path}'s source {entry['source']} cannot be read") from error
    found, compared = drift(mine, theirs, set(entry["dropped"]), "")
    verdict.examined = compared
    if found:
        verdict.mark, verdict.detail = "FAIL", found
    else:
        verdict.detail = f"equal to {entry['source']} over the fields it keeps"
    return verdict


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
PY
