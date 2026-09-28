"""The example taxonomy is public and synthetic (SPEC-045 A12, R1).

The public scrub reads the whole tree with `deploy/config/readings-taxonomy.example.json` named as a
subject too, and must be green with the example among the files it examined. The example must be a
taxonomy of the schema the readings read, so it shows the file's real shape, and a copy of it that
holds a planted private shape is refused by name, so a scrub gone blind fails here. With the
maintainer's private list in `$PERSONA_CORE_DENY_LIST`, the scrub also reads the owner's own
literals, deck names among them; public CI judges the public shapes alone.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO

SCRUB = REPO / "scripts" / "public-scrub.py"
EXAMPLE = REPO / "deploy" / "config" / "readings-taxonomy.example.json"
SCHEMA = "deckstreak.readings.taxonomy.v1"
#: The scrub's own count of the files it read.
EXAMINED = re.compile(r"^examined (\d+) file\(s\)", re.MULTILINE)
#: A private shape, built at run time so this file holds none for the tree scan to find.
PLANTED = ".".join(["10", "20", "30", "40"])


def scrub(*arguments):
    """Runs the public scrub over the repository with `arguments`, writing no bytecode."""
    return subprocess.run(
        [sys.executable, str(SCRUB), "--root", str(REPO), *arguments],
        capture_output=True,
        text=True,
        check=False,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
    )


def examined_files(output):
    """How many files the scrub says it read, or 0 when it printed no count."""
    counted = EXAMINED.search(output)
    return int(counted.group(1)) if counted else 0


class TheExampleTaxonomyIsPublic(unittest.TestCase):
    def test_the_public_scrub_is_green_with_the_example_taxonomy(self):
        self.assertTrue(
            EXAMPLE.is_file(),
            f"{EXAMPLE.relative_to(REPO)} is missing: R1's synthetic taxonomy shows the file's shape",
        )
        taxonomy = json.loads(EXAMPLE.read_text(encoding="utf-8"))
        self.assertEqual(taxonomy["schema"], SCHEMA)
        self.assertGreaterEqual(len(taxonomy["law"]["roots"]), 1)
        self.assertGreaterEqual(len(taxonomy["law"]["bands"]), 1)
        self.assertGreaterEqual(len(taxonomy["languages"]), 1)
        self.assertGreaterEqual(len(taxonomy["writing_roots"]), 1)

        # The tree, and the example named as a subject too: green, with the example among the
        # files read.
        done = scrub("--subject", str(EXAMPLE))
        self.assertEqual(done.returncode, 0, done.stdout)
        print(f"examined {examined_files(done.stdout)} file(s) with the example taxonomy")
        self.assertGreater(examined_files(done.stdout), 1, done.stdout)

        # The example alone is read, and a copy of it holding a planted shape is refused by name.
        alone = scrub("--no-tree", "--subject", str(EXAMPLE))
        self.assertEqual((alone.returncode, examined_files(alone.stdout)), (0, 1), alone.stdout)
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "readings-taxonomy.planted.json"
            taxonomy["writing_roots"] = [PLANTED]
            planted.write_text(json.dumps(taxonomy, indent=2) + "\n", encoding="utf-8")
            refused = scrub("--no-tree", "--subject", str(planted))
        self.assertEqual(refused.returncode, 1, refused.stdout)
        self.assertIn("readings-taxonomy.planted.json", refused.stdout)
        self.assertIn("ipv4", refused.stdout)


if __name__ == "__main__":
    unittest.main()
