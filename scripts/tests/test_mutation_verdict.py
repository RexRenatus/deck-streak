"""A pull request's mutation plan and verdict, and the weekly battery's survivors (SPEC-039 A12 to
A19).

Each test builds a fixture repository in a temporary directory with a base commit and a head
commit, runs `plan` over their diff, and judges synthetic reports written in each tool's own
format: cargo-mutants 27.1.0's `outcomes.json`, the mutation-testing-elements `mutation.json`
StrykerJS writes, and the rows runner's report. No tool runs here.
"""

import argparse
import ast
import contextlib
import dataclasses
import importlib.util
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from unittest import mock

from _support import REPO, examined

VERDICT = REPO / "scripts" / "mutation-verdict.py"
SCRUB = REPO / "scripts" / "public-scrub.py"
HEADER = {
    "_": [
        "A fixture's rows (SPEC-039).",
        "target spelling: MUTATIONS is crate-relative, crates/{cell 1}/{cell 2}",
        "target spelling: SCRIPT_MUTATIONS is repo-rooted, {cell 1}",
        "target spelling: CARGO_KILLED_SCRIPT_MUTATIONS is repo-rooted, {cell 1}",
    ],
    "arities": {"MUTATIONS": [7], "CARGO_KILLED_SCRIPT_MUTATIONS": [7], "SCRIPT_MUTATIONS": [6]},
    "tables": {"MUTATIONS": [], "CARGO_KILLED_SCRIPT_MUTATIONS": [], "SCRIPT_MUTATIONS": []},
}
LIB = "crates/fix/src/lib.rs"
LIB_TEXT = (
    "/// Doubles.\n"
    "pub fn double(x: i64) -> i64 {\n"
    "    x * 2\n"
    "}\n"
    "\n"
    "/// The hour bound.\n"
    "pub const LAST_HOUR: u8 = 23;\n"
)
KILLERS = "crates/fix/tests/double.rs"
KILLERS_TEXT = "#[test]\nfn two_doubles_to_four() {\n    assert_eq!(fix::double(2), 4);\n}\n"
ROW_ON_CONSTANT = [
    "S00050-LAST-HOUR",
    "fix",
    "src/lib.rs",
    "pub const LAST_HOUR: u8 = 23;",
    "pub const LAST_HOUR: u8 = 24;",
    "double::two_doubles_to_four",
    "the last hour of the day is 23",
]


def git(root, *args):
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=True
    ).stdout


class Fixture:
    """A repository with a base commit; `head()` commits the pull request's change."""

    def __init__(self, test, files=None, rows=()):
        scratch = tempfile.TemporaryDirectory()
        test.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name) / "repo"
        self.out = Path(scratch.name) / "out"
        base = {LIB: LIB_TEXT, KILLERS: KILLERS_TEXT, "README.md": "a fixture\n"}
        base.update(files or {})
        base["scripts/mutation-rows.json"] = json.dumps(HEADER, indent=2) + "\n"
        for relative, text in base.items():
            self.write(relative, text)
        self.rows(rows)
        git(self.root, "init", "-q", "-b", "dev")
        git(self.root, "config", "user.email", "fixture@example.invalid")
        git(self.root, "config", "user.name", "fixture")
        self.base = self.commit("the base")

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def rows(self, rows):
        tables = {"MUTATIONS": [], "CARGO_KILLED_SCRIPT_MUTATIONS": [], "SCRIPT_MUTATIONS": []}
        for table, row in rows:
            tables[table].append(row)
        self.write("scripts/mutation-rows.d/S00000-S00099.json", json.dumps({"tables": tables}))

    def commit(self, message):
        git(self.root, "add", "-A")
        git(self.root, "commit", "-q", "-m", message)
        return git(self.root, "rev-parse", "HEAD").strip()

    def head(self, changes, deleted=()):
        for relative, text in changes.items():
            self.write(relative, text)
        for relative in deleted:
            (self.root / relative).unlink()
        return self.commit("the pull request")

    def verdict(self, *args):
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        env.pop("GITHUB_OUTPUT", None)
        return subprocess.run(
            [sys.executable, str(VERDICT), *args],
            capture_output=True,
            text=True,
            env=env,
            timeout=300,
            check=False,
        )

    def plan(self, *extra):
        done = self.verdict(
            "plan",
            "--base",
            self.base,
            "--head",
            "HEAD",
            "--root",
            str(self.root),
            "--out",
            str(self.out),
            *extra,
        )
        if done.returncode != 0:
            raise AssertionError(f"plan failed: {done.stdout}{done.stderr}")
        return json.loads((self.out / "plan.json").read_text(encoding="utf-8"))

    def report(self, name, document):
        path = self.out / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(document), encoding="utf-8")
        return path

    def judge(self, klass, *extra):
        # The fixture's own root: the verdict reads the equivalence record there (SPEC-057 R4).
        return self.verdict(
            "judge",
            "--plan",
            str(self.out / "plan.json"),
            "--class",
            klass,
            "--root",
            str(self.root),
            *extra,
        )


def outcomes(caught=0, missed=(), timeout=0, unviable=0, file=LIB, total=None):
    """cargo-mutants 27.1.0's outcomes.json, with one entry per missed mutant. `total` above the
    counts is a report the tool wrote before it finished: it writes the file as it goes."""
    entries = [{"scenario": "Baseline", "summary": "Success"}]
    for name in missed:
        entries.append(
            {
                "scenario": {"Mutant": {"name": name, "file": file, "package": "fix"}},
                "summary": "MissedMutant",
            }
        )
    counted = caught + len(missed) + timeout + unviable
    total = counted if total is None else total
    return {
        "outcomes": entries,
        "total_mutants": total,
        "caught": caught,
        "missed": len(missed),
        "timeout": timeout,
        "unviable": unviable,
        "success": 0,
        "cargo_mutants_version": "27.1.0",
    }


