"""Tests that kill the carry module's listed mutants in process (SPEC-493).

`scripts/mutation_carry.py` is loaded by the one `load()` below, afresh for each test, so its own
lines run inside the test that judges them. Each class holds one table-driven method per class of
the kill table, and `scripts/mutation-python.json` maps this module first among the carry module's
test modules, so under the failfast legs a kill ends a mutant's run before the slower child-process
tests start. Every case is labelled by its mutant's `<line>:<col> <operator>`, runs under its own
`subTest`, and asserts a whole value the mutant changes. A call a mutant makes raise is read
through `attempt`, so the case fails by assertion and not by an exception.
"""

import collections
import importlib.util
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

CARRY = REPO / "scripts" / "mutation_carry.py"


def load():
    spec = importlib.util.spec_from_file_location("mutation_carry_under_kill", CARRY)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def attempt(thunk):
    """What `thunk()` returns, or the name and text of what it raises."""
    try:
        return thunk()
    except Exception as error:
        return ("raised", type(error).__name__, str(error))


class Base(unittest.TestCase):
    """A fresh carry module and a scratch directory under TMPDIR for each test."""

    def setUp(self):
        self.mc = load()
        self.scratch = Path(tempfile.mkdtemp(prefix="carry-kills-"))
        self.addCleanup(shutil.rmtree, self.scratch)

    def judge(self, table):
        """Each case's (label, thunk, want): the thunk's value, or what it raises, is `want`."""
        for label, thunk, want in table:
            with self.subTest(case=label):
                self.assertEqual(attempt(thunk), want)


# The splits of one walk up, counted across the parts each split returns.
WALKS = collections.Counter()


class Walk(str):
    """A path whose every split is counted in WALKS, each part a Walk too; a ninth split is
    refused, so a walk up that never ends fails at its ninth step and not at a leg's bound."""

    def rsplit(self, sep=None, maxsplit=-1):
        WALKS["rsplit"] += 1
        if WALKS["rsplit"] > 8:
            raise AssertionError(f"the walk up split {WALKS['rsplit']} times: it never ends")
        return [Walk(part) for part in str.rsplit(self, sep, maxsplit)]


class AncestorsEnd(Base):
    """K01: ancestors walks up to the root in one split per directory."""

    def test_each_walk_up_ends_at_the_root_in_a_bounded_count(self):
        def walk(path):
            WALKS.clear()
            return self.mc.ancestors(Walk(path)), WALKS["rsplit"]

        table = [
            (
                '224:11 replace "/" with ""',
                lambda: walk("scripts/tests/x.py"),
                (["scripts/tests", "scripts", "."], 2),
            ),
        ]
        self.judge(table)
        examined("K01 cases", table)


if __name__ == "__main__":
    unittest.main()
