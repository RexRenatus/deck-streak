"""The verdict reads the Python mutation runner's reports (SPEC-087 A13 to A19).

Each test builds a fixture repository with a base and a head commit, runs `plan` and `shards` over
their diff, and judges reports written in the runner's own schema, `deckstreak.mutation-python.v1`:
the runner's `list_source` gives each mutant its real coordinates, so a record binds it as it binds
a real one. No mutant is run here.
"""

import importlib.util
import json
import math
import os
import subprocess
import sys
import unittest
from pathlib import Path

from _support import REPO, examined
from test_mutation_verdict import Fixture, outcomes, stryker, write_scope

SCHEMA = "deckstreak.mutation-python.v1"
SCRIPT = "scripts/guard.py"
SCRIPT_TEXT = "def guard(x):\n    return x + 1\n"
SCRIPT_HEAD = "def guard(x):\n    return x + 2\n"
GENERATOR = "tools/parity-oracle/generate.py"
GENERATOR_TEXT = "def gen(n):\n    return n * 2\n"
GENERATOR_HEAD = "def gen(n):\n    return n * 3\n"
MODULE = "scripts/tests/test_guard.py"
MODULE_TEXT = (
    "import unittest\n\n\n"
    "class TheGuard(unittest.TestCase):\n"
    "    def test_it_adds(self):\n"
    "        self.assertEqual(1, 1)\n\n"
    "    def test_twice(self):\n"
    "        self.assertEqual(1, 1)\n\n"
    "    def test_twice(self):\n"
    "        self.assertEqual(2, 2)\n"
)
MAP = {
    SCRIPT: {"dir": "scripts/tests", "modules": ["test_guard"]},
    GENERATOR: {"dir": "scripts/tests", "modules": ["test_guard"]},
}
FILES = {
    SCRIPT: SCRIPT_TEXT,
    GENERATOR: GENERATOR_TEXT,
    MODULE: MODULE_TEXT,
    "scripts/mutation-python.json": json.dumps(MAP),
}
PLUS = "replace + with - in guard"
REACHED = "test_guard.TheGuard.test_it_adds"
VERDICT = REPO / "scripts" / "mutation-verdict.py"


def runner():
    """scripts/mutation_python.py as a module, so a test reads the listing it writes."""
    spec = importlib.util.spec_from_file_location(
        "mutation_python_listing", REPO / "scripts" / "mutation_python.py"
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def listed(path, text):
    """The listing entries of `text`, as `mutation_python.py list --out` writes them."""
    return [
        {
            "name": m.name(path),
            "file": path,
            "line": m.line,
            "column": m.column,
            "start_line": m.start_line,
            "end_line": m.end_line,
            "end_column": m.end_column,
            "mutant": m.text,
            "operator": m.operator,
        }
        for m in runner().list_source(text)
    ]


def mutant_named(path, text, description):
    found = [entry for entry in listed(path, text) if entry["mutant"] == description]
    assert len(found) == 1, (description, [entry["mutant"] for entry in listed(path, text)])
    return found[0]


def file_entry(
    path, text, outcomes_by_description, void=None, byte_readers=(), only=None, slot=None
):
    """A report's entry for one file: each mutant of `text` with the outcome the map gives its
    description, `killed` for one it does not name."""
    mutants = []
    for index, entry in enumerate(listed(path, text)):
        if only is not None and entry["mutant"] != only:
            continue
        if slot is not None and index % slot[1] != slot[0]:
            continue
        outcome = outcomes_by_description.get(entry["mutant"], "killed")
        mutants.append(
            {**entry, "outcome": outcome, "killers": ["test_guard.TheGuard.test_it_adds"]}
        )
    return {
        "path": path,
        "modules": ["test_guard"],
        "control": {"ran": 3, "failures": 0, "seconds": 1.0},
        "bound": 60.0,
        "byte_readers": list(byte_readers),
        "void": void,
        "mutants": mutants,
    }


def report_of(files, code=0, shard="0/1", **extra):
    counts = {
        name: 0 for name in ("killed", "survived", "uncovered", "unviable", "timeout", "void")
    }
    for entry in files:
        for mutant in entry["mutants"]:
            counts[mutant["outcome"]] += 1
    counts["examined"] = counts["killed"] + counts["survived"] + counts["uncovered"]
    return {
        "schema": SCHEMA,
        "selection": "plan",
        "shard": shard,
        "failfast": True,
        "files": files,
        "counts": counts,
        "exit": code,
        **extra,
    }


def write_shard(root, index, document, raw=None):
    directory = Path(root) / f"mutation-python-shard-{index}"
    directory.mkdir(parents=True, exist_ok=True)
    text = raw if raw is not None else json.dumps(document)
    (directory / "report.json").write_text(text, encoding="utf-8")


def python_listing(fixture, entries, name="python-listed.json"):
    path = fixture.out / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps({"schema": SCHEMA, "listed": len(entries), "mutants": entries}),
        encoding="utf-8",
    )
    return path


