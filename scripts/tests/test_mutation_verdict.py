"""A pull request's mutation plan and verdict, and the weekly battery's survivors (SPEC-039 A12 to
A19).

Each test builds a fixture repository in a temporary directory with a base commit and a head
commit, runs `plan` over their diff, and judges synthetic reports written in each tool's own
format: cargo-mutants 27.1.0's `outcomes.json`, the mutation-testing-elements `mutation.json`
StrykerJS writes, and the rows runner's report. No tool runs here.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

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
        return self.verdict(
            "judge", "--plan", str(self.out / "plan.json"), "--class", klass, *extra
        )


def outcomes(caught=0, missed=(), timeout=0, unviable=0, file=LIB):
    """cargo-mutants 27.1.0's outcomes.json, with one entry per missed mutant."""
    entries = [{"scenario": "Baseline", "summary": "Success"}]
    for name in missed:
        entries.append(
            {
                "scenario": {"Mutant": {"name": name, "file": file, "package": "fix"}},
                "summary": "MissedMutant",
            }
        )
    total = caught + len(missed) + timeout + unviable
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
            "scripts/check.py": "other",
            "docs/notes.md": "other",
        }
        fixture = Fixture(self)
        fixture.head({path: "let x = 1;\n" for path in expected})
        plan = fixture.plan()
        found = {entry["path"]: entry["class"] for entry in plan["files"]}
        for path in examined("classified paths", sorted(expected)):
            self.assertEqual(found.get(path), expected[path], path)


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
        cases = [
            ("pull_request", "dev", "", "diff", "the pull request into dev is judged on its diff"),
            ("pull_request", "main", "", "not-applicable", "a release pull request into main"),
            ("push", "", merge, "not-applicable", "this push merges #42"),
            ("push", "", "chore: a push that merges nothing", "diff", "names no pull request"),
        ]
        for event, base_ref, subject, decision, reason in examined("events", cases):
            plan = fixture.plan("--event", event, "--base-ref", base_ref, "--subject", subject)
            scope = plan.get("scope") or {}
            self.assertEqual(scope.get("decision"), decision, event)
            self.assertIn(reason, scope.get("reason", ""), event)
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


if __name__ == "__main__":
    unittest.main()
