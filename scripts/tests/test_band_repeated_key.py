"""A mutation-row band file that repeats a key is refused, not read as its last table (SPEC-122).

A1 builds the merge the way it arises, in a temporary git repository: a base band file, two
branches that each add a table under one key at different places in the file, and a `git merge`
that completes without a conflict. Every other case plants the merged shape directly.
"""

import json
import subprocess
import sys
import unittest

from _support import REPO, examined
from test_mutation_rows import Fixture, git, script_row

sys.path.insert(0, str(REPO / "scripts"))
import mutation_rows  # noqa: E402

BAND = "S00000-S00099.json"
BAND_PATH = f"scripts/mutation-rows.d/{BAND}"
HEADER_PATH = "scripts/mutation-rows.json"
RETIRED_PATH = "scripts/mutation-rows.retired.json"
BASE = '{\n  "tables": {\n    "SCRIPT_MUTATIONS": [],\n    "CARGO_KILLED_SCRIPT_MUTATIONS": []\n  }\n}\n'
BRANCH_A = (
    '{\n  "tables": {\n    "MUTATIONS": [],\n    "SCRIPT_MUTATIONS": [],\n'
    '    "CARGO_KILLED_SCRIPT_MUTATIONS": []\n  }\n}\n'
)
BRANCH_B = (
    '{\n  "tables": {\n    "SCRIPT_MUTATIONS": [],\n    "CARGO_KILLED_SCRIPT_MUTATIONS": [],\n'
    '    "MUTATIONS": []\n  }\n}\n'
)


def sentence(where, key):
    """The whole refusal `parse_document` forms, so a test pins every word of it, not a part."""
    return f"{where} repeats the key '{key}' in one object"


def with_repeat(document, key):
    """The JSON object `document` with `key` and its own value appended again, as a merge leaves
    it: the last value equals the first, so a reader that keeps it reads the document unchanged."""
    return document.rstrip()[:-1] + f', "{key}": {json.dumps(json.loads(document)[key])}}}\n'


