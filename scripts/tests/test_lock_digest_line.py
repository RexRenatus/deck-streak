"""The workspace's lock holds no block-buffer below 0.12.1 and no digest below 0.11.0 (SPEC-565 A1;
ADR-583). GHSA-qwgh-2vcv-g2f7 affects block-buffer below 0.12.1, and block-buffer 0.10 comes only
through digest 0.10, so `scripts/lock-digest-line.py` reads every block-buffer and digest package
of `Cargo.lock` against both floors. A planted block-buffer 0.10.4 turns the check red first, so a
green reading is never one that examined nothing."""

import re
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "lock-digest-line.py"
LOCKFILE = REPO / "Cargo.lock"
BELOW = re.compile(r"block-buffer below 0\.12\.1: (\d+) of (\d+)")
#: One extra stanza: the version GHSA-qwgh-2vcv-g2f7 flagged in this lock before the move.
PLANTED = (
    '\n[[package]]\nname = "block-buffer"\nversion = "0.10.4"\n'
    'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
    'dependencies = [\n "generic-array",\n]\n'
)


def guard(lock):
    """One run of the guard over `lock`: its exit code and its printed lines."""
    done = subprocess.run(
        [sys.executable, str(SCRIPT), str(lock)], capture_output=True, text=True, check=False
    )
    return done.returncode, done.stdout + done.stderr


def below_the_fix(output):
    """The guard's block-buffer count: how many are below 0.12.1, of how many examined."""
    found = BELOW.search(output)
    if found is None:
        raise AssertionError(f"the guard printed no block-buffer count: {output}")
    return int(found.group(1)), int(found.group(2))


class TheLockHoldsNoBlockBufferBelowTheFix(unittest.TestCase):
    def test_a_planted_block_buffer_0_10_4_turns_the_check_red(self):
        """The control: the workspace's lock with one block-buffer 0.10.4 stanza added reads RED,
        with one more block-buffer below 0.12.1 than the lock itself."""
        _, unplanted = guard(LOCKFILE)
        with tempfile.TemporaryDirectory(prefix="lock-digest-line-") as scratch:
            planted = Path(scratch) / "Cargo.lock"
            planted.write_text(LOCKFILE.read_text(encoding="utf-8") + PLANTED, encoding="utf-8")
            code, output = guard(planted)
        self.assertEqual(code, 1, output)
        low, total = below_the_fix(unplanted)
        self.assertEqual(below_the_fix(output), (low + 1, total + 1), output)
        self.assertIn("0.10.4 <- ", output)
        self.assertIn("lock-digest-line: RED", output)

    def test_the_lock_holds_no_block_buffer_below_0_12_1_and_no_digest_below_0_11_0(self):
        """A1: every block-buffer is 0.12.1 or later, and every digest 0.11.0 or later."""
        packages = tomllib.loads(LOCKFILE.read_text(encoding="utf-8")).get("package", [])
        examined(
            "block-buffer and digest package(s) in Cargo.lock",
            [package for package in packages if package["name"] in ("block-buffer", "digest")],
        )
        code, output = guard(LOCKFILE)
        self.assertEqual(code, 0, output)
        self.assertIn("block-buffer below 0.12.1: 0 of", output)
        self.assertIn("digest below 0.11.0: 0 of", output)
        self.assertIn("lock-digest-line: GREEN", output)


if __name__ == "__main__":
    unittest.main()
