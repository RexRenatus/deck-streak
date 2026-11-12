"""Planned SPECs wait in docs/specs/planned/ and never share a number with any SPEC (ADR-016)."""

import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SPECS = REPO / "docs" / "specs"
NAME = re.compile(r"^SPEC-(\d{3})-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")


def numbers(directory):
    found = []
    for path in sorted(directory.glob("SPEC-*.md")):
        match = NAME.match(path.name)
        if match and "-amendment" not in path.name:
            found.append((int(match.group(1)), path.name))
    return found


STATUS = re.compile(r"^- \*\*Status:\*\*(.*)$", re.MULTILINE)
STRUCK = re.compile(r"~~.*?~~")


def reads_planned(path):
    """True when the Status line's text, struck spans removed, opens with the word planned."""
    found = STATUS.search(path.read_text(encoding="utf-8"))
    if not found:
        return False
    words = STRUCK.sub("", found.group(1)).split()
    return bool(words) and words[0].lower().strip("`*_,.;:") == "planned"


def collisions(held):
    """Every number held by two documents, as (number, first, second)."""
    seen, found = {}, []
    for number, name in held:
        if number in seen:
            found.append((number, seen[number], name))
        seen.setdefault(number, name)
    return found


class PlannedSpecsNeverCollide(unittest.TestCase):
    def test_every_spec_file_is_named_by_the_convention(self):
        for path in examined(
            "SPEC files",
            list(SPECS.glob("SPEC-*.md")) + list((SPECS / "planned").glob("SPEC-*.md")),
        ):
            self.assertRegex(path.name, NAME, f"{path.name} does not follow SPEC-NNN-slug.md")

    def test_no_number_is_held_twice_across_judged_and_planned(self):
        planted = [(7, "SPEC-007-a.md"), (7, "SPEC-007-b.md")]
        self.assertEqual(collisions(planted), [(7, "SPEC-007-a.md", "SPEC-007-b.md")])
        held = examined("numbered SPECs", numbers(SPECS) + numbers(SPECS / "planned"))
        self.assertEqual(collisions(held), [])

    def test_no_judged_spec_reads_planned(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "SPEC-001-x.md"
            planted.write_text("- **Status:** planned (in `docs/specs/planned/`) until\n", "utf-8")
            self.assertTrue(reads_planned(planted))
            planted.write_text(
                "- **Status:** delivered by #1 (ADR-016). ~~planned (in `docs/specs/planned/`)~~\n",
                "utf-8",
            )
            self.assertFalse(reads_planned(planted))
        judged = examined("judged SPECs", sorted(SPECS.glob("SPEC-*.md")))
        planned = [path.name for path in judged if reads_planned(path)]
        self.assertEqual(
            planned, [], "a SPEC in docs/specs/ is delivered, so it never reads planned"
        )


if __name__ == "__main__":
    unittest.main()
