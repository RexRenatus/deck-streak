"""The owner's signed rulings are counted where readers look for decisions.

`docs/rulings/` holds the owner's signed rulings, one `OWNER-RULING-*.md` file each. The
documentation index (`docs/README.md`) and the agent instructions (`AGENTS.md`, under "Where
decisions live") each link the folder once and state how many signed rulings it holds. A ruling
that lands without its count moving, or an index that drops the folder, turns this test red.
"""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RULINGS = REPO / "docs" / "rulings"
INDEXES = (
    (REPO / "docs" / "README.md", "rulings/"),
    (REPO / "AGENTS.md", "docs/rulings/"),
)
STATED = re.compile(r"\]\(([^)]+)\)[^\n]*?\b(\d+) signed rulings\b")


def signed_rulings(folder: Path) -> list[str]:
    """The signed ruling files the folder holds, by name."""
    return sorted(p.name for p in folder.glob("OWNER-RULING-*.md"))


def stated_counts(text: str, target: str) -> list[int]:
    """Every count a line states beside a link to the rulings folder."""
    return [int(n) for link, n in STATED.findall(text) if link == target]


class RulingIndexTest(unittest.TestCase):
    def test_each_index_states_the_count_of_signed_rulings(self):
        held = signed_rulings(RULINGS)
        self.assertGreater(len(held), 0, "docs/rulings/ holds no signed ruling file")
        for index, target in INDEXES:
            with self.subTest(index=index.relative_to(REPO).as_posix()):
                stated = stated_counts(index.read_text(encoding="utf-8"), target)
                self.assertEqual(
                    stated,
                    [len(held)],
                    f"{index.relative_to(REPO).as_posix()} must link {target} once and "
                    f"state {len(held)} signed rulings",
                )


if __name__ == "__main__":
    unittest.main()
