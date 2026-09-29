"""Tests that kill the Python mutation runner's own survivors in its listing, map and child.

`scripts/mutation_python.py` lines 50 to 510 (SPEC-087): the module constants, `Mutant.new_source`,
`Lister`, `load_map`, `census_problems`, `Collect`, `skipped` and `child`. Each test pins a whole
observable (a whole listing, a whole problem list, a whole report) so that a changed token in it
fails by assertion. A call that a mutant makes raise is read through `attempt`, so the test fails
by assertion and not by an exception.
"""

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO

SCRIPT = REPO / "scripts" / "mutation_python.py"
MAP = "scripts/mutation-python.json"
SECONDS = 120


def load():
    spec = importlib.util.spec_from_file_location("mutation_python_under_kill", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPT.parent))
    try:
        spec.loader.exec_module(module)
    finally:
        sys.path.remove(str(SCRIPT.parent))
    return module


MP = load()


def attempt(function, *args, **kwargs):
    """What the call returns, or the exception it raised, named, so a test compares either."""
    try:
        return function(*args, **kwargs)
    except Exception as error:  # noqa: BLE001
        return f"{type(error).__name__}: {error}"


def listed(source):
    """The source's mutants, or the exception listing it raised, named."""
    return attempt(MP.list_source, source)


def pick(source, *names):
    """The named fields of each mutant of the source, or the exception listing it raised."""
    found = listed(source)
    if isinstance(found, str):
        return found
    return [tuple(getattr(m, name) for name in names) for m in found]


def records(source):
    found = listed(source)
    if isinstance(found, str):
        return found
    return [
        (
            m.line,
            m.column,
            m.start_line,
            m.end_line,
            m.end_column,
            m.start,
            m.end,
            m.old,
            m.new,
            m.operator,
            m.function,
        )
        for m in found
    ]


def python(*args):
    return subprocess.run(
        [sys.executable, *args], capture_output=True, text=True, timeout=SECONDS, check=False
    )


LISTED = """x = a and b
y = a == b
z = not   w
t = 1 if not\t q else 2
class K:
    def f(self, *, a, k=1):
        for i in a:
            if i:
                break
            continue
        return "s", True
u = not(w)
"""


class TheModuleHoldsItsConstants(unittest.TestCase):
    def test_importing_the_runner_stops_bytecode_and_puts_its_directory_first(self):
        code = (
            "import sys; import mutation_python; print(sys.dont_write_bytecode); print(sys.path[0])"
        )
        drop = ("PYTHONDONTWRITEBYTECODE", "PYTHONPYCACHEPREFIX")
        env = {k: v for k, v in os.environ.items() if k not in drop}
        with tempfile.TemporaryDirectory() as scratch:
            for name in ("mutation_python.py", "mutation_rows.py"):
                (Path(scratch) / name).write_text(
                    (SCRIPT.parent / name).read_text(encoding="utf-8"), encoding="utf-8"
                )
            env["PYTHONPATH"] = scratch
            ran = subprocess.run(
                [sys.executable, "-c", code],
                capture_output=True,
                text=True,
                timeout=SECONDS,
                check=False,
                env=env,
            )
            written = sorted(
                p.name for p in Path(scratch).rglob("*") if p.name.startswith("mutation_rows")
            )
        self.assertEqual(ran.stdout.split("\n")[:2], ["True", scratch], ran.stderr)
        self.assertEqual(written, ["mutation_rows.py"])

    def test_a_mutant_is_frozen_and_has_slots(self):
        mutant = MP.list_source("x = a and b\n")[0]
        self.assertEqual(
            attempt(setattr, mutant, "line", 9),
            "FrozenInstanceError: cannot assign to field 'line'",
        )
        self.assertEqual(hasattr(mutant, "__dict__"), False)

    def test_the_longest_old_text_is_forty_characters_and_cut_to_thirty_seven_and_dots(self):
        self.assertEqual(MP.LONGEST_OLD, 40)
        forty = '"' + "a" * 38 + '"'
        forty_one = '"' + "a" * 39 + '"'
        found = pick(f"x = {forty}\ny = {forty_one}\n", "old", "new")
        self.assertEqual(found, [(forty, '""'), ('"' + "a" * 36 + "...", '""')])
        self.assertEqual(len(found[1][0]), MP.LONGEST_OLD)


