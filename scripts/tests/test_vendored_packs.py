"""The vendored packs are byte-identical to the phoenix-v2 commit they name (ADR-004), and every
exclusion the manifest records is one a tool can apply (SPEC-037 A7)."""

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


def not_machine_applicable(entry):
    """Why one `excluded` entry is prose a tool cannot apply, or [] when it is not (SPEC-037 R2):
    exactly `globs` and `why`, and each glob one pattern over source paths, relative to the
    source's root, with no whitespace a sentence would carry."""
    if not isinstance(entry, dict):
        return ["is not an object"]
    problems = []
    prose = sorted(set(entry) - {"globs", "why"})
    if prose:
        problems.append(f"carries prose keys: {', '.join(prose)}")
    globs = entry.get("globs")
    if not isinstance(globs, list) or not globs:
        problems.append("carries no globs")
    else:
        for glob in globs:
            if not isinstance(glob, str) or not glob or any(char.isspace() for char in glob):
                problems.append(f"holds a glob that is not one pattern: {glob!r}")
            elif glob.startswith("/") or ".." in glob.split("/") or "\\" in glob:
                problems.append(f"holds a glob outside the source tree: {glob}")
    why = entry.get("why")
    if not isinstance(why, str) or not why.strip():
        problems.append("gives no why")
    return problems


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

    def test_every_exclusion_is_machine_applicable(self):
        planted = {"from": "skills/packs/x/** and scripts/y.py", "why": "a sentence, not a glob"}
        self.assertEqual(
            not_machine_applicable(planted), ["carries prose keys: from", "carries no globs"]
        )
        spaced = {"globs": ["skills/packs/x/** and scripts/y.py"], "why": "one glob, two paths"}
        self.assertEqual(
            not_machine_applicable(spaced),
            ["holds a glob that is not one pattern: 'skills/packs/x/** and scripts/y.py'"],
        )
        for entry in examined("exclusions", self.manifest["excluded"]):
            self.assertEqual(not_machine_applicable(entry), [], entry)

    def test_the_manifest_names_a_full_commit(self):
        self.assertRegex(self.manifest["vendored_from"], r"^[0-9a-f]{40}$")
        methodology = json.loads((REPO / "methodology.json").read_text(encoding="utf-8"))
        self.assertEqual(methodology["vendored_from"], self.manifest["vendored_from"])


if __name__ == "__main__":
    unittest.main()
