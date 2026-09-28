#!/usr/bin/env bash
# The packs that cannot run in public CI, run on the maintainer's box against a DeckStreak commit
# (ADR-004, ADR-030, SPEC-030 R10 to R14): the packs built into phxd, and the subscription-proxy
# client scan, whose probe names a private secret and so is not vendored. Their verdicts are posted
# on the pull request.
#
#   PHOENIX=/path/to/a/phoenix-v2/checkout PHXD=/path/to/phxd bash scripts/box-packs.sh [--rev REV] [ROOT]
#
# * It judges the COMMITTED tree at REV (default HEAD) of ROOT (default this repository), exported
#   with `git archive` into a scratch directory outside both repositories, without the vendored rule
#   code: `.packs/` and every path `.packs/VENDORED.json` lists. A rule's own source is never read
#   as DeckStreak's code. The wiring and the pin are read from that same commit.
# * It runs each pack `.packs/wiring.json` names under `box.packs` with the verb its catalog row
#   admits, read from `phxd pack list` and never hard-coded: `phxd.pack.probe.v1` is
#   `phxd pack probe`, `phxd.pack.run.v1` is `phxd pack run` against a scratch ledger made with
#   `phxd init` and `phxd project register`, and `phxd.seo-pipeline.v1` is
#   `phxd verify seo-pipeline` over `web/site/dist`, once the judged tree holds it (#59).
# * It judges each card's rows against the wiring: a red row it does not name under `expected_red`
#   fails the run by name, a named row that is no longer red is refused as stale, and an advisory
#   row never fails. A pack that examines nothing reads `pending` with the issue the wiring names,
#   is VOID without one, and is stale once it examines a row. The proxy scan is read by its rows,
#   never its exit: any RED fails, and it reads `pending` while it examines no settings document.
# * Before any pack runs, it reads the state of every issue the `box` section names (each
#   `expected_red` row's issue, each `pending`, and the proxy scan's `pending`) once, with
#   `gh issue view <n> --json state` run in ROOT, so gh resolves the repository from ROOT's remotes
#   or from $GH_REPO. An expectation whose issue is CLOSED is stale, and fails its pack by name.
#   When gh is not on the path, is not logged in, cannot reach GitHub or answers no state, the run
#   is VOID with that reason and runs no pack: it never passes on an issue it could not read
#   (SPEC-054 R4).
#
# It prints one line per pack (its examined count, its unexpected, expected and stale rows) and a
# summary, and exits 0 when no pack failed, 1 when one did, and 2 when it cannot judge (an unset
# variable, a pin that does not match, a malformed wiring, an issue whose state gh cannot read).
# PHXD must be built from the phoenix-v2 commit `.packs/VENDORED.json` names, because a prebuilt
# phxd embeds an older catalog. The scratch directory (the exported tree and the ledger) is removed
# when the run ends; each pack's card is kept under $BOX_PACKS_OUT (a fresh temporary directory by
# default) for the pull request.
set -euo pipefail
BOX_PACKS_SELF="${BASH_SOURCE[0]}" exec python3 - "$@" <<'PY'
"""The box-pack runner: this file's opening comment is its documentation (ADR-030)."""

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
from dataclasses import dataclass, field
from pathlib import Path

SELF = Path(os.environ["BOX_PACKS_SELF"]).resolve()
WIRING = ".packs/wiring.json"
VENDORED = ".packs/VENDORED.json"
# phxd's verbs, by the card schema a catalog row declares. Which pack takes which verb is read
# from `phxd pack list`, never written here.
VERBS = {
    "phxd.pack.probe.v1": "probe",
    "phxd.pack.run.v1": "run",
    "phxd.seo-pipeline.v1": "verify",
}
SITE = "web/site/dist"
SCAN = "proxy-client-scan"
BOX_KEYS = {"packs", SCAN, "note"}
PACK_KEYS = {"expected_red", "pending", "note"}
SCAN_KEYS = {"pending", "note"}
ISSUE = re.compile(r"^#\d+$")
REGISTERED = re.compile(r"registered\s+\D*(\d+)")
SCAN_ROW = re.compile(r"^PROXY-CLIENT (GREEN|RED|ADVISORY|VOID) (\S+): (\d+) (.+?) examined\b")
SCAN_ALL = re.compile(r"^PROXY-CLIENT ALL \w+: blocking (\d+) green, (\d+) red, (\d+) void\b")
SETTINGS = "settings document(s)"
# The states `gh issue view --json state` answers, and its exit when it is not logged in.
STATES = ("OPEN", "CLOSED")
GH_NOT_LOGGED_IN = 4


