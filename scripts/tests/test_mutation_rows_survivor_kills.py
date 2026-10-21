"""The rows runner's survivors of the release run, each killed by a whole observable (SPEC-492 R2,
R4 and R5).

Each test calls the runner's own functions over a fixture tree in a temporary directory, or over a
literal input, and compares the whole result with its expected value spelled literally: a mutant
of the token a test names changes that value. A test that stands in for the runner's own process
calls (`run_in_own_group`, `run_tool`) or its git reads (`tracked_changes`, `load_revision`) keeps
every other input. `TheEquivalentShapesAreGone` holds the runner free of the shapes whose mutants
no test can observe (A4).
"""

import ast
import contextlib
import dataclasses
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest import mock

from _support import REPO, examined

sys.path.insert(0, str(REPO / "scripts"))
import mutation_rows as runner  # noqa: E402

RUNNER = REPO / "scripts" / "mutation_rows.py"
LEGEND = ["target spelling: SCRIPT_MUTATIONS is repo-rooted"]
CHILD_IMPORT = "import sys; sys.path.insert(0, sys.argv[1]); import mutation_rows; print(sys.dont_write_bytecode)"
SCRIPT_KILLER_TEST = (
    "import sys\n"
    "import unittest\n\n\n"
    "class C(unittest.TestCase):\n"
    "    def test_t(self):\n"
    "        self.assertTrue(sys.dont_write_bytecode)\n"
)
CRATE_MANIFEST = '[package]\nname = "p"\nversion = "0.1.0"\n'
#: sha256 of the fixture target `VALUE = 1\n`, the bytes `prove_row` saves and restores.
TARGET_SHA = "e13df8c44af5dea1e412403910b99cc5a48f2ccbf68a66b3374d6ab9cef9fc65"


def scratch(test):
    holder = tempfile.TemporaryDirectory()
    test.addCleanup(holder.cleanup)
    return Path(holder.name)


def tree(test, files):
    """A fixture root holding `files`, {path from the root: text}."""
    root = scratch(test)
    for relative, text in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return root


def a_row(**cells):
    """A scripts-table row; each cell the test names replaces its default."""
    fields = {
        "id": "S1-A",
        "table": "SCRIPT_MUTATIONS",
        "target": "scripts/t.py",
        "find": "VALUE = 1",
        "replace": "VALUE = 2",
        "killer": "m.C.test_t",
        "crate": None,
        "description": "d",
    }
    fields.update(cells)
    return runner.Row(**fields)


def population(*rows):
    return {
        "_": list(LEGEND),
        "arities": {"SCRIPT_MUTATIONS": [6]},
        "tables": {"SCRIPT_MUTATIONS": list(rows)},
    }


def call_main(*argv):
    """`main` over `argv`: (its return or its exit, what it printed, what it wrote to stderr)."""
    out, err = io.StringIO(), io.StringIO()
    with (
        contextlib.redirect_stdout(out),
        contextlib.redirect_stderr(err),
        mock.patch.object(runner, "end_on_sigterm", lambda: None),
    ):
        try:
            code = runner.main(list(argv))
        except SystemExit as stop:
            code = ("exit", stop.code)
    return code, out.getvalue(), err.getvalue()


def flat(text):
    return " ".join(text.split())


def answering(seen, stdout="", stderr="", code=0, raises=None):
    """A stand-in for `run_in_own_group` that records its call and answers with one process."""

    def stand_in(command, *, cwd, env, timeout):
        seen.append((command, cwd, env, timeout))
        if raises is not None:
            raise raises
        return subprocess.CompletedProcess(command, code, stdout, stderr)

    return stand_in


def killer_tree(test):
    """A root with a script killer's module, a target and a crate holding a lib test and an
    integration test target that is a directory."""
    return tree(
        test,
        {
            "scripts/t.py": "VALUE = 1\n",
            "scripts/tests/m.py": SCRIPT_KILLER_TEST,
            "crates/c/Cargo.toml": CRATE_MANIFEST,
            "crates/c/src/lib.rs": "#[test]\nfn t() {}\n",
            "crates/c/tests/it/main.rs": "#[test]\nfn t() {}\n",
        },
    )


SCRIPT_KILLER = runner.Killer("script", "m.C.test_t", "scripts/tests/m.py", cwd="scripts/tests")
LIB_KILLER = runner.Killer("cargo", "t", "crates/c/src", package="p", target="lib")


