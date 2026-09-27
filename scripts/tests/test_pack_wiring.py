"""Every vendored pack has an honest wiring state (ADR-004; SPEC-002 A4)."""

import json
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

WIRING = REPO / ".packs" / "wiring.json"
MANIFEST = REPO / "docs" / "issues-manifest.json"
ISSUE = re.compile(r"^#(\d+)$")


def issue_numbers():
    if not MANIFEST.is_file():
        return set()
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    return {entry["number"] for entry in data["issues"].values()}


class PackWiringIsHonest(unittest.TestCase):
    def setUp(self):
        self.wiring = json.loads(WIRING.read_text(encoding="utf-8"))["packs"]

    def test_every_vendored_pack_has_a_wiring_state(self):
        vendored = [
            p.name for p in (REPO / ".packs" / "skills" / "packs").iterdir() if (p / "checks.json").is_file()
        ]
        for pack in examined("vendored packs", vendored):
            self.assertIn(pack, self.wiring, f"{pack} has no wiring state")

    def test_every_waiting_pack_or_row_names_an_open_issue_by_number(self):
        numbers = issue_numbers()
        waiting = []
        for pack, entry in self.wiring.items():
            if entry["state"] in ("pending", "deferred"):
                waiting.append((pack, entry["enforced_by"]))
            for row, issue in entry.get("deferred_rows", {}).items():
                waiting.append((f"{pack}:{row}", issue))
        for name, issue in examined("waiting packs and rows", waiting):
            match = ISSUE.match(issue)
            self.assertIsNotNone(match, f"{name} waits on {issue!r}, which is not an issue number")
            self.assertIn(int(match.group(1)), numbers, f"{name} waits on {issue}, which is not in the manifest")

    def test_the_runner_refuses_a_wiring_that_forgets_a_pack(self):
        tmp = Path(tempfile.mkdtemp())
        shutil.copytree(REPO / ".packs", tmp / ".packs")
        shutil.copytree(REPO / "scripts", tmp / "scripts")
        data = json.loads((tmp / ".packs" / "wiring.json").read_text())
        dropped = sorted(data["packs"])[0]
        del data["packs"][dropped]
        (tmp / ".packs" / "wiring.json").write_text(json.dumps(data))
        done = subprocess.run(
            [sys.executable, str(tmp / "scripts" / "pack-rows.py"), "--root", str(tmp)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn(dropped, done.stdout)


if __name__ == "__main__":
    unittest.main()