class Refusal(Exception):
    """The run cannot judge; the message names why (exit 2)."""


@dataclass
class Card:
    """One pack's card, read by its schema: how many rows it examined, and each row's state."""

    examined: int
    rows: dict[str, str]


@dataclass
class Verdict:
    """One pack's line: `ok`, `pending` or `FAIL`, with the rows that decided it."""

    pack: str
    verb: str
    mark: str = "ok"
    examined: int = 0
    unexpected: list[str] = field(default_factory=list)
    expected: list[str] = field(default_factory=list)
    stale: list[str] = field(default_factory=list)
    detail: str = ""

    def line(self) -> str:
        head = f"{self.mark:8} {self.pack:20} {self.verb:6} examined {self.examined}"
        parts = []
        if self.examined or self.unexpected or self.stale:
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
        return f"{head}: {'; '.join(parts)}"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="box-packs.sh",
        description="run the phxd packs and the proxy scan against a DeckStreak commit",
    )
    parser.add_argument("--rev", default="HEAD", help="the commit to judge (default HEAD)")
    parser.add_argument("root", nargs="?", default=str(SELF.parents[1]), help="the repository")
    args = parser.parse_args(argv)
    # A signal exits through the scratch directory's cleanup, never around it.
    for number in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, lambda signum, _frame: sys.exit(128 + signum))
    try:
        return run(args)
    except Refusal as refusal:
        print(f"box-packs: {refusal}", flush=True)
        return 2


def run(args: argparse.Namespace) -> int:
    phoenix = required("PHOENIX", "a phoenix-v2 checkout at the vendored commit", directory=True)
    phxd = required("PHXD", "a phxd built from that checkout", directory=False)
    root = Path(args.root).resolve()
    sha = git(root, "rev-parse", "--verify", "--quiet", f"{args.rev}^{{commit}}")
    out = cards_directory()
    verdicts = []
    with tempfile.TemporaryDirectory(prefix="deckstreak-box-packs.") as name:
        scratch = Path(name).resolve()
        for repository, what in ((root, "the DeckStreak repository"), (phoenix, "phoenix-v2")):
            if scratch.is_relative_to(repository):
                raise Refusal(f"the scratch directory is inside {what}; set TMPDIR outside both")
        tree = export(root, sha, scratch / "tree")
        wiring = read_json(tree / WIRING)
        vendored = read_json(tree / VENDORED)
        box = box_of(wiring)
        pinned = str(vendored.get("vendored_from", ""))
        have = git(phoenix, "rev-parse", "HEAD")
        if pinned != have:
            raise Refusal(
                f"phoenix-v2 is at {have}, the judged commit's vendored copy names {pinned}; "
                "re-pin one of them"
            )
        strip(tree, vendored)
        issues = named_issues(box)
        states = issue_states(root, issues)
        listed = ", ".join(f"{issue} {states[issue]}" for issue in issues)
        named = f"box-packs: {len(issues)} issue(s) the wiring names"
        print(named + (f": {listed}" if listed else ""), flush=True)
        closed = {issue for issue, state in states.items() if state == "CLOSED"}
        print(
            f"box-packs: judging {sha[:12]} ({args.rev}) without the vendored rule code, "
            f"with phoenix-v2 {have[:12]}",
            flush=True,
        )
        driver = Phxd(phxd, scratch, tree, phoenix / "skills")
        catalog = driver.pack_list()
        for pack, expectation in sorted(box["packs"].items()):
            verdicts.append(judge_pack(driver, pack, expectation, catalog, out, closed))
            print(verdicts[-1].line(), flush=True)
        verdicts.append(judge_scan(box.get(SCAN, {}), phoenix, tree, scratch, out, closed))
        print(verdicts[-1].line(), flush=True)
    print(f"cards: {out}")
    failed = [verdict.pack for verdict in verdicts if verdict.mark == "FAIL"]
    if failed:
        print(f"BOX PACKS FAILED: {len(failed)} of {len(verdicts)} pack(s): {', '.join(failed)}")
        return 1
    pending = sum(verdict.mark == "pending" for verdict in verdicts)
    print(f"BOX PACKS OK: {len(verdicts)} pack(s), {pending} pending")
    return 0


def required(variable: str, what: str, directory: bool) -> Path:
    value = os.environ.get(variable)
    if not value:
        raise Refusal(f"set {variable} to {what}")
    path = Path(value).resolve()
    usable = path.is_dir() if directory else path.is_file() and os.access(path, os.X_OK)
    if not usable:
        raise Refusal(f"{variable}={value} is not {what}")
    return path


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


