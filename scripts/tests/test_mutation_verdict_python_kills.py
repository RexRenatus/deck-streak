"""The verdict's Python paths, held by whole observables (SPEC-087 R11, R12, R14).

Each test calls the verdict's own functions over a fixture tree and compares a whole result: the
plan's class table, a listing, the voids a set of shard reports raises, the counts a class
examines, the problems a record draws. A mutant of one of those tokens changes the observable.
"""

import contextlib
import io
import json
import tempfile
import unittest
from unittest import mock
from argparse import Namespace
from collections import defaultdict
from pathlib import Path

from test_mutation_python_verdict import (
    FILES,
    GENERATOR,
    GENERATOR_HEAD,
    REACHED,
    SCRIPT,
    SCRIPT_HEAD,
    a_record,
    file_entry,
    listed,
    report_of,
    write_shard,
)
from test_mutation_verdict import verdict_module

V = verdict_module()
BIG = "def guard(x):\n    return x + 2 + 3 + 4 + 5\n"


def scratch(test):
    holder = tempfile.TemporaryDirectory()
    test.addCleanup(holder.cleanup)
    return Path(holder.name)


def tree(test, extra=None):
    """A root holding the fixture's scripts, tests and population map."""
    root = scratch(test) / "repo"
    files = dict(FILES)
    files[SCRIPT] = SCRIPT_HEAD
    files[GENERATOR] = GENERATOR_HEAD
    files.update(extra or {})
    for relative, text in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return root


def quiet(call, *args):
    sink = io.StringIO()
    with contextlib.redirect_stdout(sink):
        result = call(*args)
    return result, sink.getvalue()


def entry_by_outcome(path, text, outcomes, void=None):
    """A report entry for `text`'s first mutants, one outcome each in order."""
    entry = file_entry(path, text, {}, void=void)
    for mutant, outcome in zip(entry["mutants"], outcomes, strict=False):
        mutant["outcome"] = outcome
    entry["mutants"] = entry["mutants"][: len(outcomes)]
    return entry


class ThePlanAndTheListingAreRead(unittest.TestCase):
    def test_a_not_applicable_plan_names_each_class_with_no_files(self):
        out = scratch(self) / "out"
        plan = V.plan_diff(scratch(self), "b", "h", out, ("not-applicable", "why"))
        expected = {
            name: {"applies": False, "files": []} for name in ("rust", "web", "oracle", "scripts")
        }
        self.assertEqual(plan.classes, expected)
        written = json.loads((out / "plan.json").read_text(encoding="utf-8"))
        self.assertEqual(written["classes"], expected)

    def test_a_python_listing_is_of_the_schema_and_holds_a_list_of_mutants(self):
        home = scratch(self)

        def read(document):
            path = home / "listing.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            return V.python_listing(str(path))

        good = {"schema": V.PYTHON_SCHEMA, "mutants": [{"a": 1}, "stray", {"b": 2}]}
        self.assertEqual(read(good), [{"a": 1}, {"b": 2}])
        self.assertIsNone(read({**good, "schema": "other"}))
        self.assertIsNone(read({**good, "mutants": "none"}))
        self.assertIsNone(read({"schema": V.PYTHON_SCHEMA}))
        self.assertIsNone(read([good]))
        self.assertIsNone(V.python_listing(None))

    def test_the_rows_are_proved_for_rust_oracle_and_scripts_and_never_for_web(self):
        found = {
            klass: V.klass_rows({}, Namespace(klass=klass))
            for klass in ("rust", "oracle", "scripts", "web", "")
        }
        self.assertEqual(
            found, {"rust": True, "oracle": True, "scripts": True, "web": False, "": False}
        )


