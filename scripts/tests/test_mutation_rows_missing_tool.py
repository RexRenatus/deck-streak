"""A tool the runner needs and cannot run is a refusal, never a survivor (SPEC-039 A46 to A48,
ADR-291, issue #431).

THE CLASS. Every process `scripts/mutation_rows.py` spawns, under every verb that reaches it, ends
the verb with ONE line naming the tool and exit 2 (`EXIT_REFUSED`) when its executable cannot be
run: absent from `PATH`, present but not executable, a directory at the name, a script whose
interpreter line names a missing program, an empty file the kernel will not execute, or a wrapper
whose `#!/usr/bin/env` line names a missing program, so it exits 127 (126 when a `PATH` entry
could not be searched on the way). Never a traceback,
never exit 1 (`EXIT_SURVIVED`), never a verdict, and the target's bytes are the ones it started
with.

THE POPULATION is generated, not listed. The spawn sites are read from the module's own source
with `ast` (every `subprocess.run`, `subprocess.Popen`, `run_in_own_group` and `run_tool` call
outside the two helpers that own a raw spawn), so a new spawn joins the population by itself and
fails the census until a scenario covers it. Each site carries the tools it can spawn and the
verbs that reach it; each member is one (site, tool, unrunnable mode, verb) built in a temporary
git repository with the suite's own fixture, run as a child process with a `PATH` (or, for the
interpreter, a `sys.executable`) that makes the tool unrunnable. The tool's place in `PATH` is an
axis too: alone, or behind an entry the runner cannot even look at, which the spawn's own search
passes over.
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
MODES = (
    "absent",
    "not executable",
    "a directory",
    "bad interpreter",
    "an empty file",
    "a wrapper whose program is missing",
)
#: What the refusal line says about each mode: the reason is part of the line, not only the name.
WHY = {
    "absent": ("not found on path", "no such file"),
    "not executable": ("not executable",),
    "a directory": ("is a directory",),
    "bad interpreter": ("no such file",),
    "an empty file": ("exec format error",),
    "a wrapper whose program is missing": ("exit 127", "exit 126"),
}
#: Where the tool sits in `PATH`: alone, or behind an entry whose name is too long to look at,
#: which raises on a look for any user (root included) and which the spawn passes over.
POSITIONS = ("alone", "behind an entry it cannot look at")
UNREADABLE_ENTRY = "n" * 300
#: The modes whose file resolution accepts, so the spawn itself fails. Its error is the first one
#: its own `PATH` search met, which behind the unreadable entry is that entry's, not the file's.
SPAWN_FAILS = ("bad interpreter", "an empty file")
#: A program no machine has: a wrapper naming it on its `#!/usr/bin/env` line exits 127, or 126
#: when `env` met an entry it could not search on the way.
NO_SUCH_PROGRAM = "mutation-rows-no-such-program-431"
#: Every name the standard library starts a process by. A call to one outside the two helpers,
#: or an import that reaches one under another name, is a spawn the site census would not see.
ANY_SPAWN = re.compile(
    r"^(subprocess\.(run|Popen|call|check_call|check_output|getoutput|getstatusoutput)"
    r"|os\.(system|popen|fork|forkpty|posix_spawnp?|startfile|exec[lv]p?e?|spawn[lv]p?e?)"
    r"|pty\.(spawn|fork)|asyncio\.create_subprocess_(exec|shell))$"
)
#: The event loop's own two spawners, called on whatever the loop is named.
LOOP_SPAWN = re.compile(r"\.subprocess_(exec|shell)$")
#: The names the standard library documents as starting a process (read from the `subprocess`,
#: `os` "Process Management", `pty` and `asyncio` subprocess pages on 2026-09-30).
DOCUMENTED_SPAWNERS = {
    "subprocess": "run Popen call check_call check_output getoutput getstatusoutput".split(),
    "os": (
        "system popen fork forkpty posix_spawn posix_spawnp startfile "
        "execl execle execlp execlpe execv execve execvp execvpe "
        "spawnl spawnle spawnlp spawnlpe spawnv spawnve spawnvp spawnvpe"
    ).split(),
    "pty": ["spawn", "fork"],
    "asyncio": ["create_subprocess_exec", "create_subprocess_shell"],
}
SPAWNING_MODULES = ("subprocess", "os", "pty", "asyncio")
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
    """Make `path` an executable that cannot be run: absent, without the execute bit, a directory,
    or a script whose interpreter line names a program that does not exist."""
    if path.is_symlink() or path.is_file():
        path.unlink()
    if mode == "not executable":
        path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        path.chmod(0o644)
    elif mode == "a directory":
        path.mkdir()
    elif mode == "bad interpreter":
        path.write_text("#!/nonexistent/interpreter\n", encoding="utf-8")
        path.chmod(0o755)
    elif mode == "an empty file":
        path.write_bytes(b"")
        path.chmod(0o755)
    elif mode == "a wrapper whose program is missing":
        path.write_text(f"#!/usr/bin/env {NO_SUCH_PROGRAM}\n", encoding="utf-8")
        path.chmod(0o755)


def raw_spawns(source=None):
    """(spawns the two helpers own, spawns and imports anywhere else) read from the module's
    source (or from `source`) by every name the standard library starts a process by, not only
    the four the site census reads."""
    tree = ast.parse(MODULE.read_text(encoding="utf-8") if source is None else source)
    owned, outside, inside = [], [], {}
    for function in ast.walk(tree):
        if isinstance(function, ast.FunctionDef) and function.name in HELPERS:
            for call in ast.walk(function):
                inside[id(call)] = function.name
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module in SPAWNING_MODULES:
            outside += [
                f"from {node.module} import {alias.name}"
                for alias in node.names
                if node.module != "os" or alias.name == "*" or ANY_SPAWN.match("os." + alias.name)
            ]
        elif isinstance(node, ast.Import):
            outside += [
                f"import {alias.name} as {alias.asname}"
                for alias in node.names
                if alias.name in SPAWNING_MODULES and alias.asname
            ]
            outside += [
                f"import {alias.name}" for alias in node.names if alias.name in ("pty", "asyncio")
            ]
        elif isinstance(node, ast.Call) and (
            ANY_SPAWN.match(ast.unparse(node.func)) or LOOP_SPAWN.search(ast.unparse(node.func))
        ):
            where = inside.get(id(node))
            spawn = f"{where or 'outside the helpers'}: {ast.unparse(node.func)}"
            (owned if where else outside).append(spawn)
    return owned, outside


CARGO_SHIM = """#!{python}
import os, stat, sys
from pathlib import Path

