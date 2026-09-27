"""The box-pack runner judges the committed tree with each pack's own verb, and names every red row
it does not expect (SPEC-030 R10 to R14, ADR-030).

`scripts/box-packs.sh` is driven here with a fake phxd and a fake phoenix checkout
(`scripts/tests/fixtures/box-packs/`), so these tests need neither phxd nor phoenix-v2 and run in CI.
The fake records every call, and the census of every tree it was asked to judge, before the runner
removes that tree. The real run stays on the maintainer's box (ADR-004).
"""

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

RUNNER = REPO / "scripts" / "box-packs.sh"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "box-packs"
FAKE_PHXD = FIXTURES / "fake-phxd"
# Temporary repositories are isolated from the machine's git configuration and hooks.
GIT = [
    "git",
    "-c",
    "user.name=box-test",
    "-c",
    "user.email=box-test@example.invalid",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "init.defaultBranch=main",
]
# The synthetic DeckStreak tree: its own files, and the rule code it vendors.
OWN = {
    "README.md": "A synthetic DeckStreak tree for the box-pack runner's tests.\n",
    "crates/sample/src/lib.rs": "//! A sample crate.\n",
    "web/app/src/app.html": "<!doctype html>\n",
}
VENDORED = {
    ".packs/scripts/rule-probe.py": "# a vendored rule: never read as DeckStreak's code\n",
    ".packs/skills/packs/alpha/checks.json": "{}\n",
    "scripts/methodology_probe.py": "# a vendored methodology probe\n",
}
# What each pack expects, when a test says nothing else: no red row, the site not built yet, and
# the proxy scan waiting for its settings document.
QUIET = {
    "packs": {"alpha": {}, "beta": {}, "seo-pipeline": {"pending": "#59"}},
    "proxy-client-scan": {"pending": "#29"},
}
SCAN_ROWS = {
    "void-settings": (
        "VOID settings-no-oauth-token: 0 settings document(s) examined; "
        "no settings document found"
    ),
    "void-launch": "VOID launch-capped: 0 launch(es) examined; no headless launch found",
    "green-surface": "GREEN credential-not-on-argv: 3 surface file(s) examined, 0 finding(s)",
    "green-settings": (
        "GREEN settings-no-oauth-token: 1 settings document(s) examined, 0 finding(s)"
    ),
    "red-surface": "RED credential-not-on-disk: 3 surface file(s) examined, 1 finding(s)",
}


def scan_script(rows, exit_code):
    """A stand-in for phoenix-v2's proxy-client-scan.py: its lines, its summary and its exit."""
    words = [SCAN_ROWS[row].split()[0] for row in rows]
    summary = (
        f"PROXY-CLIENT ALL X: blocking {words.count('GREEN')} green, {words.count('RED')} red, "
        f"{words.count('VOID')} void; advisory 0 green, 0 advisory, 0 void"
    )
    lines = [
        f"print({('PROXY-CLIENT ' + SCAN_ROWS[row])!r}, file=sys.stderr)"
        if SCAN_ROWS[row].startswith("VOID")
        else f"print({('PROXY-CLIENT ' + SCAN_ROWS[row])!r})"
        for row in rows
    ]
    return "\n".join(
        ["import sys", *lines, f"print({summary!r})", f"sys.exit({exit_code})"]
    )


def git(*args):
    return subprocess.run(
        [*GIT, *args], capture_output=True, text=True, check=True
    ).stdout.strip()


def commit(root, message):
    git("-C", str(root), "add", "-A")
    git("-C", str(root), "commit", "-q", "-m", message)
    return git("-C", str(root), "rev-parse", "HEAD")


def write(root, files):
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


