"""SPEC-046 A2 and A9: the packs' reading rows are green on the golden readings.

The rows run from a checkout of the packs (``PACKS_CHECKOUT``, the box's default when unset); a
box without that checkout skips, because the rows are the packs' own and are not copied here.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKS = Path(os.environ.get("PACKS_CHECKOUT", "/tmp/phx-worktrees/deckstreak-packs-b41ce8e"))
EXAMINED = re.compile(r"^examined (\d+)$", re.MULTILINE)


def rows(pack: str, severities: tuple[str, ...], ids: tuple[str, ...] = ()) -> list[dict]:
    """The pack's rows of the given severities, optionally only the named ones."""
    checks = json.loads((PACKS / "skills" / "packs" / pack / "checks.json").read_text())
    return [
        row
        for row in checks["checks"]
        if row["severity"] in severities and (not ids or row["id"].split(".")[-1] in ids)
    ]


def run(row: dict) -> tuple[int, int, str]:
    """Run one row's probe over this repository: (exit code, examined count, output)."""
    skills = str(PACKS / "skills")
    command = [
        part.replace("{skills}", skills).replace("{root}", str(ROOT))
        for part in row["probe"]["command"]
    ]
    command[0] = sys.executable
    done = subprocess.run(
        command,
        cwd=PACKS,
        capture_output=True,
        text=True,
        timeout=row["probe"].get("timeout_seconds", 120),
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        check=False,
    )
    found = EXAMINED.findall(done.stdout)
    return done.returncode, int(found[-1]) if found else 0, done.stdout + done.stderr


@unittest.skipUnless(PACKS.is_dir(), "the packs' checkout is not present")
class ReadingRowsOnTheGoldens(unittest.TestCase):
    def assert_green(self, row: dict) -> None:
        code, examined, output = run(row)
        self.assertEqual(code, 0, f"{row['id']} is red on the goldens:\n{output}")
        self.assertGreater(examined, 0, f"{row['id']} examined nothing:\n{output}")

    def test_the_length_and_scale_rows_are_green_on_the_goldens(self) -> None:
        picked = rows("study-duties", ("block", "advisory"), ("reading-length", "reading-scale"))
        self.assertEqual([row["id"] for row in picked], ["reading-length", "reading-scale"])
        for row in picked:
            self.assert_green(row)

    def test_every_blocking_reading_row_is_green_on_the_goldens(self) -> None:
        reading = (
            ("study-duties", ("reading-length",)),
            (
                "learning-science",
                (
                    "retrieval-prompts",
                    "answers-hidden",
                    "deep-questions",
                    "key-terms-defined",
                ),
            ),
            ("law-professors", ("rule-cites-corpus", "citations-resolve")),
            ("language-mentors", ("cefr-ratio", "i1-glosses")),
        )
        seen = 0
        for pack, ids in reading:
            picked = rows(pack, ("block",), ids)
            self.assertEqual(len(picked), len(ids), f"{pack} lost a blocking row")
            for row in picked:
                self.assert_green(row)
                seen += 1
        self.assertEqual(seen, 9)


if __name__ == "__main__":
    unittest.main()