def read_json(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise Refusal(f"the judged commit's {path.name} cannot be read: {error}") from error
    if not isinstance(data, dict):
        raise Refusal(f"the judged commit's {path.name} is not a JSON object")
    return data


def box_of(wiring: dict) -> dict:
    """The wiring's `box` section, refused unless every expectation names an issue (R12, R13)."""
    box = wiring.get("box")
    if not isinstance(box, dict) or not isinstance(box.get("packs"), dict) or not box["packs"]:
        raise Refusal(f"{WIRING} names no pack under box.packs")
    unknown = sorted(set(box) - BOX_KEYS)
    if unknown:
        raise Refusal(f"{WIRING}: box has unknown key(s) {unknown}")
    for pack, entry in box["packs"].items():
        if not isinstance(entry, dict) or set(entry) - PACK_KEYS:
            raise Refusal(f"{WIRING}: box.packs.{pack} takes only {sorted(PACK_KEYS)}")
        if "pending" in entry and entry.get("expected_red"):
            raise Refusal(f"{WIRING}: box.packs.{pack} is pending, so it expects no red row")
        issues = [entry["pending"]] if "pending" in entry else []
        issues += list(entry.get("expected_red", {}).values())
        for issue in issues:
            if not ISSUE.match(str(issue)):
                raise Refusal(f"{WIRING}: box.packs.{pack} waits on {issue!r}, not an issue")
    scan = box.get(SCAN, {})
    if not isinstance(scan, dict) or set(scan) - SCAN_KEYS:
        raise Refusal(f"{WIRING}: box.{SCAN} takes only {sorted(SCAN_KEYS)}")
    if "pending" in scan and not ISSUE.match(str(scan["pending"])):
        raise Refusal(f"{WIRING}: box.{SCAN} waits on {scan['pending']!r}, not an issue")
    packs = wiring.get("packs", {})
    phxd_state = {name for name, entry in packs.items() if entry.get("state") == "phxd"}
    unrun = sorted(phxd_state - set(box["packs"]))
    if unrun:
        raise Refusal(f"{WIRING} marks {unrun} phxd, and box.packs does not run them")
    return box


def named_issues(box: dict) -> list[str]:
    """Every issue the box section names, once each, in number order (SPEC-054 R4)."""
    named = set()
    for entry in box["packs"].values():
        named.update(entry.get("expected_red", {}).values())
        if "pending" in entry:
            named.add(entry["pending"])
    if "pending" in box.get(SCAN, {}):
        named.add(box[SCAN]["pending"])
    return sorted(named, key=lambda issue: int(issue[1:]))


def issue_states(root: Path, issues: list[str]) -> dict[str, str]:
    """Each issue's state, OPEN or CLOSED, as `gh issue view <n> --json state` answers in ROOT.
    An issue gh cannot answer for makes the run VOID: a refusal (exit 2) naming why, because an
    expectation whose issue may be closed cannot be judged (SPEC-054 R4)."""
    if not issues:
        return {}
    gh = shutil.which("gh")
    if gh is None:
        raise Refusal(
            f"VOID: gh is not on the path, so the state of {', '.join(issues)} cannot be read"
        )
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
                f"VOID: gh is not logged in (exit {GH_NOT_LOGGED_IN}), so the state of {issue} "
                "cannot be read"
            )
        if done.returncode != 0:
            raise Refusal(
                f"VOID: gh could not read the state of {issue} (exit {done.returncode}): "
                f"{tail(done.stderr)}"
            )
        try:
            state = json.loads(done.stdout).get("state")
        except (json.JSONDecodeError, AttributeError):
            state = None
        if state not in STATES:
            raise Refusal(f"VOID: gh answered no state for {issue}: {tail(done.stdout)}")
        states[issue] = state
    return states


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


def strip(tree: Path, vendored: dict) -> None:
    """Remove the vendored rule code before any pack reads the tree (R11)."""
    shutil.rmtree(tree / ".packs", ignore_errors=True)
    for entry in vendored.get("files", []):
        path = (tree / str(entry.get("path", ""))).resolve()
        if path.is_relative_to(tree) and path.is_file():
            path.unlink()


def tail(text: str) -> str:
    lines = [line for line in text.strip().splitlines() if line.strip()]
    return lines[-1][:200] if lines else "(nothing printed)"