class TheModuleSetup(unittest.TestCase):
    def test_loading_the_rows_runner_turns_off_bytecode(self):
        cache = scratch(self)
        env = {
            name: value for name, value in os.environ.items() if name != "PYTHONDONTWRITEBYTECODE"
        }
        env["PYTHONPYCACHEPREFIX"] = str(cache)
        done = subprocess.run(
            [sys.executable, "-c", CHILD_IMPORT, str(REPO / "scripts")],
            capture_output=True,
            text=True,
            env=env,
            timeout=120,
            check=False,
        )
        self.assertEqual(done.stdout, "True\n", done.stderr)

    def test_each_bound_is_the_figure_its_spec_states(self):
        self.assertEqual(runner.BUILD_SECONDS, 3600)
        self.assertEqual(runner.PARSE_SECONDS, 60)
        self.assertEqual(runner.TEST_SECONDS, 900)

    def test_a_row_and_a_killer_are_frozen_and_slotted(self):
        row, killer = a_row(), runner.Killer("script", "m.C.t", "scripts/tests/m.py")
        with self.assertRaises(dataclasses.FrozenInstanceError):
            row.id = "S2-B"
        with self.assertRaises(dataclasses.FrozenInstanceError):
            killer.kind = "cargo"
        for each in examined("dataclasses held slotted", [row, killer]):
            self.assertFalse(hasattr(each, "__dict__"), each)


class TheModuleTables(unittest.TestCase):
    def test_cells_holds_each_entry(self):
        self.assertEqual(
            runner.CELLS,
            {
                "MUTATIONS": {"find": 3, "replace": 4, "crate": 1, "killer": 5, "description": 6},
                "CARGO_KILLED_SCRIPT_MUTATIONS": {
                    "find": 2,
                    "replace": 3,
                    "crate": 4,
                    "killer": 5,
                    "description": 6,
                },
                "SCRIPT_MUTATIONS": {
                    "find": 2,
                    "replace": 3,
                    "crate": None,
                    "killer": 5,
                    "description": 4,
                },
            },
        )

    def test_test_attribute_holds_each_entry(self):
        self.assertEqual(runner.TEST_ATTRIBUTE.pattern, r"^\s*#\[(?:[a-z_]+::)*test\b")
        self.assertTrue(runner.TEST_ATTRIBUTE.match("    #[tokio::test]"))
        self.assertIsNone(runner.TEST_ATTRIBUTE.match("#[testing]"))

    def test_row_id_holds_each_entry(self):
        self.assertEqual(runner.ROW_ID.pattern, r"S[0-9]+-[A-Z0-9]+(?:-[A-Z0-9]+)*")
        self.assertTrue(runner.ROW_ID.fullmatch("S49201-THE-ROW-LOCATOR"))
        self.assertIsNone(runner.ROW_ID.fullmatch("S49201-the-row"))


class TheRowId(unittest.TestCase):
    def test_each_integer_of__row_id_is_observed(self):
        self.assertEqual(runner._row_id(["S1-A", 5]), "S1-A")

    def test_each_operand_of__row_id_is_observed(self):
        self.assertIsNone(runner._row_id([5]))
        self.assertIsNone(runner._row_id("S1-A"))
        self.assertIsNone(runner._row_id([]))
        self.assertEqual(runner._row_id(["S1-A"]), "S1-A")


class TheSpecBand(unittest.TestCase):
    def test_each_operand_of_spec_band_is_observed(self):
        self.assertFalse(runner.spec_band("S100-S150.json"))
        self.assertFalse(runner.spec_band("S150-S249.json"))
        self.assertFalse(runner.spec_band("rows.json"))
        self.assertTrue(runner.spec_band("S49200-S49299.json"))


MONOLITH = {
    "_": list(LEGEND),
    "arities": {"SCRIPT_MUTATIONS": [6]},
    "tables": {"SCRIPT_MUTATIONS": []},
}
ROW_900 = ["S901-A", "scripts/a.py", "x", "y", "d", "m.C.t"]
ROW_1000 = ["S1001-B", "scripts/b.py", "x", "y", "d", "m.C.t"]


class TheAssemble(unittest.TestCase):
    def test_each_operand_of_assemble_is_observed(self):
        with self.assertRaisesRegex(
            runner.PopulationRefused,
            "^scripts/mutation-rows.json must declare arities and tables objects$",
        ):
            runner.assemble({"arities": {}, "tables": []}, [])
        with self.assertRaisesRegex(
            runner.PopulationRefused,
            "^scripts/mutation-rows.d/S100-S199.json must hold one key, `tables`$",
        ):
            runner.assemble(MONOLITH, [("S100-S199.json", {"tables": {}, "x": 1})])

    def test_each_integer_of_assemble_is_observed(self):
        fragments = [
            ("S1000-S1099.json", {"tables": {"SCRIPT_MUTATIONS": [ROW_1000]}}),
            ("S900-S999.json", {"tables": {"SCRIPT_MUTATIONS": [ROW_900]}}),
        ]
        assembled = runner.assemble(MONOLITH, fragments)
        self.assertEqual(assembled["tables"], {"SCRIPT_MUTATIONS": [ROW_900, ROW_1000]})
        with self.assertRaisesRegex(
            runner.PopulationRefused,
            "^scripts/mutation-rows.d/S100-S199.json and scripts/mutation-rows.d/S100-S199.json overlap$",
        ):
            runner.assemble(
                MONOLITH, [("S100-S199.json", {"tables": {}}), ("S100-S199.json", {"tables": {}})]
            )

    def test_each_comparison_of_assemble_is_observed(self):
        held = dict(MONOLITH, tables={"SCRIPT_MUTATIONS": [ROW_900, ROW_900]})
        with self.assertRaisesRegex(
            runner.PopulationRefused,
            "^the row id S901-A is held twice in scripts/mutation-rows.json$",
        ):
            runner.assemble(held, [])
        once = dict(MONOLITH, tables={"SCRIPT_MUTATIONS": [ROW_900]})
        self.assertEqual(runner.assemble(once, [])["tables"], {"SCRIPT_MUTATIONS": [ROW_900]})


