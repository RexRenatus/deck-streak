"""The box driver judges the committed tree with each pack's own verb, and names every red row it
does not expect (SPEC-030 R10 to R14, ADR-030). It reads the state of every issue the wiring names,
fails an expectation whose issue is closed, and reads VOID when it cannot (SPEC-054 R4). It reads
its wiring and pin from a private file, keeps the removed row runner's judgment for the packs
section, runs the methodology probes and the proxy-client scan from the checkout, checks DeckStreak's
owned data for drift, and posts one verdict-only status (SPEC-056 A11 to A15, ADR-069). It runs the
apiKeyHelper scan from the checkout with the removed gate step's refusals (SPEC-056 A17).

`scripts/box-packs.sh` is driven here with a fake runner, a synthetic checkout, a private file in a
temporary directory and a fake gh (`scripts/tests/fixtures/box-packs/`), so these tests need neither
the maintainer's runner, its checkout nor GitHub, and run in CI. The fakes record every call, and
the fake runner the census of every tree it was asked to judge, before the driver removes that tree.
The real run stays on the maintainer's box (ADR-069).
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

DRIVER = REPO / "scripts" / "box-packs.sh"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "box-packs"
FAKE_RUNNER = FIXTURES / "fake-runner"
# The directory holding the fake `gh`, put first on every run's path.
FAKE_GH = FIXTURES / "bin"
# The tools the driver itself calls by name, so a path of these alone has everything but gh.
DRIVER_TOOLS = ("bash", "python3", "git", "tar")
# Temporary repositories are isolated from the git configuration and hooks around them.
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
# The synthetic DeckStreak tree.
OWN = {
    "README.md": "A synthetic DeckStreak tree for the box driver's tests.\n",
    "crates/sample/src/lib.rs": "//! A sample crate.\n",
    "web/app/src/app.html": "<!doctype html>\n",
}
# The checkout's methodology probes and its two scans, by the paths the private file names.
SCRIPTS = {
    "sdd": "scripts/spec-probe.py",
    "ddd": "scripts/domain-probe.py",
    "tdd": "scripts/test-probe.py",
    "proxy-client-scan": "scripts/client-scan.py",
    "no-apikeyhelper": "scripts/helper-scan.py",
}
# Each methodology probe's verdict prefix and the classes its stand-in reports.
PROBES = {
    "sdd": ("SDD", ("spec-sections", "adr-alternatives")),
    "ddd": ("DDD", ("context-map-parses",)),
    "tdd": ("TDD", ("acceptance-has-a-test", "red-first-recorded")),
}
# What each box pack expects, when a test says nothing else: no red row, the site not built yet,
# and both scans waiting for a settings document.
QUIET = {
    "packs": {"alpha": {}, "beta": {}, "seo-pipeline": {"pending": "#59"}},
    "proxy-client-scan": {"pending": "#29"},
    "no-apikeyhelper": {"pending": "#29"},
}
SCAN_ROWS = {
    "void-settings": (
        "VOID settings-no-oauth-token: 0 settings document(s) examined; no settings document found"
    ),
    "void-launch": "VOID launch-capped: 0 launch(es) examined; no headless launch found",
    "green-surface": "GREEN credential-not-on-argv: 3 surface file(s) examined, 0 finding(s)",
    "green-settings": (
        "GREEN settings-no-oauth-token: 1 settings document(s) examined, 0 finding(s)"
    ),
    "red-surface": "RED credential-not-on-disk: 3 surface file(s) examined, 1 finding(s)",
}
# A stand-in script records its argv and the census of its --root in FAKE_PROBE_LOG.
RECORDER = """import hashlib, json, os, sys
from pathlib import Path
argv = sys.argv[1:]
root = Path(argv[argv.index("--root") + 1]) if "--root" in argv else None
census = {} if root is None else {
    str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
    for p in sorted(root.rglob("*")) if p.is_file() and ".git" not in p.relative_to(root).parts
}
with open(os.environ["FAKE_PROBE_LOG"], "a", encoding="utf-8") as handle:
    handle.write(json.dumps({"script": sys.argv[0], "argv": argv, "cwd": os.getcwd(),
                             "census": census}) + "\\n")