class TheListingIsHeldWhole(unittest.TestCase):
    def test_every_site_of_a_source_is_listed_with_its_whole_record(self):
        self.assertEqual(
            records(LISTED),
            [
                (1, 7, 1, 1, 12, 6, 9, "and", "or", "boolean", "<module>"),
                (2, 7, 2, 2, 11, 18, 20, "==", "!=", "comparison", "<module>"),
                (3, 5, 3, 3, 12, 27, 33, "not", "nothing", "not", "<module>"),
                (4, 5, 4, 4, 6, 39, 40, "1", "2", "integer constant", "<module>"),
                (4, 10, 4, 4, 16, 44, 49, "not", "nothing", "not", "<module>"),
                (4, 22, 4, 4, 23, 56, 57, "2", "3", "integer constant", "<module>"),
                (6, 25, 6, 6, 26, 91, 92, "1", "2", "integer constant", "<module>"),
                (9, 17, 9, 9, 22, 149, 154, "break", "continue", "loop", "K.f"),
                (10, 13, 10, 10, 21, 167, 175, "continue", "break", "loop", "K.f"),
                (11, 16, 11, 11, 19, 191, 194, '"s"', '""', "string constant", "K.f"),
                (11, 16, 11, 11, 25, 191, 200, "return value", "return None", "return", "K.f"),
                (11, 21, 11, 11, 25, 196, 200, "True", "False", "boolean constant", "K.f"),
                (12, 5, 12, 12, 11, 205, 208, "not", "nothing", "not", "<module>"),
            ],
        )

    def test_a_boolean_of_three_operands_lists_both_joints(self):
        self.assertEqual(
            pick("x = a or b or c\n", "column", "old", "new"),
            [(7, "or", "and"), (12, "or", "and")],
        )

    def test_a_keyword_only_argument_without_a_default_does_not_hide_the_others(self):
        self.assertEqual(
            pick("def f(*, a, b=1):\n    pass\n", "old", "function"),
            [("1", "<module>")],
        )

    def test_an_applied_mutant_swaps_nothing_for_empty_and_return_none_for_none(self):
        source = "x = not y\ndef f():\n    return 5\n"
        applied = attempt(lambda: [m.apply(source) for m in MP.list_source(source)])
        self.assertEqual(
            applied,
            ["x = y\ndef f():\n    return 5\n", "x = not y\ndef f():\n    return 6\n"]
            + ["x = not y\ndef f():\n    return None\n"],
        )

    def test_new_source_is_the_new_text_but_for_nothing_and_return_none(self):
        def mutant(new):
            return MP.Mutant(1, 1, 1, 1, 1, 0, 1, "o", new, "x", "f")

        self.assertEqual(
            [mutant(n).new_source for n in ("nothing", "return None", "or", "")],
            ["", "None", "or", ""],
        )

    def test_a_column_that_cuts_a_character_is_read_ignoring_the_stray_byte(self):
        lister = MP.Lister("é = 1\n")
        self.assertEqual(attempt(lister.char, 1, 1), (1, 0))
        self.assertEqual(attempt(lister.char, 1, 2), (1, 1))

    def test_an_annotation_and_a_docstring_are_never_mutated_and_a_default_is(self):
        source = 'def f(a: "1 + 2", b=3) -> "x":\n    """Doc: 1 + 2."""\n    return a\n'
        self.assertEqual(
            pick(source, "old", "new"),
            [("3", "4"), ("return value", "return None")],
        )


class TheMapAndTheCensusAreHeld(unittest.TestCase):
    def plant(self, mapping=None, text=None, names=()):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name)
        (root / "scripts" / "tests").mkdir(parents=True)
        for name in names:
            (root / "scripts" / f"{name}.py").write_text("", encoding="utf-8")
        if mapping is not None or text is not None:
            body = text if text is not None else json.dumps(mapping)
            (root / MAP).write_text(body, encoding="utf-8")
        return root

    def test_a_missing_map_is_named_with_the_reason_and_an_empty_map(self):
        root = self.plant()
        error = f"[Errno 2] No such file or directory: {str(root / MAP)!r}"
        self.assertEqual(attempt(MP.load_map, root), ({}, [f"{MAP}: cannot be read: {error}"]))

    def test_a_map_that_is_not_an_object_is_named_with_an_empty_map(self):
        root = self.plant(text="[]")
        self.assertEqual(
            attempt(MP.load_map, root), ({}, [f"{MAP}: is not an object of file to entry"])
        )

    def test_a_readable_map_is_returned_with_no_problem(self):
        root = self.plant(mapping={"scripts/a.py": {}})
        self.assertEqual(attempt(MP.load_map, root), ({"scripts/a.py": {}}, []))

    def test_an_unreadable_map_ends_the_census_with_its_problem(self):
        root = self.plant()
        error = f"[Errno 2] No such file or directory: {str(root / MAP)!r}"
        self.assertEqual(
            attempt(MP.census_problems, root), ({}, [f"{MAP}: cannot be read: {error}"])
        )

    def test_the_census_reports_every_bad_entry_and_goes_on_past_each(self):
        mapping = {
            "scripts/a.py": "x",
            "scripts/b.py": {"dir": "scripts/tests", "modules": "abc"},
            "scripts/c.py": {"dir": "scripts/tests", "modules": ["missing"]},
            "scripts/d.py": {"dir": "scripts/tests"},
            "scripts/e.py": {"dir": "scripts/tests", "modules": []},
            "scripts/zz.py": {},
        }
        root = self.plant(mapping=mapping, names="abcde")
        self.assertEqual(
            attempt(MP.census_problems, root),
            (
                mapping,
                [
                    "scripts/zz.py: a key outside the population",
                    "scripts/a.py: the entry is not an object",
                    "scripts/b.py: the entry needs a dir string and a modules list",
                    "scripts/c.py: the module missing has no file in scripts/tests",
                    "scripts/d.py: the entry needs a dir string and a modules list",
                    "scripts/zz.py: the entry needs a dir string and a modules list",
                ],
            ),
        )


