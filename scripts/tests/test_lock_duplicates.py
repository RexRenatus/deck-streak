"""Each crate name the workspace's lock resolves at more than one version, or from more than one
source, is listed with its reason or reported (SPEC-668 R1 to R9; ADR-691 D1 to D4).

`deny.toml` sets cargo-deny's `multiple-versions = "warn"`, so its bans check prints a warning for
each duplicated crate and refuses none. This census keeps the same rule as a listing:

- the population is every `[[package]]` entry of the root `Cargo.lock`, read by `tomllib` for its
  name, version and source, and an empty population is VOID, never a pass (R1);
- a name is a site when its entries hold two or more versions, or one version from two or more
  sources (R2); a dependency's kind is not in the lock, so a version pulled only by a dev or a
  build dependency stays in the population (R3);
- a dependency string is written `name`, `name version` or `name version (source)`, with a git
  source's `#<commit>` dropped, as `test_xp_crate_graph.py` reads it; each version's dependents are
  named from those strings, and one that names no one entry is UNDECIDED and fails (R4);
- `LISTED` holds each site of the lock at the delivery's base with the reason each version is
  there (R5); an unlisted site prints `WARN duplicate-crate-version <name>` and passes while
  `LEVEL` is `warn`, and fails at `fail` (R6); a listed name that is no site fails as `STALE` at
  every level (R7).

Moving `LEVEL` to `fail` is its own delivery, under ADR-450 D6's promotion rule (R8).
"""

import collections
import re
import tomllib
import unittest

from _support import REPO

LOCKFILE = REPO / "Cargo.lock"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "lock-duplicates"
RULE = "duplicate-crate-version"
LEVELS = ("warn", "fail")
#: The census starts at warn (R8); moving it to fail is its own delivery.
LEVEL = "warn"
DEPENDENCY = re.compile(r"^(?P<name>\S+)(?: (?P<version>[^\s(]+))?(?: \((?P<source>[^)]+)\))?$")

#: Each name the lock holds more than once at the delivery's base, with each version's dependent
#: and the workspace crate it is reached from (R5).
LISTED = {}


def packages(text):
    """Every `[[package]]` entry of a lock's text, as `tomllib` reads it."""
    return tomllib.loads(text).get("package", [])


def referenced(source):
    """A package's source as a dependency string writes it: a git source loses its `#<commit>`."""
    return source.split("#", 1)[0] if source is not None else None


def kind(source):
    """A source by its kind: a workspace path has none."""
    if source is None:
        return "path"
    return "git" if source.startswith("git+") else "registry"


def key(entry):
    """One entry of the lock: its name, version and source."""
    return entry["name"], entry.get("version"), entry.get("source")


def dependents(entries):
    """{entry key: names of the entries that depend on it}, and each dependency string that names
    no one entry (R4)."""
    by_name = collections.defaultdict(list)
    for entry in entries:
        by_name[entry["name"]].append(entry)
    found, undecided = collections.defaultdict(set), []
    for entry in entries:
        for dependency in entry.get("dependencies", []):
            spelled = DEPENDENCY.match(dependency)
            matches = [
                other
                for other in (by_name.get(spelled["name"], []) if spelled else [])
                if spelled["version"] in (None, other.get("version"))
                and spelled["source"] in (None, referenced(other.get("source")))
            ]
            if len(matches) == 1:
                found[key(matches[0])].add(entry["name"])
            else:
                undecided.append(f"{entry['name']}: {dependency}")
    return found, undecided


def form(held):
    """Why the entries of one name are a site: two versions, two sources, or None (R2)."""
    if len({entry.get("version") for entry in held}) >= 2:
        return "versions"
    if len({entry.get("source") for entry in held}) >= 2:
        return "sources"
    return None


def verdict(text, listed, level):
    """The census's printed lines and its failures over one lock's text, at `level`."""
    if level not in LEVELS:
        raise AssertionError(f"{level!r} is not a level: {LEVELS}")
    return ["examined 0 duplicated name(s)"], []


class TheWorkspaceLockHoldsOnlyListedDuplicates(unittest.TestCase):
    def test_every_duplicated_name_in_the_lock_is_listed_or_warned(self):
        """The real lock, at `LEVEL`: every site is listed or warned, and no listing entry is
        stale."""
        lines, failures = verdict(LOCKFILE.read_text(encoding="utf-8"), LISTED, LEVEL)
        print("\n".join(lines))
        self.assertEqual(failures, [])

    def test_the_census_starts_at_warn(self):
        self.assertEqual(LEVEL, "warn")


class TheCensusJudgesItsPlantedLocks(unittest.TestCase):
    """Each planted lock is read as text from `fixtures/lock-duplicates/`; none is run."""

    def judge(self, name, listed, level):
        return verdict((FIXTURES / name).read_text(encoding="utf-8"), listed, level)

    def test_a_name_at_two_versions_warns_at_warn_and_fails_at_fail(self):
        warned = (
            f"WARN {RULE} alpha (versions): 1.0.0 registry by planted-root; 2.0.0 registry by beta"
        )
        lines, failures = self.judge("site.lock.fixture", {}, "warn")
        self.assertIn(warned, lines)
        self.assertEqual(failures, [])
        lines, failures = self.judge("site.lock.fixture", {}, "fail")
        self.assertEqual(failures, [warned])

    def test_a_name_at_one_version_is_clean(self):
        for level in LEVELS:
            with self.subTest(level=level):
                lines, failures = self.judge("clean.lock.fixture", {}, level)
                self.assertEqual(lines, ["examined 0 duplicated name(s)"])
                self.assertEqual(failures, [])

    def test_a_listed_duplicate_is_clean_at_both_levels(self):
        listed = {"gamma": "planted: both versions are held on purpose"}
        shown = (
            f"LISTED {RULE} gamma: 0.1.0 registry by planted-root; 0.2.0 registry by planted-root"
        )
        for level in LEVELS:
            with self.subTest(level=level):
                lines, failures = self.judge("listed.lock.fixture", listed, level)
                self.assertEqual(lines, ["examined 1 duplicated name(s)", shown])
                self.assertEqual(failures, [])

    def test_one_name_from_two_sources_is_a_site(self):
        warned = (
            f"WARN {RULE} delta (sources): 1.0.0 registry by planted-root; "
            "1.0.0 git by planted-root"
        )
        lines, failures = self.judge("two-sources.lock.fixture", {}, "warn")
        self.assertIn(warned, lines)
        self.assertEqual(failures, [])
        lines, failures = self.judge("two-sources.lock.fixture", {}, "fail")
        self.assertEqual(failures, [warned])

    def test_a_listed_name_no_longer_duplicated_is_stale_at_both_levels(self):
        for level in LEVELS:
            with self.subTest(level=level):
                _lines, failures = self.judge("stale.lock.fixture", {"alpha": "planted"}, level)
                self.assertEqual(failures, [f"STALE {RULE} alpha"])

    def test_an_empty_lock_is_void(self):
        with self.assertRaises(AssertionError) as raised:
            self.judge("empty.lock.fixture", {}, "warn")
        self.assertIn("examined 0 package(s) in the lock", str(raised.exception))

    def test_a_dependency_that_names_no_one_entry_is_undecided_at_both_levels(self):
        for level in LEVELS:
            with self.subTest(level=level):
                _lines, failures = self.judge("undecided.lock.fixture", {"alpha": "planted"}, level)
                self.assertEqual(failures, [f"UNDECIDED {RULE} planted-root: alpha"])


if __name__ == "__main__":
    unittest.main()