class Box:
    """A fake phoenix checkout and a synthetic DeckStreak repository pinned to it."""

    def __init__(self, case):
        scratch = tempfile.TemporaryDirectory()
        case.addCleanup(scratch.cleanup)
        self.tmp = Path(scratch.name).resolve()
        self.phoenix = self.tmp / "phoenix"
        self.repo = self.tmp / "deckstreak"
        self.log = self.tmp / "phxd.log"
        self.cards = self.tmp / "cards"
        self.temp_root = self.tmp / "temp"
        self.temp_root.mkdir()
        shutil.copytree(FIXTURES / "phoenix", self.phoenix)
        git("init", "-q", str(self.phoenix))
        self.repo.mkdir()
        git("init", "-q", str(self.repo))
        write(self.repo, {**OWN, **VENDORED})
        self.box = json.loads(json.dumps(QUIET))
        self.set_scan(["void-settings", "void-launch", "green-surface"], 2)

    def set_scan(self, rows, exit_code):
        """Plant the proxy scan's output, and re-pin the synthetic tree to the new checkout."""
        write(
            self.phoenix, {"scripts/proxy-client-scan.py": scan_script(rows, exit_code)}
        )
        self.repin()

    def swap_verbs(self, first, second):
        """Swap two packs' catalog schemas: the verbs must follow the catalog, never the names."""
        path = self.phoenix / "skills" / "catalog.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        rows = {skill["id"]: skill for skill in data["skills"]}
        one, two = rows[f"packs/{first}"], rows[f"packs/{second}"]
        one["requires_phxd_schema"], two["requires_phxd_schema"] = (
            two["requires_phxd_schema"],
            one["requires_phxd_schema"],
        )
        path.write_text(json.dumps(data, indent=2), encoding="utf-8")
        self.repin()

    def repin(self):
        sha = commit(self.phoenix, "the fake phoenix checkout")
        files = [
            {"path": name, "from": name.removeprefix(".packs/")} for name in VENDORED
        ]
        vendored = {
            "schema": "deckstreak.vendored-packs.v1",
            "vendored_from": sha,
            "files": files,
        }
        write(self.repo, {".packs/VENDORED.json": json.dumps(vendored, indent=2)})
        self.set_box(self.box)

    def set_box(self, box):
        """Commit the wiring's box section as the synthetic tree's."""
        self.box = box
        wiring = {
            "schema": "deckstreak.pack-wiring.v1",
            "packs": {"alpha": {"state": "phxd"}},
            "box": box,
        }
        write(self.repo, {".packs/wiring.json": json.dumps(wiring, indent=2)})
        commit(self.repo, "the synthetic DeckStreak tree")

    def run(self, *args, red=()):
        """Run the real runner with the fake phxd; (the finished process, the calls it made)."""
        self.log.write_text("", encoding="utf-8")
        env = dict(
            os.environ,
            PHOENIX=str(self.phoenix),
            PHXD=str(FAKE_PHXD),
            FAKE_PHXD_LOG=str(self.log),
            FAKE_PHXD_RED=",".join(red),
            BOX_PACKS_OUT=str(self.cards),
            TMPDIR=str(self.temp_root),
            PYTHONDONTWRITEBYTECODE="1",
        )
        env.pop("PHX_LEDGER", None)
        done = subprocess.run(
            ["bash", str(RUNNER), *args, str(self.repo)],
            env=env,
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )
        calls = [json.loads(line) for line in self.log.read_text().splitlines() if line]
        return done, calls


def calls_of(calls, verb):
    """The argv of every call whose verb (after an optional --ledger) is `verb`."""
    found = []
    for call in calls:
        argv = call["argv"]
        words = argv[2:] if argv[:1] == ["--ledger"] else argv
        if words[: len(verb)] == verb:
            found.append(call)
    return found


def option(argv, name):
    return argv[argv.index(name) + 1] if name in argv else None


def pack_line(stdout, pack):
    """The runner's one line for a pack: its mark, its name, its verb, then its counts."""
    lines = [line for line in stdout.splitlines() if line.split()[1:2] == [pack]]
    return lines[0] if len(lines) == 1 else f"<{len(lines)} lines for {pack}>\n{stdout}"


