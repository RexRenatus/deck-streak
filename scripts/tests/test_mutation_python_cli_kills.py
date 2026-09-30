"""The Python mutation runner's command line: `prepare`, `listing`, `run` and `main` (SPEC-087).

Each test calls the runner's `main` in this process, over a planted git repository, so a returned
value is read as it is returned: a `return None` exits a process as success, where the same call
here compares against the exit it should be. What the runner prints and writes is compared whole.
"""

import contextlib
import importlib.util
import io
import json
import re
import sys
import unittest
from pathlib import Path
from unittest import mock

from test_mutation_python import BROKEN, CALC, QUIT, RUNNER, SPIN, Tree

SCHEMA = "deckstreak.mutation-python.v1"
CALC_NAMES = [
    "scripts/calc.py:2:12: replace return value with return None in double",
    "scripts/calc.py:2:14: replace * with / in double",
    "scripts/calc.py:2:16: replace 2 with 3 in double",
]
USAGE = "mutation-python: usage: "


def runner():
    spec = importlib.util.spec_from_file_location("mutation_python_cli_kills", RUNNER)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def call(tree, *args):
    """`main` over the tree: (its return value, what it printed, what it wrote to stderr)."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = runner().main([*args, "--root", str(tree.root)])
        except Exception as error:
            code = f"raised {error!r}"
    return code, out.getvalue(), err.getvalue()


class ThePreparationRefusesWithItsUsageExit(unittest.TestCase):
    def test_each_refusal_is_the_usage_exit_and_names_its_reason(self):
        tree = Tree(self, {"calc": CALC})
        for verb in ("list", "run"):
            with self.subTest(verb=verb, case="two selections"):
                code, out, err = call(tree, verb, "--all", "--file", "scripts/calc.py")
                self.assertEqual(
                    (code, out, err),
                    (2, "", USAGE + "exactly one of --plan, --all and --file\n"),
                )
            with self.subTest(verb=verb, case="no selection"):
                code, out, err = call(tree, verb)
                self.assertEqual(
                    (code, out, err),
                    (2, "", USAGE + "exactly one of --plan, --all and --file\n"),
                )
            with self.subTest(verb=verb, case="a file outside the population"):
                code, out, err = call(tree, verb, "--file", "scripts/elsewhere.py")
                self.assertEqual(
                    (code, out, err),
                    (2, "", USAGE + "--file scripts/elsewhere.py is outside the population\n"),
                )
            with self.subTest(verb=verb, case="an unreadable plan"):
                code, out, err = call(tree, verb, "--plan", str(tree.root / "absent.json"))
                self.assertEqual((code, out), (2, ""))
                self.assertTrue(err.startswith(USAGE + "the selection cannot be read: "), err)
                self.assertEqual(err.count("\n"), 1)
        refused = Tree(self, {"calc": CALC}, mapping={})
        for verb in ("list", "run"):
            with self.subTest(verb=verb, case="a map the census refuses"):
                code, out, err = call(refused, verb, "--all")
                self.assertEqual(
                    (code, out, err),
                    (
                        2,
                        "",
                        "mutation-python: REFUSED: scripts/calc.py: the map omits this "
                        "population file\n" + USAGE + "the map is refused by the census\n",
                    ),
                )

    def test_a_shard_that_is_not_k_of_n_is_the_usage_exit(self):
        tree = Tree(self, {"calc": CALC})
        code, out, err = call(tree, "list", "--all", "--shard", "x")
        self.assertEqual((code, out), (2, ""))
        self.assertEqual(err, USAGE + "--shard 'x' is not k/n\n")


class TheListingCarriesEachMutantWhole(unittest.TestCase):
    def test_the_listing_prints_its_count_and_names_and_writes_the_whole_document(self):
        tree = Tree(self, {"calc": CALC})
        target = tree.root / "listed.json"
        code, out, err = call(tree, "list", "--all", "--out", str(target))
        self.assertEqual(code, 0)
        self.assertEqual(err, "")
        self.assertEqual(out, "mutation-python: listed 3\n" + "".join(n + "\n" for n in CALC_NAMES))
        expected = [
            ("return value with return None in double", "return", 12, 17),
            ("* with / in double", "arithmetic", 14, 17),
            ("2 with 3 in double", "integer constant", 16, 17),
        ]
        document = {
            "schema": SCHEMA,
            "listed": 3,
            "mutants": [
                {
                    "name": name,
                    "file": "scripts/calc.py",
                    "line": 2,
                    "column": column,
                    "start_line": 2,
                    "end_line": 2,
                    "end_column": end,
                    "mutant": "replace " + text,
                    "operator": operator,
                }
                for name, (text, operator, column, end) in zip(CALC_NAMES, expected, strict=True)
            ],
        }
        self.assertEqual(target.read_text(encoding="utf-8"), json.dumps(document, indent=2) + "\n")
        self.assertEqual(
            target.read_text(encoding="utf-8").split("\n")[1], '  "schema": "' + SCHEMA + '",'
        )
        self.assertTrue(target.read_text(encoding="utf-8").endswith("}\n"))


class TheRunPrintsAndWritesItsReport(unittest.TestCase):
    def report_of(self, tree, *args):
        target = tree.root.parent / f"{tree.root.name}-cli.json"
        code, out, err = call(tree, "run", *args, "--report", str(target))
        text = target.read_text(encoding="utf-8")
        target.unlink()
        return code, out, err, text

    def test_a_clean_run_is_exit_zero_and_prints_the_counts_and_the_selection_it_ran(self):
        tree = Tree(self, {"calc": CALC})
        plan = tree.root.parent / f"{tree.root.name}-plan.json"
        plan.write_text(
            json.dumps(
                {
                    "classes": {"scripts": {"applies": True}},
                    "files": [{"path": "scripts/calc.py", "class": "scripts", "code": [2]}],
                }
            ),
            encoding="utf-8",
        )
        counts = (
            "mutation-python: killed 3, survived 0, uncovered 0, unviable 0, timeout 0, void 0\n"
        )
        for selection, args in (
            ("plan", ["--plan", str(plan)]),
            ("all", ["--all"]),
            ("file", ["--file", "scripts/calc.py"]),
        ):
            with self.subTest(selection=selection):
                code, out, err, text = self.report_of(tree, *args, "--test-seconds", "5")
                self.assertEqual((code, out, err), (0, counts + "examined 3\n", ""))
                document = json.loads(text)
                self.assertEqual(document["selection"], selection)
                self.assertEqual(document["exit"], 0)
                self.assertEqual(text, json.dumps(document, indent=2) + "\n")
                self.assertEqual(text.split("\n")[1], '  "schema": "' + SCHEMA + '",')

    def test_the_scratch_is_removed_and_its_removal_never_raises(self):
        tree = Tree(self, {"calc": CALC})
        module = runner()
        with mock.patch.object(module.shutil, "rmtree") as removed:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = module.main(["run", "--all", "--root", str(tree.root)])
        self.assertEqual(code, 0)
        self.assertEqual(removed.call_count, 1)
        self.assertEqual(removed.call_args.kwargs, {"ignore_errors": True})

    def test_a_tree_git_cannot_read_is_the_usage_exit(self):
        tree = Tree(self, {"calc": CALC})
        module = runner()
        out, err = io.StringIO(), io.StringIO()
        with (
            mock.patch.object(module.mutation_rows, "tracked_changes", side_effect=OSError("gone")),
            contextlib.redirect_stdout(out),
            contextlib.redirect_stderr(err),
        ):
            code = module.main(["run", "--all", "--root", str(tree.root)])
        self.assertEqual(
            (code, out.getvalue(), err.getvalue()),
            (2, "", USAGE + "the tree cannot be read by git: gone\n"),
        )

    def test_a_dirty_tree_is_the_usage_exit(self):
        tree = Tree(self, {"calc": CALC})
        tree.write("scripts/calc.py", CALC + "# a tracked change\n")
        code, out, err = call(tree, "run", "--all")
        self.assertEqual((code, out), (2, ""))
        self.assertEqual(
            err,
            USAGE + "the tree has tracked changes, so nothing was mutated: scripts/calc.py\n",
        )

    def test_a_voided_file_is_named_once_and_its_mutants_are_not_named_again(self):
        tree = Tree(self, {"broken": BROKEN})
        code, out, err, _ = self.report_of(tree, "--all", "--test-seconds", "3")
        self.assertEqual(code, 3)
        self.assertEqual(
            out,
            "mutation-python: VOID: scripts/broken.py: the control failed unmutated: "
            "test_broken.Five.test_five_is_six\n"
            "mutation-python: killed 0, survived 0, uncovered 0, unviable 0, timeout 0, void 2\n"
            "examined 0\n",
        )

    def test_a_timeout_and_a_void_mutant_are_each_named_in_a_healthy_file(self):
        spin = Tree(self, {"spin": SPIN})
        code, out, err, _ = self.report_of(spin, "--all", "--test-seconds", "3")
        self.assertEqual(code, 3)
        self.assertEqual(
            out,
            "mutation-python: VOID: timeout: scripts/spin.py:4:15: replace + with - in count\n"
            "mutation-python: killed 3, survived 1, uncovered 0, unviable 0, timeout 1, void 0\n"
            "examined 4\n",
        )
        quit_ = Tree(self, {"quit": QUIT})
        code, out, err, _ = self.report_of(quit_, "--all", "--test-seconds", "3")
        self.assertEqual(code, 3)
        self.assertEqual(
            out,
            "mutation-python: VOID: void: scripts/quit.py:5:10: replace == with != in value\n"
            "mutation-python: killed 1, survived 2, uncovered 0, unviable 0, timeout 0, void 1\n"
            "examined 3\n",
        )


class TheMainReturnsItsExit(unittest.TestCase):
    def test_help_is_exit_zero_and_describes_the_runner_by_its_first_paragraph_only(self):
        module = runner()
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = module.main(["--help"])
        self.assertEqual(code, 0)
        paragraphs = module.__doc__.split("\n\n")
        squash = lambda text: re.sub(r"\s+", " ", text).strip()  # noqa: E731
        shown = squash(out.getvalue())
        self.assertIn(squash(paragraphs[0]), shown)
        self.assertNotIn(squash(paragraphs[1]), shown)

    def test_an_unknown_argument_is_the_usage_exit_and_the_census_verb_returns_its_exit(self):
        tree = Tree(self, {"calc": CALC})
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(runner().main(["list", "--nonsense"]), 2)
        code, out, err = call(tree, "census")
        self.assertEqual((code, out, err), (0, "examined 1\n", ""))

    def test_the_list_verb_returns_its_exit_and_the_repository_map_runs_the_module(self):
        self.assertEqual(Path(RUNNER).name, "mutation_python.py")
        tree = Tree(self, {"calc": CALC})
        code, _, _ = call(tree, "list", "--all")
        self.assertIs(type(code), int)
        self.assertEqual(code, 0)