"""


# The apiKeyHelper scan's stand-in, and the one verdict line and exit the scan gives for each thing
# it can find: no settings file, clean files, a finding, or a file it cannot read (SPEC-056 R9).
HELPER_SCRIPT = SCRIPTS["no-apikeyhelper"]
HELPER_VERDICTS = {
    "none": (2, "VOID 0 settings file(s) under <root>: a scan that examined nothing cannot say"),
    "clean": (0, "GREEN 2 settings file(s) scanned, none carries the shape"),
    "found": (1, "RED 1 finding(s) in 2 file(s):"),
    "unreadable": (2, "VOID <root>/agent/settings.json: not valid JSON: Expecting value"),
}


def helper_script(found):
    """A stand-in for the apiKeyHelper scan: its verdict line (VOID on stderr) and its exit."""
    code, line = HELPER_VERDICTS[found]
    stream = ", file=sys.stderr" if line.startswith("VOID") else ""
    printed = f"print({('NO-APIKEYHELPER ' + line)!r}{stream})"
    return "\n".join([RECORDER, printed, f"sys.exit({code})"]) + "\n"


def scan_script(rows, exit_code):
    """A stand-in for the proxy-client scan: its lines, its summary and its exit."""
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
    return "\n".join([RECORDER, *lines, f"print({summary!r})", f"sys.exit({exit_code})"])


def probe_script(prefix, classes, refused=(), void=()):
    """A stand-in methodology probe: a finding line and a verdict line per class, and its exit."""
    lines, code = [RECORDER], 0
    for name in classes:
        if name in refused:
            lines.append(f"print({f'{name}: a planted finding'!r})")
            verdict = f"{prefix} {name} REFUSED: examined 2 item(s), 1 finding(s)"
            lines.append(f"print({verdict!r})")
            code = max(code, 1)
        elif name in void:
            lines.append(f"print({f'{prefix} {name} VOID: examined 0 item(s)'!r})")
            code = max(code, 3)
        else:
            lines.append(f"print({f'{prefix} {name} OK: examined 2 item(s)'!r})")
    lines.append(f"sys.exit({code})")
    return "\n".join(lines) + "\n"


def git(*args):
    return subprocess.run([*GIT, *args], capture_output=True, text=True, check=True).stdout.strip()


def commit(root, message):
    git("-C", str(root), "add", "-A")
    git("-C", str(root), "commit", "-q", "--allow-empty", "-m", message)
    return git("-C", str(root), "rev-parse", "HEAD")


def write(root, files):
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


class Box:
    """A synthetic checkout, a synthetic DeckStreak repository, and the private file that pins the
    checkout to judge the repository."""

    def __init__(self, case):
        scratch = tempfile.TemporaryDirectory()
        case.addCleanup(scratch.cleanup)
        self.tmp = Path(scratch.name).resolve()
        self.checkout = self.tmp / "checkout"
        self.repo = self.tmp / "deckstreak"
        self.private = self.tmp / "private"
        self.private.mkdir()
        self.wiring_file = self.private / "box-wiring.json"
        self.log = self.tmp / "runner.log"
        self.gh_log = self.tmp / "gh.log"
        self.probe_log = self.tmp / "probes.log"
        self.cards = self.tmp / "cards"
        self.temp_root = self.tmp / "temp"
        self.temp_root.mkdir()
        shutil.copytree(FIXTURES / "checkout", self.checkout)
        git("init", "-q", str(self.checkout))
        self.repo.mkdir()
        git("init", "-q", str(self.repo))
        write(self.repo, OWN)
        commit(self.repo, "the synthetic DeckStreak tree")
        self.wiring = {
            "schema": "deckstreak.box-wiring.v1",
            "pin": "",
            "skills": "skills",
            "scripts": dict(SCRIPTS),
            "packs": {},
            "box": json.loads(json.dumps(QUIET)),
            "owned": {},
        }
        self.set_probes()
        self.set_helper("none")
        self.set_scan(["void-settings", "void-launch", "green-surface"], 2)

    @property
    def box(self):
        return self.wiring["box"]

    def save(self):
        self.wiring_file.write_text(json.dumps(self.wiring, indent=2), encoding="utf-8")

    def repin(self):
        """Commit the checkout, and pin the private file to its new head."""
        self.wiring["pin"] = commit(self.checkout, "the synthetic checkout")
        self.save()

    def set_scan(self, rows, exit_code):
        """Plant the proxy scan's output in the checkout, and re-pin to it."""
        write(self.checkout, {SCRIPTS["proxy-client-scan"]: scan_script(rows, exit_code)})
        self.repin()

    def set_helper(self, found):
        """Plant the apiKeyHelper scan's verdict in the checkout, and re-pin to it."""
        write(self.checkout, {HELPER_SCRIPT: helper_script(found)})
        self.repin()

    def set_probes(self, refused=None, void=None):
        """Plant the three methodology probes: every class OK but the ones named, and re-pin."""
        refused, void = refused or {}, void or {}
        for name, (prefix, classes) in PROBES.items():
            text = probe_script(prefix, classes, refused.get(name, ()), void.get(name, ()))
            write(self.checkout, {SCRIPTS[name]: text})
        self.repin()

    def set_box(self, box):
        self.wiring["box"] = box
        self.save()

    def set_packs(self, packs):
        self.wiring["packs"] = packs
        self.save()

    def set_owned(self, owned):
        self.wiring["owned"] = owned
        self.save()

    def swap_verbs(self, first, second):
        """Swap two packs' catalog schemas: the verbs must follow the catalog, never the names."""
        path = self.checkout / "skills" / "catalog.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        rows = {skill["id"]: skill for skill in data["skills"]}
        one, two = rows[f"packs/{first}"], rows[f"packs/{second}"]
        one["requires_schema"], two["requires_schema"] = (
            two["requires_schema"],
            one["requires_schema"],
        )
        path.write_text(json.dumps(data, indent=2), encoding="utf-8")
        self.repin()

    def run(self, *args, red=(), void=(), error=(), closed=(), gh="fake", unset=()):
        """Run the real driver with the fake runner; (the finished process, the runner's calls).

        The fake gh answers CLOSED for each issue `closed` names and OPEN for any other. `gh` set
        to `auth`, `offline` or `garbled` makes it fail the way gh does, and None runs with no gh on
        the path at all. `unset` names the driver's variables to leave out."""
        for log in (self.log, self.gh_log, self.probe_log):
            log.write_text("", encoding="utf-8")
        env = dict(
            os.environ,
            PACKS_WIRING=str(self.wiring_file),
            PACKS_CHECKOUT=str(self.checkout),
            PACKS_RUNNER=str(FAKE_RUNNER),
            FAKE_RUNNER_LOG=str(self.log),
            FAKE_RUNNER_RED=",".join(red),
            FAKE_RUNNER_VOID=",".join(void),
            FAKE_RUNNER_ERROR=",".join(error),
            FAKE_GH_LOG=str(self.gh_log),
            FAKE_GH_CLOSED=",".join(issue.removeprefix("#") for issue in closed),
            FAKE_PROBE_LOG=str(self.probe_log),
            BOX_PACKS_OUT=str(self.cards),
            TMPDIR=str(self.temp_root),
            PYTHONDONTWRITEBYTECODE="1",
        )
        for inherited in ("FAKE_GH_FAIL", "GH_REPO", *unset):
            env.pop(inherited, None)
        if gh is None:
            env["PATH"] = str(self.tools_without_gh())
        else:
            env["PATH"] = os.pathsep.join([str(FAKE_GH), os.environ.get("PATH", "")])
            if gh != "fake":
                env["FAKE_GH_FAIL"] = gh
        done = subprocess.run(
            ["bash", str(DRIVER), *args, str(self.repo)],
            env=env,
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )
        calls = [json.loads(line) for line in self.log.read_text().splitlines() if line]
        return done, calls

    def gh_calls(self):
        """Every call the fake gh answered in the last run: its argv and working directory."""
        return [json.loads(line) for line in self.gh_log.read_text().splitlines() if line]

    def probe_calls(self):
        """Every call a stand-in probe or scan recorded in the last run."""
        return [json.loads(line) for line in self.probe_log.read_text().splitlines() if line]

    def tools_without_gh(self):
        """A directory holding the driver's own tools and no gh, to stand for a path without it."""
        tools = self.tmp / "tools-without-gh"
        if not tools.is_dir():
            tools.mkdir()
            for name in DRIVER_TOOLS:
                found = shutil.which(name)
                if found is None:
                    raise AssertionError(f"no {name} is on the path")
                (tools / name).symlink_to(found)
        return tools


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
    """The driver's one line for a pack, probe or owned file: its mark, its name, then its counts."""
    lines = [line for line in stdout.splitlines() if line.split()[1:2] == [pack]]
    return lines[0] if len(lines) == 1 else f"<{len(lines)} lines for {pack}>\n{stdout}"