class TheTreeFragments(unittest.TestCase):
    def test_each_return_value_of_tree_fragments_is_observed(self):
        self.assertEqual(runner.tree_fragments(scratch(self)), [])


class TheDeclaredSpellings(unittest.TestCase):
    def test_each_operand_of_declared_spellings_is_observed(self):
        lines = ["target spelling:  is repo-rooted", "target spelling: T is crate-root"]
        for line in examined("legend lines each operand alone refuses", lines):
            with self.assertRaisesRegex(
                runner.UnresolvableTarget, "declares no spelling this reader knows"
            ):
                runner.declared_spellings({"_": [line]})
        legend = ["target spelling: SCRIPT_MUTATIONS is repo-rooted, from the root"]
        self.assertEqual(
            runner.declared_spellings({"_": legend, "tables": {"SCRIPT_MUTATIONS": []}}),
            {"SCRIPT_MUTATIONS": "repo-rooted"},
        )

    def test_each_string_of_declared_spellings_is_observed(self):
        document = {"_": ["target spelling: T1 is repo-rooted"], "tables": {"T1": [], "T2": []}}
        with self.assertRaisesRegex(
            runner.UnresolvableTarget, "^table T2 declares no target spelling$"
        ):
            runner.declared_spellings(document)


class TheRowTarget(unittest.TestCase):
    def test_each_integer_of_row_target_is_observed(self):
        crate = {"MUTATIONS": "crate-relative"}
        with self.assertRaisesRegex(
            runner.UnresolvableTarget, "^MUTATIONS row 'S1-A' carries no crate and path$"
        ):
            runner.row_target("MUTATIONS", ["S1-A", "c"], crate)
        self.assertEqual(
            runner.row_target("MUTATIONS", ["S1-A", "c", "src/../src/x.rs"], crate),
            "crates/c/src/x.rs",
        )
        rooted = {"SCRIPT_MUTATIONS": "repo-rooted"}
        self.assertEqual(
            runner.row_target("SCRIPT_MUTATIONS", ["S1-A", "scripts/./x.py"], rooted),
            "scripts/x.py",
        )

    def test_each_string_of_row_target_is_observed(self):
        with self.assertRaisesRegex(
            runner.UnresolvableTarget, "^SCRIPT_MUTATIONS row '<no id>' carries no path$"
        ):
            runner.row_target("SCRIPT_MUTATIONS", [], {"SCRIPT_MUTATIONS": "repo-rooted"})


class TheTestFunctions(unittest.TestCase):
    def test_each_integer_of_test_functions_is_observed(self):
        self.assertEqual(runner.test_functions("#[test]\n\n\n\n\nfn x() {}\n", "x"), 0)
        self.assertEqual(runner.test_functions("#[test]\n\n\n\nfn x() {}\n", "x"), 1)


class TheLocateKiller(unittest.TestCase):
    def test_each_string_of_locate_killer_is_observed(self):
        root = killer_tree(self)
        self.assertEqual(runner.locate_killer(root, a_row()), SCRIPT_KILLER)
        integration = a_row(
            table="MUTATIONS", target="crates/c/src/lib.rs", crate="c", killer="it::t"
        )
        self.assertEqual(
            runner.locate_killer(root, integration),
            runner.Killer("cargo", "t", "crates/c/tests/it/main.rs", package="p", target="it"),
        )

    def test_a_cargo_row_is_located_in_its_crate(self):
        root = killer_tree(self)
        cargo = a_row(table="MUTATIONS", target="crates/c/src/lib.rs", crate="c", killer="lib::t")
        self.assertEqual(runner.locate_killer(root, cargo), LIB_KILLER)


class TheResolveKiller(unittest.TestCase):
    def test_each_return_value_of_resolve_killer_is_observed(self):
        root = killer_tree(self)
        self.assertEqual(runner.resolve_killer(root, a_row()), SCRIPT_KILLER)
        cargo = a_row(table="MUTATIONS", target="crates/c/src/lib.rs", crate="c", killer="lib::t")
        self.assertEqual(runner.resolve_killer(root, cargo), LIB_KILLER)

    def test_a_cargo_row_resolves_to_its_one_test_function(self):
        root = killer_tree(self)
        cargo = a_row(table="MUTATIONS", target="crates/c/src/lib.rs", crate="c", killer="lib::t")
        self.assertEqual(runner.resolve_killer(root, cargo), LIB_KILLER)
        absent = a_row(table="MUTATIONS", target="crates/c/src/lib.rs", crate="c", killer="lib::u")
        with self.assertRaisesRegex(
            runner.KillerUnresolved, "^names no test: crates/c/src declares u 0 times$"
        ):
            runner.resolve_killer(root, absent)


