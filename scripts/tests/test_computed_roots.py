"""Every repository root a Python file computes from `Path(__file__)` reaches the repository root
(SPEC-424 A10 to A13). The guard walks every `.py` file under the gate's five roots, skips the names
`FENCED` lists before it opens anything, and reads each file's syntax tree: every assignment to a
name holding `root` or `repo`, in any case, whose value is `Path(__file__)` (or
`pathlib.Path(__file__)`), optionally `.resolve()`, then `.parents[N]` or `.parent` repeated, must
reach the repository root, or the directory `LISTED_ROOTS` names for it. A count of parents that is
off by one reads a sibling directory with no error, which is why nothing but this guard sees it."""

import ast
import fnmatch
import os
import pathlib
import tempfile
import unittest

from _support import REPO, examined

# The roots the gate lints (scripts/check.sh's RUFF_ROOTS), which the guard walks too.
ROOTS = ("scripts", "tools", "deploy", "agent", "crates")
# Names the walk skips before any open, matched case-blind against a file's or a directory's own
# name: the drill and readings_tree surfaces and the two deploy stand-in test modules, whose review
# is held apart (SPEC-424 R10).
FENCED = ("*drill*", "*readings_tree*", "test_deploy_standin.py", "_standin_checks.py")
# `<path>::<name>` to the repository-relative directory that root is meant to reach, and why. A
# listed root still proves its count, against the directory named here; it is empty at delivery.
LISTED_ROOTS = {}


def fenced_name(name, fenced):
    """Whether a file's or a directory's own name matches a `fenced` pattern, case-blind."""
    return any(fnmatch.fnmatchcase(name.lower(), pattern) for pattern in fenced)


def walk(tree, fenced=FENCED):
    """([file, ...], [skipped, ...]): every `.py` file under the five roots of `tree`, and every
    file or directory a `fenced` pattern skipped, each repository-relative and sorted. A fenced
    directory is pruned before it is entered, and a fenced file is held out before it is opened."""
    tree = pathlib.Path(tree)
    files, skipped = [], []
    for root in ROOTS:
        for here, dirs, names in os.walk(tree / root):
            place = pathlib.Path(here).relative_to(tree)
            held = sorted(name for name in dirs if fenced_name(name, fenced))
            skipped += [(place / name).as_posix() for name in held]
            dirs[:] = sorted(name for name in dirs if name not in held)
            for name in sorted(names):
                if not name.endswith(".py"):
                    continue
                path = (place / name).as_posix()
                (skipped if fenced_name(name, fenced) else files).append(path)
    return sorted(files), sorted(skipped)


def path_of_file(node):
    """Whether `node` is the call `Path(__file__)` or `pathlib.Path(__file__)`."""
    if not isinstance(node, ast.Call):
        return False
    func = node.func
    named = (isinstance(func, ast.Name) and func.id == "Path") or (
        isinstance(func, ast.Attribute)
        and func.attr == "Path"
        and isinstance(func.value, ast.Name)
        and func.value.id == "pathlib"
    )
    return (
        named
        and not node.keywords
        and len(node.args) == 1
        and isinstance(node.args[0], ast.Name)
        and node.args[0].id == "__file__"
    )


def chain(value):
    """(starts, index): whether `value`'s receiver chain starts at `Path(__file__)`, and the index
    into the file's parents its tail reaches, or None for a tail the guard cannot resolve. The tail
    it resolves is an optional `.resolve()`, then `.parents[N]` with an integer literal or `.parent`
    repeated `k` times, which is index `k - 1`."""
    tail, node = [], value
    while not path_of_file(node):
        if isinstance(node, ast.Attribute):
            tail.append(node.attr)
            node = node.value
        elif isinstance(node, ast.Subscript):
            tail.append(node.slice)
            node = node.value
        elif isinstance(node, ast.Call):
            tail.append(node)
            node = node.func
        else:
            return False, None
    tail.reverse()
    if (
        len(tail) >= 2
        and tail[0] == "resolve"
        and isinstance(tail[1], ast.Call)
        and not tail[1].args
        and not tail[1].keywords
    ):
        tail = tail[2:]
    if len(tail) == 2 and tail[0] == "parents" and isinstance(tail[1], ast.Constant):
        depth = tail[1].value
        if isinstance(depth, int) and not isinstance(depth, bool) and depth >= 0:
            return True, depth
        return True, None
    if tail and all(step == "parent" for step in tail):
        return True, len(tail) - 1
    return True, None


def root_named(name):
    """Whether a name holds `root` or `repo`, in any case."""
    return "root" in name.lower() or "repo" in name.lower()


def sites(tree, files):
    """[(path, line, name, index), ...]: every computed root in `files`, each with the index into
    its path's parents that its chain reaches, or None for a chain the guard cannot resolve; a file
    that does not parse is one entry, (path, None, None, None). A site is an assignment, at any
    scope, plain or annotated, whose one target is a root-named name and whose value's receiver
    chain starts at `Path(__file__)`; a string, an expression statement and a tuple target are
    not."""
    found = []
    for path in files:
        source = (pathlib.Path(tree) / path).read_bytes()
        try:
            module = ast.parse(source, filename=path)
        except (SyntaxError, ValueError):
            found.append((path, None, None, None))
            continue
        here = []
        for node in ast.walk(module):
            if isinstance(node, ast.Assign) and len(node.targets) == 1:
                target, value = node.targets[0], node.value
            elif isinstance(node, ast.AnnAssign) and node.value is not None:
                target, value = node.target, node.value
            else:
                continue
            if not isinstance(target, ast.Name) or not root_named(target.id):
                continue
            starts, index = chain(value)
            if starts:
                here.append((path, node.lineno, target.id, index))
        found += sorted(here)
    return found


def refusals(tree, found, listed=LISTED_ROOTS):
    """One line for each site of `found` that does not reach the repository root, or the directory
    `listed` names for it, and for each file that does not parse."""
    refused = []
    for path, line, name, index in found:
        if line is None:
            refused.append(f"{path}: does not parse")
            continue
        if index is None:
            refused.append(f"{path}:{line}: {name} is a chain the guard cannot resolve")
            continue
        parents = pathlib.PurePosixPath(path).parents
        reached = parents[index].as_posix() if index < len(parents) else ".."
        meant = listed.get(f"{path}::{name}", (".", "the repository root"))[0]
        if reached != meant:
            refused.append(f"{path}:{line}: {name} resolves to {reached}, not the repository root")
    return refused


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


class EveryComputedRootInTheTreeResolves(unittest.TestCase):
    def test_every_computed_root_in_the_tree_resolves(self):
        files, skipped = walk(REPO)
        found = sites(REPO, files)
        examined("python files under the five roots", files)
        examined("computed repository roots", [site for site in found if site[1] is not None])
        self.assertEqual(refusals(REPO, found), [])
        self.assertEqual(LISTED_ROOTS, {})
        print(f"skipped {len(skipped)} fenced name(s), none opened")
        for name in skipped:
            with self.subTest(skipped=name):
                self.assertTrue(fenced_name(pathlib.PurePosixPath(name).name, FENCED), name)


if __name__ == "__main__":
    unittest.main()
