"""SPEC-001's parity matrix accounts for every predecessor and second-brain feature (SPEC-001 A1-A5)."""

import json
import re
import unittest

from _support import REPO, examined

SPEC = REPO / "docs" / "specs" / "SPEC-001-campaign-prd-parity-and-waves.md"
IDS = REPO / "docs" / "parity" / "v9-feature-ids.txt"
MAP = REPO / "docs" / "CONTEXT-MAP.md"
MANIFEST = REPO / "docs" / "issues-manifest.json"
WAVES = {f"W{n}" for n in range(10)}
NON_CRATE_CONTEXTS = {"deploy", "repo", "miniapp", "landing", "migration"}


def section_rows(title_word):
    """The data rows of the first table under the `## ` heading that names `title_word`."""
    text = SPEC.read_text(encoding="utf-8")
    parts = re.split(r"(?m)^## ", text)
    block = next(p for p in parts if title_word in p.splitlines()[0])
    rows = []
    for line in block.splitlines():
        if line.startswith("|") and not line.startswith("|---"):
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            rows.append(cells)
    return rows[1:]


def contexts():
    fence = re.search(r"```context-map\n(.*?)```", MAP.read_text(encoding="utf-8"), re.S).group(1)
    return {line.split()[0].removeprefix("deck-streak-") for line in fence.splitlines() if line.strip()}


def manifest_numbers():
    data = json.loads(MANIFEST.read_text(encoding="utf-8")) if MANIFEST.is_file() else {"issues": {}}
    return {entry["number"] for entry in data["issues"].values()}


class ParityMatrixIsComplete(unittest.TestCase):
    def setUp(self):
        self.rows = section_rows("Appendix A")
        self.by_id = {}
        for row in self.rows:
            self.by_id.setdefault(row[0].strip("`"), []).append(row)

    def test_every_v9_feature_is_accounted_for_once(self):
        population = [line for line in IDS.read_text(encoding="utf-8").splitlines() if line]
        for fid in examined("predecessor feature ids", population):
            self.assertEqual(len(self.by_id.get(fid, [])), 1, f"{fid} appears {len(self.by_id.get(fid, []))} times")
        self.assertEqual(sorted(self.by_id), sorted(population))

    def test_every_exclusion_carries_its_reason(self):
        excluded = [row for row in self.rows if row[5].startswith("exclude")]
        for row in examined("excluded rows", excluded):
            self.assertRegex(row[5], r"^exclude: \S", row[0])
            self.assertRegex(row[6], r"#\d+", f"{row[0]} names no owner decision issue")

    def test_every_built_row_names_a_context_and_a_wave(self):
        declared = contexts() | NON_CRATE_CONTEXTS
        built = [row for row in self.rows if row[5] == "build"]
        for row in examined("built rows", built):
            self.assertIn(row[3], declared, f"{row[0]} names context {row[3]}")
            self.assertIn(row[4], WAVES, f"{row[0]} names wave {row[4]}")

    def test_every_built_row_names_its_issue(self):
        numbers = manifest_numbers()
        built = [row for row in self.rows if row[5] == "build"]
        for row in examined("built rows", built):
            cited = [int(n) for n in re.findall(r"#(\d+)", row[6])]
            self.assertTrue(cited, f"{row[0]} names no issue")
            for number in cited:
                self.assertIn(number, numbers, f"{row[0]} names #{number}, which is not in the manifest")

    def test_every_second_brain_feature_is_accounted_for(self):
        rows = section_rows("Appendix B")
        ids = [row[0].strip("`") for row in rows]
        self.assertEqual(sorted(examined("second-brain rows", ids)), sorted(f"SB-U{n}" for n in range(1, 23)))
        for row in rows:
            self.assertTrue(row[4] == "build" or row[4].startswith("exclude: "), row[0])


if __name__ == "__main__":
    unittest.main()
