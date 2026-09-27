"""The public scrub refuses private shapes and literals and passes harmless ones (SPEC-002 A7)."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO

SCRUB = REPO / "scripts" / "public-scrub.py"
# Built at run time, so this file itself carries no address for the tree scan to find.
PLANTED = ".".join(["10", "20", "30", "40"])


def scrub(text, private_literals=()):
    tmp = Path(tempfile.mkdtemp())
    subject = tmp / "subject"
    subject.mkdir()
    (subject / "note.md").write_text(text, encoding="utf-8")
    args = [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree", "--subject", str(subject)]
    if private_literals:
        deny = tmp / "private.json"
        deny.write_text(json.dumps({
            "schema": "phx.persona.deny.v1", "key_markers": [], "patterns": [],
            "literals": list(private_literals), "journal_paths": [],
        }))
        args += ["--deny-list", str(deny)]
    return subprocess.run(args, capture_output=True, text=True, check=False)


class PublicScrubHoldsTheLine(unittest.TestCase):
    def test_an_internal_address_is_refused_by_rule_name(self):
        done = scrub(f"the host answers at {PLANTED} today\n")
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("note.md:1: ipv4", done.stdout)
        self.assertNotIn(PLANTED, done.stdout)

    def test_a_private_literal_is_refused_by_index_never_by_value(self):
        done = scrub("the deck is called Secret Deck Name\n", ["secret deck name"])
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("private literal #0", done.stdout)
        self.assertNotIn("Secret Deck Name", done.stdout)

    def test_loopback_documentation_and_versions_pass(self):
        done = scrub("bind 127.0.0.1, see 192.0.2.10, release v1.2.3.4 and [::2]\n")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("examined 1 file(s)", done.stdout)

    def test_an_empty_subject_is_void(self):
        tmp = Path(tempfile.mkdtemp())
        done = subprocess.run(
            [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree", "--subject", str(tmp)],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(done.returncode, 3, done.stdout)


if __name__ == "__main__":
    unittest.main()
