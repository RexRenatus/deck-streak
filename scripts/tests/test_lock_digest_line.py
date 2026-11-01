"""The workspace's lock holds no block-buffer below 0.12.1 and no digest below 0.11.0 (SPEC-565 A1;
ADR-583). GHSA-qwgh-2vcv-g2f7 affects block-buffer below 0.12.1, and block-buffer 0.10 comes only
through digest 0.10, so `scripts/lock-digest-line.py` reads every block-buffer and digest package
of `Cargo.lock` against both floors. A planted block-buffer 0.10.4 turns the check red first, so a
green reading is never one that examined nothing.

The guard's whole output is pinned too, over small locks each test writes for itself: every line it
prints and its exit code for a GREEN, a RED and a VOID lock, a pre-release at and above the floor,
each package shape it refuses, the dependents it names for each way a lock spells a dependency,
and the usage line it prints without a lock."""

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


def package(name, version, *dependencies):
    """One `[[package]]` stanza as Cargo writes it, with its dependency entries as given."""
    stanza = f'[[package]]\nname = "{name}"\nversion = "{version}"\n'
    if dependencies:
        stanza += "dependencies = [\n" + "".join(f' "{entry}",\n' for entry in dependencies) + "]\n"
    return stanza


def lock(*stanzas):
    """A lock file's text: the format line, then each stanza."""
    return "version = 4\n\n" + "\n".join(stanzas)


def judged(text):
    """One run of the guard over a lock holding `text`: the finished run and the lock's path."""
    with tempfile.TemporaryDirectory(prefix="lock-digest-line-") as scratch:
        path = Path(scratch) / "Cargo.lock"
        path.write_text(text, encoding="utf-8")
        done = subprocess.run(
            [sys.executable, str(SCRIPT), str(path)], capture_output=True, text=True, check=False
        )
    return done, path


def printed(path, counts, *lines):
    """The guard's stdout, spelled out: its count line over `path`, then each verdict line."""
    return f"lock-digest-line: {path}: {counts}\n" + "".join(
        f"lock-digest-line: {line}\n" for line in lines
    )


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


class TheGuardPrintsItsWholeVerdict(unittest.TestCase):
    def assert_printed(self, text, code, counts, *lines):
        done, path = judged(text)
        self.assertEqual(
            (done.returncode, done.stdout, done.stderr), (code, printed(path, counts, *lines), "")
        )

    def test_a_green_lock_prints_each_count_and_nothing_after_it(self):
        """Every block-buffer at 0.12.1 or later and every digest at 0.11.0 or later: one line per
        count, each ending at its count, then GREEN and exit 0."""
        self.assert_printed(
            lock(
                package("block-buffer", "0.12.1"),
                package("block-buffer", "0.12.3"),
                package("digest", "0.11.0", "block-buffer 0.12.1"),
            ),
            0,
            "3 packages; examined 2 block-buffer, 1 digest",
            "block-buffer below 0.12.1: 0 of 2",
            "digest below 0.11.0: 0 of 1",
            "GREEN",
        )

    def test_a_lock_with_no_block_buffer_reads_void_and_says_why(self):
        """Nothing examined is never green: the count line, the VOID line naming the missing
        block-buffer, and exit 2."""
        self.assert_printed(
            lock(package("digest", "0.11.0"), package("sha2", "0.11.0", "digest")),
            2,
            "2 packages; examined 0 block-buffer, 1 digest",
            "VOID: no block-buffer in the lock",
        )

    def test_a_release_candidate_of_the_fix_reads_red(self):
        """0.12.1-rc.0 sorts below its release 0.12.1, so it is below the floor."""
        self.assert_printed(
            lock(
                package("block-buffer", "0.12.1-rc.0"),
                package("digest", "0.11.0", "block-buffer"),
            ),
            1,
            "2 packages; examined 1 block-buffer, 1 digest",
            "block-buffer below 0.12.1: 1 of 1: 0.12.1-rc.0 <- digest 0.11.0",
            "digest below 0.11.0: 0 of 1",
            "RED",
        )

    def test_a_pre_release_above_the_floor_reads_green(self):
        """0.12.2-rc.1 sorts below 0.12.2 but above 0.12.1: only a pre-release of the floor's own
        release is below it."""
        self.assert_printed(
            lock(
                package("block-buffer", "0.12.2-rc.1"),
                package("digest", "0.11.0", "block-buffer"),
            ),
            0,
            "2 packages; examined 1 block-buffer, 1 digest",
            "block-buffer below 0.12.1: 0 of 1",
            "digest below 0.11.0: 0 of 1",
            "GREEN",
        )