def planted_case():
    class Case(unittest.TestCase):
        def test_x(self):
            with self.subTest(i=1):
                self.fail("one")

        def test_y(self):
            self.fail("two")

    return Case


class Named:
    def __init__(self, name):
        self.name = name

    def id(self):
        return self.name


class TheChildReportsWhatFailed(unittest.TestCase):
    def test_a_failed_test_is_named_by_id_and_a_failed_subtest_by_its_description(self):
        Case = planted_case()
        result = MP.Collect()
        unittest.TestSuite([Case("test_x"), Case("test_y")]).run(result)
        self.assertEqual(len(result.failed), 2)
        self.assertRegex(result.failed[0], r"^test_x \(.*Case\.test_x\) \(i=1\)$")
        self.assertEqual(result.failed[1], Case("test_y").id())
        self.assertEqual(sorted(result.broken), sorted([Case("test_x").id(), Case("test_y").id()]))

    def test_a_skip_names_a_test_or_a_test_and_its_subtest_and_never_a_longer_name(self):
        test = Named("m.C.test_a")
        self.assertEqual(
            [
                MP.skipped(test, ["m.C.test_a"]),
                MP.skipped(test, ["m.C.test_a (i=1)"]),
                MP.skipped(test, ["m.C.test_ab"]),
                MP.skipped(test, ["m.C.test_a2 (i=1)"]),
                MP.skipped(test, []),
            ],
            [True, True, False, False, False],
        )

    def run_child(self, name, source, *extra):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        (Path(scratch.name) / f"{name}.py").write_text(source, encoding="utf-8")
        ran = python(str(SCRIPT), "tests", "--dir", scratch.name, "--module", name, *extra)
        self.assertEqual(ran.returncode, 0, ran.stderr)
        return json.loads(ran.stdout.strip().splitlines()[-1])

    def test_the_child_keeps_its_own_main_module_and_reports_a_pass(self):
        source = (
            "import sys, unittest\n"
            "class P(unittest.TestCase):\n"
            "    def test_main_is_kept(self):\n"
            "        self.assertIn('__main__', sys.modules)\n"
        )
        self.assertEqual(
            self.run_child("probe_main", source), {"ran": 1, "failed": [], "broken": 0}
        )

    def test_the_child_reports_its_whole_record_and_all_failures_without_failfast(self):
        source = (
            "import unittest\n"
            "class P(unittest.TestCase):\n"
            "    def test_f(self):\n"
            "        self.fail('x')\n"
            "    def test_g(self):\n"
            "        pass\n"
        )
        self.assertEqual(
            self.run_child("probe_fail", source),
            {"ran": 2, "failed": ["probe_fail.P.test_f"], "broken": 1},
        )

    def test_failfast_reports_only_the_first_of_the_failures_the_collector_holds(self):
        source = (
            "import sys, unittest\n"
            "class P(unittest.TestCase):\n"
            "    def test_two(self):\n"
            "        frame = sys._getframe()\n"
            "        while not isinstance(frame.f_locals.get('result'), unittest.TestResult):\n"
            "            frame = frame.f_back\n"
            "        frame.f_locals['result'].failed.append('held earlier')\n"
            "        self.fail('b')\n"
        )
        self.assertEqual(
            self.run_child("probe_fast", source, "--failfast"),
            {"ran": 1, "failed": ["held earlier"], "broken": 1},
        )
        self.assertEqual(
            self.run_child("probe_slow", source.replace("probe_fast", "probe_slow")),
            {"ran": 1, "failed": ["held earlier", "probe_slow.P.test_two"], "broken": 1},
        )

    def test_the_child_function_returns_the_ok_exit_and_not_none(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "probe_ok.py").write_text(
                "import unittest\nclass P(unittest.TestCase):\n    def test_a(self):\n"
                "        pass\n",
                encoding="utf-8",
            )
            code = (
                "import argparse, sys\n"
                "sys.path.insert(0, %r)\n"
                "import mutation_python as mp\n"
                "args = argparse.Namespace(dir=%r, module=['probe_ok'], skip=[], failfast=False)\n"
                "print('RETURNED', repr(mp.child(args)), file=sys.stderr)\n"
            ) % (str(SCRIPT.parent), scratch)
            ran = python("-c", code)
        self.assertIn("RETURNED 0\n", ran.stderr)


if __name__ == "__main__":
    unittest.main()