def shard_the_plan(fixture, entries, expect=0):
    """`shards --python-listed` over the plan, with the step outputs it writes."""
    listing = python_listing(fixture, entries)
    sink = fixture.out / "github-output"
    sink.write_text("", encoding="utf-8")
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", GITHUB_OUTPUT=str(sink))
    done = subprocess.run(
        [
            sys.executable,
            str(VERDICT),
            "shards",
            "--plan",
            str(fixture.out / "plan.json"),
            "--python-listed",
            str(listing),
        ],
        capture_output=True,
        text=True,
        env=env,
        timeout=300,
        check=False,
    )
    assert done.returncode == expect, done.stdout + done.stderr
    written = dict(
        line.split("=", 1) for line in sink.read_text(encoding="utf-8").splitlines() if "=" in line
    )
    return done, written


def judged(fixture, klass, reports, *extra):
    return fixture.judge(klass, "--python", str(reports), *extra)


def changed_fixture(test, both=False, script_text=SCRIPT_TEXT, head_text=SCRIPT_HEAD):
    files = dict(FILES)
    files[SCRIPT] = script_text
    fixture = Fixture(test, files=files)
    changes = {SCRIPT: head_text}
    if both:
        changes[GENERATOR] = GENERATOR_HEAD
    fixture.head(changes)
    fixture.plan()
    return fixture