class TheResolveTest(unittest.TestCase):
    """`resolve_test` and `locate_test` over a row's crate and killer cells answer as
    `resolve_killer` and `locate_killer` answer over the row (SPEC-492 R5)."""

    def test_resolve_test_answers_as_resolve_killer_over_the_same_cells(self):
        root = killer_tree(self)
        cargo = {"table": "MUTATIONS", "target": "crates/c/src/lib.rs", "crate": "c"}
        integration = runner.Killer(
            "cargo", "t", "crates/c/tests/it/main.rs", package="p", target="it"
        )
        resolved = (
            (a_row(), SCRIPT_KILLER),
            (a_row(**cargo, killer="lib::t"), LIB_KILLER),
            (a_row(**cargo, killer="it::t"), integration),
        )
        for row, expected in examined("rows both resolvers resolve", resolved):
            with self.subTest(killer=row.killer):
                self.assertEqual(runner.resolve_killer(root, row), expected)
                self.assertEqual(runner.resolve_test(root, row.crate, row.killer), expected)
                self.assertEqual(runner.locate_killer(root, row), expected)
                self.assertEqual(runner.locate_test(root, row.crate, row.killer), expected)
        refused = (
            (a_row(**cargo, killer="lib::u"), "names no test: crates/c/src declares u 0 times"),
            (a_row(killer="m.C.test_u"), "names no test: scripts/tests/m.py has no C.test_u"),
        )
        for row, expected in examined("rows both resolvers refuse", refused):
            with self.subTest(killer=row.killer):
                with self.assertRaises(runner.KillerUnresolved) as by_row:
                    runner.resolve_killer(root, row)
                with self.assertRaises(runner.KillerUnresolved) as by_cells:
                    runner.resolve_test(root, row.crate, row.killer)
                self.assertEqual(
                    [str(by_row.exception), str(by_cells.exception)], [expected, expected]
                )


class TheCensus(unittest.TestCase):
    def test_each_integer_of_census_is_observed(self):
        findings, rows = runner.census(scratch(self) / "nothing")
        self.assertEqual(rows, 0)
        self.assertEqual(len(findings), 1)
        self.assertTrue(
            findings[0].startswith("census: the population does not assemble: "), findings
        )

    def test_each_loop_step_of_census_is_observed(self):
        rows = [
            ["S1-A", "scripts/absent.py", "x", "y", "d", "m.C.t"],
            ["S2-B", "scripts/present.py", "zz", "y", "d", "m.C.t"],
        ]
        root = tree(
            self,
            {
                "scripts/mutation-rows.json": json.dumps(population(*rows)),
                "scripts/present.py": "a = 1\n",
            },
        )
        self.assertEqual(
            runner.census(root),
            (
                [
                    "census: S1-A: its target scripts/absent.py does not exist",
                    "census: S2-B: its anchor occurs 0 times in scripts/present.py",
                    "census: S2-B: its killer m.C.t names no test: 0 unittest roots hold m.py",
                ],
                2,
            ),
        )


class TheCargoFlags(unittest.TestCase):
    def test_each_string_of_cargo_flags_is_observed(self):
        self.assertEqual(runner.cargo_flags(LIB_KILLER), ["--lib"])

    def test_each_return_value_of_cargo_flags_is_observed(self):
        self.assertEqual(runner.cargo_flags(LIB_KILLER), ["--lib"])
        integration = runner.Killer("cargo", "t", "crates/c/tests/it.rs", package="p", target="it")
        self.assertEqual(runner.cargo_flags(integration), ["--test", "it"])