class TheShardReportsAreReadWhole(unittest.TestCase):
    def reports(self, documents):
        """A reports directory: shard n holds documents[n], a str verbatim, None nothing."""
        home = scratch(self) / "reports"
        for index, document in enumerate(documents):
            if document is None:
                continue
            raw = document if isinstance(document, str) else None
            write_shard(home, index, document, raw=raw)
        return home

    def read(self, documents, count=None, directory=True):
        home = self.reports(documents) if directory else None
        verdict = V.Verdict("scripts")
        plan = {"python": {"count": len(documents) if count is None else count}}
        whole, _ = quiet(V.python_reports, verdict, plan, str(home) if home else None)
        return whole, verdict.voids

    def test_a_plan_with_no_python_shards_promised_no_report(self):
        whole, voids = self.read([], count=0)
        self.assertEqual(whole, [])
        self.assertEqual(
            voids, ["the plan names no python shards, so no shard's report was promised"]
        )
        verdict = V.Verdict("scripts")
        quiet(V.python_reports, verdict, {}, "somewhere")
        self.assertEqual(len(verdict.voids), 1)

    def test_no_directory_leaves_every_shard_without_a_report(self):
        whole, voids = self.read([report_of([])] * 2, directory=False)
        self.assertEqual(whole, [])
        self.assertEqual(
            voids,
            ["mutation-python-shard-0: no report", "mutation-python-shard-1: no report"],
        )

    def test_a_bad_shard_is_named_and_the_shards_after_it_are_still_read(self):
        good = report_of([])
        wrong = {**report_of([]), "schema": "other"}
        whole, voids = self.read([None, "{", wrong, good])
        self.assertEqual(whole, [("mutation-python-shard-3", good)])
        self.assertEqual(
            voids,
            [
                "mutation-python-shard-0: no report",
                "mutation-python-shard-1: unreadable",
                f"mutation-python-shard-2: not of the schema {V.PYTHON_SCHEMA}",
            ],
        )

    def test_a_restore_failure_is_void_by_its_exit_or_by_its_own_word(self):
        exit_four = report_of([], code=4)
        named = report_of([], code=0, restore_failed="the tree was left mutated")
        both = report_of([], code=4, restore_failed="a lock")
        other = report_of([], code=5)
        empty = report_of([], code=0, restore_failed="")
        whole, voids = self.read([exit_four, named, both, other, empty])
        self.assertEqual(
            voids,
            [
                "mutation-python-shard-0: a restore failed: exit 4",
                "mutation-python-shard-1: a restore failed: the tree was left mutated",
                "mutation-python-shard-2: a restore failed: a lock",
            ],
        )
        self.assertEqual(
            [where for where, _ in whole],
            [
                "mutation-python-shard-3",
                "mutation-python-shard-4",
            ],
        )

    def test_the_battery_counts_a_restore_failure_partial_by_exit_or_by_word(self):
        cases = [
            (report_of([], code=4), True),
            (report_of([], code=0, restore_failed="x"), True),
            (report_of([], code=5), False),
            (report_of([], code=0), False),
        ]
        for report, partial in cases:
            home = self.reports([report])
            code, said = quiet(V.battery, home, 0, "python", None, 1)
            self.assertEqual(code, 1 if partial else 0, said)
            self.assertEqual(
                "battery: PARTIAL mutation-python-shard-0: a restore failed" in said, partial, said
            )

    def test_the_table_reads_each_shard_and_counts_what_it_killed(self):
        root = tree(self)
        for report, voided in [
            (report_of([], code=4), True),
            (report_of([], code=0, restore_failed="x"), True),
            (report_of([], code=5), False),
        ]:
            voids = []
            tallies = defaultdict(V.Tally)
            sink = io.StringIO()
            with contextlib.redirect_stdout(sink):
                read = V.table_python(
                    root, self.reports([report]), tallies, print, voids.append, False
                )
            self.assertEqual(voids, ["mutation-python-shard-0: a restore failed"] if voided else [])
            self.assertEqual(read, 0 if voided else 1)
        empty = scratch(self) / "none"
        empty.mkdir()
        tallies = defaultdict(V.Tally)
        self.assertEqual(V.table_python(root, empty, tallies, print, print, False), 0)
        self.assertEqual(dict(tallies), {})
        every = listed(SCRIPT, SCRIPT_HEAD)
        report = report_of(
            [
                file_entry(GENERATOR, GENERATOR_HEAD, {}, void="the sentinel left no test"),
                file_entry(SCRIPT, SCRIPT_HEAD, {}),
            ]
        )
        voids, findings = [], []
        tallies = defaultdict(V.Tally)
        read, _ = quiet(
            V.table_python,
            root,
            self.reports([report]),
            tallies,
            findings.append,
            voids.append,
            False,
        )
        tally = tallies[V.PYTHON_CLASS]
        self.assertEqual((read, len(voids), findings), (1, 1, []))
        self.assertEqual((tally.killed, tally.unexplained), (len(every), 0))


class TheJudgeExaminesWhatTheShardsCarry(unittest.TestCase):
    def judge(self, files, root=None, records=None):
        root = root or tree(self)
        home = scratch(self) / "reports"
        write_shard(home, 0, report_of(files))
        verdict = V.Verdict("scripts")
        plan = {
            "rows": [],
            "classes": {"scripts": {"applies": True}},
            "files": [],
            "python": {"count": 1},
        }
        args = Namespace(
            klass="scripts", rows=None, python=str(home), python_whole=None, root=str(root)
        )
        _, said = quiet(V.judge_python, verdict, plan, args)
        return verdict, said

    def test_a_void_file_and_a_timeout_do_not_stop_the_reading_and_the_counts_add_up(self):
        outcomes = ["timeout", "killed", "killed", "survived", "uncovered"]
        files = [
            file_entry(SCRIPT, SCRIPT_HEAD, {}, void="the sentinel left no test"),
            entry_by_outcome(SCRIPT, BIG, outcomes),
        ]
        verdict, said = self.judge(files)
        self.assertIn("examined 4: generated 4, rows 0", said)
        self.assertEqual(verdict.examined, 4)
        self.assertEqual(len(verdict.voids), 2)
        self.assertEqual(len(verdict.failures), 2)
        self.assertIn("survived 1: equivalent 0, unexplained 1", said)