class TheVerdictReadsThePythonReports(unittest.TestCase):
    def test_a_guard_script_is_its_own_class_and_its_tests_are_not(self):
        expected = {
            "scripts/check.py": "scripts",
            "scripts/mutation-verdict.py": "scripts",
            "scripts/tests/test_check.py": "other",
            "scripts/deep/check.py": "other",
            "scripts/check.sh": "other",
            "scripts/notes.md": "other",
            "tools/parity-oracle/generate.py": "oracle",
            "tools/parity-oracle/test_generate.py": "other",
            "crates/fix/src/lib.rs": "rust",
        }
        fixture = Fixture(self)
        fixture.head({path: "x = 1\n" for path in expected})
        plan = fixture.plan()
        found = {entry["path"]: entry["class"] for entry in plan["files"]}
        for path in examined("classified paths", sorted(expected)):
            self.assertEqual(found.get(path), expected[path], path)
        self.assertEqual(sorted(plan["classes"]), ["oracle", "rust", "scripts", "web"])
        # A script's changed code line makes its class apply, and only its class.
        changed = changed_fixture(self)
        plan = json.loads((changed.out / "plan.json").read_text(encoding="utf-8"))
        self.assertTrue(plan["classes"]["scripts"]["applies"])
        self.assertFalse(plan["classes"]["oracle"]["applies"])
        self.assertIn(SCRIPT, plan["classes"]["scripts"]["files"])
        said = changed.verdict(
            "plan",
            "--base",
            changed.base,
            "--head",
            "HEAD",
            "--root",
            str(changed.root),
            "--out",
            str(changed.out),
        )
        self.assertIn("scripts 1", said.stdout)
        self.assertIn("mutation: plan: scripts applies", said.stdout)
        # A push that names the pull request it merges is judged on that pull request's run.
        merged = changed.plan(
            "--event", "push", "--subject", "Merge pull request #9 from RexRenatus/feat/x"
        )
        self.assertEqual(merged["scope"]["decision"], "not-applicable")
        self.assertFalse(merged["classes"]["scripts"]["applies"])
        done = changed.judge("scripts", "--python", str(changed.out / "none"))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("not-applicable", done.stdout)

    def test_the_plan_sizes_the_python_matrix_from_its_listing(self):
        fixture = changed_fixture(self)
        cases = [
            (0, 1),
            (1, 1),
            (40, 1),
            (41, 2),
            (80, 2),
            (81, 3),
            (320, 8),
            (321, 9),
            (840, 21),
            (1280, 32),
            (1281, 32),
            (2000, 32),
        ]
        for count, shards in examined("listing sizes", cases):
            entries = [
                {"name": f"scripts/guard.py:{n}:1: replace + with - in guard", "file": SCRIPT}
                for n in range(1, count + 1)
            ]
            expected = min(32, max(1, math.ceil(count / 40)))
            self.assertEqual(expected, shards, count)
            _done, written = shard_the_plan(fixture, entries)
            plan = json.loads((fixture.out / "plan.json").read_text(encoding="utf-8"))
            python = plan["python"]
            self.assertEqual(python["count"], shards, f"{count} listed")
            self.assertEqual(written["python_shards"], str(shards), f"{count} listed")
            self.assertEqual(
                written["python_matrix"], json.dumps(list(range(shards))), f"{count} listed"
            )
            held = [name for shard in python["shards"] for name in shard["mutants"]]
            self.assertEqual(sorted(held), sorted(entry["name"] for entry in entries))
            for shard in python["shards"]:
                names = [entry["name"] for entry in entries][shard["shard"] :: shards]
                self.assertEqual(shard["mutants"], names, f"{count} listed, shard {shard['shard']}")
        # The rust matrix is sized as before, and a listing nobody can read is VOID.
        self.assertEqual(plan["shards"]["count"], 1)
        (fixture.out / "broken.json").write_text("{", encoding="utf-8")
        broken = fixture.verdict(
            "shards",
            "--plan",
            str(fixture.out / "plan.json"),
            "--python-listed",
            str(fixture.out / "broken.json"),
        )
        self.assertEqual(broken.returncode, 3, broken.stdout + broken.stderr)
        self.assertIn("VOID", broken.stdout)

    def test_a_plan_of_python_mutants_and_no_rust_mutant_lists_zero(self):
        # `listed` is the RUST listing's own count (SPEC-290 R1); the Python shards' mutants are
        # not in it, or `mutation-rust` would start over nothing (SPEC-087 section 3).
        fixture = changed_fixture(self)
        entries = [
            {"name": f"scripts/guard.py:{n}:1: replace + with - in guard", "file": SCRIPT}
            for n in range(1, 4)
        ]
        _done, written = examined("plans", [shard_the_plan(fixture, entries)])[0]
        plan = json.loads((fixture.out / "plan.json").read_text(encoding="utf-8"))
        held = sum(len(shard["mutants"]) for shard in plan["python"]["shards"])
        self.assertEqual(held, 3)
        self.assertEqual(written["listed"], "0")
        self.assertEqual(written["python_shards"], "1")
        self.assertEqual(written["python_matrix"], "[0]")

    def test_a_python_class_that_examined_nothing_is_void(self):
        fixture = changed_fixture(self, both=True)
        # The plan lists no mutant, so the one report that examined none is what it lists.
        shard_the_plan(fixture, [])
        empty = fixture.out / "empty"
        write_shard(empty, 0, report_of([]))
        for klass in ("scripts", "oracle"):
            done = judged(fixture, klass, empty)
            self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
            self.assertIn(f"mutation: {klass}: VOID", done.stdout)
            self.assertRegex(done.stdout, r"(?m)^examined 0$")
            self.assertNotIn("has no generated mutants", done.stdout)
        # A report holding one killed mutant of a script and one of the generator: each class
        # examines its own one.
        whole = fixture.out / "whole"
        script_only = "replace + with - in guard"
        gen_only = "replace * with / in gen"
        # The plan for this run lists just those two, so the report examined what it lists.
        shard_the_plan(
            fixture,
            [m for m in listed(SCRIPT, SCRIPT_HEAD) if m["mutant"] == script_only]
            + [m for m in listed(GENERATOR, GENERATOR_HEAD) if m["mutant"] == gen_only],
        )
        write_shard(
            whole,
            0,
            report_of(
                [
                    file_entry(SCRIPT, SCRIPT_HEAD, {}, only=script_only),
                    file_entry(GENERATOR, GENERATOR_HEAD, {}, only=gen_only),
                ]
            ),
        )
        scripts = judged(fixture, "scripts", whole)
        self.assertEqual(scripts.returncode, 0, scripts.stdout + scripts.stderr)
        self.assertRegex(scripts.stdout, r"(?m)^examined 1$")
        oracle = judged(fixture, "oracle", whole)
        self.assertEqual(oracle.returncode, 0, oracle.stdout + oracle.stderr)
        self.assertRegex(oracle.stdout, r"(?m)^examined 1$")
        self.assertIn("examined 1: generated 1, rows 0", oracle.stdout)
        self.assertNotIn("has no generated mutants", oracle.stdout)
        # A row the diff selects, on a changed line, examines the class with no mutant: one, from
        # the row.
        row = [
            "S00077-GUARD",
            "scripts/guard.py",
            "return x + 2",
            "return x + 3",
            "test_guard.TheGuard.test_it_adds",
            "the guard adds two",
        ]
        rowed = Fixture(self, files=dict(FILES), rows=[("SCRIPT_MUTATIONS", row)])
        rowed.head({SCRIPT: SCRIPT_HEAD})
        rowed.plan()
        shard_the_plan(rowed, [])
        rows = rowed.report("rows.json", [{"id": row[0], "verdict": "KILLED", "target": SCRIPT}])
        write_shard(rowed.out / "reports", 0, report_of([]))
        carried = judged(rowed, "scripts", rowed.out / "reports", "--rows", str(rows))
        self.assertEqual(carried.returncode, 0, carried.stdout + carried.stderr)
        self.assertIn("examined 1: generated 0, rows 1", carried.stdout)
        # A diff of blanks and comments reads not-applicable, with no report to read.
        quiet = changed_fixture(self, head_text="# the guard\n\n" + SCRIPT_TEXT)
        idle = judged(quiet, "scripts", quiet.out / "nothing")
        self.assertEqual(idle.returncode, 0, idle.stdout + idle.stderr)
        self.assertIn(
            f"not-applicable: {SCRIPT}: 2 changed line(s), all blank or comments", idle.stdout
        )

    def test_a_python_survivor_fails_and_a_timeout_is_void_by_name(self):
        fixture = changed_fixture(self)
        shard_the_plan(fixture, listed(SCRIPT, SCRIPT_HEAD))
        plus = mutant_named(SCRIPT, SCRIPT_HEAD, PLUS)["name"]
        record = a_record(PLUS, "return x + 2")
        fixture.write(
            "scripts/mutation-equivalent.d/python.json", json.dumps({"records": [record]})
        )

        def judge_with(name, entry, code=0, klass="scripts", **kwargs):
            reports = fixture.out / name
            write_shard(reports, 0, report_of([entry], code=code, **kwargs))
            return judged(fixture, klass, reports)

        survived = judge_with("survived", file_entry(SCRIPT, SCRIPT_HEAD, {PLUS: "survived"}))
        self.assertEqual(survived.returncode, 0, survived.stdout + survived.stderr)
        self.assertIn(f"EQUIVALENT {plus}", survived.stdout)
        fixture.write("scripts/mutation-equivalent.d/python.json", json.dumps({"records": []}))
        survived = judge_with("survived2", file_entry(SCRIPT, SCRIPT_HEAD, {PLUS: "survived"}))
        self.assertEqual(survived.returncode, 1, survived.stdout + survived.stderr)
        self.assertIn(f"SURVIVED {plus}", survived.stdout)
        uncovered = judge_with("uncovered", file_entry(SCRIPT, SCRIPT_HEAD, {PLUS: "uncovered"}))
        self.assertEqual(uncovered.returncode, 1, uncovered.stdout + uncovered.stderr)
        self.assertIn(f"UNCOVERED {plus}", uncovered.stdout)
        # A timeout is VOID, and a record naming it excuses nothing.
        fixture.write(
            "scripts/mutation-equivalent.d/python.json", json.dumps({"records": [record]})
        )
        timed = judge_with("timeout", file_entry(SCRIPT, SCRIPT_HEAD, {PLUS: "timeout"}))
        self.assertEqual(timed.returncode, 3, timed.stdout + timed.stderr)
        self.assertIn(f"VOID timeout: {plus}", timed.stdout)
        self.assertNotIn("EQUIVALENT", timed.stdout)
        void = judge_with("void", file_entry(SCRIPT, SCRIPT_HEAD, {PLUS: "void"}))
        self.assertEqual(void.returncode, 3, void.stdout + void.stderr)
        self.assertIn(f"VOID void: {plus}", void.stdout)
        entry = file_entry(SCRIPT, SCRIPT_HEAD, {}, void="the control failed unmutated: t")
        for mutant in entry["mutants"]:
            mutant["outcome"] = "void"
        file_void = judge_with("file", entry)
        self.assertEqual(file_void.returncode, 3, file_void.stdout + file_void.stderr)
        self.assertIn(f"VOID {SCRIPT}: the control failed unmutated: t", file_void.stdout)
        self.assertNotIn(f"VOID void: {plus}", file_void.stdout)
        # A run whose restore failed writes its report first, with exit 4: VOID, by name.
        restored = judge_with(
            "restore",
            file_entry(SCRIPT, SCRIPT_HEAD, {}),
            code=4,
            restore_failed="scripts/guard.py",
        )
        self.assertEqual(restored.returncode, 3, restored.stdout + restored.stderr)
        self.assertIn(
            "VOID mutation-python-shard-0: a restore failed: scripts/guard.py", restored.stdout
        )
        # An unviable mutant and a byte reader are named and change no count; a record naming an
        # unviable mutant would be UNNEEDED, so none is held here.
        fixture.write("scripts/mutation-equivalent.d/python.json", json.dumps({"records": []}))
        entry = file_entry(
            SCRIPT,
            SCRIPT_HEAD,
            {"replace + with - in guard": "unviable"},
            byte_readers=["test_guard.TheGuard.test_it_adds"],
        )
        named = judge_with("named", entry)
        self.assertEqual(named.returncode, 0, named.stdout + named.stderr)
        self.assertIn(f"unviable: {plus}", named.stdout)
        self.assertIn(f"byte reader: {SCRIPT}: test_guard.TheGuard.test_it_adds", named.stdout)
        killed = sum(1 for m in entry["mutants"] if m["outcome"] == "killed")
        self.assertRegex(named.stdout, rf"(?m)^examined {killed}$")

    def test_a_missing_or_partial_python_shard_is_void(self):
        text = "".join(f"def f{i}(x):\n    return x + {i}\n" for i in range(27))
        fixture = changed_fixture(self, head_text=text)
        shard_the_plan(fixture, listed(SCRIPT, text))
        entries = [file_entry(SCRIPT, text, {}, slot=(shard, 3)) for shard in range(3)]
        whole = fixture.out / "whole"
        for shard in range(3):
            write_shard(whole, shard, report_of([entries[shard]], shard=f"{shard}/3"))
        green = judged(fixture, "scripts", whole)
        self.assertEqual(green.returncode, 0, green.stdout + green.stderr)
        self.assertRegex(green.stdout, r"(?m)^examined 81$")
        cases = {
            "last": (2, None, "no report"),
            "first": (0, None, "no report"),
            "garbled": (1, "{not json", "unreadable"),
            "schema": (1, json.dumps({"schema": "other.v1", "files": []}), "not of the schema"),
        }
        for name, (broken, raw, why) in examined("broken shards", cases.items()):
            reports = fixture.out / name
            for shard in range(3):
                if shard == broken and raw is None:
                    continue
                write_shard(
                    reports,
                    shard,
                    report_of([entries[shard]], shard=f"{shard}/3"),
                    raw=raw if shard == broken else None,
                )
            done = judged(fixture, "scripts", reports)
            self.assertEqual(done.returncode, 3, f"{name}: {done.stdout}{done.stderr}")
            self.assertIn(f"VOID mutation-python-shard-{broken}: {why}", done.stdout, name)
        # A plan the python matrix was never sized for promises no report, and says so.
        fixture.plan()
        unsized = judged(fixture, "scripts", whole)
        self.assertEqual(unsized.returncode, 3, unsized.stdout + unsized.stderr)
        self.assertIn("VOID the plan names no python shards", unsized.stdout)

    def test_a_python_record_excuses_exactly_its_survivor_and_carries_its_argument(self):
        fixture = changed_fixture(self)
        whole = python_listing(
            fixture, listed(SCRIPT, SCRIPT_HEAD) + listed(GENERATOR, GENERATOR_TEXT)
        )
        shard_the_plan(fixture, listed(SCRIPT, SCRIPT_HEAD))
        plus = mutant_named(SCRIPT, SCRIPT_HEAD, PLUS)["name"]
        other = mutant_named(SCRIPT, SCRIPT_HEAD, "replace 2 with 3 in guard")["name"]

        def judge_recorded(name, outcomes_by, records, listing=whole):
            fixture.write(
                "scripts/mutation-equivalent.d/python.json", json.dumps({"records": records})
            )
            reports = fixture.out / name
            write_shard(reports, 0, report_of([file_entry(SCRIPT, SCRIPT_HEAD, outcomes_by)]))
            return judged(fixture, "scripts", reports, "--python-whole", str(listing))

        record = a_record(PLUS, "return x + 2")
        done = judge_recorded(
            "bound", {PLUS: "survived", "replace 2 with 3 in guard": "survived"}, [record]
        )
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(f"EQUIVALENT {plus}: {record['reason']} (#218)", done.stdout)
        self.assertIn(f"SURVIVED {other}", done.stdout)
        self.assertNotIn(f"SURVIVED {plus}", done.stdout)
        self.assertIn("survived 2: equivalent 1, unexplained 1", done.stdout)
        both = judge_recorded(
            "both",
            {PLUS: "survived", "replace 2 with 3 in guard": "survived"},
            [record, a_record("replace 2 with 3 in guard", "return x + 2")],
        )
        self.assertEqual(both.returncode, 0, both.stdout + both.stderr)
        self.assertIn("survived 2: equivalent 2, unexplained 0", both.stdout)
        for outcome, word in examined(
            "record refutations",
            [("killed", "REFUTED"), ("unviable", "UNNEEDED"), ("uncovered", "UNCOVERED")],
        ):
            refuted = judge_recorded(outcome, {PLUS: outcome}, [record])
            self.assertEqual(refuted.returncode, 1, refuted.stdout + refuted.stderr)
            self.assertIn(f"{word} python.json record 1", refuted.stdout)
        stale = judge_recorded("stale", {}, [a_record("replace + with * in guard", "return x + 2")])
        self.assertEqual(stale.returncode, 1, stale.stdout + stale.stderr)
        self.assertIn("STALE python.json record 1", stale.stdout)
        # Two mutants of one description inside one anchor are AMBIGUOUS; a span resolves it.
        text = "def guard(x):\n    return x + 2 + 3\n"
        twice = changed_fixture(self, script_text=SCRIPT_TEXT, head_text=text)
        entries = listed(SCRIPT, text)
        pluses = [entry for entry in entries if entry["mutant"] == PLUS]
        self.assertEqual(len(pluses), 2)
        shard_the_plan(twice, entries)
        listing = python_listing(twice, entries, "whole.json")
        anchor = a_record(PLUS, "return x + 2 + 3")
        twice.write("scripts/mutation-equivalent.d/python.json", json.dumps({"records": [anchor]}))
        reports = twice.out / "reports"
        write_shard(reports, 0, report_of([file_entry(SCRIPT, text, {PLUS: "survived"})]))
        ambiguous = judged(twice, "scripts", reports, "--python-whole", str(listing))
        self.assertEqual(ambiguous.returncode, 1, ambiguous.stdout + ambiguous.stderr)
        self.assertIn("AMBIGUOUS python.json record 1", ambiguous.stdout)
        lines = text.splitlines(keepends=True)
        chosen = min(pluses, key=lambda entry: entry["column"])
        start = sum(map(len, lines[: chosen["line"] - 1])) + chosen["column"] - 1
        end = sum(map(len, lines[: chosen["end_line"] - 1])) + chosen["end_column"] - 1
        anchor["span"] = text[start:end]
        twice.write("scripts/mutation-equivalent.d/python.json", json.dumps({"records": [anchor]}))
        write_shard(reports, 0, report_of([file_entry(SCRIPT, text, {PLUS: "survived"})]))
        resolved = judged(twice, "scripts", reports, "--python-whole", str(listing))
        self.assertNotIn("AMBIGUOUS", resolved.stdout)
        self.assertEqual(resolved.stdout.count("EQUIVALENT"), 1, resolved.stdout)

    def test_the_census_holds_a_python_record_whole(self):
        fixture = Fixture(self, files=dict(FILES))
        fixture.head({"README.md": "a change\n"})

        def census_of(record):
            fixture.write(
                "scripts/mutation-equivalent.d/python.json", json.dumps({"records": [record]})
            )
            return fixture.verdict("census", "--root", str(fixture.root))

        record = a_record(PLUS, "return x + 1")
        clean = census_of(record)
        self.assertEqual(clean.returncode, 0, clean.stdout + clean.stderr)
        self.assertRegex(clean.stdout, r"(?m)^examined 1 record\(s\)$")
        for field in examined(
            "record fields",
            ["file", "mutant", "anchor", "reason", "evidence", "reached_by", "issue"],
        ):
            lacking = {name: value for name, value in record.items() if name != field}
            done = census_of(lacking)
            self.assertEqual(done.returncode, 1, f"{field}: {done.stdout}{done.stderr}")
            self.assertIn(f"lacks {field}", done.stdout, field)
        empty = census_of({**record, "evidence": ""})
        self.assertIn("lacks evidence", empty.stdout)
        repeats = census_of({**record, "evidence": record["reason"]})
        self.assertIn("its evidence repeats its reason", repeats.stdout)
        for reached in ("test_guard.TheGuard.test_missing", "test_guard.TheGuard.test_twice"):
            done = census_of({**record, "reached_by": reached})
            self.assertEqual(done.returncode, 1, reached)
            self.assertIn("reached_by: ", done.stdout, reached)
        outside = census_of({**record, "file": "scripts/tests/test_guard.py"})
        self.assertEqual(outside.returncode, 1)
        self.assertIn("lies outside the population", outside.stdout)

    def test_a_rehearsal_promises_the_python_shards_it_ran(self):
        fixture = changed_fixture(self)
        reports = fixture.out / "rehearsal"
        entry = file_entry(SCRIPT, SCRIPT_HEAD, {PLUS: "killed"}, slot=(0, 1))
        write_shard(reports, 0, report_of([entry], shard="0/1"))
        base = ("battery", "--reports", str(reports), "--shards", "1", "--package", "python")
        for extra, code, line in examined(
            "python shard promises",
            [
                (("--python-shards", "1"), 0, "battery: counted 1 of 1 reports whole"),
                ((), 1, "battery: MISSING mutation-python-shard-1: no report.json"),
                (("--python-shards", "2"), 1, "battery: MISSING mutation-python-shard-1"),
            ],
        ):
            done = fixture.verdict(*base, *extra)
            self.assertEqual(done.returncode, code, done.stdout + done.stderr)
            self.assertIn(line, done.stdout, extra)

    def test_the_battery_the_table_and_the_survivors_read_the_python_reports(self):
        fixture = changed_fixture(self)
        plus = mutant_named(SCRIPT, SCRIPT_HEAD, PLUS)
        record = a_record(PLUS, "return x + 2")
        fixture.write(
            "scripts/mutation-equivalent.d/python.json", json.dumps({"records": [record]})
        )
        reports = fixture.out / "weekly"
        held = {
            PLUS: "survived",
            "replace 2 with 3 in guard": "survived",
            "replace return value with return None in guard": "unviable",
        }
        for shard in range(16):
            if shard == 7:
                continue
            entry = file_entry(SCRIPT, SCRIPT_HEAD, held, slot=(shard, 16))
            write_shard(reports, shard, report_of([entry], shard=f"{shard}/16"))
        base = ("battery", "--reports", str(reports), "--shards", "4", "--package", "python")
        missing = fixture.verdict(*base)
        self.assertEqual(missing.returncode, 1, missing.stdout + missing.stderr)
        self.assertIn("battery: MISSING mutation-python-shard-7: no report.json", missing.stdout)
        self.assertIn("battery: counted 15 of 16 reports whole", missing.stdout)
        write_shard(
            reports, 7, report_of([file_entry(SCRIPT, SCRIPT_HEAD, {}, slot=(7, 16))], shard="7/16")
        )
        whole = fixture.verdict(*base)
        self.assertEqual(whole.returncode, 0, whole.stdout + whole.stderr)
        self.assertIn("battery: counted 16 of 16 reports whole", whole.stdout)
        # A crate's scope and the Mini App's promise no python report.
        listing = fixture.report("crate.json", [{"name": "n", "package": "fix"}])
        crate = fixture.out / "crate"
        (crate / "mutants-shard-0" / "mutants.out").mkdir(parents=True)
        (crate / "mutants-shard-0" / "cargo-mutants.exit").write_text("0\n", encoding="utf-8")
        write_scope(crate / "mutants-shard-0")
        (crate / "mutants-shard-0" / "mutants.out" / "outcomes.json").write_text(
            json.dumps(outcomes(caught=1)), encoding="utf-8"
        )
        scoped = fixture.verdict(
            "battery",
            "--reports",
            str(crate),
            "--shards",
            "1",
            "--package",
            "fix",
            "--listed",
            str(listing),
        )
        self.assertEqual(scoped.returncode, 0, scoped.stdout + scoped.stderr)
        self.assertIn("battery: counted 2 of 2 reports whole", scoped.stdout)
        self.assertNotIn("python", scoped.stdout)
        sweep = fixture.out / "miniapp" / "stryker"
        sweep.mkdir(parents=True)
        (sweep / "mutation.json").write_text(json.dumps(stryker(["Killed"])), encoding="utf-8")
        mini = fixture.verdict(
            "battery",
            "--reports",
            str(fixture.out / "miniapp"),
            "--shards",
            "1",
            "--package",
            "miniapp",
        )
        self.assertEqual(mini.returncode, 0, mini.stdout + mini.stderr)
        self.assertNotIn("python", mini.stdout)
        # The table's python line: listed is its four counts summed.
        table = fixture.verdict(
            "table", "--reports", str(reports), "--root", str(fixture.root), "--package", "python"
        )
        self.assertEqual(table.returncode, 1, table.stdout + table.stderr)
        self.assertRegex(
            table.stdout,
            r"(?m)^table: python: listed (\d+), killed (\d+), equivalent 1, unexplained 1, "
            r"unviable 1$",
        )
        numbers = [
            int(n)
            for n in __import__("re")
            .search(
                r"table: python: listed (\d+), killed (\d+), equivalent (\d+), unexplained (\d+), "
                r"unviable (\d+)",
                table.stdout,
            )
            .groups()
        ]
        self.assertEqual(numbers[0], sum(numbers[1:]))
        self.assertIn("table: python: UNEXPLAINED", table.stdout)
        self.assertIn(f"table: python: EQUIVALENT {plus['name']}", table.stdout)
        for name, outcome in examined("table voids", [("timeout", "timeout"), ("void", "void")]):
            damaged = fixture.out / name
            for shard in range(16):
                entry = file_entry(SCRIPT, SCRIPT_HEAD, {PLUS: outcome}, slot=(shard, 16))
                write_shard(damaged, shard, report_of([entry], shard=f"{shard}/16"))
            done = fixture.verdict(
                "table",
                "--reports",
                str(damaged),
                "--root",
                str(fixture.root),
                "--package",
                "python",
            )
            self.assertEqual(done.returncode, 3, f"{name}: {done.stdout}{done.stderr}")
            self.assertIn(
                f"table: VOID mutation-python-shard-1: {outcome}: {plus['name']}", done.stdout
            )
        gone = fixture.out / "voidfile"
        for shard in range(16):
            entry = file_entry(
                SCRIPT,
                SCRIPT_HEAD,
                {},
                void="the sentinel left no test" if shard == 5 else None,
                slot=(shard, 16),
            )
            write_shard(gone, shard, report_of([entry], shard=f"{shard}/16"))
        done = fixture.verdict(
            "table", "--reports", str(gone), "--root", str(fixture.root), "--package", "python"
        )
        self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
        self.assertIn(
            f"table: VOID mutation-python-shard-5: {SCRIPT}: the sentinel left no test", done.stdout
        )
        # The survivors: one draft per file, titled as SPEC-039 titles each, naming the file's
        # unexplained mutants, none for a file with none, none while an open issue holds its title.
        drafts = fixture.out / "drafts"
        titles = fixture.report("titles.json", [])
        made = fixture.verdict(
            "survivors",
            "--reports",
            str(reports),
            "--out",
            str(drafts),
            "--open-titles",
            str(titles),
            "--root",
            str(fixture.root),
        )
        self.assertEqual(made.returncode, 0, made.stdout + made.stderr)
        manifest = json.loads((drafts / "drafts.json").read_text(encoding="utf-8"))
        self.assertEqual([item["title"] for item in manifest], [f"Mutation survivors: {SCRIPT}"])
        body = (drafts / manifest[0]["body"]).read_text(encoding="utf-8")
        self.assertIn("replace 2 with 3 in guard", body)
        self.assertNotIn(f"`{plus['name']}`", body)
        self.assertFalse(manifest[0]["open"])
        opened = fixture.report("open.json", [f"Mutation survivors: {SCRIPT}"])
        again = fixture.verdict(
            "survivors",
            "--reports",
            str(reports),
            "--out",
            str(drafts / "2"),
            "--open-titles",
            str(opened),
            "--root",
            str(fixture.root),
        )
        self.assertIn("already open, not filed again", again.stdout)
        clean = fixture.out / "clean"
        for shard in range(16):
            write_shard(clean, shard, report_of([file_entry(SCRIPT, SCRIPT_HEAD, {})]))
        none = fixture.verdict(
            "survivors",
            "--reports",
            str(clean),
            "--out",
            str(drafts / "3"),
            "--open-titles",
            str(titles),
            "--root",
            str(fixture.root),
        )
        self.assertEqual(json.loads((drafts / "3" / "drafts.json").read_text("utf-8")), [])
        self.assertIn("examined 0 file(s) with survivors", none.stdout)


def a_record(mutant, anchor, file=SCRIPT, **more):
    return {
        "file": file,
        "mutant": mutant,
        "anchor": anchor,
        "reason": "the mutated value never reaches a caller",
        "evidence": "scripts/guard.py's guard is only called with its result discarded",
        "reached_by": REACHED,
        "issue": "#218",
        **more,
    }


if __name__ == "__main__":
    unittest.main()