class TheRunKiller(unittest.TestCase):
    def test_each_string_of_run_killer_is_observed(self):
        root = killer_tree(self)
        self.assertEqual(
            runner.run_killer(root, SCRIPT_KILLER, scratch(self)), runner.Run(1, True, "")
        )
        self.check_refusals()

    def check_refusals(self):
        root = killer_tree(self)
        cases = [
            (
                LIB_KILLER,
                {"raises": subprocess.TimeoutExpired(["cargo"], 1)},
                runner.Run(0, False, "it timed out"),
            ),
            (
                LIB_KILLER,
                {
                    "stdout": "running 1 test\ntest t ... ignored\n\ntest result: ok. 0 passed; "
                    "0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s\n"
                },
                runner.Run(1, False, "its test is ignored"),
            ),
            (
                SCRIPT_KILLER,
                {"stderr": "s\n" + "-" * 70 + "\nRan 1 test in 0.000s\n\nOK (skipped=1)\n"},
                runner.Run(1, False, "its test is skipped"),
            ),
        ]
        for killer, answer, expected in examined("killer runs that end in a note", cases):
            seen = []
            with mock.patch.object(runner, "run_in_own_group", answering(seen, **answer)):
                self.assertEqual(runner.run_killer(root, killer, scratch(self)), expected)

    def test_each_arithmetic_of_run_killer_is_observed(self):
        seen = []
        root = killer_tree(self)
        with mock.patch.object(
            runner, "run_in_own_group", answering(seen, stdout="running 1 test\n")
        ):
            runner.run_killer(root, LIB_KILLER, scratch(self))
        (command, cwd, _env, timeout), *rest = seen
        self.assertEqual(rest, [])
        self.assertEqual(
            command, ["cargo", "test", "--locked", "-p", "p", "--lib", "--", "--exact", "t"]
        )
        self.assertEqual(cwd, root)
        self.assertEqual(timeout, 4500)

    def test_each_return_value_of_run_killer_is_observed(self):
        self.check_refusals()

    def test_each_integer_of_run_killer_is_observed(self):
        self.check_refusals()

    def test_each_flag_of_run_killer_is_observed(self):
        self.check_refusals()


class TheShellParser(unittest.TestCase):
    def test_each_string_of_shell_parser_is_observed(self):
        self.assertEqual(runner.shell_parser("x.bash", b"#!/bin/sh \xff\necho\n"), "sh")
        self.assertEqual(runner.shell_parser("x.bash", b"echo\n#!/bin/sh\n"), "bash")


class TheParses(unittest.TestCase):
    def test_each_string_of_parses_is_observed(self):
        def stand_in(command, **_options):
            return subprocess.CompletedProcess(command, 2, b"", b"\xff: bad\nmore\n")

        with mock.patch.object(runner, "run_tool", stand_in):
            self.assertEqual(runner.parses("sh", b"x"), "the mutant does not parse: sh: �: bad")


class TheBuilds(unittest.TestCase):
    def test_each_flag_of_builds_is_observed(self):
        seen = []

        def stand_in(command, **options):
            seen.append((command, options))
            return subprocess.CompletedProcess(command, 0, "", "")

        root = killer_tree(self)
        row = a_row(table="MUTATIONS", target="crates/c/src/lib.rs", crate="c", killer="lib::t")
        with mock.patch.object(runner, "run_tool", stand_in):
            self.assertIsNone(runner.builds(root, row, LIB_KILLER, b"fn a() {}\n"))
        (command, options), *rest = seen
        self.assertEqual(rest, [])
        self.assertEqual(command, ["cargo", "test", "--locked", "-p", "p", "--lib", "--no-run"])
        self.assertEqual(
            {name: options[name] for name in ("cwd", "capture_output", "text", "timeout", "check")},
            {"cwd": root, "capture_output": True, "text": True, "timeout": 3600, "check": False},
        )
        self.assertEqual(options["env"]["CARGO_TERM_COLOR"], "never")


class TheSha256(unittest.TestCase):
    def test_each_return_value_of_sha256_is_observed(self):
        self.assertEqual(
            runner.sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        )


def entry(verdict, reason, control=None, mutant=None, target="scripts/t.py"):
    return {
        "id": "S1-A",
        "target": target,
        "control": control,
        "mutant": mutant,
        "sha256": TARGET_SHA,
        "verdict": verdict,
        "reason": reason,
    }