class TheBoxRunnerJudgesHonestly(unittest.TestCase):
    def test_each_pack_runs_with_the_verb_its_catalog_admits(self):
        box = Box(self)
        done, calls = box.run()
        skills = str(box.phoenix / "skills")
        listing = calls_of(calls, ["pack", "list"])
        self.assertEqual(len(listing), 1, f"phxd pack list was called once: {calls}")
        self.assertEqual(option(listing[0]["argv"], "--skills-root"), skills)
        probes = calls_of(calls, ["pack", "probe"])
        self.assertEqual(
            [option(c["argv"], "--pack") for c in probes], ["alpha"], calls
        )
        self.assertEqual(option(probes[0]["argv"], "--skills-root"), skills)
        runs = calls_of(calls, ["pack", "run"])
        self.assertEqual([option(c["argv"], "--pack") for c in runs], ["beta"], calls)
        run = runs[0]["argv"]
        self.assertEqual(run[0], "--ledger", run)
        ledger = Path(run[1])
        self.assertEqual(option(run, "--project"), "1")
        self.assertEqual(option(run, "--skills-root"), skills)
        self.assertEqual(
            [c["argv"] for c in calls_of(calls, ["init"])],
            [["--ledger", run[1], "init"]],
        )
        register = calls_of(calls, ["project", "register"])
        self.assertEqual(len(register), 1, calls)
        self.assertEqual(register[0]["argv"][:2], ["--ledger", run[1]])
        # The scratch ledger sits in a temporary directory outside both repositories, and the run
        # removes it when it ends.
        self.assertIn(box.temp_root, ledger.parents)
        self.assertNotIn(box.repo, ledger.parents)
        self.assertNotIn(box.phoenix, ledger.parents)
        self.assertFalse(ledger.exists(), f"{ledger} outlived the run")
        self.assertEqual(
            list(box.temp_root.iterdir()), [], "the scratch directory outlived the run"
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # The verbs follow the catalog: swapped there, they swap here.
        box.swap_verbs("alpha", "beta")
        done, calls = box.run()
        self.assertEqual(
            [option(c["argv"], "--pack") for c in calls_of(calls, ["pack", "probe"])],
            ["beta"],
        )
        self.assertEqual(
            [option(c["argv"], "--pack") for c in calls_of(calls, ["pack", "run"])],
            ["alpha"],
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # The seo-pipeline pack is pending while the tree holds no built site, and verified once
        # it holds one.
        self.assertEqual(calls_of(calls, ["verify", "seo-pipeline"]), [])
        self.assertIn("pending #59", pack_line(done.stdout, "seo-pipeline"))
        write(box.repo, {"web/site/dist/index.html": "<!doctype html>\n"})
        box.set_box(dict(box.box, packs=dict(box.box["packs"], **{"seo-pipeline": {}})))
        done, calls = box.run()
        verified = calls_of(calls, ["verify", "seo-pipeline"])
        self.assertEqual(len(verified), 1, calls)
        self.assertTrue(
            option(verified[0]["argv"], "--subject").endswith("/web/site/dist")
        )
        self.assertEqual(list(verified[0]["census"]), ["index.html"])
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    def test_the_judged_tree_holds_no_vendored_rule_code(self):
        box = Box(self)
        # Neither an uncommitted change nor an untracked file is part of the committed tree.
        write(
            box.repo,
            {
                "README.md": "changed, never committed\n",
                "crates/sample/src/draft.rs": "",
            },
        )
        done, calls = box.run()
        judged = calls_of(calls, ["pack", "probe"]) + calls_of(calls, ["pack", "run"])
        for call in examined("judged trees", judged):
            census = call["census"]
            for name in census:
                self.assertFalse(
                    name.startswith(".packs/"), f"{name} is vendored rule code"
                )
            for name in VENDORED:
                self.assertNotIn(name, census, f"{name} is vendored rule code")
            for name, text in OWN.items():
                self.assertEqual(census.get(name), digest(text), f"{name} as committed")
            self.assertNotIn("crates/sample/src/draft.rs", census)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # --rev judges the tree of the commit it names.
        commit_later = {"docs/later.md": "a later commit\n"}
        write(box.repo, commit_later)
        commit(box.repo, "a later commit")
        _, calls = box.run()
        self.assertIn("docs/later.md", calls_of(calls, ["pack", "probe"])[0]["census"])
        _, calls = box.run("--rev", "HEAD~1")
        earlier = calls_of(calls, ["pack", "probe"])[0]["census"]
        self.assertIn("crates/sample/src/lib.rs", earlier)
        self.assertNotIn("docs/later.md", earlier)

    def test_an_unexpected_red_row_fails_the_run_by_name(self):
        box = Box(self)
        done, _ = box.run(red=["alpha.second"])
        line = pack_line(done.stdout, "alpha")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("unexpected 1", line)
        self.assertIn("alpha.second", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("BOX PACKS FAILED", done.stdout)
        # Named with the issue that builds its subject, the same red is expected.
        expected = {"expected_red": {"alpha.second": "#23"}}
        box.set_box(dict(box.box, packs=dict(box.box["packs"], alpha=expected)))
        done, _ = box.run(red=["alpha.second"])
        self.assertIn("expected 1", pack_line(done.stdout, "alpha"))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # An advisory row never fails the run.
        done, _ = box.run(red=["alpha.second", "alpha.heuristic", "beta.heuristic"])
        self.assertTrue(pack_line(done.stdout, "beta").startswith("ok"), done.stdout)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    def test_an_expected_red_row_that_turns_green_is_refused_as_stale(self):
        box = Box(self)
        expected = {"expected_red": {"beta.first": "#23"}}
        box.set_box(dict(box.box, packs=dict(box.box["packs"], beta=expected)))
        done, _ = box.run()
        line = pack_line(done.stdout, "beta")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("stale 1", line)
        self.assertIn("beta.first (#23)", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        # While it is still red, it is expected.
        done, _ = box.run(red=["beta.first"])
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # A pack the wiring holds pending is stale once it examines a row.
        box.set_box(
            dict(box.box, packs=dict(box.box["packs"], alpha={"pending": "#23"}))
        )
        done, _ = box.run()
        line = pack_line(done.stdout, "alpha")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("stale 1", line)
        self.assertIn("pending #23", line)
        self.assertEqual(done.returncode, 1, done.stdout)

    def test_the_proxy_scan_without_a_settings_document_reads_pending(self):
        box = Box(self)
        done, _ = box.run()
        line = pack_line(done.stdout, "proxy-client-scan")
        self.assertTrue(line.startswith("pending"), line)
        self.assertIn("pending #29", line)
        self.assertIn("0 settings document(s)", line)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # With no pending issue, the same VOID scan fails the run: VOID is never green.
        box.set_box(dict(box.box, **{"proxy-client-scan": {}}))
        done, _ = box.run()
        line = pack_line(done.stdout, "proxy-client-scan")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("VOID", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        # A red row fails the run, though the scan's own exit says VOID over RED.
        box.set_box(dict(box.box, **{"proxy-client-scan": {"pending": "#29"}}))
        box.set_scan(["void-settings", "green-surface", "red-surface"], 2)
        done, _ = box.run()
        line = pack_line(done.stdout, "proxy-client-scan")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("credential-not-on-disk", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        # Once a settings document is examined, the pending issue is stale.
        box.set_scan(["green-settings", "green-surface"], 0)
        done, _ = box.run()
        line = pack_line(done.stdout, "proxy-client-scan")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("stale", line)
        self.assertEqual(done.returncode, 1, done.stdout)


if __name__ == "__main__":
    unittest.main()
