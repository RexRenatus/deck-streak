"""The python stage's module runner runs a suite's test modules at once, each in its own process,
and its last line sums each suite's tests and exit (SPEC-633 A1 and A2, SPEC-054 R7)."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

RUNNER = REPO / "scripts" / "python_stage.py"
BARRIERS = REPO / "scripts" / "tests" / "fixtures" / "python_stage"
# Every serial tail entry with the reason the runner gives it; the deploy pattern is planted as one
# module that matches it.
TAIL = (
    ("test_bin_kind_census.py", "spawns cargo"),
    ("test_check_gate.py", "spawns cargo"),
    ("test_ci_workflows.py", "unmeasured on the box"),
    ("test_deploy_planted.py", "unmeasured on the box"),
    ("test_dispatch_shards.py", "unmeasured on the box"),
    ("test_engine_pin.py", "spawns cargo"),
    ("test_fork_lock_guard.py", "spawns cargo"),
    ("test_mutation_rows.py", "spawns cargo"),
)
# A module that ends its process while it is imported, so it prints no Ran line.
QUIET = "import os\n\nos._exit(0)\n"


def planted(name, tests=1, red=False):
    """A test module whose `tests` passing tests each print `<name> <n>`, plus one failing test
    when `red`."""
    body = "".join(f"    def test_{n}(self):\n        print('{name} {n}')\n" for n in range(tests))
    if red:
        body += "    def test_red(self):\n        self.assertEqual(1, 2)\n"
    return f"import unittest\n\n\nclass Planted(unittest.TestCase):\n{body}"


def run_stage(where, tree, suites, **env):
    """Plant `tree` (suite: {module file: text}) under `where` and run the runner there over
    `suites`, with `env` added to this process's environment."""
    for suite, modules in tree.items():
        (where / suite).mkdir(parents=True)
        for name, text in modules.items():
            (where / suite / name).write_text(text, encoding="utf-8")
    return subprocess.run(
        ["python3", str(RUNNER), *suites],
        cwd=where,
        env={**os.environ, **env},
        capture_output=True,
        text=True,
        check=False,
        timeout=120,
    )


class TheModulesRunAtOnce(unittest.TestCase):
    def test_two_modules_run_at_once(self):
        """A1: two planted modules that each wait for the other pass only when run at once, and
        PYTHON_STAGE_JOBS=1 runs them one after another, so the first one times out."""
        barriers = examined("planted barrier modules", sorted(BARRIERS.glob("barrier_*.py")))
        self.assertEqual([path.name for path in barriers], ["barrier_a.py", "barrier_b.py"])
        tree = {"suite": {f"test_{p.name}": p.read_text(encoding="utf-8") for p in barriers}}
        timeout = "AssertionError: barrier timeout: module b did not arrive within 1 s"
        cases = (
            ("2", "10", [], "python: suite ran 2 test(s), exit 0", 0),
            ("1", "1", [timeout], "python: suite ran 2 test(s), exit 1", 1),
        )
        for jobs, wait, timeouts, verdict, code in cases:
            with self.subTest(jobs=jobs), tempfile.TemporaryDirectory() as scratch:
                where = Path(scratch)
                (where / "meeting").mkdir()
                env = {
                    "PYTHON_STAGE_JOBS": jobs,
                    "PYTHON_STAGE_BARRIER": str(where / "meeting"),
                    "PYTHON_STAGE_BARRIER_WAIT": wait,
                }
                done = run_stage(where, tree, ["suite"], **env)
                lines = done.stdout.splitlines()
                failures = [line for line in lines if line.startswith("AssertionError: barrier")]
                self.assertEqual(failures, timeouts)
                self.assertEqual(lines[-1], verdict)
                self.assertEqual(done.returncode, code, done.stdout)


