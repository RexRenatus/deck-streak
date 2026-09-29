"""SPEC-057's campaign table (section 7): each crate's delivery fills its row from the scoped
battery's `table` line, and a closed row reads no unexplained mutant (A16 to A25).

The reader takes section 7 wherever the SPEC then lives, `docs/specs/planned/` or `docs/specs/`,
and each row's fragment under `scripts/mutation-equivalent.d/`. A row that is still open, or whose
opening sweep found unexplained mutants, fails by name: that is its delivery's red.
"""

import json
import re
import unittest

from _support import REPO, examined

SPECS = REPO / "docs" / "specs"
FRAGMENTS = REPO / "scripts" / "mutation-equivalent.d"
#: One row of section 7: order, crate, listed, killed, equivalent, unexplained, unviable, PR.
ROW = re.compile(r"(?m)^\| (\d+) \| `([^`]+)` \|" + r" ([^|]+?) \|" * 6 + r"$")
#: A delivered row's PR cell: its pull request and both runs of the scoped battery (R16).
DELIVERED = re.compile(r"^#(\d+) \(runs (\d+), (\d+)\)$")
COUNTS = ("listed", "killed", "equivalent", "unexplained", "unviable")


def campaign():
    """{crate: {count: value, "pr": cell}} from section 7 of SPEC-057."""
    specs = sorted(SPECS.rglob("SPEC-057-*.md"))
    if len(specs) != 1:
        raise AssertionError(f"SPEC-057 lives in {len(specs)} places under docs/specs, not one")
    text = specs[0].read_text(encoding="utf-8")
    section = re.search(r"(?ms)^## 7\. The campaign plan and its table\n(.*?)(?=^## )", text)
    if section is None:
        raise AssertionError(f"{specs[0].name} has no section 7")
    rows = {}
    for match in ROW.finditer(section.group(1)):
        cells = [cell.strip() for cell in match.groups()[2:]]
        rows[match.group(2)] = dict(zip((*COUNTS, "pr"), cells, strict=True))
    return rows


def records(package):
    path = FRAGMENTS / f"{package}.json"
    if not path.is_file():
        return 0
    return len(json.loads(path.read_text(encoding="utf-8"))["records"])


class TheCampaignTable(unittest.TestCase):
    def assert_closed(self, crate):
        rows = campaign()
        examined("rows of the campaign's table", list(rows))
        self.assertIn(crate, rows, f"section 7 has no row for {crate}")
        row = rows[crate]
        for count in COUNTS:
            self.assertRegex(
                row[count], r"^\d+$", f"{crate}: {count} is {row[count]!r}, unmeasured"
            )
        listed, killed, equivalent, unexplained, unviable = (int(row[count]) for count in COUNTS)
        self.assertEqual(unexplained, 0, f"{crate}: {unexplained} unexplained mutant(s) in its row")
        self.assertEqual(listed, killed + equivalent + unexplained + unviable, crate)
        self.assertEqual(equivalent, records(crate), f"{crate}: equivalent against its fragment")
        delivered = DELIVERED.match(row["pr"])
        self.assertIsNotNone(delivered, f"{crate}: {row['pr']!r} names no pull request and runs")
        self.assertNotEqual(delivered.group(2), delivered.group(3), f"{crate}: one run twice")

    def test_the_vault_row_reads_no_unexplained_mutant(self):
        self.assert_closed("deck-streak-vault")

    def test_the_ingest_row_reads_no_unexplained_mutant(self):
        self.assert_closed("deck-streak-ingest")

    def test_the_kernel_row_reads_no_unexplained_mutant(self):
        self.assert_closed("deck-streak-kernel")

    def test_the_daemon_row_reads_no_unexplained_mutant(self):
        self.assert_closed("deck-streak-daemon")

    def test_the_identity_row_reads_no_unexplained_mutant(self):
        self.assert_closed("deck-streak-identity")

    def test_the_api_row_reads_no_unexplained_mutant(self):
        self.assert_closed("deck-streak-api")


if __name__ == "__main__":
    unittest.main()
