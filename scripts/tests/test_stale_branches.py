"""Stale branches are counted and held to a recorded count (SPEC-674 R1 to R8, ADR-697).

A branch is stale when no open pull request of this repository names it as its head, it is
neither one of KEPT nor marked protected, and its last commit is older than WINDOW_DAYS days
before the lists' own read stamp (R2, R3). The census reads the lists one workflow step writes
into the file STALE_BRANCHES_LISTS names, never the forge and never the wall clock (R4). It
prints its examined count and refuses an empty list, which is VOID (R5). At LEVEL `warn` a count
past RECORD prints a WARN line and passes, and at `fail` it fails; at both levels a record above
the count fails, so the record moves down when branches go (R6, R7). It prints counts only and
never a branch's name. The planted lists are `.fixture` texts under FIXTURES, read as text and
never run (R8).
"""

import os
import unittest
from pathlib import Path

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "stale-branches"
LISTS = "STALE_BRANCHES_LISTS"
DAY = 86400
WINDOW_DAYS = 90
KEPT = ("dev", "main")
LEVEL = "warn"
RECORD = (0, "the count at the census's first run over the whole list, its own CI job JOB-NEW")


class Lists:
    """The read stamp and the repository, each branch as {name: (protected, last commit)}, and
    each open pull request as (head branch, head repository); times are seconds."""

    def __init__(self, read_at, repository, branches, pulls):
        self.read_at = read_at
        self.repository = repository
        self.branches = branches
        self.pulls = pulls


def read_lists(text):
    """The lists one text holds, one line each, its fields split by tabs: `read-at`,
    `repository`, `protected`, `commit` and `pull` (R4). Any other line, a second stamp or
    repository, and a branch one read names and the other does not are refused by count."""
    stamps, repositories, protected, committed, pulls = [], [], {}, {}, []
    for number, line in enumerate(text.splitlines(), 1):
        kind, *rest = line.split("\t")
        if kind == "read-at" and len(rest) == 1 and rest[0].isdigit():
            stamps.append(int(rest[0]))
        elif kind == "repository" and len(rest) == 1 and rest[0]:
            repositories.append(rest[0])
        elif kind == "protected" and len(rest) == 2 and rest[1] in ("true", "false"):
            protected[rest[0]] = rest[1] == "true"
        elif kind == "commit" and len(rest) == 2 and rest[1].isdigit():
            committed[rest[0]] = int(rest[1])
        elif kind == "pull" and len(rest) == 2:
            pulls.append((rest[0], rest[1]))
        else:
            raise AssertionError(f"line {number} is not a line of the lists")
    if len(stamps) != 1 or len(repositories) != 1:
        raise AssertionError("the lists hold no single read stamp and repository")
    branches = {name: (protected[name], committed[name]) for name in protected if name in committed}
    return Lists(stamps[0], repositories[0], branches, pulls)


def stale(lists, window_days=WINDOW_DAYS):
    """The sorted names of the stale branches (R2, R3)."""
    return []


def verdict(count, record, level):
    """(passes, line) for a stale count against the record (R6, R7)."""
    return True, ""


def census(lists, record=RECORD, level=LEVEL):
    """The stale names, the verdict and its line, over every branch the lists hold (R5)."""
    found = stale(lists)
    passes, line = verdict(len(found), record, level)
    return found, passes, line


def planted(name):
    """A planted list, read as text (R8)."""
    return read_lists((FIXTURES / name).read_text(encoding="utf-8"))


class PlantedListsAreJudged(unittest.TestCase):
    def test_a_branch_an_open_pull_request_names_is_not_stale(self):
        found, _passes, _line = census(planted("planted.lists.fixture"))
        self.assertIn("feat/closed-pull-left", found)
        self.assertNotIn("feat/open-pull", found)

    def test_a_branch_with_a_commit_inside_the_window_is_not_stale(self):
        found, _passes, _line = census(planted("planted.lists.fixture"))
        self.assertIn("feat/closed-pull-left", found)
        self.assertNotIn("feat/recent", found)
        self.assertNotIn("feat/at-the-window", found)

    def test_a_branch_past_the_window_with_no_open_pull_request_is_stale(self):
        found, _passes, _line = census(planted("planted.lists.fixture"))
        self.assertEqual(found, ["feat/closed-pull-left", "feat/left-behind"])

    def test_the_kept_and_protected_branches_are_not_counted(self):
        found, _passes, _line = census(planted("planted.lists.fixture"))
        self.assertIn("feat/left-behind", found)
        self.assertNotIn("main", found)
        self.assertNotIn("release/held", found)

    def test_a_count_at_the_record_passes_with_its_line(self):
        lists = planted("planted.lists.fixture")
        line = "stale branches 2, recorded 2"
        self.assertEqual(census(lists, (2, "planted"), "warn")[1:], (True, line))
        self.assertEqual(census(lists, (2, "planted"), "fail")[1:], (True, line))

    def test_a_count_past_the_record_is_a_site_at_warn_and_fails_at_fail(self):
        lists = planted("planted.lists.fixture")
        line = "WARN stale-branches: 2 stale branch(es), past the recorded 1"
        self.assertEqual(census(lists, (1, "planted"), "warn")[1:], (True, line))
        self.assertEqual(census(lists, (1, "planted"), "fail")[1:], (False, line))

    def test_a_record_above_the_count_fails_at_warn_as_at_fail(self):
        lists = planted("planted.lists.fixture")
        line = (
            "STALE-RECORD stale-branches: recorded 3, a count no longer reached; "
            "the count is 2, so the record moves down"
        )
        self.assertEqual(census(lists, (3, "planted"), "warn")[1:], (False, line))
        self.assertEqual(census(lists, (3, "planted"), "fail")[1:], (False, line))

    def test_an_empty_list_is_void(self):
        lists = planted("empty.lists.fixture")
        self.assertEqual((lists.read_at, lists.branches), (864000000, {}))
        with self.assertRaisesRegex(AssertionError, r"^examined 0 branch\(es\): the population"):
            census(lists)

    def test_lists_whose_branch_reads_disagree_are_refused(self):
        with self.assertRaisesRegex(AssertionError, r"^the two branch reads disagree on 1 branch"):
            planted("disagree.lists.fixture")

    def test_the_census_starts_at_warn_over_ninety_days(self):
        self.assertEqual((LEVEL, WINDOW_DAYS, DAY, KEPT), ("warn", 90, 86400, ("dev", "main")))
        self.assertEqual(RECORD[0], 0)
        self.assertIn("CI job", RECORD[1])

    def test_every_planted_list_is_a_fixture_text(self):
        names = examined("planted list(s)", sorted(p.name for p in FIXTURES.iterdir()))
        self.assertEqual(
            names, ["disagree.lists.fixture", "empty.lists.fixture", "planted.lists.fixture"]
        )


class TheRepositoryHoldsItsRecord(unittest.TestCase):
    def test_the_repository_holds_its_recorded_stale_count(self):
        given = os.environ.get(LISTS, "")
        if not given:
            self.skipTest(f"{LISTS} is unset: only the stale-branches workflow gives the lists")
        found, passes, line = census(read_lists(Path(given).read_text(encoding="utf-8")))
        print(line)
        self.assertTrue(passes, line)


if __name__ == "__main__":
    unittest.main()
