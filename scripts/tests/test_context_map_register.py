"""The predecessor's table register in docs/CONTEXT-MAP.md counts what its prose counts (#420)."""

import re
import unittest

from _support import REPO, examined

MAP = REPO / "docs" / "CONTEXT-MAP.md"
HEADER = "| v9 table | owning context | privacy class in v9 |"
OWN = "### DeckStreak's own tables"
PROSE = re.compile(r"starting assignment of v9's (\d+) tables")


def lines():
    return MAP.read_text(encoding="utf-8").split("\n")


def register_names():
    """Every table name in the predecessor's register, in order, repeats kept."""
    held = lines()
    start = held.index(HEADER) + 2
    names = []
    for line in held[start:]:
        if not line.startswith("|"):
            break
        names.append(re.match(r"\| `([^`]+)` \|", line).group(1))
    return names


def own_names():
    held = lines()
    start = held.index(OWN)
    names = []
    for line in held[start:]:
        match = re.match(r"\| `([^`]+)` \| `[a-z]+` \| `?migrations/", line)
        if match:
            names.append(match.group(1))
    return names


class RegisterCountsWhatItsProseCounts(unittest.TestCase):
    def test_the_register_holds_the_number_of_unique_names_its_prose_states(self):
        stated = [int(n) for n in PROSE.findall("\n".join(lines()))]
        self.assertEqual(len(stated), 1, "the prose states the register's count exactly once")
        names = examined("register rows", register_names())
        self.assertEqual(len(set(names)), stated[0])

    def test_no_name_is_registered_twice(self):
        names = examined("register rows", register_names())
        self.assertIn("schema_versions", names)
        repeated = sorted({n for n in names if names.count(n) > 1})
        self.assertEqual(repeated, [])

    def test_a_table_only_deckstreak_creates_is_not_a_table_to_import(self):
        own_only = {"xp_settlement"}
        migrations = examined("own-table rows", own_names())
        for name in own_only:
            self.assertIn(name, migrations, f"{name} is a DeckStreak table")
            self.assertNotIn(name, register_names(), f"{name} is not one of v9's tables")

    def test_the_counting_reads_a_planted_register(self):
        planted = [
            "| `a` | `x` | y |",
            "| `a` | `x` | y |",
        ]
        names = [re.match(r"\| `([^`]+)` \|", line).group(1) for line in planted]
        self.assertEqual((len(names), len(set(names))), (2, 1))


if __name__ == "__main__":
    unittest.main()
