"""Every repository root a Python file computes from `Path(__file__)` reaches the repository root
(SPEC-424 A10 to A13). The guard walks every `.py` file under the gate's five roots, skips the names
`FENCED` lists before it opens anything, and reads each file's syntax tree: every assignment to a
name holding `root` or `repo`, in any case, whose value is `Path(__file__)` (or
`pathlib.Path(__file__)`), optionally `.resolve()`, then `.parents[N]` or `.parent` repeated, must
reach the repository root, or the directory `LISTED_ROOTS` names for it. A count of parents that is
off by one reads a sibling directory with no error, which is why nothing but this guard sees it."""

import pathlib
import tempfile
import unittest

from _support import examined

# The roots the gate lints (scripts/check.sh's RUFF_ROOTS), which the guard walks too.
ROOTS = ("scripts", "tools", "deploy", "agent", "crates")
# Names the walk skips before any open, matched case-blind against a file's or a directory's own
# name: the drill and readings_tree surfaces and the two deploy stand-in test modules, whose review
# is held apart (SPEC-424 R10).
FENCED = ("*drill*", "*readings_tree*", "test_deploy_standin.py", "_standin_checks.py")
# `<path>::<name>` to the repository-relative directory that root is meant to reach, and why. A
# listed root still proves its count, against the directory named here; it is empty at delivery.
LISTED_ROOTS = {}


def walk(tree, fenced=FENCED):
    """([file, ...], [skipped, ...]): every `.py` file under the five roots of `tree`, and every
    file or directory a `fenced` pattern skipped, each repository-relative and sorted."""
    return [], []


def sites(tree, files):
    """[(path, line, name, index), ...]: every computed root in `files`, each with the index into
    its path's parents that its chain reaches, or None for a chain the guard cannot resolve; a file
    that does not parse is one entry, (path, None, None, None)."""
    return []


def refusals(tree, found, listed=LISTED_ROOTS):
    """One line for each site of `found` that does not reach the repository root, or the directory
    `listed` names for it, and for each file that does not parse."""
    return []


def plant(tree, files):
    """Write each `files` path, relative to `tree`, with its source."""
    for path, source in files.items():
        target = pathlib.Path(tree) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(source, encoding="utf-8")


# Each planted file of A10: the right depth, the wrong one for each chain shape, a listed root, an
# unresolvable chain and a file that does not parse.
PLANTED_DEPTHS = {
    "scripts/a.py": "ROOT = Path(__file__).resolve().parents[1]\n",
    "scripts/b.py": "ROOT = Path(__file__).resolve().parents[2]\n",
    "scripts/tests/c.py": "REPO = Path(__file__).parent.parent\n",
    "scripts/tests/d.py": "REPO = Path(__file__).parent.parent.parent\n",
    "deploy/x/e.py": "def f():\n    repository = Path(__file__).resolve().parents[2]\n",
    "tools/f.py": "import pathlib\nREPO_ROOT: pathlib.Path = pathlib.Path(__file__).parents[0]\n",
    "crates/g.py": "WEB_ROOT = Path(__file__).parents[0]\n",
    "scripts/h.py": "ROOT = Path(__file__).resolve().parents[DEPTH]\n",
    "scripts/i.py": "ROOT = (\n",
}
WRONG_DEPTHS = [
    "scripts/b.py:1: ROOT resolves to .., not the repository root",
    "scripts/h.py:1: ROOT is a chain the guard cannot resolve",
    "scripts/i.py: does not parse",
    "scripts/tests/c.py:1: REPO resolves to scripts, not the repository root",
    "tools/f.py:2: REPO_ROOT resolves to tools, not the repository root",
]


class AComputedRootIsProvedByItsSyntaxTree(unittest.TestCase):
    def test_a_planted_root_at_the_wrong_depth_is_refused_by_place(self):
        with tempfile.TemporaryDirectory() as scratch:
            plant(scratch, PLANTED_DEPTHS)
            files, skipped = walk(scratch)
            found = sites(scratch, files)
            at_crates = {"crates/g.py::WEB_ROOT": ("crates", "a planted listed root")}
            at_tools = {"crates/g.py::WEB_ROOT": ("tools", "a planted listed root")}
            self.assertEqual(refusals(scratch, found, listed=at_crates), WRONG_DEPTHS)
            self.assertEqual(
                refusals(scratch, found, listed=at_tools),
                ["crates/g.py:1: WEB_ROOT resolves to crates, not the repository root"]
                + WRONG_DEPTHS,
            )
        self.assertEqual(files, sorted(PLANTED_DEPTHS))
        self.assertEqual(skipped, [])
        examined("planted files", files)

    def test_only_root_and_repo_assignments_are_judged_and_strings_are_not(self):
        with tempfile.TemporaryDirectory() as scratch:
            plant(
                scratch,
                {
                    "scripts/j.py": "root = Path(__file__).parents[0]\n",
                    "scripts/m.py": (
                        "HERE = Path(__file__).resolve().parents[1]\n"
                        'x = "ROOT = Path(__file__).resolve().parents[2]"\n'
                        "Path(__file__).parents[1]\n"
                        "ROOT, OTHER = Path(__file__).parents[0], 1\n"
                    ),
                },
            )
            files, _ = walk(scratch)
            found = sites(scratch, files)
            refused = refusals(scratch, found)
        self.assertEqual(found, [("scripts/j.py", 1, "root", 0)])
        self.assertEqual(
            refused, ["scripts/j.py:1: root resolves to scripts, not the repository root"]
        )
        examined("planted sites", found)

    def test_the_walk_never_opens_a_fenced_file(self):
        with tempfile.TemporaryDirectory() as scratch:
            plant(
                scratch,
                {
                    "scripts/x_held_out.py": "ROOT = (\n",
                    "scripts/k.py": "ROOT = Path(__file__).resolve().parents[1]\n",
                },
            )
            files, skipped = walk(scratch, fenced=("*held_out*",))
            found = sites(scratch, files)
            refused = refusals(scratch, found)
        self.assertEqual(skipped, ["scripts/x_held_out.py"])
        self.assertEqual(files, ["scripts/k.py"])
        self.assertEqual(found, [("scripts/k.py", 1, "ROOT", 1)])
        self.assertEqual([line for line in refused if "held_out" in line], [])
        # Matched on the lower-cased name, and a fenced directory is pruned whole, never entered.
        with tempfile.TemporaryDirectory() as scratch:
            plant(
                scratch,
                {
                    "scripts/Z_HELD_OUT.py": "ROOT = (\n",
                    "scripts/held_out_dir/m.py": "ROOT = (\n",
                    "scripts/n.py": "REPO = Path(__file__).parent.parent\n",
                },
            )
            files, skipped = walk(scratch, fenced=("*held_out*",))
            refused = refusals(scratch, sites(scratch, files))
        self.assertEqual(skipped, ["scripts/Z_HELD_OUT.py", "scripts/held_out_dir"])
        self.assertEqual(files, ["scripts/n.py"])
        self.assertEqual(refused, [])
        self.assertEqual(
            FENCED, ("*drill*", "*readings_tree*", "test_deploy_standin.py", "_standin_checks.py")
        )
        examined("planted files outside the fence", files)


if __name__ == "__main__":
    unittest.main()
