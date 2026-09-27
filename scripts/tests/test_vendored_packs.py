"""The vendored packs are byte-identical to the phoenix-v2 commit they name (ADR-004)."""

import hashlib
import json
import unittest

from _support import REPO, examined

MANIFEST = REPO / ".packs" / "VENDORED.json"
OURS = {".packs/VENDORED.json", ".packs/wiring.json"}


def unlisted_candidates():
    return sorted(
        path.relative_to(REPO).as_posix()
        for path in (REPO / ".packs").rglob("*")
        if path.is_file() and "__pycache__" not in path.parts
    )


class VendoredPacksMatchTheirSource(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_every_listed_file_has_its_recorded_digest(self):
        for entry in examined("vendored files", self.manifest["files"]):
            path = REPO / entry["path"]
            self.assertTrue(path.is_file(), f"{entry['path']} is listed but missing")
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            self.assertEqual(digest, entry["sha256"], f"{entry['path']} differs from its source")

    def test_nothing_under_packs_is_unlisted(self):
        listed = {entry["path"] for entry in self.manifest["files"]} | OURS
        present = examined("files under .packs", unlisted_candidates())
        planted = set(present) | {".packs/skills/packs/sdd/planted.md"}
        self.assertEqual(sorted(planted - listed), [".packs/skills/packs/sdd/planted.md"])
        self.assertEqual(sorted(set(present) - listed), [])

    def test_the_manifest_names_a_full_commit(self):
        self.assertRegex(self.manifest["vendored_from"], r"^[0-9a-f]{40}$")
        methodology = json.loads((REPO / "methodology.json").read_text(encoding="utf-8"))
        self.assertEqual(methodology["vendored_from"], self.manifest["vendored_from"])


if __name__ == "__main__":
    unittest.main()