class TheReaderRefusesARepeatedKey(unittest.TestCase):
    def refused(self, fixture, where, key):
        """`ids` exits 2 and the library raises, each naming the file and the key."""
        done = fixture.run("ids")
        self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
        self.assertEqual(done.stdout, "", "a refused population lists no id")
        self.assertIn(sentence(where, key), done.stderr)
        with self.assertRaises(mutation_rows.PopulationRefused) as raised:
            mutation_rows.load_tree(fixture.root)
        self.assertEqual(str(raised.exception), sentence(where, key))

    def test_a_merge_git_completes_without_a_conflict_and_the_reader_refuses_the_result(self):
        fixture = Fixture(self)
        fixture.write(BAND_PATH, BASE)
        fixture.commit("the base band file, two tables")
        git(fixture.root, "checkout", "-q", "-b", "branch-a")
        fixture.write(BAND_PATH, BRANCH_A)
        fixture.commit("branch a adds a MUTATIONS table first")
        git(fixture.root, "checkout", "-q", "dev")
        git(fixture.root, "checkout", "-q", "-b", "branch-b")
        fixture.write(BAND_PATH, BRANCH_B)
        fixture.commit("branch b adds a MUTATIONS table last")
        git(fixture.root, "checkout", "-q", "branch-a")
        merged = subprocess.run(
            ["git", "-C", str(fixture.root), "merge", "--no-edit", "branch-b"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(merged.returncode, 0, "git must merge the two branches: " + merged.stdout)
        self.assertNotIn("CONFLICT", merged.stdout + merged.stderr)
        text = (fixture.root / BAND_PATH).read_text(encoding="utf-8")
        self.assertEqual(text.count('"MUTATIONS"'), 2, "the merge holds the key twice")
        self.assertIn("MUTATIONS", json.loads(text)["tables"], "and json.load still reads it")
        self.refused(fixture, BAND_PATH, "MUTATIONS")

    def test_a_key_repeated_at_the_top_or_inside_tables_is_refused_naming_it(self):
        planted = {
            "tables": '{"tables": {"SCRIPT_MUTATIONS": []}, "tables": {"SCRIPT_MUTATIONS": []}}\n',
            "SCRIPT_MUTATIONS": '{"tables": {"SCRIPT_MUTATIONS": [], "SCRIPT_MUTATIONS": []}}\n',
        }
        for key, text in examined("planted repeats", planted.items()):
            with self.subTest(key=key):
                fixture = Fixture(self)
                fixture.write(BAND_PATH, text)
                self.refused(fixture, BAND_PATH, key)

    def test_a_key_repeated_at_any_depth_is_refused_by_the_parser(self):
        deep = '{"a": {"b": [1, {"c": [{"key": 1, "key": 2}]}]}}'
        with self.assertRaises(mutation_rows.PopulationRefused) as raised:
            mutation_rows.parse_document("scripts/deep.json", deep)
        self.assertEqual(str(raised.exception), sentence("scripts/deep.json", "key"))
        self.assertEqual(
            mutation_rows.parse_document("x.json", '{"a": {"b": 1}, "b": {"a": 1}}'),
            {"a": {"b": 1}, "b": {"a": 1}},
            "one key in two objects is not a repeat",
        )

    def test_a_header_that_repeats_a_key_is_refused_in_the_tree(self):
        fixture = Fixture(self)
        header = (fixture.root / HEADER_PATH).read_text(encoding="utf-8")
        fixture.write(HEADER_PATH, with_repeat(header, "arities"))
        self.refused(fixture, HEADER_PATH, "arities")

    def test_retired_over_a_revision_whose_band_file_repeats_a_key_is_refused(self):
        fixture = Fixture(self)
        fixture.write(BAND_PATH, '{"tables": {"MUTATIONS": [], "MUTATIONS": []}}\n')
        bad = fixture.commit("a band file that repeats a key")
        fixture.write(BAND_PATH, '{"tables": {"MUTATIONS": []}}\n')
        fixture.commit("the tree reads")
        self.assertEqual(fixture.run("ids").returncode, 0, "the tree itself is sound")
        done = fixture.run("retired", "--base", bad)
        self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
        self.assertIn(sentence(BAND_PATH, "MUTATIONS"), done.stderr)
        self.assertNotIn("Traceback", done.stderr)

    def test_a_revision_whose_header_repeats_a_key_is_refused(self):
        fixture = Fixture(self)
        good = (fixture.root / HEADER_PATH).read_text(encoding="utf-8")
        fixture.write(HEADER_PATH, with_repeat(good, "arities"))
        bad = fixture.commit("a header that repeats a key")
        fixture.write(HEADER_PATH, good)
        fixture.commit("the header reads")
        with self.assertRaises(mutation_rows.PopulationRefused) as raised:
            mutation_rows.load_revision(fixture.root, bad)
        self.assertEqual(str(raised.exception), sentence(HEADER_PATH, "arities"))
        done = fixture.run("retired", "--base", bad)
        self.assertEqual(done.returncode, 2, done.stdout + done.stderr)

    def test_every_committed_band_file_reads_as_plain_json_reads_it(self):
        files = examined(
            "committed band files", sorted((REPO / "scripts/mutation-rows.d").glob("*.json"))
        )
        plain = sum(
            len(rows)
            for path in files
            for rows in json.loads(path.read_text(encoding="utf-8"))["tables"].values()
        )
        rows = mutation_rows.rows_of(mutation_rows.load_tree(REPO))
        header = json.loads((REPO / HEADER_PATH).read_text(encoding="utf-8"))
        held = plain + sum(len(rows) for rows in header["tables"].values())
        self.assertEqual(len(rows), held)
        self.assertGreater(len(rows), 0)


class TheRetiredListRefusesARepeatedKey(unittest.TestCase):
    """`retired` reads the retirement list through the one parser (SPEC-122, #385)."""

    def leaving_row(self):
        """A fixture whose one row leaves while its target stays, and the base it leaves from."""
        row = script_row(
            "S00030-DOUBLE", "x * 2", "x * 3", "test_fixmod.Double.test_two_doubles_to_four"
        )
        fixture = Fixture(self, [("SCRIPT_MUTATIONS", row)])
        base = git(fixture.root, "rev-parse", "HEAD").strip()
        fixture.rows([])
        return fixture, base

    def test_a_retired_list_that_repeats_a_key_is_refused_naming_it(self):
        entry = '{"id": "S00030-DOUBLE", "reason": "moved", "approval": "the maintainer"'
        planted = {
            "retired": '{"retired": [], "retired": []}\n',
            "approval": '{"retired": [' + entry + ', "approval": "another"}]}\n',
        }
        for key, text in examined("planted retired lists", planted.items()):
            with self.subTest(key=key):
                fixture, base = self.leaving_row()
                fixture.write(RETIRED_PATH, text)
                fixture.commit("a retired list that repeats a key")
                done = fixture.run("retired", "--base", base)
                self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
                self.assertIn(f"mutation_rows: REFUSED: {sentence(RETIRED_PATH, key)}", done.stderr)
                self.assertNotIn("Traceback", done.stderr)
                self.assertNotIn("S00030-DOUBLE", done.stdout, "a refused list retires nothing")

    def test_a_well_formed_retired_list_still_admits_its_row(self):
        fixture, base = self.leaving_row()
        record = {
            "retired": [
                {"id": "S00030-DOUBLE", "reason": "moved", "approval": "the maintainer, in #1"}
            ]
        }
        fixture.write(RETIRED_PATH, json.dumps(record) + "\n")
        fixture.commit("the maintainer approved the retirement")
        done = fixture.run("retired", "--base", base)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("retired with approval: the maintainer, in #1", done.stdout)
        self.assertRegex(done.stdout, r"(?m)^examined 1\b")


if __name__ == "__main__":
    unittest.main()
