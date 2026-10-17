"""The equivalence record's census (SPEC-057 A1 and A2, R4 to R7, ADR-070).

Each test plants fragments under `scripts/mutation-equivalent.d/` in a temporary tree that holds
one Cargo package with a test target and one Mini App file, and runs
`scripts/mutation-verdict.py census` over it. The census needs no mutation tool: it reads the
records, the files they name and the tests their `reached_by` names.
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

VERDICT = REPO / "scripts" / "mutation-verdict.py"
FRAGMENTS = "scripts/mutation-equivalent.d"
LIB = "crates/fix/src/lib.rs"
LIB_TEXT = (
    "/// Doubles.\n"
    "pub fn double(x: i64) -> i64 {\n"
    "    x * 2\n"
    "}\n"
    "\n"
    "/// Adds.\n"
    "pub fn add(x: i64, y: i64) -> i64 {\n"
    "    x + y\n"
    "}\n"
)
TESTS = "crates/fix/tests/double.rs"
TESTS_TEXT = (
    "#[test]\n"
    "fn two_doubles_to_four() {\n"
    "    assert_eq!(fix::double(2), 4);\n"
    "}\n"
    "\n"
    "#[test]\n"
    "fn one_and_two_add_to_three() {\n"
    "    assert_eq!(fix::add(1, 2), 3);\n"
    "}\n"
    "\n"
    "mod again {\n"
    "    #[test]\n"
    "    fn one_and_two_add_to_three() {\n"
    "        assert_eq!(fix::add(1, 2), 3);\n"
    "    }\n"
    "}\n"
)
OTHER = "crates/other/src/lib.rs"
WEB = "web/app/src/lib/start.ts"
WEB_TEXT = "export const ready = (a: boolean, b: boolean): boolean => a && b;\n"
FILES = {
    "crates/fix/Cargo.toml": '[package]\nname = "deck-streak-fix"\nversion = "0.1.0"\n',
    LIB: LIB_TEXT,
    TESTS: TESTS_TEXT,
    "crates/other/Cargo.toml": '[package]\nname = "deck-streak-other"\nversion = "0.1.0"\n',
    OTHER: "pub fn other() -> bool {\n    true\n}\n",
    WEB: WEB_TEXT,
    "web/app/src/lib/start.test.ts": "import { ready } from './start';\n",
}


def rust_record(**changes):
    """A whole Rust record of the fixture's package; a change to None leaves that field out."""
    record = {
        "file": LIB,
        "mutant": "replace * with + in double",
        "anchor": "x * 2",
        "reason": "the fixture's only caller doubles 2, where two times two and two plus two agree",
        "evidence": "double is called once, as double(2), so x * 2 and x + 2 both return 4",
        "reached_by": "double::two_doubles_to_four",
        "issue": "#1",
    }
    record.update(changes)
    return {name: value for name, value in record.items() if value is not None}


def web_record(**changes):
    """A whole Mini App record: it names no test, since Stryker says which tests ran it."""
    record = {
        "file": WEB,
        "mutant": "ConditionalExpression: true",
        "anchor": "a && b",
        "span": "a && b",
        "reason": "the fixture's only caller passes two true values",
        "evidence": "ready is called once, as ready(true, true)",
        "issue": "#2",
    }
    record.update(changes)
    return {name: value for name, value in record.items() if value is not None}


def tree(test, fragments, files=None):
    """A temporary tree holding FILES (and `files`), and each fragment given as {name: records}."""
    scratch = tempfile.TemporaryDirectory()
    test.addCleanup(scratch.cleanup)
    root = Path(scratch.name)
    for relative, text in {**FILES, **(files or {})}.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    for name, records in fragments.items():
        path = root / FRAGMENTS / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"records": records}, indent=2) + "\n", encoding="utf-8")
    return root


def census(root):
    return subprocess.run(
        [sys.executable, str(VERDICT), "census", "--root", str(root)],
        capture_output=True,
        text=True,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
        timeout=120,
        check=False,
    )


