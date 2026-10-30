"""SPEC-125: the dev profile keeps line tables only in workspace crates and none in dependencies."""

import re
import tomllib
import unittest

from _support import REPO, examined

MANIFEST = REPO / "Cargo.toml"
OVERRIDE = re.compile(r"CARGO_PROFILE_|--config[=\s]+['\"]?profile\.")
PLANTED = (
    "CARGO_PROFILE_DEV_DEBUG=2",
    "cargo build --config profile.dev.debug=2",
    "cargo build --config 'profile.dev.debug=2'",
    "cargo build --config=profile.dev.debug=2",
    "cargo build --config='profile.dev.debug=2'",
)


def profile():
    return tomllib.loads(MANIFEST.read_text(encoding="utf-8")).get("profile", {})


class BuildProfile(unittest.TestCase):
    def test_workspace_crates_build_with_line_tables_only(self):
        """A1: the dev profile (which the test profile inherits) keeps line tables only."""
        dev = profile().get("dev", {})
        self.assertEqual(dev.get("debug"), "line-tables-only")

    def test_every_dependency_builds_without_debuginfo(self):
        """A1: the dependency override (package "*") turns debuginfo off, as the boolean false."""
        override = profile().get("dev", {}).get("package", {}).get("*", {})
        self.assertIs(override.get("debug"), False)

    def test_the_profile_lives_in_the_manifest_and_no_job_overrides_it(self):
        """A2: CI builds on the manifest's profile; no workflow or script sets a second one."""
        self.assertIn("dev", profile(), "the manifest declares no [profile.dev]")
        held = []
        for root in (REPO / ".github", REPO / "scripts"):
            held += [
                p
                for p in sorted(root.rglob("*"))
                if p.is_file()
                and p.suffix in {".yml", ".yaml", ".sh", ".py"}
                and p.name != "test_build_profile.py"
            ]
        for path in examined("workflow and script files", held):
            text = path.read_text(encoding="utf-8", errors="replace")
            self.assertIsNone(OVERRIDE.search(text), f"{path} overrides the build profile")

    def test_the_override_scan_catches_every_planted_spelling(self):
        """A2: the scan's own cases, the equals form of --config included."""
        for planted in PLANTED:
            self.assertIsNotNone(OVERRIDE.search(planted), f"the scan misses: {planted}")


if __name__ == "__main__":
    unittest.main()