class TheGuardRefusesAPackageItCannotRead(unittest.TestCase):
    def assert_refused(self, text):
        """The one VOID line for a lock whose packages are not each a table with a name and a
        version, and exit 2."""
        done, path = judged(text)
        refusal = (
            f"lock-digest-line: VOID: {path} does not parse: a [[package]] without a name and a "
            "version\n"
        )
        self.assertEqual((done.returncode, done.stdout, done.stderr), (2, refusal, ""))

    def test_a_package_without_a_version_reads_void(self):
        self.assert_refused(
            lock(package("block-buffer", "0.12.1"), '[[package]]\nname = "digest"\n')
        )

    def test_a_package_without_a_name_reads_void(self):
        self.assert_refused(
            lock(package("block-buffer", "0.12.1"), '[[package]]\nversion = "0.11.0"\n')
        )

    def test_a_package_that_is_not_a_table_reads_void(self):
        self.assert_refused('version = 4\npackage = ["block-buffer"]\n')

    def test_a_package_table_in_place_of_the_array_reads_void(self):
        """`[package]` holds one table, not an array of them: refused, never read as a lock with
        no packages."""
        self.assert_refused("version = 4\n\n[package]\n")


class TheGuardNamesEachVersionsDependents(unittest.TestCase):
    def assert_printed(self, text, counts, *lines):
        done, path = judged(text)
        self.assertEqual(
            (done.returncode, done.stdout, done.stderr), (1, printed(path, counts, *lines), "")
        )

    def test_a_dependency_named_alone_labels_the_one_version_it_resolves_to(self):
        """Cargo names a dependency by its name alone when the lock holds one version of it."""
        self.assert_printed(
            lock(package("block-buffer", "0.10.4"), package("digest", "0.10.7", "block-buffer")),
            "2 packages; examined 1 block-buffer, 1 digest",
            "block-buffer below 0.12.1: 1 of 1: 0.10.4 <- digest 0.10.7",
            "digest below 0.11.0: 1 of 1: 0.10.7 <- no dependent",
            "RED",
        )

    def test_a_dependency_named_with_its_version_labels_that_version(self):
        """With two versions in the lock Cargo writes `name version`, and only that version takes
        the dependent."""
        self.assert_printed(
            lock(
                package("block-buffer", "0.10.4"),
                package("block-buffer", "0.12.1"),
                package("digest", "0.10.7", "block-buffer 0.10.4"),
                package("digest", "0.11.0", "block-buffer 0.12.1"),
            ),
            "4 packages; examined 2 block-buffer, 2 digest",
            "block-buffer below 0.12.1: 1 of 2: 0.10.4 <- digest 0.10.7",
            "digest below 0.11.0: 1 of 2: 0.10.7 <- no dependent",
            "RED",
        )

    def test_a_name_with_two_versions_loses_only_its_label(self):
        """A bare name that resolves to two versions labels neither, and the entries after it are
        still read; two versions below the floor are listed apart."""
        self.assert_printed(
            lock(
                package("block-buffer", "0.10.4"),
                package("block-buffer", "0.10.5"),
                package("digest", "0.10.7", "block-buffer 0.10.5"),
                package("sha2", "0.10.8", "block-buffer", "digest"),
            ),
            "4 packages; examined 2 block-buffer, 1 digest",
            "block-buffer below 0.12.1: 2 of 2: 0.10.4 <- no dependent; 0.10.5 <- digest 0.10.7",
            "digest below 0.11.0: 1 of 1: 0.10.7 <- sha2 0.10.8",
            "RED",
        )


class TheGuardWithoutALockPrintsItsUsage(unittest.TestCase):
    def test_no_lock_argument_prints_the_usage_on_stderr_and_reads_void(self):
        done = subprocess.run(
            [sys.executable, str(SCRIPT)], capture_output=True, text=True, check=False
        )
        self.assertEqual(
            (done.returncode, done.stdout, done.stderr),
            (2, "", "usage: lock-digest-line.py LOCK\n"),
        )


if __name__ == "__main__":
    unittest.main()
