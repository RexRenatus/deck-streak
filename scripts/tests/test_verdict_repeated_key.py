"""The verdict's plan and the equivalence records refuse a repeated key (SPEC-122, issue #345).

`mutation_rows.parse_document` forms the one refusal sentence for the band files. The plan reads
those band files, and the census reads the equivalence records; neither may keep the last value of
a repeated key, nor answer with a traceback.
"""

import json
import re
import unittest

from _support import examined
from test_mutation_equivalent import FRAGMENTS, census, rust_record, tree
from test_mutation_verdict import Fixture

BAND_PATH = "scripts/mutation-rows.d/S00000-S00099.json"
BAND_WHERE = "scripts/mutation-rows.d/S00000-S00099.json"
RECORD_NAME = "deck-streak-fix.json"
RECORD_WHERE = f"{FRAGMENTS}/{RECORD_NAME}"


def sentence(where, key):
    """The whole refusal `parse_document` forms, so a test pins every word of it."""
    return f"{where} repeats the key '{key}' in one object"


BAND_REPEATED = (
    '{"tables": {"MUTATIONS": [], "CARGO_KILLED_SCRIPT_MUTATIONS": [], '
    '"SCRIPT_MUTATIONS": []}, "tables": {"SCRIPT_MUTATIONS": []}}\n'
)


class ThePlanRefusesARepeatedKey(unittest.TestCase):
    def test_a_band_file_that_repeats_a_key_is_refused_by_the_plan_without_a_traceback(self):
        fixture = Fixture(self)
        fixture.head({BAND_PATH: BAND_REPEATED, "README.md": "changed\n"})
        done = fixture.verdict(
            "plan", "--base", fixture.base, "--head", "HEAD",
            "--root", str(fixture.root), "--out", str(fixture.out),
        )  # fmt: skip
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(sentence(BAND_WHERE, "tables"), done.stderr)
        self.assertNotIn("Traceback", done.stderr)

    def test_a_well_formed_band_file_is_planned_and_its_row_count_printed(self):
        fixture = Fixture(self)
        fixture.head({"README.md": "changed\n"})
        plan = fixture.plan()
        done = fixture.verdict(
            "plan", "--base", fixture.base, "--head", "HEAD",
            "--root", str(fixture.root), "--out", str(fixture.out),
        )  # fmt: skip
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        printed = re.search(r"(\d+) row\(s\) selected", done.stdout)
        self.assertIsNotNone(printed, done.stdout)
        self.assertEqual(int(printed.group(1)), len(plan["rows"]))
        self.assertEqual(len(examined("changed paths", plan["files"])), 1)


class TheEquivalenceRecordsRefuseARepeatedKey(unittest.TestCase):
    def written(self, text):
        root = tree(self, {})
        (root / FRAGMENTS).mkdir(parents=True, exist_ok=True)
        (root / RECORD_WHERE).write_text(text, encoding="utf-8")
        return root

    def refused(self, text, key):
        done = census(self.written(text))
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(sentence(RECORD_WHERE, key), done.stdout + done.stderr)
        self.assertNotIn("Traceback", done.stderr)

    def test_a_record_fragment_that_repeats_a_key_at_the_top_is_refused(self):
        body = json.dumps([rust_record()])
        self.refused(f'{{"records": {body}, "records": []}}\n', "records")

    def test_a_record_that_repeats_a_key_inside_an_entry_is_refused(self):
        entry = json.dumps(rust_record())[:-1] + ', "issue": "#9"}'
        self.refused(f'{{"records": [{entry}]}}\n', "issue")

    def test_a_well_formed_record_fragment_is_read_and_counted(self):
        root = tree(self, {RECORD_NAME: [rust_record()]})
        done = census(root)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        counted = re.search(r"(\d+) record", done.stdout)
        self.assertIsNotNone(counted, done.stdout)
        self.assertEqual(len(examined("records", range(int(counted.group(1))))), 1)


if __name__ == "__main__":
    unittest.main()