class TheSurvivorsAreDraftedFromTheReports(unittest.TestCase):
    def test_an_uncovered_mutant_is_drafted_and_a_recorded_survivor_is_counted_once(self):
        root = tree(
            self,
            {
                "scripts/mutation-equivalent.d/python.json": json.dumps(
                    {"records": [a_record("replace + with - in guard", "return x + 2")]}
                )
            },
        )
        entry = file_entry(SCRIPT, SCRIPT_HEAD, {})
        names = {}
        for mutant in entry["mutants"]:
            names[mutant["mutant"]] = mutant["name"]
        plus, other = "replace + with - in guard", "replace 2 with 3 in guard"
        for mutant in entry["mutants"]:
            mutant["outcome"] = {plus: "survived", other: "uncovered"}.get(
                mutant["mutant"], "killed"
            )
        home = scratch(self) / "weekly"
        write_shard(home, 0, report_of([entry]))
        found, excused = V.survivors_in(home, root)
        self.assertEqual((dict(found), excused), ({SCRIPT: [names[other]]}, 1))
        self.assertEqual(dict(found), {SCRIPT: [names[other]]})


class TheRecordsAreHeldToTheirPopulation(unittest.TestCase):
    def test_a_record_that_binds_no_python_mutant_is_stale_by_that_noun(self):
        root = tree(self)
        record = V.Record("python.json", 1, a_record("replace + with - in guard", "return x + 2"))
        excuses_root = tree(
            self,
            {"scripts/mutation-equivalent.d/python.json": json.dumps({"records": [record.fields]})},
        )
        verdict = V.Verdict("scripts")
        quiet(V.python_excuses, verdict, excuses_root, [], None)
        self.assertEqual(
            verdict.failures,
            [
                "STALE python.json record 1 (scripts/guard.py: replace + with - in guard): "
                "binds no python mutant"
            ],
        )
        self.assertTrue(root.is_dir())

    def test_a_python_record_is_of_the_scripts_or_the_oracle_and_reached_by_a_mapped_test(self):
        root = tree(self)
        sources = V.Sources(root)

        def problems(**more):
            fields = a_record("replace 3 with 4 in gen", "return n * 3", file=GENERATOR)
            fields.update(more)
            return V.record_problems(root, V.Record("python.json", 1, fields), None, sources)

        self.assertEqual(problems(), [])
        self.assertEqual(problems(file=SCRIPT, anchor="return x + 2"), [])
        outside = problems(file="scripts/tests/test_guard.py", anchor="import unittest")
        self.assertEqual(
            outside[0],
            "its file scripts/tests/test_guard.py lies outside the population (SPEC-087 R12)",
        )
        self.assertEqual(len(outside), 2)

    def test_a_reached_by_names_a_test_of_a_module_the_map_gives_the_file(self):
        root = tree(self)
        record = V.Record("python.json", 1, a_record("m", "a"))
        self.assertEqual(V.python_reached(root, record, SCRIPT), [])
        lacking = V.Record("python.json", 1, {"file": SCRIPT})
        self.assertEqual(
            V.python_reached(root, lacking, SCRIPT),
            ["reached_by: names no test: 0 unittest roots hold .py"],
        )
        (root / "scripts/mutation-python.json").write_text(
            json.dumps({SCRIPT: {"modules": ["test_other"]}}), encoding="utf-8"
        )
        self.assertEqual(
            V.python_reached(root, record, SCRIPT),
            [f"reached_by: {REACHED} is no test of a module {V.PYTHON_POPULATION} gives {SCRIPT}"],
        )
        self.assertEqual(
            V.python_reached(root, record, ""),
            [f"reached_by: {REACHED} is no test of a module {V.PYTHON_POPULATION} gives "],
        )

    def test_the_killer_is_resolved_from_a_row_of_the_scripts_table_naming_the_file(self):
        root = tree(self)
        record = V.Record("python.json", 1, a_record("m", "a"))
        seen = []
        real = V.mutation_rows.resolve_killer

        def spy(where, row):
            seen.append(row)
            return real(where, row)

        with mock.patch.object(V.mutation_rows, "resolve_killer", spy):
            self.assertEqual(V.python_reached(root, record, SCRIPT), [])
        (row,) = seen
        self.assertEqual(
            (row.table, row.target, row.find, row.replace, row.description, row.crate),
            ("SCRIPT_MUTATIONS", SCRIPT, "", "", "", None),
        )
        self.assertEqual(row.killer, REACHED)

    def test_a_record_with_no_file_still_has_its_reached_by_read(self):
        root = tree(self)
        fields = a_record("m", "a")
        del fields["file"]
        found = V.record_problems(root, V.Record("python.json", 1, fields), None, V.Sources(root))
        self.assertEqual(
            found,
            [
                "lacks file",
                f"reached_by: {REACHED} is no test of a module {V.PYTHON_POPULATION} gives ",
            ],
        )


if __name__ == "__main__":
    unittest.main()