class TheProveRow(unittest.TestCase):
    def proof(self, row, runs, files=None):
        """`prove_row` over the fixture with each killer run answered in turn by `runs`; the
        target's bytes are read back after it."""
        root = killer_tree(self)
        for relative, text in (files or {}).items():
            (root / relative).write_text(text, encoding="utf-8")
        answers = iter(runs)
        with mock.patch.object(
            runner, "run_killer", lambda _root, _killer, _scratch: next(answers)
        ):
            found = runner.prove_row(root, row, scratch(self))
        self.assertEqual((root / row.target).read_text(encoding="utf-8"), "VALUE = 1\n")
        return found

    def test_each_string_of_prove_row_is_observed(self):
        self.check_verdicts()

    def check_verdicts(self):
        cargo = a_row(table="MUTATIONS", target="crates/c/src/lib.rs", crate="c", killer="lib::t")
        cases = [
            (
                a_row(killer="absent.C.t"),
                [],
                entry(
                    "VOID", "its killer absent.C.t names no test: 0 unittest roots hold absent.py"
                ),
            ),
            (
                a_row(find="VALUE = 1", replace="VALUE = 2"),
                [runner.Run(0, False, "it timed out")],
                entry("VOID", "without its mutant, it timed out", {"selected": 0, "passed": False}),
            ),
            (
                a_row(),
                [runner.Run(1, True), runner.Run(0, False, "it timed out")],
                entry(
                    "VOID",
                    "with its mutant, it timed out",
                    {"selected": 1, "passed": True},
                    {"selected": 0, "passed": False},
                ),
            ),
            (
                a_row(),
                [runner.Run(1, True), runner.Run(2, False)],
                entry(
                    "VOID",
                    "with its mutant, its killer selected 2 tests",
                    {"selected": 1, "passed": True},
                    {"selected": 2, "passed": False},
                ),
            ),
            (
                a_row(),
                [runner.Run(1, True), runner.Run(1, True)],
                entry(
                    "SURVIVED",
                    "its killer passed with the mutant installed",
                    {"selected": 1, "passed": True},
                    {"selected": 1, "passed": True},
                ),
            ),
            (
                a_row(),
                [runner.Run(1, True), runner.Run(1, False)],
                entry(
                    "KILLED",
                    "its killer passed without the mutant and failed with it",
                    {"selected": 1, "passed": True},
                    {"selected": 1, "passed": False},
                ),
            ),
        ]
        for row, runs, expected in examined("proofs, one per verdict", cases):
            self.assertEqual(self.proof(row, runs), expected)
        no_package = self.proof(
            dataclasses.replace(
                cargo, find="VALUE = 1", replace="VALUE = 2", target="scripts/t.py"
            ),
            [],
            {"crates/c/Cargo.toml": "not toml ["},
        )
        self.assertEqual(
            no_package,
            entry("VOID", "its killer lib::t runs in crates/c, which holds no Cargo package"),
        )

    def test_each_return_value_of_prove_row_is_observed(self):
        self.check_verdicts()

    def test_each_integer_of_prove_row_is_observed(self):
        seen = []

        def build(_root, _row, _killer, mutated):
            seen.append(mutated)
            return "planted"

        root = killer_tree(self)
        with (
            mock.patch.object(runner, "builds", build),
            mock.patch.object(
                runner, "run_killer", lambda _root, _killer, _scratch: runner.Run(1, True)
            ),
        ):
            found = runner.prove_row(root, a_row(), scratch(self))
        self.assertEqual(seen, [b"VALUE = 2\n"])
        self.assertEqual(found, entry("VOID", "planted", {"selected": 1, "passed": True}))


def prove_args(report=None):
    return Namespace(all=True, row=[], band=None, rows_from=None, report=report)


THREE_ROWS = population(
    ["S1-A", "scripts/a.py", "x", "y", "d", "m.C.t"],
    ["S2-B", "scripts/b.py", "x", "y", "d", "m.C.t"],
    ["S3-C", "scripts/c.py", "x", "y", "d", "m.C.t"],
)
THREE_ENTRIES = {
    "S1-A": {"id": "S1-A", "verdict": "KILLED", "reason": "k"},
    "S2-B": {"id": "S2-B", "verdict": "SURVIVED", "reason": "s"},
    "S3-C": {"id": "S3-C", "verdict": "VOID", "reason": "v"},
}


def proving(test, root, args, prove_row=None, load_tree=None):
    """`prove` over `root` with no tracked change; (its exit, what it printed)."""
    out = io.StringIO()
    rows = load_tree or (lambda _root: THREE_ROWS)
    one = prove_row or (lambda _root, row, _scratch: dict(THREE_ENTRIES[row.id]))
    with (
        contextlib.redirect_stdout(out),
        mock.patch.object(runner, "tracked_changes", lambda _root: []),
        mock.patch.object(runner, "load_tree", rows),
        mock.patch.object(runner, "prove_row", one),
    ):
        code = runner.prove(root, args)
    return code, out.getvalue()


def refusing(_root):
    raise runner.PopulationRefused("planted")


def unrestored(_root, row, _scratch):
    raise runner.RestoreFailed(f"{row.target} was not restored byte for byte after {row.id}")


class TheProve(unittest.TestCase):
    def test_each_return_value_of_prove_is_observed(self):
        root = scratch(self)
        self.assertEqual(
            proving(self, root, prove_args(), load_tree=refusing), (2, "prove: REFUSED: planted\n")
        )
        self.assertEqual(
            proving(self, root, prove_args(), prove_row=unrestored),
            (4, "prove: RESTORE FAILED: scripts/a.py was not restored byte for byte after S1-A\n"),
        )

    def test_each_arithmetic_of_prove_is_observed(self):
        self.assertEqual(
            proving(self, scratch(self), prove_args()),
            (
                1,
                "S1-A: KILLED: k\nS2-B: SURVIVED: s\nS3-C: VOID: v\n"
                "rows: examined 3: killed 1, survived 1, void 1\nexamined 3\n",
            ),
        )

    def test_each_integer_of_prove_is_observed(self):
        report = scratch(self) / "report.json"
        proving(self, scratch(self), prove_args(str(report)))
        self.assertEqual(report.read_text(encoding="utf-8"), REPORT_TEXT)

    def test_each_string_of_prove_is_observed(self):
        report = scratch(self) / "report.json"
        proving(self, scratch(self), prove_args(str(report)))
        self.assertEqual(report.read_text(encoding="utf-8"), REPORT_TEXT)


