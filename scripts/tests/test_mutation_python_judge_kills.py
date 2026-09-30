"""Kills for the judging half of `scripts/mutation_python.py` (SPEC-087, lane k3).

The survivors this module pins are the module constants of `Ran`, `judge_mutant`, and every
observable of `Judge`: its scratch names, the runner copy, the child's environment and report
reading, `kill`, and the record and the void reasons `Judge.file` writes. Each test compares a
whole observable, so a mutated token anywhere in it changes what the test reads.
"""

import argparse
import importlib.util
import os
import shutil
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined

RUNNER = REPO / "scripts" / "mutation_python.py"


def load(path, name):
    """`path` as a module named `name`, registered so a dataclass can find its module."""
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class Base(unittest.TestCase):
    def setUp(self):
        self.mp = load(RUNNER, "mutation_python_judge_kills")
        self.addCleanup(sys.modules.pop, "mutation_python_judge_kills", None)
        scratch = tempfile.mkdtemp(prefix="judge-kills-")
        self.addCleanup(shutil.rmtree, scratch, ignore_errors=True)
        self.root = Path(scratch)

    def args(self, **given):
        values = {"test_seconds": None, "control_seconds": None, "failfast": False}
        values.update(given)
        return argparse.Namespace(**values)

    def judge(self, mapping, **given):
        judge = self.mp.Judge(self.root, mapping, self.args(**given))
        self.addCleanup(shutil.rmtree, judge.scratch, ignore_errors=True)
        return judge


class TheRanReportStartsEmpty(Base):
    def test_a_ran_is_empty_until_a_child_reports(self):
        ran = self.mp.Ran()
        self.assertEqual(
            (ran.reported, ran.timed_out, ran.ran, ran.failed, ran.broken, ran.seconds),
            (False, False, 0, [], 0, 0.0),
        )
        self.assertFalse(hasattr(ran, "__dict__"), "Ran is a slotted dataclass")


class TheMutantIsJudged(Base):
    def test_a_run_that_reported_nothing_or_ran_nothing_is_void(self):
        target = self.root / "t.py"
        target.write_text("x = 1\n", encoding="utf-8")
        original = target.read_bytes()
        for ran in (self.mp.Ran(reported=True, ran=0), self.mp.Ran(reported=False, ran=3)):
            outcome = self.mp.judge_mutant(target, original, "x = 2\n", lambda ran=ran: ran)
            self.assertEqual(outcome, ("void", []), ran)
        self.assertEqual(target.read_bytes(), original)


class TheJudgeSetsUp(Base):
    def test_the_scratch_and_its_pycache_are_named(self):
        judge = self.judge({})
        self.assertTrue(judge.scratch.name.startswith("mutation-python-"), judge.scratch)
        self.assertEqual(judge.copy, judge.scratch / "runner")
        self.assertEqual(judge.pycache, judge.scratch / "pycache")

    def test_the_runner_copy_follows_absolute_imports_and_never_relative_ones(self):
        here = self.root / "scripts"
        here.mkdir()
        shutil.copy(RUNNER, here / "mutation_python.py")
        (here / "mutation_rows.py").write_text(
            "from helper import value\nfrom .relative import other\nimport not_a_file\n",
            encoding="utf-8",
        )
        (here / "helper.py").write_text("value = 1\n", encoding="utf-8")
        (here / "relative.py").write_text("other = 2\n", encoding="utf-8")
        module = load(here / "mutation_python.py", "mutation_python_judge_copy")
        self.addCleanup(sys.modules.pop, "mutation_python_judge_copy", None)
        judge = module.Judge(self.root, {}, self.args())
        self.addCleanup(shutil.rmtree, judge.scratch, ignore_errors=True)
        copied = sorted(examined("copied files", (p.name for p in judge.copy.iterdir())))
        self.assertEqual(copied, ["helper.py", "mutation_python.py", "mutation_rows.py"])