def stryker(statuses, file="src/lib/start.ts"):
    """A mutation-testing-elements report, as StrykerJS 10.0.0's json reporter writes it."""
    mutants = [
        {
            "id": str(index),
            "mutatorName": "ConditionalExpression",
            "replacement": "false",
            "location": {"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 9}},
            "status": status,
        }
        for index, status in enumerate(statuses)
    ]
    return {
        "schemaVersion": "1",
        "thresholds": {"high": 100, "low": 99},
        "files": {file: {"language": "typescript", "source": "x", "mutants": mutants}},
    }


class ThePlanReadsTheDiff(unittest.TestCase):
    def test_the_production_classes_are_exact(self):
        expected = {
            "crates/fix/src/lib.rs": "rust",
            "crates/fix/src/deep/mod.rs": "rust",
            "crates/fix/tests/double.rs": "other",
            "crates/fix/build.rs": "other",
            "web/app/src/lib/start.ts": "web",
            "web/app/src/routes/+page.svelte": "web",
            "web/app/src/lib/start.test.ts": "other",
            "web/app/src/lib/start.spec.ts": "other",
            "web/app/src/app.d.ts": "other",
            "web/app/src/lib/paraglide/messages.js": "other",
            "web/app/tests/smoke.spec.ts": "other",
            "tools/parity-oracle/generate.py": "oracle",
            "tools/parity-oracle/test_generate.py": "other",
            "tools/parity-oracle/registry/spec_001.py": "other",
            "tools/parity-oracle/golden.rs": "other",
            "scripts/check.py": "scripts",
            "docs/notes.md": "other",
        }
        fixture = Fixture(self)
        fixture.head({path: "let x = 1;\n" for path in expected})
        plan = fixture.plan()
        found = {entry["path"]: entry["class"] for entry in plan["files"]}
        for path in examined("classified paths", sorted(expected)):
            self.assertEqual(found.get(path), expected[path], path)

    def test_a_comment_opener_inside_a_string_is_not_a_comment(self):
        rust = (
            'pub const MEMBERS: &str = "crates/*";\n'
            'pub const RAW: &str = r#"a "*/" b /* c"#;\n'
            "pub const OPEN: char = '/';\n"
            "pub const STAR: char = '*';\n"
            'pub const TEXT: &str = "first line\n'
            "// second line, inside the string\n"
            '";\n'
            "\n"
            "/// Big.\n"
            "pub fn big(x: i64) -> bool {\n"
            "    x > 3\n"
            "}\n"
        )
        web = (
            "export const GLOB = '/*';\n"
            "export const DOC = `\n"
            "// inside a template\n"
            "${'/*'} still the template\n"
            "`;\n"
            "export const SHAPE = /[/*]x/;\n"
            "export const next = (n: number): number => n + 1;\n"
        )
        lib, start = "crates/fix/src/strings.rs", "web/app/src/lib/strings.ts"
        fixture = Fixture(self, files={lib: rust, start: web})
        fixture.head(
            {
                lib: rust.replace("// second line, inside", "// the second line, inside")
                .replace("/// Big.", "/// Big: past three.")
                .replace("x > 3", "x > 4"),
                start: web.replace("// inside a template", "// inside the template").replace(
                    "n + 1", "n + 2"
                ),
            }
        )
        plan = fixture.plan()
        files = {entry["path"]: entry for entry in plan["files"]}
        # Code: a line inside a string, and a comparison after four literals that each hold a
        # comment opener. Quiet: a doc comment.
        self.assertEqual(files[lib]["code"], [6, 11])
        self.assertEqual(files[lib]["quiet"], [9])
        self.assertEqual(files[start]["code"], [3, 7])
        self.assertTrue(plan["classes"]["rust"]["applies"])
        self.assertTrue(plan["classes"]["web"]["applies"])
        # The control: a genuine comment and a blank line stay quiet in both languages.
        quiet = Fixture(self, files={lib: rust, start: web})
        quiet.head(
            {
                lib: rust.replace("/// Big.", "/// Big.\n//\n\n/* a block */"),
                start: web + "// the end\n/* a block\n   comment */\n",
            }
        )
        still = {entry["path"]: entry for entry in quiet.plan()["files"]}
        self.assertEqual(still[lib]["quiet"], [10, 11, 12])
        self.assertEqual(still[start]["quiet"], [8, 9, 10])
        self.assertEqual(still[lib]["code"] + still[start]["code"], [])


class TheVerdictReadsTheToolsOwnReport(unittest.TestCase):
    def test_a_production_diff_that_examined_nothing_is_void(self):
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        fixture.plan()
        report = fixture.report("mutants.out/outcomes.json", outcomes())
        void = fixture.judge("rust", "--outcomes", str(report), "--tool-exit", "0")
        self.assertEqual(void.returncode, 3, void.stdout + void.stderr)
        self.assertIn("VOID", void.stdout)
        self.assertRegex(void.stdout, r"examined 0\b")
        # The control: the same diff with two caught mutants is green, and says how many.
        report = fixture.report("mutants.out/outcomes.json", outcomes(caught=2))
        green = fixture.judge("rust", "--outcomes", str(report), "--tool-exit", "0")
        self.assertEqual(green.returncode, 0, green.stdout + green.stderr)
        self.assertRegex(green.stdout, r"examined 2\b")

    def test_a_diff_of_blanks_and_comments_reads_not_applicable(self):
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("    x * 2\n", "    // the doubling\n\n    x * 2\n")})
        plan = fixture.plan()
        rust = plan["classes"].get("rust")
        self.assertIsNotNone(rust, "the plan judges no rust class")
        self.assertFalse(rust["applies"])
        done = fixture.judge("rust")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(
            f"not-applicable: {LIB}: 2 changed line(s), all blank or comments", done.stdout
        )
        # A file whose change only deletes lines reads the same way, with its count.
        deleting = Fixture(self)
        deleting.head({LIB: LIB_TEXT.replace("/// The hour bound.\n", "")})
        deleting.plan()
        gone = deleting.judge("rust")
        self.assertEqual(gone.returncode, 0, gone.stdout + gone.stderr)
        self.assertIn(f"not-applicable: {LIB}: 1 line(s) deleted, none added", gone.stdout)

    def test_a_missed_mutant_fails_and_an_unviable_one_is_not_examined(self):
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        fixture.plan()
        missed = "crates/fix/src/lib.rs:3:5: replace + with - in double"
        report = fixture.report("mutants.out/outcomes.json", outcomes(caught=1, missed=[missed]))
        failed = fixture.judge("rust", "--outcomes", str(report), "--tool-exit", "2")
        self.assertEqual(failed.returncode, 1, failed.stdout + failed.stderr)
        self.assertIn(f"MISSED {missed}", failed.stdout)
        self.assertRegex(failed.stdout, r"examined 2\b")
        report = fixture.report("mutants.out/outcomes.json", outcomes(unviable=3))
        void = fixture.judge("rust", "--outcomes", str(report), "--tool-exit", "0")
        self.assertEqual(void.returncode, 3, void.stdout + void.stderr)
        self.assertIn("unviable 3", void.stdout)
        self.assertRegex(void.stdout, r"examined 0\b")
        # Stryker's words: a survivor, or a mutant no test covered, fails the web class.
        web = Fixture(self, files={"web/app/src/lib/start.ts": "export const a = 1;\n"})
        web.head({"web/app/src/lib/start.ts": "export const a = (b: boolean) => b;\n"})
        web.plan()
        report = web.report("mutation.json", stryker(["Killed", "NoCoverage", "CompileError"]))
        uncovered = web.judge("web", "--stryker", str(report))
        self.assertEqual(uncovered.returncode, 1, uncovered.stdout + uncovered.stderr)
        self.assertIn("NoCoverage", uncovered.stdout)
        self.assertRegex(uncovered.stdout, r"examined 2\b")

    def test_a_partial_report_is_void_never_complete(self):
        # cargo-mutants writes outcomes.json as it goes, so a run the runner killed leaves a report
        # that counts only the mutants it reached: here 1 of 3, under a SIGKILL's exit 137.
        fixture = Fixture(self, rows=[("MUTATIONS", ROW_ON_CONSTANT)])
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        plan = fixture.plan()
        self.assertIn("S00050-LAST-HOUR", plan["rows"])
        rows = fixture.report(
            "rows.json", [{"id": "S00050-LAST-HOUR", "verdict": "KILLED", "target": LIB}]
        )
        partial = fixture.report("mutants.out/outcomes.json", outcomes(caught=1, total=3))
        killed = fixture.judge(
            "rust", "--outcomes", str(partial), "--tool-exit", "137", "--rows", str(rows)
        )
        self.assertEqual(killed.returncode, 3, killed.stdout)
        self.assertIn("VOID cargo-mutants exit 137", killed.stdout)
        # Exit 0 over a report short of its total is partial too: the counts decide, not the exit.
        short = fixture.judge(
            "rust", "--outcomes", str(partial), "--tool-exit", "0", "--rows", str(rows)
        )
        self.assertEqual(short.returncode, 3, short.stdout)
        self.assertIn("VOID the report counts 1 of 3 mutants reported", short.stdout)
        # The controls: a whole report reads as the tool left it, a survivor as a failure.
        whole = fixture.report("mutants.out/outcomes.json", outcomes(caught=1, unviable=2))
        green = fixture.judge(
            "rust", "--outcomes", str(whole), "--tool-exit", "0", "--rows", str(rows)
        )
        self.assertEqual(green.returncode, 0, green.stdout)
        self.assertIn(
            "cargo-mutants examined 1 (caught 1, missed 0, timeout 0), unviable 2", green.stdout
        )
        missed = "crates/fix/src/lib.rs:3:5: replace + with - in double"
        survived = fixture.report("mutants.out/outcomes.json", outcomes(caught=1, missed=[missed]))
        red = fixture.judge(
            "rust", "--outcomes", str(survived), "--tool-exit", "2", "--rows", str(rows)
        )
        self.assertEqual(red.returncode, 1, red.stdout)
        self.assertIn(f"MISSED {missed}", red.stdout)

    def test_a_proved_row_on_a_changed_line_carries_its_file(self):
        fixture = Fixture(self, rows=[("MUTATIONS", ROW_ON_CONSTANT)])
        # The constant's line changes, and cargo-mutants never mutates a constant.
        fixture.head(
            {
                LIB: LIB_TEXT.replace(
                    "pub const LAST_HOUR: u8 = 23;",
                    "pub const LAST_HOUR: u8 = 23; // the day's last hour",
                )
            }
        )
        fixture.plan()
        report = fixture.report("mutants.out/outcomes.json", outcomes())
        rows = fixture.report(
            "rows.json", [{"id": "S00050-LAST-HOUR", "verdict": "KILLED", "target": LIB}]
        )
        done = fixture.judge(
            "rust", "--outcomes", str(report), "--tool-exit", "0", "--rows", str(rows)
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("examined 0 by cargo-mutants and 1 by rows", done.stdout)
        self.assertEqual(done.stdout.splitlines()[-1], "examined 1")
        # A row that did not kill fails the job, even when the tool examined something.
        rows = fixture.report(
            "rows.json", [{"id": "S00050-LAST-HOUR", "verdict": "SURVIVED", "target": LIB}]
        )
        report = fixture.report("mutants.out/outcomes.json", outcomes(caught=1))
        failed = fixture.judge(
            "rust", "--outcomes", str(report), "--tool-exit", "0", "--rows", str(rows)
        )
        self.assertEqual(failed.returncode, 1, failed.stdout + failed.stderr)
        self.assertIn("S00050-LAST-HOUR: SURVIVED", failed.stdout)

    def test_a_diff_selects_its_rows(self):
        on_path = ROW_ON_CONSTANT
        on_killer = [
            "S00051-DOUBLE",
            "other",
            "src/lib.rs",
            "a * 2",
            "a * 3",
            "double::two_doubles_to_four",
            "d",
        ]
        untouched = [
            "S00052-UNTOUCHED",
            "other",
            "src/lib.rs",
            "a * 2",
            "a * 4",
            "only::the_other_test",
            "d",
        ]
        files = {
            "crates/other/src/lib.rs": "pub fn f(a: i64) -> i64 {\n    a * 2\n}\n",
            "crates/other/tests/double.rs": "#[test]\nfn two_doubles_to_four() {}\n",
            "crates/other/tests/only.rs": "#[test]\nfn the_other_test() {}\n",
        }
        rows = [("MUTATIONS", on_path), ("MUTATIONS", on_killer), ("MUTATIONS", untouched)]
        fixture = Fixture(self, files=files, rows=rows)
        added = [
            "S00053-ADDED",
            "other",
            "src/lib.rs",
            "a * 2",
            "a * 5",
            "only::the_other_test",
            "d",
        ]
        fixture.write(
            "crates/other/tests/double.rs", "#[test]\nfn two_doubles_to_four() { assert!(true); }\n"
        )
        fixture.write(LIB, LIB_TEXT.replace("x * 2", "x + x"))
        fixture.rows(rows + [("MUTATIONS", added)])
        fixture.commit("the pull request")
        plan = fixture.plan()
        self.assertEqual(
            plan["rows"], ["S00050-LAST-HOUR", "S00051-DOUBLE", "S00053-ADDED"], json.dumps(plan)
        )

    def test_a_missing_report_is_void_never_zero(self):
        fixture = Fixture(self, files={"web/app/src/lib/start.ts": "export const a = 1;\n"})
        fixture.head(
            {
                LIB: LIB_TEXT.replace("x * 2", "x + x"),
                "web/app/src/lib/start.ts": "export const a = (b: boolean) => b;\n",
            }
        )
        fixture.plan()
        absent = str(fixture.out / "absent.json")
        rust = fixture.judge("rust", "--outcomes", absent, "--tool-exit", "0")
        self.assertEqual(rust.returncode, 3, rust.stdout + rust.stderr)
        self.assertIn("VOID", rust.stdout)
        self.assertIn("no report", rust.stdout)
        web = fixture.judge("web", "--stryker", absent)
        self.assertEqual(web.returncode, 3, web.stdout + web.stderr)
        self.assertIn("no report", web.stdout)
        # A tool that exited as a usage error or a red baseline examined nothing, whatever its
        # report says.
        report = fixture.report("mutants.out/outcomes.json", outcomes(caught=4))
        baseline = fixture.judge("rust", "--outcomes", str(report), "--tool-exit", "4")
        self.assertEqual(baseline.returncode, 3, baseline.stdout + baseline.stderr)
        self.assertIn("exit 4", baseline.stdout)


ZERO_SCOPE = {
    "in_force": True,
    "state": "done",
    "reason": None,
    "oom": 0,
    "oom_kill": 0,
    "max": 0,
    "peak_percent": 0,
}


def write_scope(directory, record=None):
    """The record every leg's memory scope writes beside `mutants.out/`: zero events unless given."""
    (directory / "memory-scope.json").write_text(
        json.dumps(ZERO_SCOPE if record is None else record), encoding="utf-8"
    )


def battery_reports(root, shards, scopes=None):
    """A battery's downloaded artifacts: {shard: (exit, outcomes or None)}, then rows and Stryker.
    `scopes` maps a shard to its memory-scope record; a shard left out gets the zero record."""
    for shard, (code, report) in shards.items():
        directory = root / f"mutants-shard-{shard}"
        (directory / "mutants.out").mkdir(parents=True)
        write_scope(directory, (scopes or {}).get(shard))
        if code is not None:
            (directory / "cargo-mutants.exit").write_text(f"{code}\n", encoding="utf-8")
        if report is not None:
            (directory / "mutants.out" / "outcomes.json").write_text(json.dumps(report), "utf-8")


class TheBatteryCountsEveryReport(unittest.TestCase):
    def battery(self, reports, shards):
        return subprocess.run(
            [
                sys.executable,
                str(VERDICT),
                "battery",
                "--reports",
                str(reports),
                "--shards",
                str(shards),
            ],
            capture_output=True,
            text=True,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            timeout=120,
            check=False,
        )

    def test_a_missing_or_partial_battery_report_fails_by_name(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        reports = Path(scratch.name) / "reports"
        survivor = "crates/kernel/src/clock.rs:47:88: delete - in now"
        battery_reports(
            reports,
            {
                0: ("2", outcomes(caught=3, missed=[survivor])),
                1: ("0", outcomes(caught=3, total=4)),
                # Shard 2 uploaded nothing: its runner was shut down.
                3: ("137", outcomes(caught=2, total=5)),
                4: (None, outcomes(caught=1)),
            },
        )
        stryker_report = reports / "stryker" / "stryker" / "mutation.json"
        stryker_report.parent.mkdir(parents=True)
        stryker_report.write_text(json.dumps(stryker(["Killed"])), encoding="utf-8")
        done = self.battery(reports, 5)
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        for finding in examined(
            "missing or partial reports",
            [
                "battery: PARTIAL mutants-shard-1: 3 of 4 mutants reported",
                "battery: MISSING mutants-shard-2: no outcomes.json",
                "battery: PARTIAL mutants-shard-3: cargo-mutants exit 137",
                "battery: PARTIAL mutants-shard-4: no cargo-mutants exit recorded",
                "battery: MISSING rows: no rows.json",
                # The whole battery owes the Python population's 16 shards (SPEC-087 R14).
                *[f"battery: MISSING mutation-python-shard-{k}: no report.json" for k in range(16)],
            ],
        ):
            self.assertIn(finding, done.stdout)
        self.assertNotIn("mutants-shard-0:", done.stdout)
        self.assertIn("battery: counted 2 of 23 reports whole", done.stdout)
        self.assertRegex(done.stdout, r"(?m)^examined 23 report")
        # The control: every shard, the rows and the sweep reported whole.
        whole = Path(scratch.name) / "whole"
        battery_reports(
            whole, {0: ("2", outcomes(caught=3, missed=[survivor])), 1: ("0", outcomes())}
        )
        (whole / "rows").mkdir()
        (whole / "rows" / "rows.json").write_text(
            json.dumps([{"id": "S00050-LAST-HOUR", "verdict": "KILLED", "target": LIB}]), "utf-8"
        )
        (whole / "stryker").mkdir()
        (whole / "stryker" / "mutation.json").write_text(json.dumps(stryker(["Killed"])), "utf-8")
        for shard in range(16):
            directory = whole / f"mutation-python-shard-{shard}"
            directory.mkdir()
            (directory / "report.json").write_text(
                json.dumps({"schema": "deckstreak.mutation-python.v1", "files": [], "exit": 0}),
                "utf-8",
            )
        green = self.battery(whole, 2)
        self.assertEqual(green.returncode, 0, green.stdout + green.stderr)
        self.assertIn("battery: counted 20 of 20 reports whole", green.stdout)


class TheWeeklySurvivorsBecomeIssues(unittest.TestCase):
    def test_survivors_become_deduplicated_scrubbed_drafts(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        reports = Path(scratch.name) / "reports"
        drafts = Path(scratch.name) / "drafts"
        shard = reports / "mutants-shard-3" / "mutants.out"
        shard.mkdir(parents=True)
        kernel = "crates/kernel/src/clock.rs:47:88: delete - in <impl Clock for SystemClock>::now"
        vault = "crates/vault/src/rails.rs:12:5: replace refuses -> bool with true"
        rust = outcomes(caught=5, missed=[kernel], file="crates/kernel/src/clock.rs")
        rust["outcomes"].append(
            {
                "scenario": {
                    "Mutant": {
                        "name": vault,
                        "file": "crates/vault/src/rails.rs",
                        "package": "deck-streak-vault",
                    }
                },
                "summary": "MissedMutant",
            }
        )
        (shard / "outcomes.json").write_text(json.dumps(rust), encoding="utf-8")
        web = reports / "stryker" / "mutation.json"
        web.parent.mkdir(parents=True)
        web.write_text(json.dumps(stryker(["Killed", "Survived"])), encoding="utf-8")
        titles = Path(scratch.name) / "open-titles.json"
        titles.write_text(
            json.dumps(["Mutation survivors: crates/vault/src/rails.rs"]), encoding="utf-8"
        )
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        done = subprocess.run(
            [
                sys.executable,
                str(VERDICT),
                "survivors",
                "--reports",
                str(reports),
                "--out",
                str(drafts),
                "--open-titles",
                str(titles),
            ],
            capture_output=True,
            text=True,
            env=env,
            timeout=120,
            check=False,
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertTrue((drafts / "drafts.json").is_file(), "no drafts were written")
        manifest = json.loads((drafts / "drafts.json").read_text(encoding="utf-8"))
        wanted = {entry["title"]: entry for entry in examined("drafts", manifest)}
        self.assertEqual(
            sorted(wanted),
            [
                "Mutation survivors: crates/kernel/src/clock.rs",
                "Mutation survivors: crates/vault/src/rails.rs",
                "Mutation survivors: web/app/src/lib/start.ts",
            ],
        )
        self.assertTrue(wanted["Mutation survivors: crates/vault/src/rails.rs"]["open"])
        self.assertFalse(wanted["Mutation survivors: crates/kernel/src/clock.rs"]["open"])
        body = (
            drafts / wanted["Mutation survivors: crates/kernel/src/clock.rs"]["body"]
        ).read_text(encoding="utf-8")
        self.assertIn(kernel, body)
        self.assertIn("EQUIVALENT", body)
        self.assertNotRegex(body, r"/home/|/tmp/|/Users/")
        scrubbed = subprocess.run(
            [
                sys.executable,
                str(SCRUB),
                "--root",
                str(REPO),
                "--no-tree",
                "--subject",
                str(drafts),
            ],
            capture_output=True,
            text=True,
            env={k: v for k, v in env.items() if k != "PERSONA_CORE_DENY_LIST"},
            timeout=300,
            check=False,
        )
        self.assertEqual(scrubbed.returncode, 0, scrubbed.stdout + scrubbed.stderr)
        self.assertRegex(scrubbed.stdout, r"examined [1-9]\d* file")


class EachEventReadsItsCase(unittest.TestCase):
    def test_each_event_reads_its_case_by_name(self):
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x"), "docs/notes.md": "a note\n"})
        merge = "Merge pull request #42 from RexRenatus/feat/x\n\nthe body"
        release = "a release pull request into main is judged on its merge diff"
        cases = [
            ("pull_request", "dev", "", "diff", "the pull request into dev is judged on its diff"),
            ("pull_request", "main", "", "diff", release),
            ("push", "", merge, "not-applicable", "this push merges #42"),
            ("push", "", "chore: a push that merges nothing", "diff", "names no pull request"),
        ]
        for event, base_ref, subject, decision, reason in examined("events", cases):
            plan = fixture.plan("--event", event, "--base-ref", base_ref, "--subject", subject)
            scope = plan.get("scope") or {}
            case = f"{event} {base_ref}".strip()
            self.assertEqual(scope.get("decision"), decision, case)
            self.assertIn(reason, scope.get("reason", ""), case)
            judged = fixture.judge("rust")
            if decision == "not-applicable":
                self.assertEqual(judged.returncode, 0, judged.stdout)
                self.assertIn(f"not-applicable: {scope['reason']}", judged.stdout)
            else:
                # The diff is judged: a changed code line with no report is VOID, never a pass.
                self.assertEqual(judged.returncode, 3, judged.stdout)
                self.assertIn("no report", judged.stdout)
        # A pull request whose diff holds no web production path names the paths it changes.
        fixture.plan("--event", "pull_request", "--base-ref", "dev", "--subject", "")
        web = fixture.judge("web")
        self.assertEqual(web.returncode, 0, web.stdout)
        self.assertIn(
            "not-applicable: the diff changes no web production file; it changes "
            f"{LIB}, docs/notes.md",
            web.stdout,
        )


def verdict_module():
    """mutation-verdict.py loaded as a module, so a test reads its constants and never restates
    them."""
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location("mutation_verdict_constants", VERDICT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def listing(packages):
    """cargo-mutants' `--list --json` over the fixture's diff: one mutant for each package given."""
    return [
        {
            "name": f"{LIB}:3:5: replace double -> i64 with {index}",
            "package": package,
            "file": LIB,
            "genre": "FnValue",
        }
        for index, package in enumerate(packages)
    ]


def run_shards(fixture, listed):
    """`shards` over the fixture's plan and `listed`, as the plan's job runs it: (the run, the plan
    it rewrote, the step outputs it wrote)."""
    outputs = fixture.out / "github-output"
    outputs.write_text("", encoding="utf-8")
    args = [sys.executable, str(VERDICT), "shards", "--plan", str(fixture.out / "plan.json")]
    if listed is not None:
        args += ["--listed", str(fixture.report("listed.json", listed))]
    done = subprocess.run(
        args,
        capture_output=True,
        text=True,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1", GITHUB_OUTPUT=str(outputs)),
        timeout=300,
        check=False,
    )
    plan = json.loads((fixture.out / "plan.json").read_text(encoding="utf-8"))
    written = outputs.read_text(encoding="utf-8").splitlines()
    return done, plan, dict(line.split("=", 1) for line in written if "=" in line)


class TheShardsFitTheirBound(unittest.TestCase):
    def test_the_plan_shards_the_listed_mutants_within_their_bound(self):
        module = verdict_module()
        costs = module.SECONDS_PER_MUTANT
        highest = max(costs.values())
        costly, cheap = max(costs, key=costs.get), min(costs, key=costs.get)
        bound, baseline = module.SHARD_BOUND_SECONDS, module.BASELINE_SECONDS
        fits = (bound - baseline) // highest
        # Two shards' worth of the costliest package, then cheap mutants, then a package the table
        # does not name, which costs the table's highest: three shards hold them.
        packages = [costly] * (2 * fits) + [cheap] * 30 + ["deck-streak-unnamed"] * 3
        listed = listing(packages)
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        fixture.plan()
        done, plan, outputs = run_shards(fixture, listed)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        sharding = plan.get("shards")
        self.assertIsNotNone(sharding, f"the plan holds no shards: {done.stdout}")

        def slowest(shards):
            totals = [baseline] * shards
            for index, package in enumerate(packages):
                totals[index % shards] += costs.get(package, highest)
            return max(totals)

        count = sharding["count"]
        self.assertEqual(count, 3, done.stdout)
        # The fewest shards whose slowest is projected within the bound.
        self.assertLessEqual(slowest(count), bound)
        self.assertTrue(all(slowest(fewer) > bound for fewer in range(1, count)), count)
        # Round-robin, as cargo-mutants assigns them: mutant i runs in shard i mod n, and only there.
        self.assertEqual([shard["shard"] for shard in sharding["shards"]], list(range(count)))
        held = Counter()
        for shard in sharding["shards"]:
            k = shard["shard"]
            names = [mutant["name"] for index, mutant in enumerate(listed) if index % count == k]
            self.assertEqual(shard["mutants"], names, k)
            self.assertLessEqual(shard["projected_seconds"], bound, k)
            held.update(shard["mutants"])
        for name in examined("listed mutants", [mutant["name"] for mutant in listed]):
            self.assertEqual(held[name], 1, name)
        self.assertEqual(outputs.get("shards"), str(count))
        self.assertEqual(json.loads(outputs.get("matrix", "null")), list(range(count)))
        # A diff beyond the most shards a matrix holds is refused with its projection: never capped.
        fixture.plan()
        beyond = listing([costly] * (module.MAX_SHARDS * fits + 1))
        refused, unsharded, none = run_shards(fixture, beyond)
        self.assertEqual(refused.returncode, 1, refused.stdout + refused.stderr)
        self.assertIn(f"REFUSED: {len(beyond)} mutant(s)", refused.stdout)
        self.assertIn(f"more than {module.MAX_SHARDS} shards", refused.stdout)
        self.assertNotIn("shards", unsharded)
        self.assertNotIn("matrix", none)
        # The Rust class applies and no listing came: VOID, never one shard of nothing.
        fixture.plan()
        unlisted, _, _ = run_shards(fixture, None)
        self.assertEqual(unlisted.returncode, 3, unlisted.stdout + unlisted.stderr)
        self.assertIn("holds no cargo-mutants listing", unlisted.stdout)
        # A file that is no plan is VOID, never one shard of nothing.
        bogus = fixture.report("bogus.json", {"not": "a plan"})
        unplanned = subprocess.run(
            [sys.executable, str(VERDICT), "shards", "--plan", str(bogus)],
            capture_output=True,
            text=True,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            timeout=300,
            check=False,
        )
        self.assertEqual(unplanned.returncode, 3, unplanned.stdout + unplanned.stderr)
        self.assertIn("is not a mutation plan", unplanned.stdout)
        # A diff with no Rust to mutate runs one shard, which reads its case by name.
        docs = Fixture(self)
        docs.head({"docs/notes.md": "a note\n"})
        docs.plan()
        quiet, plan, outputs = run_shards(docs, None)
        self.assertEqual(quiet.returncode, 0, quiet.stdout + quiet.stderr)
        self.assertEqual(plan["shards"]["count"], 1)
        self.assertEqual(json.loads(outputs.get("matrix", "null")), [0])


def sharded(test):
    """A fixture whose diff lists three shards' worth of the costliest mutants: (the fixture, each
    shard's planned mutants, in shard order)."""
    module = verdict_module()
    costs = module.SECONDS_PER_MUTANT
    costly = max(costs, key=costs.get)
    fits = (module.SHARD_BOUND_SECONDS - module.BASELINE_SECONDS) // costs[costly]
    fixture = Fixture(test)
    fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
    fixture.plan()
    done, plan, _ = run_shards(fixture, listing([costly] * (2 * fits + 1)))
    test.assertEqual(done.returncode, 0, done.stdout + done.stderr)
    return fixture, [shard["mutants"] for shard in plan["shards"]["shards"]]


def shard_outcomes(names, missed=(), total=None):
    """A shard's outcomes.json, as cargo-mutants writes it: every one of `names` caught but the
    `missed` ones. `total` above the count is a report left partial."""
    entries = [{"scenario": "Baseline", "summary": "Success"}]
    for name in names:
        entries.append(
            {
                "scenario": {"Mutant": {"name": name, "file": LIB, "package": "fix"}},
                "summary": "MissedMutant" if name in missed else "CaughtMutant",
            }
        )
    return {
        "outcomes": entries,
        "total_mutants": len(names) if total is None else total,
        "caught": len(names) - len(missed),
        "missed": len(missed),
        "timeout": 0,
        "unviable": 0,
        "success": 0,
        "cargo_mutants_version": "27.1.0",
    }


def shard_reports(root, reports, scopes=None):
    """Each shard's artifact, as the verdict's job downloads it: {shard: (exit, outcomes or None)}.
    A shard left out uploaded nothing. `scopes` maps a shard to its memory-scope record; a shard
    left out gets the zero record."""
    for shard, (code, report) in reports.items():
        directory = root / f"mutation-rust-shard-{shard}"
        directory.mkdir(parents=True)
        write_scope(directory, (scopes or {}).get(shard))
        (directory / "cargo-mutants.exit").write_text(f"{code}\n", encoding="utf-8")
        if report is not None:
            (directory / "mutants.out").mkdir()
            (directory / "mutants.out" / "outcomes.json").write_text(json.dumps(report), "utf-8")
    return str(root)


class TheVerdictCountsEveryShard(unittest.TestCase):
    def test_a_missing_or_partial_shard_report_is_void_by_name(self):
        fixture, planned = sharded(self)
        self.assertEqual(len(planned), 3)
        survivor = planned[0][0]
        broken = shard_reports(
            fixture.out / "broken",
            {
                0: ("2", shard_outcomes(planned[0], missed=[survivor])),
                # Shard 1 uploaded nothing: its runner was shut down.
                2: ("137", shard_outcomes(planned[2], total=len(planned[2]) + 4)),
            },
        )
        done = fixture.judge("rust", "--shard-reports", broken)
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        for finding in examined(
            "shard findings",
            [
                f"mutation-rust-shard-0: MISSED {survivor}",
                "VOID mutation-rust-shard-1: no report",
                "VOID mutation-rust-shard-2: cargo-mutants exit 137",
            ],
        ):
            self.assertIn(finding, done.stdout)
        # The last shard missing alone fails the verdict, however much the others examined.
        last = shard_reports(
            fixture.out / "last",
            {0: ("0", shard_outcomes(planned[0])), 1: ("0", shard_outcomes(planned[1]))},
        )
        missing = fixture.judge("rust", "--shard-reports", last)
        self.assertEqual(missing.returncode, 3, missing.stdout + missing.stderr)
        self.assertIn("VOID mutation-rust-shard-2: no report", missing.stdout)
        # The missing shard is named once, not once for each mutant it held.
        self.assertNotIn("never tested", missing.stdout)
        # The control: every shard the plan promised reported whole.
        whole = shard_reports(
            fixture.out / "whole",
            {shard: ("0", shard_outcomes(names)) for shard, names in enumerate(planned)},
        )
        green = fixture.judge("rust", "--shard-reports", whole)
        self.assertEqual(green.returncode, 0, green.stdout + green.stderr)
        self.assertRegex(green.stdout, rf"(?m)^examined {sum(map(len, planned))}$")
        # A plan the shards were never sized for promises no report, and says so.
        fixture.plan()
        unsized = fixture.judge("rust", "--shard-reports", whole)
        self.assertEqual(unsized.returncode, 3, unsized.stdout + unsized.stderr)
        self.assertIn("VOID the plan names no shards", unsized.stdout)

    def test_the_shards_reports_hold_every_listed_mutant_once(self):
        fixture, planned = sharded(self)
        whole = {shard: ("0", shard_outcomes(names)) for shard, names in enumerate(planned)}
        # Shard 1 also tested shard 2's first mutant: one mutant in two shards.
        stray = planned[2][0]
        twice = shard_reports(
            fixture.out / "twice", {**whole, 1: ("0", shard_outcomes(planned[1] + [stray]))}
        )
        doubled = fixture.judge("rust", "--shard-reports", twice)
        self.assertEqual(doubled.returncode, 1, doubled.stdout + doubled.stderr)
        self.assertIn(
            f"{stray}: tested in 2 shard(s) (mutation-rust-shard-1, mutation-rust-shard-2), "
            "listed 1 time(s)",
            doubled.stdout,
        )
        # A mutant the listing never named fails the same way.
        unlisted = f"{LIB}:3:5: replace double -> i64 with -1"
        extra = shard_reports(
            fixture.out / "extra", {**whole, 0: ("0", shard_outcomes(planned[0] + [unlisted]))}
        )
        surplus = fixture.judge("rust", "--shard-reports", extra)
        self.assertEqual(surplus.returncode, 1, surplus.stdout + surplus.stderr)
        self.assertIn(
            f"{unlisted}: tested in 1 shard(s) (mutation-rust-shard-0), listed 0 time(s)",
            surplus.stdout,
        )
        # Every shard whole, and one listed mutant in none: VOID, by name.
        lost = planned[1][-1]
        short = shard_reports(
            fixture.out / "short", {**whole, 1: ("0", shard_outcomes(planned[1][:-1]))}
        )
        never = fixture.judge("rust", "--shard-reports", short)
        self.assertEqual(never.returncode, 3, never.stdout + never.stderr)
        self.assertIn(f"VOID never tested: {lost}, listed for mutation-rust-shard-1", never.stdout)
        # The control: each shard holds exactly the mutants the plan gave it.
        exact = shard_reports(fixture.out / "exact", whole)
        green = fixture.judge("rust", "--shard-reports", exact)
        self.assertEqual(green.returncode, 0, green.stdout + green.stderr)
        for name in examined("listed mutants", [name for names in planned for name in names]):
            self.assertNotIn(f"{name}: tested in", green.stdout)

    def test_a_diff_the_tool_lists_no_mutant_of_needs_no_shard_report(self):
        # cargo-mutants lists no mutant of a changed constant, and with none to test it exits 0 and
        # writes no mutants.out at all (measured, SPEC-039 section 8).
        fixture = Fixture(self, rows=[("MUTATIONS", ROW_ON_CONSTANT)])
        fixture.head(
            {
                LIB: LIB_TEXT.replace(
                    "pub const LAST_HOUR: u8 = 23;",
                    "pub const LAST_HOUR: u8 = 23; // the day's last hour",
                )
            }
        )
        plan = fixture.plan()
        self.assertTrue(plan["classes"]["rust"]["applies"])
        done, plan, _ = run_shards(fixture, [])
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(plan["shards"]["count"], 1)
        nothing = shard_reports(fixture.out / "nothing", {0: ("0", None)})
        rows = fixture.report(
            "rows.json", [{"id": "S00050-LAST-HOUR", "verdict": "KILLED", "target": LIB}]
        )
        carried = fixture.judge("rust", "--shard-reports", nothing, "--rows", str(rows))
        self.assertEqual(carried.returncode, 0, carried.stdout + carried.stderr)
        self.assertIn("examined 0 by cargo-mutants and 1 by rows", carried.stdout)
        self.assertEqual(carried.stdout.splitlines()[-1], "examined 1")
        # With no row to carry it, the changed code line examined nothing: VOID.
        bare = fixture.judge("rust", "--shard-reports", nothing)
        self.assertEqual(bare.returncode, 3, bare.stdout + bare.stderr)
        self.assertIn("VOID production code changed and nothing was examined", bare.stdout)
        # A shard the plan gave mutants still owes its report, and one whose run did not exit 0 too.
        fixture.plan()
        run_shards(fixture, listing(["deck-streak-unnamed"]))
        owed = fixture.judge("rust", "--shard-reports", nothing, "--rows", str(rows))
        self.assertEqual(owed.returncode, 3, owed.stdout + owed.stderr)
        self.assertIn("VOID mutation-rust-shard-0: no report", owed.stdout)
        fixture.plan()
        run_shards(fixture, [])
        failed = shard_reports(fixture.out / "failed", {0: ("1", None)})
        usage = fixture.judge("rust", "--shard-reports", failed, "--rows", str(rows))
        self.assertEqual(usage.returncode, 3, usage.stdout + usage.stderr)
        self.assertIn("VOID mutation-rust-shard-0: no report", usage.stdout)


# --------------------------------------------------------------------------- test-only lines (SPEC-057 R22)

SPANS = "crates/fix/src/spans.rs"
#: Production code around three test items: a `#[cfg(test)]` function under a second attribute, a
#: free function under `#[tokio::test(...)]`, whose path ends in `test`, and a `#[cfg(test)]` module
#: under an attribute of its own, whose two `#[test]` functions hold a string that holds a brace.
#: Before them a string, a raw string, a character and two comments hold a brace, `#[cfg(test)]` or
#: `#[test]`, and a `#[cfg(not(test))]` function is production code, which cargo-mutants mutates.
SPANS_TEXT = (
    "//! Production code around three test items.\n"
    "\n"
    "/// Doubles.\n"
    "pub fn double(x: i64) -> i64 {\n"
    "    x * 2\n"
    "}\n"
    "\n"
    "/// The hour bound: a constant, which no tool mutates.\n"
    "pub const LAST_HOUR: u8 = 23;\n"
    "\n"
    "/// A brace, a test mark and a test attribute in a literal open nothing.\n"
    "pub fn braces() -> usize {\n"
    '    "}".len() + r#"{ #[cfg(test)] mod t {"#.len() + \'{\'.len_utf8()\n'
    "}\n"
    "\n"
    "// #[cfg(test)] in a comment marks nothing, /* nor { this */\n"
    "/* #[test] fn not_a_test() { */\n"
    "pub fn after_comments(x: i64) -> i64 {\n"
    "    x + 1\n"
    "}\n"
    "\n"
    "/// Compiled in production: `not(test)` is no test mark.\n"
    "#[cfg(not(test))]\n"
    "pub fn only_in_production(x: i64) -> i64 {\n"
    "    x - 1\n"
    "}\n"
    "\n"
    "#[cfg(test)]\n"
    "#[allow(dead_code)]\n"
    "fn helper() -> i64 {\n"
    "    double(21)\n"
    "}\n"
    "\n"
    '#[tokio::test(flavor = "multi_thread")]\n'
    "async fn a_free_test() {\n"
    "    assert_eq!(helper(), 42);\n"
    "}\n"
    "\n"
    "#[allow(clippy::unwrap_used)]\n"
    "#[cfg(test)]\n"
    "mod tests {\n"
    "    use super::double;\n"
    "\n"
    "    #[test]\n"
    "    fn two_doubles_to_four() {\n"
    "        assert_eq!(double(2), 4);\n"
    '        assert_eq!("}".len(), 1);\n'
    "    }\n"
    "\n"
    "    #[test]\n"
    "    fn three_doubles_to_six() {\n"
    "        assert_eq!(double(3), 6);\n"
    "    }\n"
    "}\n"
    "\n"
    "/// After the tests: production again.\n"
    "pub fn big(x: i64) -> bool {\n"
    "    x > 3\n"
    "}\n"
)
#: Lines 31, 36, 43 (a test of one line, added), 47 and 53: each inside a test item.
TEST_ONLY_HEAD = (
    SPANS_TEXT.replace("double(21)", "double(20) + 2")
    .replace("assert_eq!(helper(), 42);", "assert_eq!(helper(), 6 * 7);")
    .replace("assert_eq!(double(2), 4);", "assert_eq!(double(2), 2 + 2);")
    .replace("assert_eq!(double(3), 6);", "assert_eq!(double(3), 3 * 2);")
    .replace(
        "    use super::double;\n",
        "    use super::double;\n"
        '    #[test] fn a_brace_closes_nothing() { assert_eq!("{".len(), 1); }\n',
    )
)
#: Line 46, in the test module, and line 58, `big`'s comparison after it.
MIXED_HEAD = SPANS_TEXT.replace("x > 3", "x > 4").replace(
    "assert_eq!(double(2), 4);", "assert_eq!(double(2), 2 + 2);"
)
#: Lines 13, 19, 25 and 54: after the literals, after the comments, under `cfg(not(test))`, and a
#: constant on the line of the brace that closes the test module, which production code shares.
PRODUCTION_HEAD = (
    SPANS_TEXT.replace("'{'.len_utf8()", "'}'.len_utf8()")
    .replace("x + 1", "x + 2")
    .replace("x - 1", "x - 2")
    .replace("}\n\n/// After the tests", "} pub const TAIL: u8 = 7;\n\n/// After the tests")
)
#: Line 9, a constant's: production code that cargo-mutants lists no mutant of.
CONSTANT_HEAD = SPANS_TEXT.replace("LAST_HOUR: u8 = 23;", "LAST_HOUR: u8 = 24;")
#: How the verdict names a test-only line.
TEST_ONLY_WHY = (
    "inside an item marked #[cfg(test)] or with a test attribute, which cargo-mutants never mutates"
)


def spans_listed(entries):
    """cargo-mutants 27.1.0's `--list --json --in-diff` over one of SPANS_TEXT's diffs, as it listed
    them in a workspace holding the fixture (a listing builds nothing): one entry for each
    (line, column, end column, description, genre)."""
    return [
        {
            "name": f"{SPANS}:{line}:{column}: {description}",
            "package": "deck-streak-fix",
            "file": SPANS,
            "genre": genre,
            "span": {
                "start": {"line": line, "column": column},
                "end": {"line": line, "column": end},
            },
        }
        for line, column, end, description, genre in entries
    ]


#: The mixed diff's listing: `big`'s five mutants, all on line 58, none in the test module.
MIXED_LISTED = spans_listed(
    [
        (58, 5, 10, "replace big -> bool with true", "FnValue"),
        (58, 5, 10, "replace big -> bool with false", "FnValue"),
        (58, 7, 8, "replace > with == in big", "BinaryOperator"),
        (58, 7, 8, "replace > with < in big", "BinaryOperator"),
        (58, 7, 8, "replace > with >= in big", "BinaryOperator"),
    ]
)
#: The production-only diff's listing: sixteen mutants on lines 13, 19 and 25, none on line 54's
#: constant.
PRODUCTION_LISTED = spans_listed(
    [
        (13, 5, 67, "replace braces -> usize with 0", "FnValue"),
        (13, 5, 67, "replace braces -> usize with 1", "FnValue"),
        (13, 51, 52, "replace + with - in braces", "BinaryOperator"),
        (13, 51, 52, "replace + with * in braces", "BinaryOperator"),
        (13, 15, 16, "replace + with - in braces", "BinaryOperator"),
        (13, 15, 16, "replace + with * in braces", "BinaryOperator"),
        (19, 5, 10, "replace after_comments -> i64 with 0", "FnValue"),
        (19, 5, 10, "replace after_comments -> i64 with 1", "FnValue"),
        (19, 5, 10, "replace after_comments -> i64 with -1", "FnValue"),
        (19, 7, 8, "replace + with - in after_comments", "BinaryOperator"),
        (19, 7, 8, "replace + with * in after_comments", "BinaryOperator"),
        (25, 5, 10, "replace only_in_production -> i64 with 0", "FnValue"),
        (25, 5, 10, "replace only_in_production -> i64 with 1", "FnValue"),
        (25, 5, 10, "replace only_in_production -> i64 with -1", "FnValue"),
        (25, 7, 8, "replace - with + in only_in_production", "BinaryOperator"),
        (25, 7, 8, "replace - with / in only_in_production", "BinaryOperator"),
    ]
)


def spans_plan(fixture):
    """`plan` over the fixture's diff: (the lines it printed, the plan, SPANS's record in it)."""
    done = fixture.verdict(
        "plan",
        "--base",
        fixture.base,
        "--head",
        "HEAD",
        "--root",
        str(fixture.root),
        "--out",
        str(fixture.out),
    )
    if done.returncode != 0:
        raise AssertionError(f"plan failed: {done.stdout}{done.stderr}")
    plan = json.loads((fixture.out / "plan.json").read_text(encoding="utf-8"))
    return done.stdout, plan, next(entry for entry in plan["files"] if entry["path"] == SPANS)


def spans_shards(fixture, listing):
    """`shards` over the fixture's plan and a listing file holding `listing`, the listing step's
    output as bytes, or none at all: (the run, the plan it rewrote, the step outputs it wrote)."""
    outputs = fixture.out / "github-output"
    outputs.write_text("", encoding="utf-8")
    listed = fixture.out / "listed.json"
    listed.unlink(missing_ok=True)
    if listing is not None:
        listed.write_bytes(listing)
    done = subprocess.run(
        [
            sys.executable,
            str(VERDICT),
            "shards",
            "--plan",
            str(fixture.out / "plan.json"),
            "--listed",
            str(listed),
        ],
        capture_output=True,
        text=True,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1", GITHUB_OUTPUT=str(outputs)),
        timeout=300,
        check=False,
    )
    plan = json.loads((fixture.out / "plan.json").read_text(encoding="utf-8"))
    written = outputs.read_text(encoding="utf-8").splitlines()
    return done, plan, dict(line.split("=", 1) for line in written if "=" in line)


def spans_caught(names):
    """A shard's outcomes.json in which cargo-mutants caught every one of `names`, in SPANS."""
    report = shard_outcomes(names)
    for outcome in report["outcomes"][1:]:
        outcome["scenario"]["Mutant"]["file"] = SPANS
    return report


class ATestOnlySrcDiffReadsNotApplicable(unittest.TestCase):
    def test_a_test_only_src_diff_reads_not_applicable_and_a_production_line_still_applies(self):
        # Four planted fixtures, one subtest each, each red before R22 for its own reason. The
        # listings are cargo-mutants 27.1.0's own over these diffs: nothing at all for the
        # test-only and the constant-only diffs, which it exits 0 on before it lists.
        with self.subTest("a test-only diff reads the rust class not-applicable by name"):
            fixture = Fixture(self, files={SPANS: SPANS_TEXT})
            fixture.head({SPANS: TEST_ONLY_HEAD})
            printed, plan, spans = spans_plan(fixture)
            self.assertFalse(
                plan["classes"]["rust"]["applies"],
                f"the rust class applies on test lines {spans['code']}",
            )
            self.assertEqual(spans.get("test"), [31, 36, 43, 47, 53])
            self.assertEqual(spans["code"], [])
            self.assertIn(
                "mutation: plan: rust does not apply: not-applicable: no production code line "
                f"changed; 5 test-only line(s) in 1 file(s), {TEST_ONLY_WHY}",
                printed,
            )
            done = fixture.judge("rust")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertIn(
                f"not-applicable: {SPANS}: 5 changed line(s): 5 test-only, {TEST_ONLY_WHY}",
                done.stdout,
            )
            self.assertIn("verdict: ok", done.stdout)
            self.assertNotIn("VOID", done.stdout)
        with self.subTest("a mixed diff applies, and its listing names its production line's"):
            fixture = Fixture(self, files={SPANS: SPANS_TEXT})
            fixture.head({SPANS: MIXED_HEAD})
            printed, plan, spans = spans_plan(fixture)
            self.assertTrue(plan["classes"]["rust"]["applies"])
            self.assertEqual(spans["code"], [58], "a test module's line is no production code")
            self.assertEqual(spans.get("test"), [46])
            self.assertIn(
                "mutation: plan: rust applies: 1 production code line(s) in 1 file(s); 1 test-only "
                f"line(s) set apart, {TEST_ONLY_WHY}",
                printed,
            )
            listing = json.dumps(MIXED_LISTED).encode()
            done, plan, _ = spans_shards(fixture, listing)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            names = [entry["name"] for entry in MIXED_LISTED]
            self.assertEqual(plan["shards"]["shards"][0]["mutants"], names)
            for entry in examined("mutants the mixed diff lists", MIXED_LISTED):
                self.assertIn(entry["span"]["start"]["line"], spans["code"], entry["name"])
            reports = shard_reports(fixture.out / "caught", {0: ("0", spans_caught(names))})
            judged = fixture.judge("rust", "--shard-reports", reports)
            self.assertEqual(judged.returncode, 0, judged.stdout + judged.stderr)
            self.assertIn(f"{SPANS}: 1 changed code line(s)", judged.stdout)
            self.assertIn(f"{SPANS}: 1 test-only line(s) set apart, {TEST_ONLY_WHY}", judged.stdout)
            self.assertIn("examined 5 by cargo-mutants and 0 by rows", judged.stdout)
        with self.subTest("a production-only diff applies, and the plan names its lines"):
            fixture = Fixture(self, files={SPANS: SPANS_TEXT})
            fixture.head({SPANS: PRODUCTION_HEAD})
            printed, plan, spans = spans_plan(fixture)
            self.assertIn(
                "mutation: plan: rust applies: 4 production code line(s) in 1 file(s)\n", printed
            )
            self.assertTrue(plan["classes"]["rust"]["applies"])
            self.assertEqual(
                spans["code"],
                [13, 19, 25, 54],
                "a line a test module's closing brace shares with a constant is production code",
            )
            self.assertEqual(spans.get("test"), [])
            done, plan, outputs = spans_shards(fixture, json.dumps(PRODUCTION_LISTED).encode())
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            names = [entry["name"] for entry in PRODUCTION_LISTED]
            self.assertEqual(plan["shards"]["shards"][0]["mutants"], names)
            self.assertEqual(json.loads(outputs.get("matrix", "null")), [0])
            for entry in examined("mutants the production-only diff lists", PRODUCTION_LISTED):
                self.assertIn(entry["span"]["start"]["line"], spans["code"], entry["name"])
            reports = shard_reports(fixture.out / "caught", {0: ("0", spans_caught(names))})
            judged = fixture.judge("rust", "--shard-reports", reports)
            self.assertEqual(judged.returncode, 0, judged.stdout + judged.stderr)
            self.assertIn(f"{SPANS}: 4 changed code line(s)", judged.stdout)
            self.assertIn("examined 16 by cargo-mutants and 0 by rows", judged.stdout)
        with self.subTest("the empty --in-diff output is an empty listing, never a missing one"):
            fixture = Fixture(self, files={SPANS: SPANS_TEXT})
            fixture.head({SPANS: CONSTANT_HEAD})
            printed, plan, spans = spans_plan(fixture)
            self.assertTrue(plan["classes"]["rust"]["applies"])
            self.assertEqual(spans["code"], [9])
            done, plan, outputs = spans_shards(fixture, b"")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertIn("mutation: shards: the listing is empty", done.stdout)
            self.assertEqual(plan["shards"]["count"], 1)
            self.assertEqual(plan["shards"]["shards"][0]["mutants"], [])
            self.assertEqual(json.loads(outputs.get("matrix", "null")), [0])
            # The constant's line examined nothing, and no row covers it: VOID, never passing.
            nothing = shard_reports(fixture.out / "nothing", {0: ("0", None)})
            void = fixture.judge("rust", "--shard-reports", nothing)
            self.assertEqual(void.returncode, 3, void.stdout + void.stderr)
            self.assertIn(
                "mutation-rust-shard-0: no mutant listed, and cargo-mutants reports none",
                void.stdout,
            )
            self.assertIn(
                f"unexamined: {SPANS}: no mutant and no row covers its changed lines", void.stdout
            )
            self.assertIn("VOID production code changed and nothing was examined", void.stdout)
            # A listing that is missing, rather than empty, is still VOID.
            fixture.plan()
            missing, unsharded, _ = spans_shards(fixture, None)
            self.assertEqual(missing.returncode, 3, missing.stdout + missing.stderr)
            self.assertIn("holds no cargo-mutants listing", missing.stdout)
            self.assertIsNone(unsharded.get("shards"))


# --------------------------------------------------------------------------- the record (SPEC-057)

FRAGMENTS = "scripts/mutation-equivalent.d"
#: The head the recorded tests judge: `double` gains a second product, so its one line holds two
#: mutants of one description, and the diff changes a code line.
RECORDED = LIB_TEXT.replace("    x * 2\n", "    x * 2 * 1\n")
HOUR = "crates/fix/src/hour.rs"
HOUR_TEXT = "/// The last hour.\npub fn last() -> u8 {\n    23\n}\n"
WEB_FILE = "web/app/src/lib/start.ts"
WEB_BASE = "export const ready = (a: boolean, b: boolean): boolean => a || b;\n"
WEB_HEAD = (
    "export const ready = (a: boolean, b: boolean): boolean => a && b;\n"
    "export const positive = (n: number): boolean => n > 0;\n"
    "export const label = (n: number): string => (n > 1 ? 'many' : 'one');\n"
)


def located(text, needle, occurrence=1, length=None):
    """The 1-based (line, column) where the `occurrence`-th `needle` starts in `text`, and the
    column just past it (or past `length` characters), as both tools report a span."""
    at = -1
    for _ in range(occurrence):
        at = text.index(needle, at + 1)
    line = text.count("\n", 0, at) + 1
    column = at - (text.rfind("\n", 0, at) + 1) + 1
    return line, column, column + (len(needle) if length is None else length)


def cargo_mutant(text, needle, description, occurrence=1, file=LIB, package="deck-streak-fix"):
    """One mutant as cargo-mutants 27.1.0 lists it (`--list --json`) and reports it (the
    `scenario.Mutant` of `outcomes.json`): its span starts at `needle` and ends past it."""
    line, column, end = located(text, needle, occurrence)
    return {
        "file": file,
        "package": package,
        "genre": "BinaryOperator",
        "function": {"function_name": "double", "return_type": "-> i64"},
        "name": f"{file}:{line}:{column}: {description}",
        "replacement": description,
        "span": {"start": {"line": line, "column": column}, "end": {"line": line, "column": end}},
    }


def cargo_report(entries):
    """A shard's outcomes.json holding each (mutant, summary), counted as cargo-mutants counts."""
    keys = {
        "CaughtMutant": "caught",
        "MissedMutant": "missed",
        "Timeout": "timeout",
        "Unviable": "unviable",
    }
    report = {
        "outcomes": [{"scenario": "Baseline", "summary": "Success"}],
        "total_mutants": len(entries),
        "caught": 0,
        "missed": 0,
        "timeout": 0,
        "unviable": 0,
        "success": 0,
        "cargo_mutants_version": "27.1.0",
    }
    for mutant, summary in entries:
        report["outcomes"].append({"scenario": {"Mutant": mutant}, "summary": summary})
        report[keys[summary]] += 1
    return report


def a_record(mutant, anchor, file=LIB, **changes):
    """A whole record of `mutant` (the tool's description), anchored on `anchor`."""
    record = {
        "file": file,
        "mutant": mutant,
        "anchor": anchor,
        "reason": "the fixture's one caller cannot tell this mutant apart",
        "evidence": "the fixture's evidence, stated where a reviewer checks it",
        "reached_by": "double::two_doubles_to_four",
        "issue": "#1",
    }
    record.update(changes)
    return {name: value for name, value in record.items() if value is not None}


def write_records(root, records, fragment="deck-streak-fix.json"):
    path = root / FRAGMENTS / fragment
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"records": records}, indent=2) + "\n", encoding="utf-8")


def judge_recorded(test, head, listed, summaries, records, whole=None, files=None, name="run"):
    """The rust class, judged over one shard holding `listed`, each with its summary, with
    `records` in the fixture's fragment and `whole` (by default `listed`) as the whole tree's
    listing. Returns the run."""
    fixture = Fixture(test, files=files)
    fixture.head({LIB: head})
    fixture.plan()
    done, plan, _ = run_shards(fixture, listed)
    test.assertEqual(plan["shards"]["count"], 1, done.stdout + done.stderr)
    write_records(fixture.root, records)
    missed = "MissedMutant" in summaries
    reports = shard_reports(
        fixture.out / name,
        {0: ("2" if missed else "0", cargo_report(list(zip(listed, summaries, strict=True))))},
    )
    listing = fixture.report(f"{name}-whole.json", listed if whole is None else whole)
    return fixture.judge("rust", "--shard-reports", reports, "--whole", str(listing))


class TheVerdictReadsTheRecord(unittest.TestCase):
    def test_a_recorded_missed_mutant_is_equivalent_and_an_unrecorded_one_fails(self):
        divide = cargo_mutant(RECORDED, "*", "replace * with / in double")
        one = cargo_mutant(RECORDED, "x * 2 * 1", "replace double -> i64 with 1")
        listed = examined("listed mutants", [divide, one])
        recorded = a_record("replace * with / in double", "x * 2")
        done = judge_recorded(self, RECORDED, listed, ["MissedMutant"] * 2, [recorded])
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(f"mutation-rust-shard-0: EQUIVALENT {divide['name']}", done.stdout)
        self.assertNotIn(f"MISSED {divide['name']}", done.stdout)
        self.assertIn(f"mutation-rust-shard-0: MISSED {one['name']}", done.stdout)
        self.assertIn("missed 2: equivalent 1, unexplained 1", done.stdout)
        # The control: each missed mutant recorded, so none is unexplained.
        both = judge_recorded(
            self,
            RECORDED,
            listed,
            ["MissedMutant"] * 2,
            [recorded, a_record("replace double -> i64 with 1", "x * 2 * 1")],
            name="both",
        )
        self.assertEqual(both.returncode, 0, both.stdout + both.stderr)
        self.assertIn("missed 2: equivalent 2, unexplained 0", both.stdout)
        self.assertIn(f"EQUIVALENT {one['name']}", both.stdout)
        self.assertRegex(both.stdout, r"(?m)^examined 2$")

    def test_a_record_binding_no_listed_mutant_is_stale_and_two_is_ambiguous(self):
        first = cargo_mutant(RECORDED, "*", "replace * with + in double")
        second = cargo_mutant(RECORDED, "*", "replace * with + in double", occurrence=2)
        # A mutant of a file the diff never touched: only the whole tree's listing holds it.
        far = cargo_mutant(HOUR_TEXT, "23", "replace last -> u8 with 0", file=HOUR)
        records = [
            a_record("replace * with - in double", "x * 2 * 1"),
            a_record("replace * with + in double", "x * 2 * 1"),
            a_record("replace last -> u8 with 0", "    23\n", file=HOUR),
        ]
        done = judge_recorded(
            self,
            RECORDED,
            [first, second],
            ["MissedMutant"] * 2,
            records,
            whole=[first, second, far],
            files={HOUR: HOUR_TEXT},
        )
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        stale = [line for line in done.stdout.splitlines() if "STALE " in line]
        ambiguous = [line for line in done.stdout.splitlines() if "AMBIGUOUS " in line]
        self.assertEqual(len(stale), 1, done.stdout)
        self.assertIn("STALE deck-streak-fix.json record 1 (", stale[0])
        self.assertIn("binds no listed mutant", stale[0])
        self.assertEqual(len(ambiguous), 1, done.stdout)
        self.assertIn("AMBIGUOUS deck-streak-fix.json record 2 (", ambiguous[0])
        self.assertIn(first["name"], ambiguous[0])
        self.assertIn(second["name"], ambiguous[0])
        # An ambiguous record excuses neither mutant it binds.
        for mutant in examined("mutants the ambiguous record binds", [first, second]):
            self.assertIn(f"MISSED {mutant['name']}", done.stdout)
        # The control: the window narrowed to the second product binds one mutant, and the record
        # of the file the diff never touched binds its mutant in the whole listing.
        narrowed = judge_recorded(
            self,
            RECORDED,
            [first, second],
            ["CaughtMutant", "MissedMutant"],
            [records[2], a_record("replace * with + in double", "2 * 1")],
            whole=[first, second, far],
            files={HOUR: HOUR_TEXT},
            name="narrowed",
        )
        self.assertEqual(narrowed.returncode, 0, narrowed.stdout + narrowed.stderr)
        self.assertNotIn("STALE", narrowed.stdout)
        self.assertNotIn("AMBIGUOUS", narrowed.stdout)
        self.assertIn(f"EQUIVALENT {second['name']}", narrowed.stdout)

    def test_a_record_whose_mutant_was_caught_is_refuted(self):
        plus = cargo_mutant(RECORDED, "*", "replace * with + in double")
        divide = cargo_mutant(RECORDED, "*", "replace * with / in double")
        second = cargo_mutant(RECORDED, "*", "replace * with - in double", occurrence=2)
        one = cargo_mutant(RECORDED, "x * 2 * 1", "replace double -> i64 with 1")
        listed = [plus, divide, second, one]
        records = [
            a_record("replace * with + in double", "x * 2"),
            a_record("replace * with / in double", "x * 2"),
            a_record("replace * with - in double", "2 * 1"),
            a_record("replace double -> i64 with 1", "x * 2 * 1"),
        ]
        summaries = ["CaughtMutant", "Timeout", "Unviable", "MissedMutant"]
        done = judge_recorded(self, RECORDED, listed, summaries, records)
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        for finding in examined(
            "records whose mutant no longer survives",
            [
                f"REFUTED deck-streak-fix.json record 1 ({LIB}: replace * with + in double): "
                f"its mutant {plus['name']} was caught",
                f"REFUTED deck-streak-fix.json record 2 ({LIB}: replace * with / in double): "
                f"its mutant {divide['name']} timed out",
                f"UNNEEDED deck-streak-fix.json record 3 ({LIB}: replace * with - in double): "
                f"its mutant {second['name']} is unviable",
            ],
        ):
            self.assertIn(finding, done.stdout)
        self.assertIn(f"EQUIVALENT {one['name']}", done.stdout)
        self.assertNotRegex(
            done.stdout, r"(REFUTED|UNNEEDED|STALE|AMBIGUOUS) deck-streak-fix\.json record 4 "
        )

    def test_a_record_follows_its_mutant_when_lines_move_above_it(self):
        record = a_record("replace * with + in double", "x * 2")
        # Written against the base, where the mutant starts at line 3, column 7.
        self.assertEqual(located(LIB_TEXT, "*")[:2], (3, 7))
        above = "/// One.\npub fn one() -> i64 {\n    1\n}\n\n" + LIB_TEXT
        moved = cargo_mutant(above, "*", "replace * with + in double")
        self.assertIn(f"{LIB}:8:7: ", moved["name"])
        done = judge_recorded(self, above, [moved], ["MissedMutant"], [record])
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(f"EQUIVALENT {moved['name']}", done.stdout)
        self.assertNotIn("STALE", done.stdout)
        # Its anchored text changes: the record binds nothing, and its mutant is unexplained.
        rewritten = above.replace("x * 2", "x * 3")
        changed = cargo_mutant(rewritten, "*", "replace * with + in double")
        stale = judge_recorded(self, rewritten, [changed], ["MissedMutant"], [record], name="new")
        self.assertEqual(stale.returncode, 1, stale.stdout + stale.stderr)
        self.assertIn("STALE deck-streak-fix.json record 1 (", stale.stdout)
        self.assertIn(f"MISSED {changed['name']}", stale.stdout)
        self.assertNotIn("EQUIVALENT", stale.stdout)


def stryker_mutant(text, needle, mutator, replacement, status, occurrence=1, length=None):
    """One mutant of a mutation-testing-elements report, as StrykerJS 10.0.0 writes it."""
    line, column, end = located(text, needle, occurrence, length)
    mutant = {
        "id": f"{mutator}-{line}-{column}-{replacement}",
        "mutatorName": mutator,
        "replacement": replacement,
        "location": {
            "start": {"line": line, "column": column},
            "end": {"line": line, "column": end},
        },
        "status": status,
    }
    if status == "Ignored":
        mutant["statusReason"] = "EQUIVALENT: the fixture says so (#3)"
    return mutant


def web_report(mutants, source=WEB_HEAD):
    return {
        "schemaVersion": "1",
        "thresholds": {"high": 100, "low": 99},
        "files": {
            "src/lib/start.ts": {"language": "typescript", "source": source, "mutants": mutants}
        },
    }


class TheWebVerdictReadsTheRecord(unittest.TestCase):
    def judged(self, mutants, records, name):
        fixture = Fixture(self, files={WEB_FILE: WEB_BASE})
        fixture.head({WEB_FILE: WEB_HEAD})
        plan = fixture.plan()
        self.assertTrue(plan["classes"]["web"]["applies"])
        write_records(fixture.root, records, fragment="miniapp.json")
        report = fixture.report(f"{name}.json", web_report(mutants))
        return fixture.judge("web", "--stryker", str(report))

    def test_a_web_record_excuses_a_survivor_and_never_an_uncovered_mutant(self):
        # Two mutants of one description start at one position: the span tells them apart.
        both = stryker_mutant(WEB_HEAD, "a && b", "ConditionalExpression", "true", "Survived")
        left = stryker_mutant(
            WEB_HEAD, "a && b", "ConditionalExpression", "true", "Killed", length=1
        )
        uncovered = stryker_mutant(WEB_HEAD, "n > 0", "EqualityOperator", "n >= 0", "NoCoverage")
        killed = stryker_mutant(WEB_HEAD, "n > 0", "ConditionalExpression", "false", "Killed")
        ignored = stryker_mutant(WEB_HEAD, "'many'", "StringLiteral", '""', "Ignored")
        mutants = examined("Stryker mutants", [both, left, uncovered, killed, ignored])
        records = [
            a_record("ConditionalExpression: true", "a && b", file=WEB_FILE, span="a && b"),
            a_record("EqualityOperator: n >= 0", "n > 0;", file=WEB_FILE),
            a_record("ConditionalExpression: false", "n > 0;", file=WEB_FILE),
            a_record('StringLiteral: ""', "'one'", file=WEB_FILE),
        ]
        done = self.judged(mutants, records, "mixed")
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(f"EQUIVALENT {WEB_FILE}:1: ConditionalExpression -> 'true'", done.stdout)
        self.assertNotIn(f"Survived {WEB_FILE}:1", done.stdout)
        for finding in examined(
            "web records that fail",
            [
                "UNCOVERED miniapp.json record 2 (",
                "REFUTED miniapp.json record 3 (",
                "STALE miniapp.json record 4 (",
                f"NoCoverage {WEB_FILE}:2: EqualityOperator -> 'n >= 0'",
                f"Ignored {WEB_FILE}:3: StringLiteral",
            ],
        ):
            self.assertIn(finding, done.stdout)
        # The control: the survivor alone, recorded, and every other mutant killed.
        whole = self.judged(
            [both, left, dict(uncovered, status="Killed"), killed], records[:1], "control"
        )
        self.assertEqual(whole.returncode, 0, whole.stdout + whole.stderr)
        self.assertIn("equivalent 1", whole.stdout)
        # Without its span the record binds both mutants that start there.
        unspanned = {name: value for name, value in records[0].items() if name != "span"}
        loose = self.judged([both, left], [unspanned], "loose")
        self.assertEqual(loose.returncode, 1, loose.stdout + loose.stderr)
        self.assertIn("AMBIGUOUS miniapp.json record 1 (", loose.stdout)


def table(reports, root, *extra):
    return subprocess.run(
        [sys.executable, str(VERDICT), "table", "--reports", str(reports), "--root", str(root)]
        + list(extra),
        capture_output=True,
        text=True,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
        timeout=300,
        check=False,
    )


TABLE_LINE = re.compile(
    r"(?m)^table: (\S+): listed (\d+), killed (\d+), equivalent (\d+), unexplained (\d+), "
    r"unviable (\d+)$"
)


class TheTableCountsTheCampaign(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.scratch = Path(scratch.name)
        self.root = self.scratch / "tree"
        for relative, text in {LIB: RECORDED, HOUR: HOUR_TEXT, WEB_FILE: WEB_HEAD}.items():
            (self.root / relative).parent.mkdir(parents=True, exist_ok=True)
            (self.root / relative).write_text(text, encoding="utf-8")
        self.plus = cargo_mutant(RECORDED, "*", "replace * with + in double")
        self.divide = cargo_mutant(RECORDED, "*", "replace * with / in double")
        self.second = cargo_mutant(RECORDED, "*", "replace * with - in double", occurrence=2)
        self.one = cargo_mutant(RECORDED, "x * 2 * 1", "replace double -> i64 with 1")
        self.zero = cargo_mutant(RECORDED, "x * 2 * 1", "replace double -> i64 with 0")
        self.last = cargo_mutant(
            HOUR_TEXT, "23", "replace last -> u8 with 0", file=HOUR, package="deck-streak-hour"
        )
        self.first = cargo_mutant(
            HOUR_TEXT, "23", "replace last -> u8 with 1", file=HOUR, package="deck-streak-hour"
        )
        self.survived = stryker_mutant(
            WEB_HEAD, "a && b", "ConditionalExpression", "true", "Survived"
        )
        self.uncovered = stryker_mutant(
            WEB_HEAD, "n > 0", "EqualityOperator", "n >= 0", "NoCoverage"
        )
        self.killed = stryker_mutant(WEB_HEAD, "n > 0", "ConditionalExpression", "false", "Killed")

    def battery(self, name, shards, stryker_mutants=None):
        """A battery's downloaded reports: {shard: (exit, [(mutant, summary)])}, and Stryker's."""
        reports = self.scratch / name
        for shard, (code, entries) in shards.items():
            directory = reports / f"mutants-shard-{shard}"
            (directory / "mutants.out").mkdir(parents=True)
            write_scope(directory)
            (directory / "cargo-mutants.exit").write_text(f"{code}\n", encoding="utf-8")
            if entries is not None:
                (directory / "mutants.out" / "outcomes.json").write_text(
                    json.dumps(cargo_report(entries)), encoding="utf-8"
                )
        if stryker_mutants is not None:
            sweep = reports / "stryker" / "stryker"
            sweep.mkdir(parents=True)
            (sweep / "mutation.json").write_text(
                json.dumps(web_report(stryker_mutants)), encoding="utf-8"
            )
        return reports

    def the_run(self):
        return self.battery(
            "run",
            {
                0: ("2", [(self.plus, "CaughtMutant"), (self.divide, "MissedMutant")]),
                1: ("2", [(self.second, "Unviable"), (self.one, "MissedMutant")]),
                2: ("3", [(self.zero, "Timeout"), (self.last, "CaughtMutant")]),
                3: ("0", [(self.first, "Unviable")]),
            },
            [self.survived, self.uncovered, self.killed],
        )

    def test_the_table_counts_each_package_in_the_campaigns_columns(self):
        write_records(self.root, [a_record("replace * with / in double", "x * 2")])
        write_records(
            self.root,
            [a_record("ConditionalExpression: true", "a && b", file=WEB_FILE, span="a && b")],
            fragment="miniapp.json",
        )
        done = table(self.the_run(), self.root)
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        rows = {match[0]: tuple(map(int, match[1:])) for match in TABLE_LINE.findall(done.stdout)}
        self.assertEqual(
            rows,
            {
                "deck-streak-fix": (5, 2, 1, 1, 1),
                "deck-streak-hour": (2, 1, 0, 0, 1),
                "miniapp": (3, 1, 1, 1, 0),
            },
            done.stdout,
        )
        for package, (listed, killed, equivalent, unexplained, unviable) in examined(
            "table lines", rows.items()
        ):
            self.assertEqual(listed, killed + equivalent + unexplained + unviable, package)
        self.assertIn(f"table: deck-streak-fix: UNEXPLAINED {self.one['name']}", done.stdout)
        self.assertIn(f"table: deck-streak-fix: EQUIVALENT {self.divide['name']}", done.stdout)
        self.assertIn(f"table: miniapp: UNEXPLAINED {WEB_FILE}:2: NoCoverage", done.stdout)
        # One package's table: its line alone, and last; with nothing unexplained it passes.
        hour = table(self.the_run_again("hour"), self.root, "--package", "deck-streak-hour")
        self.assertEqual(hour.returncode, 0, hour.stdout + hour.stderr)
        self.assertEqual(
            TABLE_LINE.findall(hour.stdout), [("deck-streak-hour", "2", "1", "0", "0", "1")]
        )
        self.assertRegex(hour.stdout.strip().splitlines()[-1], TABLE_LINE)
        # A record that fails fails the table, though nothing is unexplained.
        write_records(
            self.root,
            [a_record("replace last -> u8 with 0", "    23\n", file=HOUR)],
            fragment="deck-streak-hour.json",
        )
        refuted = table(self.the_run_again("refuted"), self.root, "--package", "deck-streak-hour")
        self.assertEqual(refuted.returncode, 1, refuted.stdout + refuted.stderr)
        self.assertIn("REFUTED deck-streak-hour.json record 1 (", refuted.stdout)
        self.assertIn("unexplained 0", refuted.stdout)

    def the_run_again(self, name):
        return self.battery(
            name,
            {
                0: ("0", [(self.last, "CaughtMutant")]),
                1: ("0", [(self.first, "Unviable")]),
            },
        )

    def test_a_listed_mutant_no_report_tested_is_void_by_name(self):
        write_records(self.root, [a_record("replace * with / in double", "x * 2")])
        listing = self.scratch / "listed.json"
        listed = [self.plus, self.divide, self.second, self.one, self.zero, self.last, self.first]
        # A mutant the listing holds and no report tested: shard 4 never reported.
        lost = cargo_mutant(RECORDED, "x * 2 * 1", "replace double -> i64 with -1")
        listing.write_text(json.dumps([*listed, lost]), encoding="utf-8")
        reports = self.battery(
            "lost",
            {
                0: ("2", [(self.plus, "CaughtMutant"), (self.divide, "MissedMutant")]),
                1: ("0", [(self.second, "Unviable"), (self.one, "CaughtMutant")]),
                2: ("3", [(self.zero, "Timeout"), (self.last, "CaughtMutant")]),
                3: ("0", [(self.first, "Unviable")]),
            },
        )
        done = table(reports, self.root, "--package", "deck-streak-fix", "--listed", str(listing))
        self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
        self.assertIn(f"table: VOID never tested: {lost['name']}", done.stdout)
        self.assertEqual(
            TABLE_LINE.findall(done.stdout), [("deck-streak-fix", "6", "3", "1", "0", "1")]
        )
        # A report cut short is VOID by name too, and so is every listed mutant it never reached.
        partial = self.battery(
            "partial",
            {
                0: ("137", [(self.plus, "CaughtMutant")]),
                1: ("0", [(self.second, "Unviable"), (self.one, "CaughtMutant")]),
                2: ("3", [(self.zero, "Timeout"), (self.last, "CaughtMutant")]),
                3: ("0", [(self.first, "Unviable")]),
            },
        )
        listing.write_text(json.dumps(listed), encoding="utf-8")
        cut = table(partial, self.root, "--package", "deck-streak-fix", "--listed", str(listing))
        self.assertEqual(cut.returncode, 3, cut.stdout + cut.stderr)
        self.assertIn("table: VOID mutants-shard-0: cargo-mutants exit 137", cut.stdout)
        self.assertIn(f"table: VOID never tested: {self.divide['name']}", cut.stdout)
        # The control: every listed mutant tested, in reports whole.
        whole = self.battery(
            "whole",
            {
                0: ("2", [(self.plus, "CaughtMutant"), (self.divide, "MissedMutant")]),
                1: ("0", [(self.second, "Unviable"), (self.one, "CaughtMutant")]),
                2: ("3", [(self.zero, "Timeout"), (self.last, "CaughtMutant")]),
                3: ("0", [(self.first, "Unviable")]),
            },
        )
        green = table(whole, self.root, "--package", "deck-streak-fix", "--listed", str(listing))
        self.assertEqual(green.returncode, 0, green.stdout + green.stderr)
        self.assertNotIn("VOID", green.stdout)
        self.assertEqual(
            TABLE_LINE.findall(green.stdout), [("deck-streak-fix", "5", "3", "1", "0", "1")]
        )


class ASquashMergeNamesItsPullRequest(unittest.TestCase):
    def test_a_squash_merge_subject_names_the_pull_request_it_merges(self):
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        cases = [
            ("feat(vault): a title (#12)\n\n* a commit\n* another", "not-applicable", "merges #12"),
            ("fix(api): a title that closes an issue (#5) (#77)", "not-applicable", "merges #77"),
            ("Merge pull request #42 from RexRenatus/feat/x\n\nthe body", "not-applicable", "#42"),
            ("chore: a subject that names (#12) mid-line", "diff", "names no pull request"),
            ("chore: a subject that names none\n\nsee (#12)", "diff", "names no pull request"),
        ]
        for subject, decision, reason in examined("push subjects", cases):
            with self.subTest(subject=subject):
                plan = fixture.plan("--event", "push", "--subject", subject)
                scope = plan.get("scope") or {}
                self.assertEqual(scope.get("decision"), decision, scope)
                self.assertIn(reason, scope.get("reason", ""))
                # The title's own issue is never read as the pull request.
                self.assertNotIn("#5", scope.get("reason", ""))
                judged = fixture.judge("rust")
                if decision == "not-applicable":
                    self.assertEqual(judged.returncode, 0, judged.stdout)
                    self.assertIn(f"not-applicable: {scope['reason']}", judged.stdout)
                else:
                    # The diff is judged: a changed code line with no report is VOID.
                    self.assertEqual(judged.returncode, 3, judged.stdout)


# --------------------------------------------------------------------------- SPEC-039 section 27
#
# A docstring-only change to a guard script is named, never VOID (#485, ADR-307). Each member of a
# population below is one pull request's edit of a fixture holding several scripts; the plan reads
# it, and the expectation is ruling 1's: `named` with each file whose change it set aside,
# `applies` when any tree differs outside docstrings or a fail-closed arm holds, and `refused` when
# the plan stops on a script that is not UTF-8, as it did before section 27.

GUARD = "scripts/guard.py"
GUARD_TEXT = (
    '"""The guard, a module docstring."""\n'
    "\n"
    "LIMIT = 3\n"
    'NAME = "a string used as a value, which is code"\n'
    '"a string statement after the docstring, which is code"\n'
    "\n"
    "if LIMIT:\n"
    '    "a string first in a block, which is code"\n'
    "\n"
    "\n"
    "class Gate:\n"
    '    """A class docstring."""\n'
    "\n"
    "    def check(self, x):\n"
    '        """A method docstring."""\n'
    "        return x + 1\n"
    "\n"
    "\n"
    "def total(x):\n"
    '    """A function docstring."""\n'
    "    return x * LIMIT\n"
    "\n"
    "\n"
    "async def wait(x):\n"
    '    """An async function docstring."""\n'
    "    return x - 1\n"
    "\n"
    "\n"
    "def note():\n"
    '    f"{LIMIT} notes, an f-string first, which is code"\n'
    "    return LIMIT\n"
    "\n"
    "\n"
    "def raw():\n"
    '    b"bytes first, which is code"\n'
    "    return LIMIT\n"
)
OTHER = "scripts/other.py"
OTHER_TEXT = '"""Another script."""\n\n\ndef other(x):\n    return x - 1\n'
OLD = "scripts/old.py"
OLD_TEXT = '"""A script a change deletes or renames."""\n\n\ndef old():\n    return 1\n'
BROKEN = "scripts/broken.py"
BROKEN_TEXT = '"""A script that does not parse."""\nx = (\n'
LATIN = "scripts/latin.py"
# A Latin-1 byte four lines below the docstring, so a hunk on the docstring's lines never shows it.
LATIN_HEAD = b'"""A Latin-1 script.\n\nIts docstring has a second paragraph.\n"""\n'
LATIN_TAIL = b"".join(b"VALUE_%d = %d\n" % (n, n) for n in range(4)) + b"# caf\xe9 au lait\n"
NAMED = "not-applicable: docstring-only: "
APPLIES = ("applies", None)
MODULE_DOC = (
    '"""The guard, a module docstring."""',
    '"""The guard, its docstring reworded."""',
)
FUNCTION_DOC = ('"""A function docstring."""', '"""A function docstring, reworded."""')


def edited(text, *pairs):
    """`text` with each (old, new) replaced, each old occurring exactly once."""
    for old, new in pairs:
        if text.count(old) != 1:
            raise AssertionError(f"{old!r} occurs {text.count(old)} time(s) in the fixture")
        text = text.replace(old, new)
    return text


def edit_fixture(test, changes, deleted=()):
    """A fixture holding every script a member can edit, at its base, with `changes` committed as
    the pull request: a text is written as UTF-8 and bytes as they are."""
    files = {GUARD: GUARD_TEXT, OTHER: OTHER_TEXT, OLD: OLD_TEXT, BROKEN: BROKEN_TEXT}
    fixture = Fixture(test, files=files)
    (fixture.root / LATIN).write_bytes(LATIN_HEAD + LATIN_TAIL)
    fixture.base = fixture.commit("the base's Latin-1 script")
    for relative, content in changes.items():
        if isinstance(content, bytes):
            (fixture.root / relative).write_bytes(content)
        else:
            fixture.write(relative, content)
    for relative in deleted:
        (fixture.root / relative).unlink()
    fixture.commit("the pull request")
    return fixture


def plan_outcome(module, fixture):
    """What the plan reads of the `scripts` class, and the plan: ("named", [files]),
    ("applies", None), ("refused", None) when it stops on a script that is not UTF-8, or
    ("not-applicable", the case) for any other case."""
    try:
        plan = module.plan_diff(
            fixture.root, fixture.base, "HEAD", fixture.out, ("diff", "a member's diff")
        )
    except UnicodeDecodeError:
        return ("refused", None), None
    scripts = plan.classes["scripts"]
    if scripts["applies"]:
        return APPLIES, plan
    if scripts["case"].startswith(NAMED):
        return ("named", scripts["case"].rsplit(": ", 1)[1].split(", ")), plan
    return ("not-applicable", scripts["case"]), plan


def judged_in_process(module, fixture, plan):
    """`judge --class scripts` over the plan, in-process: its exit and what it printed."""
    document = json.loads(json.dumps(dataclasses.asdict(plan)))
    verdict = module.Verdict("scripts")
    args = argparse.Namespace(
        klass="scripts",
        rows=None,
        python=None,
        python_whole=None,
        root=str(fixture.root),
    )
    with contextlib.redirect_stdout(io.StringIO()) as said:
        module.judge_python(verdict, document, args)
        code = verdict.close()
    return code, said.getvalue()


def is_string_statement(statement):
    return (
        isinstance(statement, ast.Expr)
        and isinstance(statement.value, ast.Constant)
        and isinstance(statement.value.value, str)
    )


def every_string_set_aside(source):
    """Ruling 2's control, a planted narrowing: every string expression read as a docstring, so a
    bare string statement is dropped wherever it stands and every other string constant blanked."""
    if source is None:
        return None
    try:
        tree = ast.parse(source.decode("utf-8"))
    except (SyntaxError, ValueError):
        return None
    for node in ast.walk(tree):
        for _, block in ast.iter_fields(node):
            if isinstance(block, list):
                block[:] = [item for item in block if not is_string_statement(item)]
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            node.value = ""
    return ast.dump(tree)


def script_edits():
    """Ruling 2's population: (name, changes, deleted, expected)."""
    doc = edited(GUARD_TEXT, FUNCTION_DOC)
    grown = (
        '"""The guard, a module docstring."""',
        '"""The guard.\n\nIt grew to three lines.\n"""',
    )
    return [
        (
            "a module docstring",
            {GUARD: edited(GUARD_TEXT, MODULE_DOC)},
            (),
            ("named", [GUARD]),
        ),
        (
            "a class docstring",
            {GUARD: edited(GUARD_TEXT, ('"""A class docstring."""', '"""A class, reworded."""'))},
            (),
            ("named", [GUARD]),
        ),
        ("a function docstring", {GUARD: doc}, (), ("named", [GUARD])),
        (
            "an async function docstring",
            {
                GUARD: edited(
                    GUARD_TEXT,
                    ("An async function docstring.", "An async one, reworded."),
                )
            },
            (),
            ("named", [GUARD]),
        ),
        (
            "a method docstring",
            {GUARD: edited(GUARD_TEXT, ("A method docstring.", "A method docstring, reworded."))},
            (),
            ("named", [GUARD]),
        ),
        (
            "a docstring grown to three lines",
            {GUARD: edited(GUARD_TEXT, grown)},
            (),
            ("named", [GUARD]),
        ),
        (
            "a string statement that is not first",
            {GUARD: edited(GUARD_TEXT, ("after the docstring, which is code", "reworded"))},
            (),
            APPLIES,
        ),
        (
            "a string statement added after a docstring",
            {
                GUARD: edited(
                    GUARD_TEXT,
                    (FUNCTION_DOC[0] + "\n", FUNCTION_DOC[0] + '\n    "more"\n'),
                )
            },
            (),
            APPLIES,
        ),
        (
            "a string used as a value",
            {GUARD: edited(GUARD_TEXT, ("used as a value, which is code", "used as a value"))},
            (),
            APPLIES,
        ),
        (
            "a code change beside a docstring change in one file",
            {GUARD: edited(GUARD_TEXT, FUNCTION_DOC, ("x * LIMIT", "x * LIMIT * 2"))},
            (),
            APPLIES,
        ),
        (
            "two files, only one docstring-only",
            {GUARD: doc, OTHER: edited(OTHER_TEXT, ("x - 1", "x - 2"))},
            (),
            APPLIES,
        ),
        (
            # README.md sorts before every script, so the plan reads it first.
            "a docstring change beside a change to a file outside the class",
            {GUARD: doc, "README.md": "a fixture, reworded\n"},
            (),
            ("named", [GUARD]),
        ),
        (
            # scripts/notes.md sorts between the two scripts, and only the second changes code.
            "a code change in a later script, a file outside the class between",
            {
                GUARD: doc,
                "scripts/notes.md": "a note\n",
                OTHER: edited(OTHER_TEXT, ("x - 1", "x - 2")),
            },
            (),
            APPLIES,
        ),
        (
            "a file added holding only a docstring",
            {GUARD: doc, "scripts/new.py": '"""Only a docstring."""\n'},
            (),
            APPLIES,
        ),
        ("a file deleted beside a docstring change", {GUARD: doc}, (OLD,), APPLIES),
        (
            "a file renamed with only its docstring changed",
            {"scripts/renamed.py": edited(OLD_TEXT, ("deletes or renames", "renamed"))},
            (OLD,),
            APPLIES,
        ),
        (
            "a parse error at both sides, its docstring changed",
            {BROKEN: edited(BROKEN_TEXT, ("does not parse", "still does not parse"))},
            (),
            APPLIES,
        ),
        (
            "a parse error at the head beside a docstring change",
            {GUARD: doc, OTHER: OTHER_TEXT + "x = (\n"},
            (),
            APPLIES,
        ),
        (
            "a script not UTF-8 whose docstring loses a paragraph, beside a docstring change",
            {GUARD: doc, LATIN: b'"""A Latin-1 script.\n"""\n' + LATIN_TAIL},
            (),
            APPLIES,
        ),
        (
            "a docstring change in a script not UTF-8",
            {LATIN: LATIN_HEAD.replace(b"A Latin-1", b"The Latin-1") + LATIN_TAIL},
            (),
            ("refused", None),
        ),
    ]


def definition_edges():
    """The definition's edges (A63): (name, changes, deleted, expected)."""
    doc = edited(GUARD_TEXT, FUNCTION_DOC)
    return [
        (
            "an f-string first in a body",
            {GUARD: edited(GUARD_TEXT, ("notes, an f-string first, which is code", "notes"))},
            (),
            APPLIES,
        ),
        (
            "a bytes literal first in a body",
            {GUARD: edited(GUARD_TEXT, ("bytes first, which is code", "bytes first"))},
            (),
            APPLIES,
        ),
        (
            "a string first in an if block",
            {GUARD: edited(GUARD_TEXT, ("first in a block, which is code", "first in a block"))},
            (),
            APPLIES,
        ),
        (
            "two docstring-only files",
            {
                GUARD: doc,
                OTHER: edited(OTHER_TEXT, ("Another script.", "Another, reworded.")),
            },
            (),
            ("named", [GUARD, OTHER]),
        ),
        (
            "a docstring-only file beside a comment-only one",
            {
                GUARD: doc,
                OTHER: edited(OTHER_TEXT, ("    return", "    # a comment\n    return")),
            },
            (),
            ("named", [GUARD]),
        ),
        (
            "a docstring added where there was none",
            {OTHER: edited(OTHER_TEXT, ("(x):\n", '(x):\n    """Added."""\n'))},
            (),
            ("named", [OTHER]),
        ),
        (
            "a re-layout with an equal tree",
            {GUARD: edited(GUARD_TEXT, ("x * LIMIT", "x*LIMIT"))},
            (),
            ("named", [GUARD]),
        ),
        (
            "a module docstring deleted, a change of deletions alone",
            {OTHER: edited(OTHER_TEXT, ('"""Another script."""\n', ""))},
            (),
            (
                "not-applicable",
                "not-applicable: no production code line changed; every changed line is blank "
                "or a comment, or deleted",
            ),
        ),
    ]


COOKIE = "scripts/cookie.py"
COOKIE_DOC = b'"""A declared script."""\n'


def cookie_fixture(test, base, head):
    """A fixture whose one declared script is `base` bytes at the base and `head` at the pull
    request."""
    fixture = Fixture(test)
    (fixture.root / COOKIE).write_bytes(base)
    fixture.base = fixture.commit("the base's declared script")
    (fixture.root / COOKIE).write_bytes(head)
    fixture.commit("the pull request")
    return fixture


def cookie_edits():
    """The PEP 263 class (A62): (name, base bytes, head bytes, expected), each reading written
    before any run. Only the control and the fail-closed member are read as before."""
    latin = b"# -*- coding: latin-1 -*-\n"
    utf8 = b"# -*- coding: utf-8 -*-\n"
    doc, reworded = COOKIE_DOC, b'"""A declared script, reworded."""\n'
    return [
        (
            "a latin-1 escape rewritten as raw bytes, a value changes",
            latin + doc + b'X = "\\xe9"\n',
            latin + doc + b'X = "\xc3\xa9"\n',
            APPLIES,
        ),
        (
            "a declaration changed utf-8 to latin-1 beside a docstring edit",
            utf8 + doc + b'S = "\xc3\xa9"\n',
            latin + reworded + b'S = "\xc3\xa9"\n',
            APPLIES,
        ),
        (
            "a latin-1 declaration added so the head no longer parses",
            doc + b"caf\xc3\xa9 = 1\n",
            latin + reworded + b"caf\xc3\xa9 = 1\n",
            APPLIES,
        ),
        (
            "an unknown encoding declared beside a docstring edit",
            doc + b"X = 1\n",
            b"# -*- coding: bogus -*-\n" + reworded + b"X = 1\n",
            APPLIES,
        ),
        (
            "a declared UTF-8 script whose change is a docstring alone",
            utf8 + doc + b'S = "\xc3\xa9"\n',
            utf8 + reworded + b'S = "\xc3\xa9"\n',
            ("named", [COOKIE]),
        ),
        (
            "a declared latin-1 script whose bytes are not UTF-8, a docstring edit",
            latin + doc + b'S = "\xe9"\n',
            latin + reworded + b'S = "\xe9"\n',
            ("refused", None),
        ),
    ]


SIMPLE_TEXT = '"""A guard."""\n\n\ndef total(x):\n    """Triples."""\n    return x * 3\n'
SIMPLE_TESTS = (
    "import sys\n"
    "import unittest\n"
    "from pathlib import Path\n\n"
    "sys.path.insert(0, str(Path(__file__).resolve().parents[1]))\n"
    "import guard  # noqa: E402\n\n\n"
    "class TheGuard(unittest.TestCase):\n"
    "    def test_five_totals(self):\n"
    "        self.assertEqual(guard.total(5), {total})\n"
)
SIMPLE_MAP = {GUARD: {"dir": "scripts/tests", "modules": ["test_guard"]}}


def in_ci_order(test, head_text, total):
    """A fixture whose one script is `head_text` at the head, and what each step CI runs printed:
    the plan, the runner's listing, shards, the runner's one shard and the verdict."""
    fixture = Fixture(
        test,
        files={
            GUARD: SIMPLE_TEXT,
            "scripts/tests/test_guard.py": SIMPLE_TESTS.format(total=15),
            "scripts/mutation-python.json": json.dumps(SIMPLE_MAP),
        },
    )
    fixture.head(
        {
            GUARD: head_text,
            "scripts/tests/test_guard.py": SIMPLE_TESTS.format(total=total),
        }
    )
    out, plan_file = fixture.out, str(fixture.out / "plan.json")
    planned = fixture.verdict(
        "plan",
        "--base",
        fixture.base,
        "--head",
        "HEAD",
        "--root",
        str(fixture.root),
        "--out",
        str(out),
    )
    test.assertEqual(planned.returncode, 0, planned.stdout + planned.stderr)

    def runner(*args):
        return subprocess.run(
            [sys.executable, str(REPO / "scripts" / "mutation_python.py"), *args],
            capture_output=True,
            text=True,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            timeout=300,
            check=False,
        )

    listing = out / "python-listed.json"
    listed = runner("list", "--root", str(fixture.root), "--plan", plan_file, "--out", str(listing))
    test.assertEqual(listed.returncode, 0, listed.stdout + listed.stderr)
    sharded = fixture.verdict("shards", "--plan", plan_file, "--python-listed", str(listing))
    test.assertEqual(sharded.returncode, 0, sharded.stdout + sharded.stderr)
    report = out / "reports" / "mutation-python-shard-0" / "report.json"
    report.parent.mkdir(parents=True)
    ran = runner(
        "run",
        "--root",
        str(fixture.root),
        "--plan",
        plan_file,
        "--shard",
        "0/1",
        "--report",
        str(report),
    )
    test.assertEqual(ran.returncode, 0, ran.stdout + ran.stderr)
    judged = fixture.judge("scripts", "--python", str(out / "reports"))
    document = json.loads(listing.read_text(encoding="utf-8"))
    return planned, document, judged


class ADocstringOnlyScriptChangeIsNamed(unittest.TestCase):
    """SPEC-039 section 27 (ADR-307, #485): a change that leaves every changed script's syntax tree
    equal once docstrings are set aside is named `docstring-only`, never VOID, and nothing that
    alters a tree outside docstrings stops the class applying."""

    def test_a_docstring_only_change_is_named_and_a_code_change_beside_it_is_examined(
        self,
    ):
        """A61"""
        docstrings = (
            ('"""A guard."""', '"""A guard, reworded."""'),
            ("Triples.", "Triples x."),
        )
        planned, document, judged = in_ci_order(self, edited(SIMPLE_TEXT, *docstrings), 15)
        self.assertIn(
            "mutation: plan: scripts does not apply: not-applicable: docstring-only: every changed "
            f"script's syntax tree equals its base's once docstrings are set aside: {GUARD}\n",
            planned.stdout,
        )
        self.assertEqual(document["listed"], 0, document)
        self.assertEqual(judged.returncode, 0, judged.stdout + judged.stderr)
        self.assertIn(
            f"mutation: scripts: not-applicable: {GUARD}: 2 changed line(s): docstring-only, its "
            "syntax tree equals its base's once docstrings are set aside\n",
            judged.stdout,
        )
        self.assertIn("mutation: scripts: verdict: ok\n", judged.stdout)
        self.assertNotIn("VOID", judged.stdout)
        # The same docstring change beside a code change in the same file: the class applies as
        # R4 says, the runner lists the code line's mutants, and the verdict counts them examined.
        beside = edited(SIMPLE_TEXT, *docstrings, ("x * 3", "x * 3 * 2"))
        planned, document, judged = in_ci_order(self, beside, 30)
        self.assertIn(
            "mutation: plan: scripts applies: 3 production code line(s) in 1 file(s)\n",
            planned.stdout,
        )
        lines = {entry["line"] for entry in examined("listed mutants", document["mutants"])}
        self.assertEqual(lines, {6}, document)
        self.assertEqual(judged.returncode, 0, judged.stdout + judged.stderr)
        listed = document["listed"]
        self.assertIn(
            f"mutation: scripts: examined {listed}: generated {listed}, rows 0\n",
            judged.stdout,
        )
        self.assertIn("mutation: scripts: verdict: ok\n", judged.stdout)

    def test_the_named_case_narrows_no_member_of_a_population_of_script_edits(self):
        """A62"""
        module = verdict_module()
        members = examined("script edits", script_edits())
        fixtures = {
            name: edit_fixture(self, changes, deleted) for name, changes, deleted, _ in members
        }
        mismatches, set_aside = [], []
        for name, changes, _, expected in members:
            found, plan = plan_outcome(module, fixtures[name])
            if found != expected:
                mismatches.append(f"{name}: read {found}, expected {expected}")
                continue
            if found[0] != "named":
                continue
            code, said = judged_in_process(module, fixtures[name], plan)
            if code != 0 or "VOID" in said or "docstring-only" not in said:
                mismatches.append(f"{name}: the verdict read {code}: {said}")
            # The lines a member set aside are its named files' `docstring` lines; a file outside
            # the class, such as README.md, sets none aside and is no source the runner lists.
            for record in (entry for entry in plan.files if entry.get("docstring")):
                lines = record["docstring"]
                set_aside.extend(f"{name}: {record['path']}:{line}" for line in lines)
                held = [
                    mutant.text
                    for mutant in module.mutation_python.list_source(changes[record["path"]])
                    if any(mutant.start_line <= line <= mutant.end_line for line in lines)
                ]
                if held:
                    mismatches.append(f"{name}: set aside the runner's mutants {held}")
        tally = Counter(expected[0] for *_, expected in members)
        print(
            f"population: {len(members)} member(s): {tally['named']} named docstring-only, "
            f"{tally['applies']} applying, {tally['refused']} refused as at the base; "
            f"mismatches {len(mismatches)}"
        )
        self.assertEqual(mismatches, [])
        examined("set-aside lines checked against the runner's listing", set_aside)
        # The control: a plan that reads every string expression as a docstring must be caught.
        with mock.patch.object(module, "tree_without_docstrings", every_string_set_aside):
            caught = [
                name
                for name, *_, expected in members
                if plan_outcome(module, fixtures[name])[0] != expected
            ]
        print(
            f"control: a plan that sets every string expression aside: {len(caught)} "
            f"mismatch(es) of {len(members)}, caught"
        )
        self.assertEqual(
            caught,
            [
                "a string statement that is not first",
                "a string statement added after a docstring",
                "a string used as a value",
            ],
        )

    def test_a_declared_encoding_decides_the_tree_compared(self):
        """A62, the PEP 263 class (#485): the tree is the one Python reads from the bytes."""
        module = verdict_module()
        members = examined("declared encodings", cookie_edits())
        mismatches, readings = [], Counter()
        for name, base, head, expected in members:
            fixture = cookie_fixture(self, base, head)
            found, _ = plan_outcome(module, fixture)
            readings[found[0]] += 1
            if found != expected:
                mismatches.append(f"{name}: read {found}, expected {expected}")
        print(f"cookie population: {len(members)} member(s); mismatches {len(mismatches)}")
        self.assertEqual(mismatches, [])
        self.assertEqual(readings, {"applies": 4, "named": 1, "refused": 1})

    def test_the_docstring_is_only_the_first_bare_string_of_a_body(self):
        """A63"""
        module = verdict_module()
        members = examined("definition edges", definition_edges())
        mismatches = []
        for name, changes, deleted, expected in members:
            fixture = edit_fixture(self, changes, deleted)
            found, plan = plan_outcome(module, fixture)
            if found != expected:
                mismatches.append(f"{name}: read {found}, expected {expected}")
            elif name == "a docstring-only file beside a comment-only one":
                code, said = judged_in_process(module, fixture, plan)
                self.assertEqual(code, 0, said)
                self.assertIn(
                    f"mutation: scripts: not-applicable: {GUARD}: 1 changed line(s): "
                    "docstring-only, its syntax tree equals its base's once docstrings are set "
                    "aside\n",
                    said,
                )
                self.assertIn(
                    f"mutation: scripts: not-applicable: {OTHER}: 1 changed line(s), all blank "
                    "or comments\n",
                    said,
                )
        print(f"edges: {len(members)} member(s); mismatches {len(mismatches)}")
        self.assertEqual(mismatches, [])

    def test_the_plan_paragraph_names_the_legs_ci_admits_and_the_outputs_it_writes(
        self,
    ):
        """A64 (#455)"""
        module = verdict_module()
        paragraph = next(
            part for part in module.__doc__.split("\n\n") if part.startswith("PLAN first")
        )
        workflow = (REPO / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        loop = re.search(r"^\s*for leg in (.+); do$", workflow, re.MULTILINE)
        admitted = examined(
            "legs ci admits a skip from", re.findall(r'"(mutation-[a-z]+):', loop[1])
        )
        source = VERDICT.read_text(encoding="utf-8")
        legs = next(
            node
            for node in ast.walk(ast.parse(source))
            if isinstance(node, ast.FunctionDef) and node.name == "legs"
        )
        judged = re.findall(r'"(mutation-[a-z]+)",', ast.get_source_segment(source, legs))
        self.assertEqual(sorted(judged), sorted(admitted))
        why = paragraph.split(". A pull request into", 1)[0]
        with self.subTest("the legs ci admits a skip from"):
            self.assertEqual(sorted(re.findall(r"`(mutation-[a-z]+)`", why)), sorted(admitted), why)
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        plan = module.plan_diff(fixture.root, fixture.base, "HEAD", fixture.out, ("diff", "a diff"))
        sink = fixture.out / "github-output"
        with (
            mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(sink)}),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            module.say_plan(plan)
        written = examined(
            "step outputs the plan writes",
            sorted(
                {line.split("=", 1)[0] for line in sink.read_text(encoding="utf-8").splitlines()}
            ),
        )
        outputs = paragraph.split("writes the step outputs", 1)[1]
        with self.subTest("the step outputs the plan writes"):
            self.assertEqual(sorted(re.findall(r"`([a-z]+)`", outputs)), written, outputs)


if __name__ == "__main__":
    unittest.main()