REPORT_TEXT = (
    "[\n"
    "  {\n"
    '    "id": "S1-A",\n'
    '    "verdict": "KILLED",\n'
    '    "reason": "k"\n'
    "  },\n"
    "  {\n"
    '    "id": "S2-B",\n'
    '    "verdict": "SURVIVED",\n'
    '    "reason": "s"\n'
    "  },\n"
    "  {\n"
    '    "id": "S3-C",\n'
    '    "verdict": "VOID",\n'
    '    "reason": "v"\n'
    "  }\n"
    "]\n"
)


class Recorder:
    """A stdout that records each write and each flush, in order."""

    def __init__(self, events):
        self.events = events

    def write(self, text):
        self.events.append(("write", text))
        return len(text)

    def flush(self):
        self.events.append(("flush",))


class TheProof(unittest.TestCase):
    def test_a_target_not_restored_exits_four(self):
        code, _out = proving(self, scratch(self), prove_args(), prove_row=unrestored)
        self.assertEqual(code, 4)

    def test_each_rows_line_is_flushed_before_the_next_row_runs(self):
        events = []

        def one(_root, row, _scratch):
            events.append(("prove", row.id))
            return dict(THREE_ENTRIES[row.id])

        with (
            contextlib.redirect_stdout(Recorder(events)),
            mock.patch.object(runner, "tracked_changes", lambda _root: []),
            mock.patch.object(runner, "load_tree", lambda _root: THREE_ROWS),
            mock.patch.object(runner, "prove_row", one),
        ):
            runner.prove(scratch(self), prove_args())
        self.assertEqual(
            events[:9],
            [
                ("prove", "S1-A"),
                ("write", "S1-A: KILLED: k"),
                ("write", "\n"),
                ("flush",),
                ("prove", "S2-B"),
                ("write", "S2-B: SURVIVED: s"),
                ("write", "\n"),
                ("flush",),
                ("prove", "S3-C"),
            ],
        )


def retiring(test, root, before):
    """`retired` over `root` against a base population `before`; (its exit, what it printed)."""
    now = population(["S1-A", "scripts/a.py", "x", "y", "d", "m.C.t"])
    out = io.StringIO()
    with (
        contextlib.redirect_stdout(out),
        mock.patch.object(runner, "load_revision", lambda _root, _base: before),
        mock.patch.object(runner, "load_tree", lambda _root: now),
    ):
        code = runner.retired(root, "base")
    return code, out.getvalue()


class TheRetired(unittest.TestCase):
    def test_each_string_of_retired_is_observed(self):
        self.assertEqual(
            retiring(self, scratch(self), None), (0, "retired: base holds no rows\nexamined 0\n")
        )
        self.check_records()

    def check_records(self):
        record = {
            "retired": [
                {"id": "S2-B", "reason": "r"},
                {"id": "S3-C", "approval": "a"},
                {"id": "S4-D", "reason": "r", "approval": "owner"},
            ]
        }
        root = tree(
            self,
            {
                "scripts/mutation-rows.retired.json": json.dumps(record),
                "scripts/a.py": "",
                "scripts/b.py": "",
                "scripts/c.py": "",
                "scripts/d.py": "",
            },
        )
        before = population(
            ["S1-A", "scripts/a.py", "x", "y", "d", "m.C.t"],
            ["S2-B", "scripts/b.py", "x", "y", "d", "m.C.t"],
            ["S3-C", "scripts/c.py", "x", "y", "d", "m.C.t"],
            ["S4-D", "scripts/d.py", "x", "y", "d", "m.C.t"],
        )
        self.assertEqual(
            retiring(self, root, before),
            (
                1,
                "retired: S2-B: REFUSED: it left while its target scripts/b.py stays, and "
                "scripts/mutation-rows.retired.json records no reason and approval for it\n"
                "retired: S3-C: REFUSED: it left while its target scripts/c.py stays, and "
                "scripts/mutation-rows.retired.json records no reason and approval for it\n"
                "retired: S4-D: its target stays; retired with approval: owner\n"
                "examined 4\n",
            ),
        )

    def test_each_return_value_of_retired_is_observed(self):
        self.assertEqual(
            retiring(self, scratch(self), None), (0, "retired: base holds no rows\nexamined 0\n")
        )

    def test_each_operand_of_retired_is_observed(self):
        self.check_records()

    def test_each_loop_step_of_retired_is_observed(self):
        self.check_records()


DOC_FIRST_PARAGRAPH = (
    "mutation_rows: DeckStreak's hand-proved mutation rows, read, censused, proved and retired "
    "(SPEC-039 R8 to R11, ADR-057)."
)
#: Each option's help beside its own option, flattened: `--band`'s text holds `--all`'s, so a
#: help text alone would still be found when the mutant empties `--all`'s.
HELP_TEXTS = (
    "--all prove every row",
    "--row ROW prove this row id",
    "--band BAND prove every row of the band S<lo>-S<hi>",
    "--rows-from ROWS_FROM prove the rows a mutation-verdict plan selected",
    "--report REPORT write each row's proof here, as JSON",
    "--base BASE retired: the revision to compare the rows against",
)