def void_lines(stdout):
    return [line for line in stdout.splitlines() if line.startswith("box-packs: VOID: ")]


class TheBoxRunnerJudgesHonestly(unittest.TestCase):
    def test_each_pack_runs_with_the_verb_its_catalog_admits(self):
        box = Box(self)
        done, calls = box.run()
        skills = str(box.checkout / "skills")
        listing = calls_of(calls, ["pack", "list"])
        self.assertEqual(len(listing), 1, f"the runner's pack list was called once: {calls}")
        self.assertEqual(option(listing[0]["argv"], "--skills-root"), skills)
        probes = calls_of(calls, ["pack", "probe"])
        self.assertEqual([option(c["argv"], "--pack") for c in probes], ["alpha"], calls)
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
        self.assertNotIn(box.checkout, ledger.parents)
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
        commit(box.repo, "the built site")
        box.set_box(dict(box.box, packs=dict(box.box["packs"], **{"seo-pipeline": {}})))
        done, calls = box.run()
        verified = calls_of(calls, ["verify", "seo-pipeline"])
        self.assertEqual(len(verified), 1, calls)
        self.assertTrue(option(verified[0]["argv"], "--subject").endswith("/web/site/dist"))
        self.assertEqual(list(verified[0]["census"]), ["index.html"])
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    def test_the_judged_tree_holds_no_vendored_rule_code(self):
        box = Box(self)
        # Neither an uncommitted change nor an untracked file is part of the committed tree, and
        # the judged tree is the committed tree alone: nothing is added to it.
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
            self.assertEqual(call["census"], {name: digest(text) for name, text in OWN.items()})
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
        box.set_box(dict(box.box, packs=dict(box.box["packs"], alpha={"pending": "#23"})))
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