me = Path(__file__)
if "--no-run" in sys.argv:
    sys.exit(0)
(me.parent / "served").write_text("the control run reached this cargo")
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
elif mode == "bad interpreter":
    me.write_text("#!/nonexistent/interpreter\\n")
    me.chmod(0o755)
elif mode == "an empty file":
    me.write_bytes(b"")
    me.chmod(0o755)
elif mode == "a wrapper whose program is missing":
    me.write_text("#!/usr/bin/env {missing}\\n")
    me.chmod(0o755)
"""


class Member:
    """One (site, tool, mode, verb) built in a temporary repository and run to its verdict."""

    def __init__(self, test, site, tool, mode, verb, position="alone"):
        self.site, self.tool, self.mode, self.verb = site, tool, mode, verb
        self.position = position
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
                CARGO_SHIM.format(python=sys.executable, mode=self.mode, missing=NO_SUCH_PROGRAM),
                encoding="utf-8",
            )
            subject.chmod(subject.stat().st_mode | stat.S_IXUSR)
            return
        unrunnable(subject, self.mode)

    def run(self):
        self.prepare()
        env = {k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"}
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        env["PATH"] = str(self.bindir)
        if self.position != "alone":
            env["PATH"] = os.pathsep.join([str(self.scratch / UNREADABLE_ENTRY), str(self.bindir)])
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
        # Every verb reaches the resolution the same way, so the position axis is crossed with
        # one verb; the interpreter is spawned by its absolute path, so no position applies to it.
        members = [
            (site, tool, mode, verb, POSITIONS[0])
            for site, tool, verbs in SCENARIOS
            for mode in MODES
            for verb in verbs
        ] + [
            (site, tool, mode, "prove-id", position)
            for site, tool, _verbs in SCENARIOS
            if tool != "python"
            for mode in MODES
            for position in POSITIONS[1:]
        ]
        searched = sum(1 for _site, tool, _verbs in SCENARIOS if tool != "python")
        expected = sum(len(verbs) for _site, _tool, verbs in SCENARIOS) * len(MODES)
        expected += searched * len(MODES) * (len(POSITIONS) - 1)
        examined("missing-tool member(s)", members)
        self.assertEqual(len(members), expected)
        for site, tool, mode, verb, position in members:
            with self.subTest(site=site, tool=tool, mode=mode, verb=verb, position=position):
                member = Member(self, site, tool, mode, verb, position)
                before = sha256(member.fixture.root / member.target)
                done = member.run()
                output = done.stdout + done.stderr
                self.assertNotIn("Traceback", output, output)
                self.assertEqual(done.returncode, 2, output)
                lines = [line for line in output.splitlines() if "REFUSED" in line]
                self.assertEqual(len(lines), 1, output)
                self.assertIn(member.named(), lines[0], output)
                reasons = WHY[mode]
                if position != "alone" and mode in SPAWN_FAILS:
                    reasons += ("file name too long",)
                self.assertTrue(any(why in lines[0].lower() for why in reasons), lines[0])
                self.assertIsNone(VERDICT_LINE.search(output), output)
                if site == "builds":
                    served = member.bindir / "served"
                    self.assertTrue(served.is_file(), "the control never reached the cargo shim")
                self.assertEqual(sha256(member.fixture.root / member.target), before)
                changed = git(member.fixture.root, "status", "--porcelain", "--untracked-files=no")
                self.assertEqual(changed, "")

    def test_no_code_outside_the_helpers_spawns_a_process_by_any_other_name(self):
        owned, outside = raw_spawns()
        self.assertEqual(
            sorted(owned), ["run_in_own_group: subprocess.Popen", "run_tool: subprocess.run"]
        )
        self.assertEqual(
            outside, [], "a spawn by a name the site census does not read skips the tool check"
        )

    def test_the_census_reads_every_documented_spawner_by_every_way_of_reaching_it(self):
        forms = (
            "import {m}\n{m}.{n}(x)\n",
            "import {m} as alias\nalias.{n}(x)\n",
            "from {m} import {n} as other\nother(x)\n",
            "from {m} import *\n",
        )
        examined = 0
        for module, names in DOCUMENTED_SPAWNERS.items():
            for name in names:
                for form in forms:
                    if form.startswith("from {m} import *") and module != "os":
                        continue
                    with self.subTest(module=module, name=name, form=form):
                        _, outside = raw_spawns(form.format(m=module, n=name))
                        self.assertNotEqual(outside, [], form.format(m=module, n=name))
                        examined += 1
        for loop_call in ("loop.subprocess_exec(f, x)", "self.loop.subprocess_shell(f, x)"):
            with self.subTest(loop_call=loop_call):
                self.assertNotEqual(raw_spawns(loop_call + "\n")[1], [])
                examined += 1
        expected = 3 * sum(map(len, DOCUMENTED_SPAWNERS.values()))
        expected += len(DOCUMENTED_SPAWNERS["os"]) + 2
        self.assertEqual(examined, expected)
        print(f"examined {examined} spawner spelling(s)")

    def test_a_path_entry_the_runner_cannot_look_at_is_passed_over_as_the_spawn_passes_it(self):
        member = Member(
            self, "git", "git", "absent", "prove-id", position="behind an entry it cannot look at"
        )
        for name in ("git", "bash", "sh"):
            (member.bindir / name).symlink_to(shutil.which(name))
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        env["PATH"] = os.pathsep.join([str(member.scratch / UNREADABLE_ENTRY), str(member.bindir)])
        done = subprocess.run(
            [sys.executable, str(RUNNER), *member.arguments(), "--root", str(member.fixture.root)],
            capture_output=True,
            text=True,
            env=env,
            timeout=600,
            check=False,
        )
        output = done.stdout + done.stderr
        self.assertEqual(done.returncode, 0, output)
        self.assertIn(f"{ID}: KILLED", output)
        self.assertNotIn("REFUSED", output)

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
            self.assertTrue(done.stdout.strip(), verb + " printed nothing")
            self.assertNotIn("REFUSED", done.stdout + done.stderr)


if __name__ == "__main__":
    unittest.main()
