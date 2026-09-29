"""A tool the runner needs and cannot run is a refusal, never a survivor (SPEC-039 A46 to A48,
ADR-291, issue #431).

THE CLASS. Every process `scripts/mutation_rows.py` spawns, under every verb that reaches it, ends
the verb with ONE line naming the tool and exit 2 (`EXIT_REFUSED`) when its executable cannot be
run: absent from `PATH`, present but not executable, or a directory at the name. Never a traceback,
never exit 1 (`EXIT_SURVIVED`), never a verdict, and the target's bytes are the ones it started
with.

THE POPULATION is generated, not listed. The spawn sites are read from the module's own source
with `ast` (every `subprocess.run`, `subprocess.Popen`, `run_in_own_group` and `run_tool` call
outside the two helpers that own a raw spawn), so a new spawn joins the population by itself and
fails the census until a scenario covers it. Each site carries the tools it can spawn and the
verbs that reach it; each member is one (site, tool, unrunnable mode, verb) built in a temporary
git repository with the suite's own fixture, run as a child process with a `PATH` (or, for the
interpreter, a `sys.executable`) that makes the tool unrunnable.
"""

import ast
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_mutation_rows import (
    DESCRIPTION,
    RUNNER,
    Fixture,
    git,
    script_row,
    sha256,
)

MODULE = REPO / "scripts" / "mutation_rows.py"
#: The functions that own a raw process spawn; every other function reaches one through them.
HELPERS = ("run_tool", "run_in_own_group")
#: What a spawn looks like in the source: the names the population is derived from.
SPAWN_NAMES = ("subprocess.run", "subprocess.Popen", "run_in_own_group", "run_tool")
MODES = ("absent", "not executable", "a directory")
ID = "S00001-DOUBLE"
BAND = "S00000-S00099"
PROVE_VERBS = ("prove-id", "prove-band", "prove-all", "prove-rows-from")
CARGO_ROW = ["S00002-CARGO", "fix", "src/lib.rs", "x * 2", "x * 3", "double::two_doubles_to_four"]
BASH_FILE = "scripts/fixparse.bash"
SH_FILE = "scripts/fixparse.sh"
#: One scenario per (site, tool): the function the source spawns from, the tool that spawn runs,
#: and the verbs that reach it. `git` is read by every verb that reads a revision or the tree's
#: state, so `retired` reaches it as well; the others are reached only through `prove`.
SCENARIOS = (
    ("git", "git", PROVE_VERBS + ("retired",)),
    ("run_killer", "cargo", PROVE_VERBS),
    ("run_killer", "python", PROVE_VERBS),
    ("builds", "cargo", PROVE_VERBS),
    ("parses", "bash", PROVE_VERBS),
    ("parses", "sh", PROVE_VERBS),
)
#: A verb that spawns nothing runs with every tool unrunnable and still succeeds.
QUIET_VERBS = ("count", "ids", "census")
VERDICT_LINE = re.compile(r"^S\d+-[A-Z0-9-]+: (KILLED|SURVIVED|VOID)\b", re.MULTILINE)


def spawn_sites():
    """{enclosing function: number of spawn calls} read from the module's source, the helpers
    that own a raw spawn excluded, and every raw `subprocess` call outside them listed apart."""
    tree = ast.parse(MODULE.read_text(encoding="utf-8"))
    sites, raw_outside = {}, []
    for function in ast.walk(tree):
        if not isinstance(function, ast.FunctionDef):
            continue
        for call in ast.walk(function):
            if not isinstance(call, ast.Call):
                continue
            name = ast.unparse(call.func)
            if name not in SPAWN_NAMES or function.name in HELPERS:
                continue
            sites[function.name] = sites.get(function.name, 0) + 1
            if name.startswith("subprocess."):
                raw_outside.append(function.name)
    return sites, raw_outside


def unrunnable(path, mode):
    """Make `path` an executable that cannot be run, in one of the three ways."""
    if path.is_symlink() or path.is_file():
        path.unlink()
    if mode == "not executable":
        path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        path.chmod(0o644)
    elif mode == "a directory":
        path.mkdir()


CARGO_SHIM = """#!{python}
import os, stat, sys
from pathlib import Path

me = Path(__file__)
if "--no-run" in sys.argv:
    sys.exit(0)
print("running 1 test")
print("test double::two_doubles_to_four ... ok")
print()
print("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out")
sys.stdout.flush()
# The control run is the one this shim serves; the mutant's build must find no cargo.
mode = {mode!r}
me.unlink()
if mode == "not executable":
    me.write_text("#!/bin/sh\\nexit 0\\n")
    me.chmod(0o644)
elif mode == "a directory":
    me.mkdir()
"""