class TheMain(unittest.TestCase):
    def test_each_return_value_of_main_is_observed(self):
        root = str(scratch(self))
        with mock.patch.object(runner, "census", lambda _root: ([], 3)):
            self.assertEqual(call_main("census", "--root", root), (0, "examined 3 row(s)\n", ""))
        with mock.patch.object(runner, "census", lambda _root: ([], 0)):
            self.assertEqual(call_main("census", "--root", root), (3, "examined 0 row(s)\n", ""))

        def refused(*_args):
            raise runner.PopulationRefused("planted")

        with mock.patch.object(runner, "retired", refused):
            self.assertEqual(
                call_main("retired", "--base", "x", "--root", root),
                (2, "", "mutation_rows: REFUSED: planted\n"),
            )
        with mock.patch.object(runner, "load_tree", refused):
            self.assertEqual(
                call_main("count", "--root", root), (2, "", "mutation_rows: REFUSED: planted\n")
            )

    def test_each_integer_of_main_is_observed(self):
        code, out, _err = call_main("--help")
        self.assertEqual(code, ("exit", 0))
        self.assertIn(DOC_FIRST_PARAGRAPH, flat(out))
        self.assertNotIn("python3 scripts/mutation_rows.py count|ids", out)
        seen = []

        def spy(root):
            seen.append(root)
            raise runner.PopulationRefused("planted")

        with mock.patch.object(runner, "load_tree", spy):
            call_main("count")
        self.assertEqual(seen, [REPO])

    def test_each_string_of_main_is_observed(self):
        _code, out, _err = call_main("--help")
        for text in examined("help texts", HELP_TEXTS):
            self.assertEqual(flat(out).count(text), 1, text)
        root = str(scratch(self))
        code, _out, err = call_main("prove", "--root", root)
        self.assertEqual(code, ("exit", 2))
        self.assertIn("error: prove needs --all, --row, --band or --rows-from", err)
        code, _out, err = call_main("retired", "--root", root)
        self.assertEqual(code, ("exit", 2))
        self.assertIn("error: retired needs --base", err)
        with mock.patch.object(runner, "census", lambda _root: (["a", "b"], 2)):
            self.assertEqual(
                call_main("census", "--root", root), (1, "a\nb\nexamined 2 row(s)\n", "")
            )


#: The functions whose equivalent shapes the cure removed (SPEC-492 R4), each judged when the
#: runner defines it.
SHAPED = (
    "declared_spellings",
    "locate_killer",
    "locate_test",
    "resolve_killer",
    "resolve_test",
    "shell_parser",
    "run_killer",
    "prove",
)


def maxsplit_reads(function):
    """Each `split`/`rsplit` call with a maxsplit of 1 read at [0] or [-1], as its text."""
    found = []
    for node in ast.walk(function):
        if not (isinstance(node, ast.Subscript) and isinstance(node.value, ast.Call)):
            continue
        call = node.value
        if not (isinstance(call.func, ast.Attribute) and call.func.attr in ("split", "rsplit")):
            continue
        limits = call.args[1:2] + [each.value for each in call.keywords if each.arg == "maxsplit"]
        one = any(isinstance(each, ast.Constant) and each.value == 1 for each in limits)
        index = node.slice
        first = isinstance(index, ast.Constant) and index.value == 0
        last = (
            isinstance(index, ast.UnaryOp)
            and isinstance(index.op, ast.USub)
            and isinstance(index.operand, ast.Constant)
            and index.operand.value == 1
        )
        if one and (first or last):
            found.append(ast.unparse(node))
    return found


def prefixed_scratch(function):
    """Each `tempfile.mkdtemp` or `tempfile.TemporaryDirectory` call given a `prefix=`."""
    found = []
    for node in ast.walk(function):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr in ("mkdtemp", "TemporaryDirectory")
            and isinstance(node.func.value, ast.Name)
            and node.func.value.id == "tempfile"
            and any(each.arg == "prefix" for each in node.keywords)
        ):
            found.append(ast.unparse(node))
    return found


class TheEquivalentShapesAreGone(unittest.TestCase):
    def test_the_rows_runner_keeps_none_of_its_equivalent_shapes(self):
        tree_ = ast.parse(RUNNER.read_text(encoding="utf-8"))
        defined = {
            node.name: node
            for node in tree_.body
            if isinstance(node, ast.FunctionDef) and node.name in SHAPED
        }
        shapes = []
        for name in examined("functions the cure judges", [n for n in SHAPED if n in defined]):
            shapes += [f"{name}: {text}" for text in maxsplit_reads(defined[name])]
            shapes += [f"{name}: {text}" for text in prefixed_scratch(defined[name])]
        self.assertEqual(shapes, [])


if __name__ == "__main__":
    unittest.main()