class TheCensusHoldsEveryRecordWhole(unittest.TestCase):
    def test_every_record_carries_its_mutant_anchor_reason_evidence_and_issue(self):
        # The tree's own record passes and says how many records it examined; it may hold none.
        own = census(REPO)
        self.assertEqual(own.returncode, 0, own.stdout + own.stderr)
        self.assertRegex(own.stdout, r"(?m)^examined \d+ record\(s\)$")
        # The control: a whole Rust record and a whole Mini App record, each counted.
        whole = census(
            tree(self, {"deck-streak-fix.json": [rust_record()], "miniapp.json": [web_record()]})
        )
        self.assertEqual(whole.returncode, 0, whole.stdout + whole.stderr)
        self.assertRegex(whole.stdout, r"(?m)^examined 2 record\(s\)$")
        self.assertNotIn("census: ", whole.stdout)
        # Each field left out, and each field left empty, is refused by name.
        fields = ["file", "mutant", "anchor", "reason", "evidence", "reached_by", "issue"]
        planted = [(f"lacks {name}", rust_record(**{name: None})) for name in fields]
        planted += [(f"lacks {name}", rust_record(**{name: "  "})) for name in fields]
        planted += [
            ("its evidence repeats its reason", rust_record(evidence=rust_record()["reason"])),
            (
                "its evidence repeats its reason",
                rust_record(evidence=" " + rust_record()["reason"].upper() + " "),
            ),
            ("its issue '1' is not #N", rust_record(issue="1")),
            ("its issue '#N' is not #N", rust_record(issue="#N")),
            ("its issue '#1 #2' is not #N", rust_record(issue="#1 #2")),
            # reached_by names exactly one test of the mutant's own package, resolved as a row's
            # killer is: no such test, a test declared twice, a target the package lacks, no path.
            ("reached_by: names no test", rust_record(reached_by="double::no_such_test")),
            (
                "reached_by: names no test",
                rust_record(reached_by="double::one_and_two_add_to_three"),
            ),
            (
                "reached_by: crates/fix has no test target nowhere",
                rust_record(reached_by="nowhere::two_doubles_to_four"),
            ),
            ("reached_by: the killer", rust_record(reached_by="two_doubles_to_four")),
        ]
        for problem, record in examined("records a field of which is refused", planted):
            with self.subTest(problem=problem, record=record):
                done = census(tree(self, {"deck-streak-fix.json": [record]}))
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertIn("census: deck-streak-fix.json: record 1 (", done.stdout)
                self.assertIn(problem, done.stdout)
                self.assertRegex(done.stdout, r"(?m)^examined 1 record\(s\)$")
        # A Mini App record names no reached_by: Stryker's own coverage says a survivor ran.
        web = census(tree(self, {"miniapp.json": [web_record(reached_by=None)]}))
        self.assertEqual(web.returncode, 0, web.stdout + web.stderr)
        self.assertRegex(web.stdout, r"(?m)^examined 1 record\(s\)$")

    def test_a_record_the_census_cannot_bind_is_refused(self):
        planted = [
            # The anchor occurs other than once in its file.
            (
                "deck-streak-fix.json",
                [rust_record(anchor="x * 3")],
                f"its anchor occurs 0 times in {LIB}",
            ),
            (
                "deck-streak-fix.json",
                [rust_record(anchor="x: i64")],
                f"its anchor occurs 2 times in {LIB}",
            ),
            (
                "deck-streak-fix.json",
                [rust_record(file="crates/fix/src/gone.rs")],
                "its file crates/fix/src/gone.rs does not exist",
            ),
            # The file lies outside the fragment's package.
            (
                "deck-streak-fix.json",
                [rust_record(file=OTHER, anchor="true", reached_by=None)],
                f"its file {OTHER} lies outside deck-streak-fix",
            ),
            (
                "deck-streak-fix.json",
                [rust_record(file=TESTS, anchor="fix::double(2)")],
                f"its file {TESTS} lies outside deck-streak-fix",
            ),
            (
                "miniapp.json",
                [web_record(file="web/app/src/lib/start.test.ts", anchor="import")],
                "its file web/app/src/lib/start.test.ts lies outside miniapp",
            ),
            (
                "miniapp.json",
                [web_record(file=LIB, anchor="x * 2")],
                f"its file {LIB} lies outside",
            ),
            # The reason spans two lines.
            (
                "deck-streak-fix.json",
                [rust_record(reason="the first line\nand a second")],
                "its reason spans 2 lines",
            ),
            # The record is held twice.
            ("deck-streak-fix.json", [rust_record(), rust_record()], "held twice"),
            # A fragment named for no package of the workspace.
            (
                "deck-streak-nowhere.json",
                [rust_record()],
                "deck-streak-nowhere.json: named for no package of the workspace, nor miniapp",
            ),
        ]
        for fragment, records, problem in examined("records the census cannot bind", planted):
            with self.subTest(problem=problem):
                done = census(tree(self, {fragment: records}))
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertIn(f"census: {fragment}: ", done.stdout)
                self.assertIn(problem, done.stdout)
                self.assertRegex(done.stdout, rf"(?m)^examined {len(records)} record\(s\)$")
        # The control: records whole, each in its own package's fragment, each anchor once.
        whole = census(
            tree(
                self,
                {
                    "deck-streak-fix.json": [rust_record(), rust_record(anchor="    x * 2\n")],
                    "deck-streak-other.json": [
                        rust_record(file=OTHER, anchor="true", reached_by="other::other_is_true")
                    ],
                    "miniapp.json": [web_record()],
                },
                files={"crates/other/tests/other.rs": "#[test]\nfn other_is_true() {}\n"},
            )
        )
        self.assertEqual(whole.returncode, 0, whole.stdout + whole.stderr)
        self.assertNotIn("census: ", whole.stdout)
        self.assertRegex(whole.stdout, r"(?m)^examined 4 record\(s\)$")


if __name__ == "__main__":
    unittest.main()