class EveryExpectationNamesAnOpenIssue(unittest.TestCase):
    def test_an_expectation_whose_issue_is_closed_fails_the_run_by_name(self):
        box = Box(self)
        expected = {"expected_red": {"alpha.second": "#23"}}
        box.set_box(dict(box.box, packs=dict(box.box["packs"], alpha=expected)))
        # The expected red row's issue is closed: the pack fails, naming the row and the issue.
        done, _ = box.run(red=["alpha.second"], closed=["#23"])
        line = pack_line(done.stdout, "alpha")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("alpha.second (#23 is closed)", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("BOX PACKS FAILED", done.stdout)
        # A pending pack's issue closed, and the proxy scan's, fail the same way.
        for pack, issue in (("seo-pipeline", "#59"), ("proxy-client-scan", "#29")):
            with self.subTest(pack=pack):
                done, _ = box.run(red=["alpha.second"], closed=[issue])
                line = pack_line(done.stdout, pack)
                self.assertTrue(line.startswith("FAIL"), line)
                self.assertIn(f"pending ({issue} is closed)", line)
                self.assertEqual(done.returncode, 1, done.stdout)
        # Every issue open, the same expectations pass, and each issue was asked once, in ROOT.
        done, _ = box.run(red=["alpha.second"])
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        asked = examined("issue state(s) read", box.gh_calls())
        self.assertEqual(sorted(call["argv"][2] for call in asked), ["23", "29", "59"])
        for call in asked:
            self.assertEqual(call["argv"], ["issue", "view", call["argv"][2], "--json", "state"])
            self.assertEqual(Path(call["cwd"]), box.repo)
        self.assertIn(
            "box-packs: 3 issue(s) the wiring names: #23 OPEN, #29 OPEN, #59 OPEN", done.stdout
        )

    def test_a_run_that_cannot_read_issue_state_is_void_with_the_reason(self):
        box = Box(self)
        reasons = {
            None: ("gh is not on the path",),
            "auth": ("gh is not logged in (exit 4)",),
            "offline": ("gh could not read the state of #", "(exit 1)", "connection refused"),
            "garbled": ("gh answered no state for #",),
        }
        for gh, words in reasons.items():
            with self.subTest(gh=gh):
                done, calls = box.run(gh=gh)
                self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
                void = void_lines(done.stdout)
                self.assertEqual(len(void), 1, done.stdout)
                for word in words:
                    self.assertIn(word, void[0])
                self.assertRegex(void[0], r"#(29|59)\b", "the line names the issue")
                self.assertEqual(calls, [], "no pack ran")
                self.assertNotIn("BOX PACKS OK", done.stdout)


class TheDriverReadsItsPrivateFile(unittest.TestCase):
    def test_the_driver_refuses_void_by_name_without_its_private_variables(self):
        box = Box(self)
        for variable in ("PACKS_WIRING", "PACKS_CHECKOUT", "PACKS_RUNNER"):
            with self.subTest(unset=variable):
                done, calls = box.run(unset=[variable])
                self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
                void = void_lines(done.stdout)
                self.assertEqual(len(void), 1, done.stdout)
                self.assertIn(variable, void[0])
                self.assertEqual(calls, [], "no pack ran")
                self.assertEqual(box.probe_calls(), [], "no probe ran")
        # A file that is not the private wiring's schema is refused by name.
        box.wiring["schema"] = "deckstreak.pack-wiring.v1"
        box.save()
        done, calls = box.run()
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn("deckstreak.box-wiring.v1", "".join(void_lines(done.stdout)))
        self.assertEqual(calls, [])
        # A pin that is not the checkout's head refuses the run.
        box.wiring["schema"] = "deckstreak.box-wiring.v1"
        box.wiring["pin"] = "0" * 40
        box.save()
        done, calls = box.run()
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn("pin", "".join(void_lines(done.stdout)))
        self.assertEqual(calls, [])
        # With all three set and the pin at the checkout's head, the run judges.
        box.repin()
        done, calls = box.run()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("BOX PACKS OK", done.stdout)
        self.assertTrue(calls)

    def test_the_packs_section_keeps_the_row_runners_judgment(self):
        box = Box(self)
        box.set_packs(
            {
                "gamma": {
                    "state": "enforced",
                    "excluded_rows": {"gamma.outside": "not DeckStreak's subject"},
                    "deferred_rows": {"gamma.later": "#44"},
                },
                "delta": {"state": "pending", "enforced_by": "#49"},
                "epsilon": {"state": "deferred", "enforced_by": "#61"},
            }
        )
        # An excluded row is never judged, a deferred row still red fails nothing, a pending pack
        # reads its VOID blocking row as pending, and a deferred pack runs no row.
        quiet = {"red": ["gamma.later", "gamma.outside"], "void": ["delta.first"]}
        done, calls = box.run(**quiet)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        gamma = pack_line(done.stdout, "gamma")
        self.assertTrue(gamma.startswith("ok"), gamma)
        self.assertIn("excluded 1", gamma)
        self.assertIn("deferred 1", gamma)
        delta = pack_line(done.stdout, "delta")
        self.assertTrue(delta.startswith("pending"), delta)
        self.assertIn("pending #49", delta)
        epsilon = pack_line(done.stdout, "epsilon")
        self.assertTrue(epsilon.startswith("deferred"), epsilon)
        self.assertIn("#61", epsilon)
        probed = {
            option(call["argv"], "--pack"): call["argv"]
            for call in calls_of(calls, ["pack", "probe"])
        }
        self.assertNotIn("epsilon", probed)
        for pack in examined("packs-section packs probed", ["gamma", "delta"]):
            self.assertEqual(option(probed[pack], "--scope"), "tree", probed[pack])
        # A deferred row that passes is refused as stale.
        done, _ = box.run(red=["gamma.outside"], void=["delta.first"])
        gamma = pack_line(done.stdout, "gamma")
        self.assertTrue(gamma.startswith("FAIL"), gamma)
        self.assertIn("STALE gamma.later", gamma)
        self.assertEqual(done.returncode, 1, done.stdout)
        # A pending pack whose every blocking row passes is refused as stale.
        done, _ = box.run(red=["gamma.later"])
        delta = pack_line(done.stdout, "delta")
        self.assertTrue(delta.startswith("FAIL"), delta)
        self.assertIn("STALE", delta)
        self.assertIn("#49", delta)
        self.assertEqual(done.returncode, 1, done.stdout)
        # An enforced pack fails on a blocking row that is red, VOID or in error, by name.
        for kind in ("red", "void", "error"):
            with self.subTest(kind=kind):
                planted = {key: list(value) for key, value in quiet.items()}
                planted.setdefault(kind, []).append("gamma.first")
                done, _ = box.run(**planted)
                gamma = pack_line(done.stdout, "gamma")
                self.assertTrue(gamma.startswith("FAIL"), gamma)
                self.assertIn("gamma.first", gamma)
                self.assertEqual(done.returncode, 1, done.stdout)
        # An advisory row never fails the pack.
        done, _ = box.run(
            red=["gamma.later", "gamma.outside", "gamma.heuristic"], void=["delta.first"]
        )
        self.assertTrue(pack_line(done.stdout, "gamma").startswith("ok"), done.stdout)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # A wiring that names a row the pack lacks is refused by name.
        box.set_packs({"gamma": {"state": "enforced", "excluded_rows": {"gamma.nowhere": "why"}}})
        done, _ = box.run()
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn("gamma.nowhere", "".join(void_lines(done.stdout)))

    def test_the_methodology_probes_and_the_scan_run_from_the_checkout(self):
        box = Box(self)
        done, _ = box.run()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        for name, (_, classes) in examined("methodology probes", list(PROBES.items())):
            line = pack_line(done.stdout, name)
            self.assertTrue(line.startswith("ok"), line)
            self.assertIn(f"examined {len(classes)}", line)
        recorded = {
            Path(call["script"]).relative_to(box.checkout).as_posix(): call
            for call in box.probe_calls()
        }
        self.assertEqual(sorted(recorded), sorted(SCRIPTS.values()))
        own = {name: digest(text) for name, text in OWN.items()}
        for script, call in recorded.items():
            self.assertEqual(call["census"], own, f"{script} judged the committed tree")
            self.assertNotEqual(Path(call["cwd"]), box.checkout)
            self.assertNotIn(box.checkout, Path(call["cwd"]).parents)
        for name in ("sdd", "ddd", "tdd"):
            self.assertEqual(recorded[SCRIPTS[name]]["argv"][-2:], ["check", "all"])
        # A refused class fails its probe by name, and so does a VOID one.
        box.set_probes(refused={"sdd": ["spec-sections"]}, void={"tdd": ["red-first-recorded"]})
        done, _ = box.run()
        self.assertEqual(done.returncode, 1, done.stdout)
        sdd = pack_line(done.stdout, "sdd")
        self.assertTrue(sdd.startswith("FAIL"), sdd)
        self.assertIn("spec-sections REFUSED", sdd)
        tdd = pack_line(done.stdout, "tdd")
        self.assertTrue(tdd.startswith("FAIL"), tdd)
        self.assertIn("red-first-recorded VOID", tdd)
        self.assertTrue(pack_line(done.stdout, "ddd").startswith("ok"), done.stdout)
        # A probe the private file does not name refuses the run.
        box.set_probes()
        del box.wiring["scripts"]["ddd"]
        box.save()
        done, _ = box.run()
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn("ddd", "".join(void_lines(done.stdout)))


class TheApiKeyHelperScanRunsOnTheBox(unittest.TestCase):
    def test_the_api_key_helper_scan_runs_from_the_checkout_and_waits_for_a_settings_file(self):
        box = Box(self)
        box.wiring["scripts"]["no-apikeyhelper"] = HELPER_SCRIPT
        box.set_box(dict(box.box, **{"no-apikeyhelper": {"pending": "#29"}}))
        box.set_helper("none")
        # No settings file yet: the scan reads pending on the issue the private file names.
        done, _ = box.run()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        line = pack_line(done.stdout, "no-apikeyhelper")
        self.assertTrue(line.startswith("pending"), line)
        self.assertIn("pending #29", line)
        # It ran once, from the checkout, over the committed tree, from outside the checkout.
        script = box.checkout / HELPER_SCRIPT
        calls = [call for call in box.probe_calls() if Path(call["script"]) == script]
        self.assertEqual(len(calls), 1, box.probe_calls())
        self.assertEqual(calls[0]["census"], {name: digest(text) for name, text in OWN.items()})
        self.assertEqual(calls[0]["argv"][:1], ["--root"])
        cwd = Path(calls[0]["cwd"])
        self.assertNotEqual(cwd, box.checkout)
        self.assertNotIn(box.checkout, cwd.parents)
        # A finding fails the run, and so does a settings file the scan cannot read.
        for found in examined("refusing verdicts", ["found", "unreadable"]):
            with self.subTest(found=found):
                box.set_helper(found)
                done, _ = box.run()
                line = pack_line(done.stdout, "no-apikeyhelper")
                self.assertTrue(line.startswith("FAIL"), line)
                self.assertEqual(done.returncode, 1, done.stdout)
        # With no issue named, finding no settings file is VOID, and fails the run.
        box.set_helper("none")
        box.set_box(dict(box.box, **{"no-apikeyhelper": {}}))
        done, _ = box.run()
        line = pack_line(done.stdout, "no-apikeyhelper")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("VOID", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        # Clean settings files pass, and a pending issue that waited for them is stale.
        box.set_helper("clean")
        done, _ = box.run()
        line = pack_line(done.stdout, "no-apikeyhelper")
        self.assertTrue(line.startswith("ok"), line)
        self.assertIn("examined 2", line)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        box.set_box(dict(box.box, **{"no-apikeyhelper": {"pending": "#29"}}))
        done, _ = box.run()
        line = pack_line(done.stdout, "no-apikeyhelper")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("stale: pending #29, but a settings file was examined", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        # The pending issue closed is stale too, by name.
        box.set_helper("none")
        done, _ = box.run(closed=["#29"])
        line = pack_line(done.stdout, "no-apikeyhelper")
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("pending (#29 is closed)", line)
        # The private file must name the scan's script.
        del box.wiring["scripts"]["no-apikeyhelper"]
        box.save()
        done, _ = box.run()
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn("no-apikeyhelper", "".join(void_lines(done.stdout)))


class OwnedDataCannotDrift(unittest.TestCase):
    def test_an_owned_file_is_judged_against_its_pinned_source(self):
        box = Box(self)
        check = {
            "id": "one",
            "severity": "block",
            "probe": {"command": ["x"], "timeout_seconds": 60},
        }
        source = {
            "schema": "sample.rails.v1",
            "fence_allow": ["alpha", "beta"],
            "periodic": {"daily": {"folder": "Daily", "format": "YYYY-MM-DD"}},
            "checks": [check],
            "note": "the source's own words",
        }
        owned = {
            "schema": "sample.rails.v1",
            "fence_allow": ["alpha", "beta"],
            "periodic": {"daily": {"folder": "Daily"}},
            "checks": [{"id": "one", "severity": "block", "probe": {"timeout_seconds": 60}}],
        }
        dropped = ["note", "periodic.daily.format", "checks[].probe.command"]

        def plant(text):
            write(box.checkout, {"skills/packs/sample/rails.json": json.dumps(text)})
            box.repin()

        plant(source)
        write(box.repo, {"data/rails.json": json.dumps(owned)})
        commit(box.repo, "the owned data")
        entry = {"source": "skills/packs/sample/rails.json", "dropped": dropped}
        box.set_owned({"data/rails.json": entry})
        done, _ = box.run()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        line = pack_line(done.stdout, "data/rails.json")
        self.assertTrue(line.startswith("ok"), line)
        # A kept field that differs from its source fails by name, and so does a field the source
        # gained that the owned file neither keeps nor drops.
        slower = dict(check, probe={"command": ["x"], "timeout_seconds": 30})
        for field, changed in (
            ("fence_allow", dict(source, fence_allow=["alpha", "gamma"])),
            ("checks[0].probe.timeout_seconds", dict(source, checks=[slower])),
            ("new field new_rail", dict(source, new_rail=["planted"])),
        ):
            with self.subTest(field=field):
                plant(changed)
                done, _ = box.run()
                line = pack_line(done.stdout, "data/rails.json")
                self.assertTrue(line.startswith("FAIL"), line)
                self.assertIn(field, line)
                self.assertEqual(done.returncode, 1, done.stdout)
        # A source missing from the checkout makes the run VOID, by name.
        plant(source)
        box.set_owned({"data/rails.json": dict(entry, source="skills/packs/sample/gone.json")})
        done, _ = box.run()
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn("skills/packs/sample/gone.json", "".join(void_lines(done.stdout)))


class TheRunPostsOneVerdict(unittest.TestCase):
    def test_post_status_posts_one_verdict_only_status(self):
        box = Box(self)
        sha = git("-C", str(box.repo), "rev-parse", "HEAD")

        def statuses():
            return [call for call in box.gh_calls() if call["argv"][:1] == ["api"]]

        def fields(call):
            argv = call["argv"]
            return dict(argv[i + 1].split("=", 1) for i, word in enumerate(argv) if word == "-f")

        # Off by default: a run posts no status.
        done, _ = box.run()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(statuses(), [])
        for state, planted in (("success", {}), ("failure", {"red": ["alpha.second"]})):
            with self.subTest(state=state):
                done, _ = box.run("--post-status", **planted)
                posted = statuses()
                self.assertEqual(len(posted), 1, done.stdout + done.stderr)
                argv = posted[0]["argv"]
                self.assertEqual(argv[1], f"repos/{{owner}}/{{repo}}/statuses/{sha}")
                self.assertEqual(option(argv, "--method"), "POST")
                self.assertEqual(Path(posted[0]["cwd"]), box.repo)
                posted_fields = fields(posted[0])
                self.assertEqual(posted_fields["context"], "box/packs")
                self.assertEqual(posted_fields["state"], state)
                description = posted_fields["description"]
                self.assertLessEqual(len(description), 60, description)
                for row in ("alpha.second", "alpha", "beta", "proxy-client-scan"):
                    self.assertNotIn(row, description)
        # A run that cannot judge posts `error`, once.
        box.wiring["pin"] = "0" * 40
        box.save()
        done, _ = box.run("--post-status")
        self.assertEqual(done.returncode, 2, done.stdout)
        posted = statuses()
        self.assertEqual(len(posted), 1, done.stdout + done.stderr)
        self.assertEqual(fields(posted[0])["state"], "error")


if __name__ == "__main__":
    unittest.main()
