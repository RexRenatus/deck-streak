"""Planned SPECs wait in docs/specs/planned/ and never share a number with any SPEC (ADR-016)."""

import re
import unittest

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


class PlannedSpecsNeverCollide(unittest.TestCase):
    def test_every_spec_file_is_named_by_the_convention(self):
        for path in examined("SPEC files", list(SPECS.glob("SPEC-*.md")) + list((SPECS / "planned").glob("SPEC-*.md"))):
            self.assertRegex(path.name, NAME, f"{path.name} does not follow SPEC-NNN-slug.md")

    def test_no_number_is_held_twice_across_judged_and_planned(self):
        held = numbers(SPECS) + numbers(SPECS / "planned")
        seen = {}
        for number, name in examined("numbered SPECs", held):
            self.assertNotIn(number, seen, f"SPEC-{number:03d} is held by {seen.get(number)} and {name}")
            seen[number] = name


if __name__ == "__main__":
    unittest.main()
