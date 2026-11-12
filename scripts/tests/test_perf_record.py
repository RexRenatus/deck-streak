"""The regression record checker (SPEC-618, ADR-640): A8 to A14.

Each test drives `scripts/perf-record.py check` as a child process over records planted in a
temporary folder, each a copy of `fixtures/perf/record.json` with one change, against the
benchmark list `fixtures/perf/benchmarks.txt`. Nothing here runs a bench or reaches CodSpeed.
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "perf-record.py"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "perf"
RECORD = FIXTURES / "record.json"
NAMES = FIXTURES / "benchmarks.txt"
BRANCH = "feat/sibling-rule"
TOTAL = "perf-record: examined %d records, %d refused\n"


def run(*args):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def check(records, branch=BRANCH, names=NAMES):
    """Check planted records (a file name and a JSON value, or a raw text) in a fresh folder."""
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp) / "regressions.d"
        for name, value in records.items():
            path = folder / name
            path.parent.mkdir(parents=True, exist_ok=True)
            text = value if isinstance(value, str) else json.dumps(value)
            path.write_text(text, encoding="utf-8")
        folder.mkdir(exist_ok=True)
        return run("check", "--dir", str(folder), "--branch", branch, "--benchmarks", str(names))


def record(**changes):
    data = json.loads(RECORD.read_text(encoding="utf-8"))
    data.update(changes)
    return data


def bench(index, **changes):
    data = record()
    data["benchmarks"][index].update(changes)
    return data


def without(key, index=None):
    data = record()
    del (data if index is None else data["benchmarks"][index])[key]
    return data


class RecordCase(unittest.TestCase):
    def assert_refused(self, cases):
        for value, problems in examined("planted records", cases):
            done = check({"r.json": value})
            said = done.stdout + done.stderr
            self.assertEqual(done.returncode, 1, said)
            lines = "".join(f"perf-record: r.json: {problem}\n" for problem in problems)
            self.assertEqual(done.stdout, lines + TOTAL % (1, 1))


class TheFolder(RecordCase):
    def test_a_complete_record_reads_ok_and_a_missing_folder_examines_none(self):
        planted = {
            "keep.json": record(),
            "other.json": record(remediation="not-needed: the change is the fix", alternatives=[]),
            "notes.txt": "not a record",
            "deeper/broken.json": "{",
        }
        done = check(planted)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(
            done.stdout,
            "perf-record: keep.json: OK\nperf-record: other.json: OK\n" + TOTAL % (2, 0),
        )
        mixed = check({"a.json": record(why=""), "b.json": record()})
        self.assertEqual(mixed.returncode, 1, mixed.stderr)
        self.assertEqual(
            mixed.stdout,
            "perf-record: a.json: why is blank\nperf-record: b.json: OK\n" + TOTAL % (2, 1),
        )
        empty = check({})
        self.assertEqual((empty.returncode, empty.stdout), (0, TOTAL % (0, 0)))
        with tempfile.TemporaryDirectory() as tmp:
            absent = str(Path(tmp) / "absent")
            folder = ["check", "--dir", absent, "--branch", BRANCH]
            done = run(*folder, "--benchmarks", str(NAMES))
            self.assertEqual((done.returncode, done.stdout), (0, TOTAL % (0, 0)))
            done = run(*folder, "--benchmarks", absent)
            self.assertEqual(done.returncode, 1, done.stdout)
            line = "perf-record: VOID cannot read the benchmark list at " + absent + "\n"
            self.assertEqual(done.stdout, line)
            full = [*folder, "--benchmarks", str(NAMES)]
            for at in examined("required options", range(1, len(full), 2)):
                usage = run(*full[:at], *full[at + 2 :])
                self.assertEqual(usage.returncode, 2, full[at] + ": " + usage.stdout)
            self.assertEqual(run("lint", *full[1:]).returncode, 2)


class TheRefusalArms(RecordCase):
    def test_an_unknown_key_is_refused(self):
        self.assert_refused(
            [
                (record(extra="x"), ["the record holds the unknown key extra"]),
                (
                    record(zeta=1, verdict="accept"),
                    [
                        "the record holds the unknown key verdict",
                        "the record holds the unknown key zeta",
                    ],
                ),
                (bench(0, verdict="accept"), ["benchmark 0 holds the unknown key verdict"]),
            ]
        )
        data = bench(0, verdict="accept")
        data["benchmarks"][1]["change_pct"] = 0
        problems = [
            "benchmark 0 holds the unknown key verdict",
            "benchmark 1 change_pct 0 is not above 0",
        ]
        self.assert_refused([(data, problems)])

    def test_a_blank_cause_or_why_is_refused(self):
        self.assert_refused(
            [
                (record(cause=""), ["cause is blank"]),
                (record(cause="   "), ["cause is blank"]),
                (record(why=""), ["why is blank"]),
                (record(why="\n"), ["why is blank"]),
                (record(cause="", why=""), ["cause is blank", "why is blank"]),
            ]
        )

    def test_a_change_of_zero_or_less_is_refused(self):
        self.assert_refused(
            [
                (bench(0, change_pct=0), ["benchmark 0 change_pct 0 is not above 0"]),
                (bench(1, change_pct=0.0), ["benchmark 1 change_pct 0.0 is not above 0"]),
                (bench(0, change_pct=-1.5), ["benchmark 0 change_pct -1.5 is not above 0"]),
            ]
        )

    def test_a_benchmark_no_bench_defines_is_refused(self):
        unknown = "benchmark 0 names 'scheduling::day_queue_v2', which no bench defines"
        self.assert_refused(
            [
                (bench(0, name="scheduling::day_queue_v2"), [unknown]),
                (bench(0, name=""), ["benchmark 0 names '', which no bench defines"]),
            ]
        )
        with tempfile.TemporaryDirectory() as tmp:
            names = Path(tmp) / "names.txt"
            names.write_text("other::bench\n", encoding="utf-8")
            done = check({"r.json": record()}, names=names)
        self.assertEqual(done.returncode, 1, done.stderr)
        self.assertEqual(
            done.stdout,
            "perf-record: r.json: benchmark 0 names 'scheduling::day_queue', which no bench "
            "defines\nperf-record: r.json: benchmark 1 names 'replay::replay', which no bench "
            "defines\n" + TOTAL % (1, 1),
        )

    def test_a_record_for_another_branch_is_refused(self):
        self.assert_refused(
            [(record(branch="dev"), ["branch 'dev' is not the head ref feat/sibling-rule"])]
        )
        done = check({"r.json": record()}, branch="feat/other")
        self.assertEqual(done.returncode, 1, done.stderr)
        line = "perf-record: r.json: branch 'feat/sibling-rule' is not the head ref feat/other\n"
        self.assertEqual(done.stdout, line + TOTAL % (1, 1))


class TheClosedSchema(RecordCase):
    def test_a_record_outside_the_schema_is_refused(self):
        remedy = ["remediation is neither requested nor not-needed: <reason>"]
        alternatives = ["alternatives is no list of text"]
        self.assert_refused(
            [
                ("{", ["the record is no JSON document"]),
                ("[]", ["the record is no JSON object"]),
                (without("spec"), ["the record lacks the key spec"]),
                (without("instrument", 0), ["benchmark 0 lacks the key instrument"]),
                (record(benchmarks=[3]), ["benchmark 0 is no JSON object"]),
                (record(benchmarks=[]), ["benchmarks holds no benchmark"]),
                (record(benchmarks="simulation"), ["benchmarks holds no benchmark"]),
                (
                    record(schema="deckstreak.perf-regression.v2"),
                    ["schema is not deckstreak.perf-regression.v1"],
                ),
                (bench(0, change_pct=True), ["benchmark 0 change_pct True is not above 0"]),
                (bench(0, change_pct="3.4"), ["benchmark 0 change_pct '3.4' is not above 0"]),
                (bench(0, base="xyz"), ["benchmark 0 base is no commit sha"]),
                (bench(0, base="DE4C4D4D"), ["benchmark 0 base is no commit sha"]),
                (bench(1, head=7), ["benchmark 1 head is no commit sha"]),
                (bench(0, instrument="walltime"), ["benchmark 0 instrument is not simulation"]),
                (record(cause=3), ["cause is blank"]),
                (record(spec=""), ["spec is blank"]),
                (record(alternatives="tried nothing"), alternatives),
                (record(alternatives=[""]), alternatives),
                (record(alternatives=[3]), alternatives),
                (record(remediation="maybe"), remedy),
                (record(remediation="not-needed: "), remedy),
                (record(remediation="requested later"), remedy),
                (record(remediation=1), remedy),
                (
                    record(schema="x", branch="dev", why=""),
                    [
                        "schema is not deckstreak.perf-regression.v1",
                        "branch 'dev' is not the head ref feat/sibling-rule",
                        "why is blank",
                    ],
                ),
            ]
        )