class TheVerdictLine(unittest.TestCase):
    def test_the_verdict_line_sums_each_suite(self):
        """A2: the last line gives each suite's tests and exit in argument order; a red module, a
        suite with no module and a module that ran no test each fail the stage; each module's
        output comes in module-name order; every serial tail entry runs and is named with its
        reason."""
        everything = {name: planted("tail") for name, _ in TAIL}
        everything["test_pooled.py"] = planted("a")
        cases = examined(
            "planted trees",
            [
                (
                    {
                        "one": {"test_b.py": planted("b", 2), "test_a.py": planted("a")},
                        "two": {
                            "test_d.py": planted("d", red=True),
                            "test_ci_workflows.py": planted("tail"),
                        },
                    },
                    ["one", "two"],
                    "python: one ran 3 test(s), exit 0; two ran 3 test(s), exit 1",
                    1,
                    ["a 0", "b 0", "b 1", "tail 0", "d 0"],
                    ["serial: two/test_ci_workflows.py (unmeasured on the box)"],
                ),
                (
                    {"one": {"test_a.py": planted("a")}, "three": {}},
                    ["one", "three"],
                    "python: one ran 1 test(s), exit 0; three ran 0 test(s), exit 0",
                    1,
                    ["a 0"],
                    [],
                ),
                (
                    {"one": {"test_a.py": planted("a")}, "three": {"test_quiet.py": QUIET}},
                    ["one", "three"],
                    "python: one ran 1 test(s), exit 0; three ran 0 test(s), exit 0",
                    1,
                    ["a 0"],
                    [],
                ),
                (
                    {"one": {"test_a.py": planted("a")}, "two": {"test_d.py": planted("d")}},
                    ["one", "two"],
                    "python: one ran 1 test(s), exit 0; two ran 1 test(s), exit 0",
                    0,
                    ["a 0", "d 0"],
                    [],
                ),
                (
                    {"one": everything},
                    ["one"],
                    f"python: one ran {len(TAIL) + 1} test(s), exit 0",
                    0,
                    ["tail 0"] * len(TAIL) + ["a 0"],
                    [f"serial: one/{name} ({why})" for name, why in TAIL],
                ),
            ],
        )
        marks = {f"{name} {n}" for name in ("a", "b", "d", "tail") for n in range(2)}
        for tree, suites, verdict, code, order, tail in cases:
            with self.subTest(verdict=verdict), tempfile.TemporaryDirectory() as scratch:
                done = run_stage(Path(scratch), tree, suites)
                lines = done.stdout.splitlines()
                self.assertEqual(lines[-1], verdict)
                self.assertEqual(done.returncode, code, done.stdout)
                self.assertEqual([line for line in lines if line in marks], order)
                self.assertEqual([line for line in lines if line.startswith("serial: ")], tail)


class TheModulesWriteNoBytecode(unittest.TestCase):
    def test_module_processes_write_no_bytecode(self):
        """A4: a planted test module that imports a planted sibling module leaves no bytecode under
        its suite, though this run's environment allows bytecode writes, so only the runner can
        turn them off."""
        env = {key: value for key, value in os.environ.items() if key != "PYTHONDONTWRITEBYTECODE"}
        with tempfile.TemporaryDirectory() as scratch:
            suite = Path(scratch) / "suite"
            suite.mkdir()
            (suite / "sibling.py").write_text("VALUE = 1\n", encoding="utf-8")
            (suite / "test_imports.py").write_text(
                "import unittest\n\nimport sibling\n\n\nclass Imports(unittest.TestCase):\n"
                "    def test_value(self):\n        self.assertEqual(sibling.VALUE, 1)\n",
                encoding="utf-8",
            )
            done = subprocess.run(
                ["python3", str(RUNNER), "suite"],
                cwd=scratch,
                env=env,
                capture_output=True,
                text=True,
                check=False,
                timeout=120,
            )
            self.assertEqual(done.stdout.splitlines()[-1], "python: suite ran 1 test(s), exit 0")
            entries = examined("planted suite entries", sorted(suite.rglob("*")))
            self.assertEqual([path.name for path in entries if path.suffix == ".pyc"], [])
            self.assertEqual([path.name for path in entries if path.name == "__pycache__"], [])