class Phxd:
    """phxd, called with the scratch directory as its working directory and no inherited ledger."""

    def __init__(self, exe: Path, scratch: Path, tree: Path, skills: Path) -> None:
        self.exe, self.scratch, self.tree, self.skills = exe, scratch, tree, skills
        self.env = {key: value for key, value in os.environ.items() if key != "PHX_LEDGER"}
        self.env["PYTHONDONTWRITEBYTECODE"] = "1"
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
        """Each catalogued pack's schemas, as `phxd pack list` reports them (R10)."""
        done = self.call("pack", "list", "--skills-root", self.skills, "--format", "json")
        try:
            packs = json.loads(done.stdout)["packs"]
            return {
                str(entry["id"]).removeprefix("packs/"): list(entry["requires_phxd_schema"])
                for entry in packs
            }
        except (json.JSONDecodeError, KeyError, TypeError) as error:
            raise Refusal(
                f"phxd pack list answered no list (exit {done.returncode}): {tail(done.stderr)}"
            ) from error

    def project_id(self) -> tuple[Path, str]:
        """A scratch ledger with the judged tree registered as a project, made once per run."""
        if self.ledger is None or self.project is None:
            ledger = self.scratch / "ledger" / "phoenix.db"
            ledger.parent.mkdir()
            made = self.call("--ledger", ledger, "init")
            if made.returncode != 0:
                raise Refusal(f"phxd init refused the scratch ledger: {tail(made.stderr)}")
            done = self.call(
                "--ledger", ledger, "project", "register",
                "--slug", "deckstreak", "--name", "DeckStreak", "--repo", self.tree,
            )
            found = REGISTERED.search(done.stdout)
            if done.returncode != 0 or found is None:
                raise Refusal(f"phxd project register failed: {tail(done.stderr or done.stdout)}")
            self.ledger, self.project = ledger, found.group(1)
        return self.ledger, self.project


def judge_pack(
    driver: Phxd, pack: str, expectation: dict, catalog: dict, out: Path, closed: set[str]
) -> Verdict:
    stale = closed_expectations(expectation, closed)
    declared = catalog.get(pack)
    if declared is None:
        detail = "phxd pack list names no such pack"
        return Verdict(pack, "?", "FAIL", stale=stale, detail=detail)
    verbs = sorted({VERBS[schema] for schema in declared if schema in VERBS})
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
        )
    else:
        ledger, project = driver.project_id()
        done = driver.call(
            "--ledger", ledger, "pack", "run", "--pack", pack, "--project", project,
            "--root", tree, "--skills-root", driver.skills, "--format", "json",
        )
    (out / f"{pack}.json").write_text(done.stdout, encoding="utf-8")
    (out / f"{pack}.err").write_text(done.stderr, encoding="utf-8")
    card = read_card(verb, done.stdout)
    if card is None:
        why = f"phxd gave no card (exit {done.returncode}): {tail(done.stderr)}"
        return Verdict(pack, verb, "FAIL", stale=stale, detail=why)
    return judged(pack, verb, expectation, card, "the walk examined no row", closed)


def read_card(verb: str, stdout: str) -> Card | None:
    """The card, read by the schema its verb stamps; anything else is no card."""
    try:
        card = json.loads(stdout)
    except json.JSONDecodeError:
        return None
    if not isinstance(card, dict):
        return None
    schema = card.get("schema")
    if verb == "probe" and schema == "phxd.pack.probe.v1":
        rows = {row["id"]: state(row.get("color"), row) for row in card.get("checks", [])}
        return Card(int(card.get("examined", len(rows))), rows)
    if verb == "run" and schema == "phxd.pack.run.v1":
        rows = {row["id"]: state(row.get("verdict"), row) for row in card.get("checks", [])}
        return Card(int(card.get("examined", 0)), rows)
    if verb == "verify" and schema == "phxd.seo-pipeline.v1":
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


def judge_scan(
    expectation: dict, phoenix: Path, tree: Path, scratch: Path, out: Path, closed: set[str]
) -> Verdict:
    """R13: the proxy scan, read by its row lines, because `check all` exits VOID over RED. A
    pending issue that is closed fails it (SPEC-054 R4)."""
    verdict = Verdict(SCAN, "scan", stale=closed_expectations(expectation, closed))
    pending = expectation.get("pending")
    script = phoenix / "scripts" / "proxy-client-scan.py"
    if not script.is_file():
        verdict.mark, verdict.detail = "FAIL", "the phoenix-v2 checkout has no proxy-client-scan.py"
        return verdict
    done = subprocess.run(
        [sys.executable, str(script), "--root", str(tree), "check", "all"],
        cwd=scratch,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
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


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
PY