class TheChildIsRunAndRead(Base):
    def run_child(self, source, bound=60.0, skip=(), failfast=False):
        judge = self.judge({})
        # The child must get both variables from Judge.tests, not from this process.
        patched = mock.patch.dict(os.environ)
        patched.start()
        self.addCleanup(patched.stop)
        os.environ.pop("PYTHONDONTWRITEBYTECODE", None)
        os.environ.pop("PYTHONPYCACHEPREFIX", None)
        (self.root / "m_child.py").write_text(
            textwrap.dedent(source).replace("@PYCACHE@", str(judge.pycache)), encoding="utf-8"
        )
        entry = {"dir": ".", "modules": ["m_child"]}
        try:
            return judge.tests(entry, list(skip), failfast, bound)
        except Exception as error:  # noqa: BLE001
            self.fail(f"Judge.tests raised {error!r}")

    def test_the_child_runs_with_no_bytecode_and_a_private_pycache(self):
        ran = self.run_child(
            """
            import os
            import unittest


            class T(unittest.TestCase):
                def test_env(self):
                    self.assertEqual(os.environ.get("PYTHONDONTWRITEBYTECODE"), "1")
                    self.assertEqual(os.environ.get("PYTHONPYCACHEPREFIX"), "@PYCACHE@")
            """
        )
        self.assertEqual((ran.reported, ran.ran, ran.failed, ran.broken), (True, 1, [], 0))
        self.assertLess(ran.seconds, 60)
        self.assertGreaterEqual(ran.seconds, 0)

    def test_a_run_past_its_bound_is_timed_out_and_timed(self):
        ran = self.run_child(
            """
            import time
            import unittest


            class T(unittest.TestCase):
                def test_hang(self):
                    time.sleep(120)
            """,
            bound=2,
        )
        self.assertEqual((ran.timed_out, ran.reported), (True, False))
        self.assertGreaterEqual(ran.seconds, 1.5)
        self.assertLess(ran.seconds, 60)

    def test_output_that_is_not_utf8_is_replaced_and_the_report_still_read(self):
        ran = self.run_child(
            """
            import os
            import unittest


            class T(unittest.TestCase):
                def test_noise(self):
                    os.write(1, b"\\xff\\xfe noise\\n")
            """
        )
        self.assertEqual((ran.reported, ran.ran, ran.failed), (True, 1, []))

    def test_only_the_last_report_line_is_read_and_junk_after_it_is_skipped(self):
        ran = self.run_child(
            """
            import atexit
            import os
            import sys
            import unittest


            def junk():
                sys.__stdout__.write('["ran", "failed"]\\n')
                sys.__stdout__.write('{"ran": 9}\\n')
                sys.__stdout__.write('{"failed": ["x"]}\\n')
                sys.__stdout__.write("plain junk\\n")


            atexit.register(junk)


            class T(unittest.TestCase):
                def test_early(self):
                    os.write(1, b'{"ran": 99, "failed": [], "broken": 0}\\n')
            """
        )
        self.assertEqual((ran.reported, ran.ran, ran.failed, ran.broken), (True, 1, [], 0))

    def test_broken_counts_tests_and_not_failures(self):
        ran = self.run_child(
            """
            import unittest


            class T(unittest.TestCase):
                def test_two_subtests_fail(self):
                    for i in range(2):
                        with self.subTest(i=i):
                            self.fail("no")
            """
        )
        self.assertEqual((ran.reported, len(ran.failed), ran.broken), (True, 2, 1))


class TheChildIsKilled(Base):
    def test_a_kill_signals_the_group_and_waits_ten_seconds(self):
        dead = self.mp.subprocess.Popen(["true"], start_new_session=True)
        dead.wait()
        waits = []

        class Process:
            pid = dead.pid

            def wait(self, timeout):
                waits.append(timeout)
                raise self.mp.subprocess.TimeoutExpired("x", timeout)

        Process.mp = self.mp
        self.mp.Judge.kill(Process())
        self.assertEqual(waits, [10])


