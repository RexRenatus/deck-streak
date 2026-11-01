"""The web lock resolves no minimatch below 11.0.1 (#1062).

Every minimatch release below 11.0.1 is open to GHSA-g3hv-75wg-m249 (moderate, a denial of service
through globparts amplification), and the web audit fails on it. The guard reads `pnpm-lock.yaml`
as text: each package key `minimatch@<version>` and each dependency line `minimatch: <version>` is
a site, every site is judged, and a version that does not parse is refused, never passed. A
release candidate of 11.0.1 sorts below it.

The planted leg holds the reader to its own fixtures, synthetic lock text in this file: a lock at
minimatch 10.2.6 is refused at each of its three sites, a release candidate of 11.0.1 is refused,
11.0.1 and 11.0.3 pass, a version that does not parse is refused, and a lock that names no
minimatch examines nothing and is refused.
"""

import re
import unittest

from _support import REPO, examined

LOCK = REPO / "pnpm-lock.yaml"
FLOOR = (11, 0, 1)
#: A package key, `  minimatch@10.2.6:` or `  'minimatch@10.2.6':`, in `packages` or `snapshots`.
KEY = re.compile(r"^  '?minimatch@(?P<version>[^'(:\s]+)")
#: A dependency line, `      minimatch: 10.2.6`, in a snapshot's or an importer's dependencies.
DEPENDENCY = re.compile(r"^ {4,}'?minimatch'?: '?(?P<version>[^'(\s]+)")
SEMVER = re.compile(r"^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")

PLANTED_10_2_6 = """\
packages:

  minimatch@10.2.6:
    resolution: {integrity: planted}

snapshots:

  '@stryker-mutator/core@10.1.1':
    dependencies:
      minimatch: 10.2.6

  minimatch@10.2.6:
    dependencies:
      brace-expansion: 5.0.12
"""
PLANTED_RELEASE_CANDIDATE = """\
packages:

  minimatch@11.0.1-rc.0:
    resolution: {integrity: planted}
"""
PLANTED_FIXED = """\
packages:

  minimatch@11.0.1:
    resolution: {integrity: planted}

  minimatch@11.0.3:
    resolution: {integrity: planted}
"""
PLANTED_UNPARSED = """\
snapshots:

  '@stryker-mutator/core@10.1.1':
    dependencies:
      minimatch: link:planted
"""
PLANTED_NONE = """\
packages:

  brace-expansion@5.0.12:
    resolution: {integrity: planted}
"""


def sites(text):
    """Each minimatch site of a lock's text, as (line number, version text)."""
    found = []
    for number, line in enumerate(text.splitlines(), start=1):
        match = KEY.match(line) or DEPENDENCY.match(line)
        if match:
            found.append((number, match.group("version")))
    return found


def below_floor(version):
    """Whether `version` is below 11.0.1; one that does not parse is below, since it cannot be
    judged."""
    match = SEMVER.match(version)
    if not match:
        return True
    release = tuple(int(part) for part in match.groups()[:3])
    candidate = "-" in version.split("+", 1)[0]
    return release < FLOOR or (release == FLOOR and candidate)


def refused(found):
    """The sites of `found` whose version is below 11.0.1."""
    return [(number, version) for number, version in found if below_floor(version)]


class TheWebLockHoldsNoMinimatchBelowTheFix(unittest.TestCase):
    def test_every_minimatch_the_lock_resolves_is_11_0_1_or_later(self):
        text = LOCK.read_text(encoding="utf-8")
        found = examined("minimatch site(s) in pnpm-lock.yaml", sites(text))
        below = refused(found)
        self.assertEqual(
            below,
            [],
            f"minimatch below 11.0.1 in pnpm-lock.yaml: {len(below)} of {len(found)}: "
            + ", ".join(f"{version} at line {number}" for number, version in below),
        )


class TheReaderRefusesItsPlantedLocks(unittest.TestCase):
    def test_a_planted_minimatch_10_2_6_is_refused_at_each_of_its_three_sites(self):
        found = sites(PLANTED_10_2_6)
        self.assertEqual(found, [(3, "10.2.6"), (10, "10.2.6"), (12, "10.2.6")])
        self.assertEqual(refused(found), found)

    def test_a_release_candidate_of_11_0_1_sorts_below_it_and_is_refused(self):
        self.assertEqual(refused(sites(PLANTED_RELEASE_CANDIDATE)), [(3, "11.0.1-rc.0")])

    def test_11_0_1_and_11_0_3_pass(self):
        found = sites(PLANTED_FIXED)
        self.assertEqual(found, [(3, "11.0.1"), (6, "11.0.3")])
        self.assertEqual(refused(found), [])

    def test_a_version_the_reader_cannot_parse_is_refused(self):
        self.assertEqual(refused(sites(PLANTED_UNPARSED)), [(5, "link:planted")])

    def test_a_lock_that_names_no_minimatch_examines_nothing_and_is_refused(self):
        with self.assertRaisesRegex(AssertionError, "examined 0 minimatch site"):
            examined("minimatch site(s) in the planted lock", sites(PLANTED_NONE))


if __name__ == "__main__":
    unittest.main()