class Member:
    """One (site, tool, mode, verb) built in a temporary repository and run to its verdict."""

    def __init__(self, test, site, tool, mode, verb):
        self.site, self.tool, self.mode, self.verb = site, tool, mode, verb
        cargo = tool == "cargo"
        parse = site == "parses"
        rows, files = [], None
        if cargo:
            rows = [("MUTATIONS", CARGO_ROW + [DESCRIPTION])]
            self.target = "crates/fix/src/lib.rs"
        elif parse:
            self.target = BASH_FILE if tool == "bash" else SH_FILE
            shebang = "#!/bin/bash\n" if tool == "bash" else "#!/bin/sh\n"
            files = {self.target: shebang + "echo $((1 + 2))\n"}
            rows = [
                (
                    "SCRIPT_MUTATIONS",
                    script_row(
                        ID,
                        "1 + 2",
                        "1 * 2",
                        "test_fixmod.Double.test_two_doubles_to_four",
                        target=self.target,
                    ),
                )
            ]
        else:
            self.target = "scripts/fixmod.py"
            killer = "test_fixmod.Double.test_two_doubles_to_four"
            rows = [("SCRIPT_MUTATIONS", script_row(ID, "x * 2", "x * 3", killer))]
        self.fixture = Fixture(test, rows, cargo=cargo, files=files)
        self.row = rows[0][1][0]
        scratch = tempfile.TemporaryDirectory()
        test.addCleanup(scratch.cleanup)
        self.scratch = Path(scratch.name)
        self.bindir = self.scratch / "bin"
        self.bindir.mkdir()
        self.python = None

    def arguments(self):
        verb = self.verb
        if verb == "retired":
            return ["retired", "--base", "HEAD"]
        if verb == "prove-id":
            return ["prove", "--row", self.row]
        if verb == "prove-band":
            return ["prove", "--band", BAND]
        if verb == "prove-all":
            return ["prove", "--all"]
        plan = self.scratch / "plan.json"
        plan.write_text(json.dumps({"rows": [self.row]}), encoding="utf-8")
        return ["prove", "--rows-from", str(plan)]

    def prepare(self):
        """The `PATH` directory: the real tools the run needs, the subject made unrunnable."""
        for name in ("git", "bash", "sh"):
            real = shutil.which(name)
            if real and name != self.tool:
                (self.bindir / name).symlink_to(real)
        subject = self.bindir / self.tool
        if self.tool == "python":
            self.python = self.scratch / "nopython"
            unrunnable(self.python, self.mode)
            return
        if self.tool == "cargo" and self.site == "builds":
            subject.write_text(
                CARGO_SHIM.format(python=sys.executable, mode=self.mode), encoding="utf-8"
            )
            subject.chmod(subject.stat().st_mode | stat.S_IXUSR)
            return
        unrunnable(subject, self.mode)

    def run(self):
        self.prepare()
        env = {k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"}
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        env["PATH"] = str(self.bindir)
        args = [*self.arguments(), "--root", str(self.fixture.root)]
        if self.python is not None:
            driver = (
                "import runpy, sys; runner = sys.argv[1]; sys.executable = sys.argv[2]; "
                "sys.argv = [runner] + sys.argv[3:]; runpy.run_path(runner, run_name='__main__')"
            )
            command = [sys.executable, "-c", driver, str(RUNNER), str(self.python), *args]
        else:
            command = [sys.executable, str(RUNNER), *args]
        return subprocess.run(
            command, capture_output=True, text=True, env=env, timeout=600, check=False
        )

    def named(self):
        """What the refusal line must name: the tool as the runner spawned it."""
        return str(self.python) if self.python is not None else self.tool


class TheMissingToolPopulation(unittest.TestCase):
    """SPEC-039 A46 to A48: the population is derived from the module's own spawn sites."""

    def test_every_spawn_site_the_module_holds_has_a_scenario_and_owns_no_raw_spawn(self):
        sites, raw_outside = spawn_sites()
        found = examined("spawn site(s)", sorted(sites))
        covered = {site for site, _tool, _verbs in SCENARIOS}
        self.assertEqual(set(found), covered, "a spawn site has no scenario, or a scenario no site")
        self.assertEqual(len(found), len(covered))
        self.assertEqual(
            raw_outside,
            [],
            "a raw subprocess spawn outside run_tool and run_in_own_group skips the tool check",
        )

    def test_a_tool_the_runner_cannot_run_is_a_refusal_at_every_site_mode_and_verb(self):
        members = [
            (site, tool, mode, verb)
            for site, tool, verbs in SCENARIOS
            for mode in MODES
            for verb in verbs
        ]
        expected = sum(len(verbs) for _site, _tool, verbs in SCENARIOS) * len(MODES)
        examined("missing-tool member(s)", members)
        self.assertEqual(len(members), expected)
        for site, tool, mode, verb in members:
            with self.subTest(site=site, tool=tool, mode=mode, verb=verb):
                member = Member(self, site, tool, mode, verb)
                before = sha256(member.fixture.root / member.target)
                done = member.run()
                output = done.stdout + done.stderr
                self.assertNotIn("Traceback", output, output)
                self.assertEqual(done.returncode, 2, output)
                lines = [line for line in output.splitlines() if "REFUSED" in line]
                self.assertEqual(len(lines), 1, output)
                self.assertIn(member.named(), lines[0], output)
                self.assertIsNone(VERDICT_LINE.search(output), output)
                self.assertEqual(sha256(member.fixture.root / member.target), before)
                changed = git(member.fixture.root, "status", "--porcelain", "--untracked-files=no")
                self.assertEqual(changed, "")

    def test_a_verb_that_spawns_nothing_runs_with_every_tool_unrunnable(self):
        fixture = Fixture(
            self,
            [
                (
                    "SCRIPT_MUTATIONS",
                    script_row(ID, "x * 2", "x * 3", "test_fixmod.Double.test_two_doubles_to_four"),
                )
            ],
        )
        empty = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, empty)
        env = dict(os.environ, PATH=str(empty), PYTHONDONTWRITEBYTECODE="1")
        for verb in examined("quiet verb(s)", QUIET_VERBS):
            done = subprocess.run(
                [sys.executable, str(RUNNER), verb, "--root", str(fixture.root)],
                capture_output=True,
                text=True,
                env=env,
                check=False,
            )
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertNotIn("REFUSED", done.stdout + done.stderr)


if __name__ == "__main__":
    unittest.main()