class TheFileIsJudged(Base):
    def setUp(self):
        super().setUp()
        self.target = self.root / "t.py"
        self.target.write_text("x = 1\n", encoding="utf-8")
        self.original = self.target.read_bytes()
        self.mutant = self.mp.list_source("x = 1\n")[0]
        self.entry = {"dir": ".", "modules": ["m"]}

    def expected(self, outcome, killers=()):
        m = self.mutant
        return {
            "name": m.name("t.py"),
            "file": "t.py",
            "line": m.line,
            "start_line": m.start_line,
            "end_line": m.end_line,
            "end_column": m.end_column,
            "column": m.column,
            "mutant": m.text,
            "operator": m.operator,
            "outcome": outcome,
            "killers": list(killers),
        }

    def file(self, outs, entry=None, **given):
        """The file's record after `outs` answered its runs in order, and the calls made."""
        calls = []
        judge = self.judge({"t.py": entry or self.entry}, **given)

        def fake(entry, skip, failfast, bound):
            calls.append((list(skip), failfast, bound))
            return outs.pop(0) if outs else self.mp.Ran(reported=True, ran=3)

        judge.tests = fake
        report = {"files": []}
        judge.file("t.py", [self.mutant], report)
        self.assertEqual(self.target.read_bytes(), self.original)
        return report["files"][0], calls

    def test_a_file_with_no_module_is_uncovered_and_its_record_is_whole(self):
        record, calls = self.file([], entry={"dir": ".", "modules": []})
        self.assertEqual(calls, [])
        self.assertEqual(
            record,
            {
                "path": "t.py",
                "modules": [],
                "control": None,
                "bound": None,
                "byte_readers": [],
                "void": None,
                "mutants": [self.expected("uncovered")],
            },
        )

    def test_a_judged_file_runs_its_control_then_its_sentinel_then_each_mutant(self):
        ran = self.mp.Ran
        record, calls = self.file(
            [
                ran(reported=True, ran=3, seconds=0.5),
                ran(reported=True, ran=3, failed=["t.A"], broken=1),
                ran(reported=True, ran=2, failed=["t.B"], broken=1),
            ],
            test_seconds=7,
        )
        self.assertEqual(calls, [([], False, 7), ([], False, 7), (["t.A"], False, 7)])
        self.assertEqual(record["control"], {"ran": 3, "failures": 0, "seconds": 0.5})
        self.assertEqual(record["bound"], 7)
        self.assertEqual(record["byte_readers"], ["t.A"])
        self.assertIsNone(record["void"])
        self.assertEqual(record["mutants"], [self.expected("killed", ["t.B"])])

    def test_a_control_past_its_bound_voids_the_file_even_when_it_reported(self):
        ran = self.mp.Ran(timed_out=True, reported=True, ran=3)
        record, _ = self.file([ran])
        self.assertEqual(record["void"], "the control ran past its bound")
        self.assertEqual(record["mutants"], [self.expected("void")])

    def test_a_control_that_printed_no_report_voids_the_file(self):
        record, _ = self.file([self.mp.Ran(reported=False, ran=0)])
        self.assertEqual(record["void"], "the control printed no report")
        self.assertEqual(record["mutants"], [self.expected("void")])

    def test_a_sentinel_past_its_bound_or_without_a_report_voids_the_file(self):
        ran = self.mp.Ran
        reason = "the sentinel run ran past its bound or printed no report"
        for sentinel in (
            ran(timed_out=True, reported=True, ran=3),
            ran(reported=False, ran=0),
        ):
            record, calls = self.file([ran(reported=True, ran=3, seconds=1.0), sentinel])
            self.assertEqual(len(calls), 2)
            self.assertEqual(record["void"], reason)
            self.assertEqual(record["mutants"], [self.expected("void")])


if __name__ == "__main__":
    unittest.main()
