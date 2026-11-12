"""The FORK-LOCK GUARD (SPEC-565 A2 to A4; ADR-583): the engine fork's lockfile at the pinned commit
holds no block-buffer below 0.12.1 and no digest below 0.11.0. The release builds the sync server
from that lockfile (`cargo install --locked` at the pinned commit), and this workspace's lock never
shows it, so the guard reads the fork's own lock.

- The fixture legs judge `scripts/lock-digest-line.py` itself, under
  `scripts/tests/fixtures/fork-lock/`: the digest line of the fork's lock at a078820 reads RED, a
  green lock reads GREEN, a planted block-buffer 0.10.4, digest 0.10.7 or release candidate
  0.12.1-rc.0 reads RED, and a lock with no block-buffer, a missing lock or one that does not parse
  reads VOID, never GREEN.
- The real-lock leg finds the pinned commit's checkout through `cargo metadata`: the package
  `anki` from the fork at the rev the root manifest's `[patch]` entry names, and the `Cargo.lock`
  two directories above its manifest. It runs the guard there. A missing checkout fails the leg;
  it never skips.
"""

import json
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "lock-digest-line.py"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "fork-lock"
MANIFEST = REPO / "Cargo.toml"
UPSTREAM = "https://github.com/ankitects/anki.git"
FORK = "https://github.com/RexRenatus/anki.git"
#: `cargo metadata` resolves the whole workspace; the bound fails a hung run by name.
METADATA_BOUND_S = 600


def guard(lock):
    """One run of the guard over `lock`: its exit code and its printed lines."""
    done = subprocess.run(
        [sys.executable, str(SCRIPT), str(lock)], capture_output=True, text=True, check=False
    )
    return done.returncode, done.stdout + done.stderr


class TheGuardJudgesItsFixtures(unittest.TestCase):
    def assert_verdict(self, name, code, *lines):
        verdict, output = guard(FIXTURES / name)
        self.assertEqual(verdict, code, output)
        for line in lines:
            self.assertIn(line, output)

    def test_the_forks_digest_line_at_a078820_reads_red(self):
        """block-buffer 0.10.4 and 0.12.0, and digest 0.10.7: 2 of 2 and 1 of 2."""
        self.assert_verdict(
            "a078820-digest-line.lock",
            1,
            "block-buffer below 0.12.1: 2 of 2",
            "digest below 0.11.0: 1 of 2",
            "lock-digest-line: RED",
        )

    def test_a_green_lock_reads_green(self):
        self.assert_verdict(
            "green.lock",
            0,
            "block-buffer below 0.12.1: 0 of 1",
            "digest below 0.11.0: 0 of 1",
            "lock-digest-line: GREEN",
        )

    def test_a_planted_block_buffer_0_10_4_reads_red(self):
        self.assert_verdict(
            "planted-block-buffer.lock",
            1,
            "block-buffer below 0.12.1: 1 of 2: 0.10.4",
            "digest below 0.11.0: 0 of 1",
        )

    def test_a_planted_digest_0_10_7_reads_red(self):
        self.assert_verdict(
            "planted-digest.lock",
            1,
            "block-buffer below 0.12.1: 0 of 1",
            "digest below 0.11.0: 1 of 2: 0.10.7",
        )

    def test_a_release_candidate_of_0_12_1_sorts_below_it_and_reads_red(self):
        self.assert_verdict(
            "planted-prerelease.lock", 1, "block-buffer below 0.12.1: 1 of 1: 0.12.1-rc.0"
        )

    def test_a_lock_that_examines_nothing_or_cannot_be_read_is_void(self):
        """VOID, exit 2, is never green: no block-buffer, a missing lock, a lock that does not
        parse."""
        with tempfile.TemporaryDirectory(prefix="fork-lock-guard-") as scratch:
            broken = Path(scratch) / "broken.lock"
            broken.write_text("[[package]\nname = \n", encoding="utf-8")
            cases = examined(
                "lock(s) the guard cannot judge",
                [FIXTURES / "void.lock", Path(scratch) / "missing.lock", broken],
            )
            for lock in cases:
                with self.subTest(lock=lock.name):
                    code, output = guard(lock)
                    self.assertEqual(code, 2, output)
                    self.assertIn("lock-digest-line: VOID", output)
                    self.assertNotIn("GREEN", output)


class TheForksLockAtThePinnedCommitHoldsNoneBelowTheFix(unittest.TestCase):
    def test_the_pinned_commits_own_lock_reads_green(self):
        """The real-lock leg: the guard over the fork's `Cargo.lock` at the pinned commit."""
        patch = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))["patch"][UPSTREAM]["anki"]
        self.assertEqual(patch["git"], FORK)
        rev = patch["rev"]
        done = subprocess.run(
            ["cargo", "metadata", "--locked", "--format-version", "1"],
            cwd=REPO,
            capture_output=True,
            text=True,
            check=False,
            timeout=METADATA_BOUND_S,
        )
        self.assertEqual(done.returncode, 0, done.stderr)
        engines = [
            package
            for package in json.loads(done.stdout)["packages"]
            if package["name"] == "anki" and (package.get("source") or "").startswith(f"git+{FORK}")
        ]
        examined("anki package(s) from the fork in the resolve", engines)
        self.assertEqual(
            [package["source"] for package in engines],
            [f"git+{FORK}?rev={rev}#{rev}"],
            "the resolve takes anki from the fork at the patch entry's rev",
        )
        lock = Path(engines[0]["manifest_path"]).parents[1] / "Cargo.lock"
        self.assertTrue(lock.is_file(), f"the pinned commit's checkout holds no lock at {lock}")
        code, output = guard(lock)
        print(output, end="")
        self.assertEqual(code, 0, output)
        self.assertIn("lock-digest-line: GREEN", output)


if __name__ == "__main__":
    unittest.main()
