"""SPEC-493 A2: a population file carries only when no path, probe or listing of its read set
changed (R4, R6, R7, R8).

Each test builds a census tree from `fixtures/mutation-carry/census-trees.json` and runs
`scripts/mutation_carry.py` as the interpreter's child: `read-set` prints a file's read set at a
tree, and `holds` judges it against the base run's and this run's diffs, written here section by
section as `git diff` writes them. No test imports a script.
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

CARRY = REPO / "scripts" / "mutation_carry.py"
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "mutation-carry"


def child(*args):
    done = subprocess.run(
        [sys.executable, str(CARRY), *map(str, args)],
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
    )
    return done.returncode, done.stdout + done.stderr


def blob(text):
    data = text.encode()
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def section(path, before=None, after=None, binary=False):
    """One file's section of a `git diff`: an added file when `before` is None, a deleted one
    when `after` is None, else a modified one."""
    lines = [f"diff --git a/{path} b/{path}"]
    if before is None:
        lines.append("new file mode 100644")
    elif after is None:
        lines.append("deleted file mode 100644")
    old = blob(before)[:7] if before is not None else "0000000"
    new = blob(after)[:7] if after is not None else "0000000"
    mode = " 100644" if before is not None and after is not None else ""
    lines.append(f"index {old}..{new}{mode}")
    if binary:
        lines.append(f"Binary files a/{path} and b/{path} differ")
        return "\n".join(lines) + "\n"
    if before is not None and after is not None:
        lines += [f"--- a/{path}", f"+++ b/{path}"]
    old_lines = before.splitlines() if before else []
    new_lines = after.splitlines() if after else []
    lines.append(f"@@ -{1 if old_lines else 0},{len(old_lines)} +{1 if new_lines else 0},"
                 f"{len(new_lines)} @@")  # fmt: skip
    lines += [f"-{line}" for line in old_lines] + [f"+{line}" for line in new_lines]
    return "\n".join(lines) + "\n"


class Census(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="census-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.trees = json.loads((FIXTURES / "census-trees.json").read_text(encoding="utf-8"))

    def tree(self, name, edits=None, at=None):
        """The census tree `name` written under the scratch, with `edits` (None deletes)."""
        tree = self.trees[name]
        root = self.tmp / (at or name)
        files = {**tree["files"], "scripts/mutation-python.json": json.dumps(tree["map"])}
        files.update(edits or {})
        for path, text in files.items():
            if text is not None:
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                (root / path).write_text(text, encoding="utf-8")
        return root, tree["file"]

    def read_set(self, root, file):
        code, text = child("read-set", "--root", root, "--file", file)
        self.assertEqual(code, 0, text)
        return json.loads(text)

    def holds(self, root, file, base, head):
        """`holds` over a base run's diff and this run's, each a list of sections."""
        where = self.tmp / f"diffs-{len(list(self.tmp.iterdir()))}"
        where.mkdir()
        (where / "base.diff").write_text("".join(base), encoding="utf-8")
        (where / "head.diff").write_text("".join(head), encoding="utf-8")
        code, text = child(
            "holds",
            "--root",
            root,
            "--file",
            file,
            "--base-diff",
            where / "base.diff",
            "--head-diff",
            where / "head.diff",
        )
        self.assertEqual(code, 0, text)
        return json.loads(text)


def own_file_only(file, base, head):
    """The rejected rule: carry when the file's own section is the same in both diffs."""

    def own(sections):
        return [s for s in sections if s.startswith(f"diff --git a/{file} b/{file}\n")]

    return own(base) == own(head)


class ReadSet(Census):
    """A2: the read set holds the closure, every probe, every constant path and every listing,
    and any construct outside R7's allowlist makes it the whole tree."""

    def test_a_bounded_file_holds_its_closure_probes_and_constant_paths(self):
        root, file = self.tree("bounded")
        found = self.read_set(root, file)
        self.assertFalse(found["whole"], found["why"])
        self.assertEqual(
            found["members"], ["scripts/helper.py", "scripts/m.py", "scripts/tests/test_m.py"]
        )
        self.assertEqual(found["entry"], self.trees["bounded"]["map"][file])
        for kind, path in (
            ("paths", "scripts/tests/helper.py"),
            ("trees", "scripts/tests/helper"),
            ("globs", "scripts/tests/helper.*"),
            ("paths", "scripts/helper.py"),
            ("paths", "scripts/tests/json.py"),
            ("trees", "scripts/data/table.json"),
            ("trees", "scripts/tests/fixtures/m"),
            ("trees", "scripts/tests/fixtures/absent.json"),
            ("listings", "scripts/tests/fixtures/m"),
            ("listings", "scripts/tests/fixtures"),
            ("listings", "scripts/tests"),
            ("listings", "scripts"),
            ("listings", "."),
        ):
            with self.subTest(kind=kind, path=path):
                self.assertIn(path, found[kind])
        self.assertNotIn("docs/other.md", found["paths"] + found["trees"])
        examined("read set entries", found["members"] + found["trees"] + found["listings"])

    def test_each_construct_outside_the_allowlist_makes_the_read_set_the_whole_tree(self):
        test = self.trees["bounded"]["files"]["scripts/tests/test_m.py"]
        tail = "\n\ndef probe(name):\n    {}\n"
        cases = {
            "an environment read": "return os.environ.get('HOME')",
            "a getenv": "return os.getenv(name)",
            "a process other than the interpreter": "return subprocess.run(['git', 'status'])",
            "a shell": "return os.system('true')",
            "a pipe": "return os.popen('ls')",
            "an exec of a process": "return os.execv(name, [name])",
            "a read of a path held in data": "return open(name).read()",
            "a listing of a path held in data": "return sorted(Path(name).iterdir())",
            "the working directory": "return sorted(Path.cwd().iterdir())",
            "the working directory by os": "return os.listdir(os.getcwd())",
            "the repository root passed to a call": "return sorted(HERE.parents[1].iterdir())",
            "an eval": "return eval(name)",
            "an import by a name held in data": "return __import__(name)",
            "a sys.path entry held in data": "sys.path.insert(0, name)",
            "an import of ctypes": "import ctypes",
            "an import of sqlite3": "import sqlite3",
            "a member that does not parse": "return (",
        }
        imports = "import os\nimport subprocess\n"
        for index, (case, line) in enumerate(cases.items()):
            with self.subTest(case=case):
                text = test.replace("import sys\n", imports + "import sys\n") + tail.format(line)
                root, file = self.tree(
                    "bounded", {"scripts/tests/test_m.py": text}, at=f"case-{index}"
                )
                found = self.read_set(root, file)
                self.assertTrue(found["whole"], case)
                self.assertIn("scripts/tests/test_m.py", found["why"])
        allowed = {
            "an interpreter child on a constant script": (
                "return subprocess.run([sys.executable, str(HERE / 'tool.py')], check=False)",
                "scripts/tests/tool.py",
            ),
            "a loader on a constant path": (
                "return importlib.util.spec_from_file_location('loaded', HERE / 'loaded.py')",
                "scripts/tests/loaded.py",
            ),
            "a temporary directory": (
                "with tempfile.TemporaryDirectory() as scratch:\n"
                "        return (Path(scratch) / 'x').write_text('x')",
                None,
            ),
        }
        for index, (case, (line, member)) in enumerate(allowed.items()):
            with self.subTest(case=case):
                more = "import importlib.util\nimport subprocess\nimport tempfile\n"
                text = test.replace("import sys\n", more + "import sys\n") + tail.format(line)
                edits = {
                    "scripts/tests/test_m.py": text,
                    "scripts/tests/tool.py": '"""A fixture tool."""\n',
                    "scripts/tests/loaded.py": '"""A fixture loaded module."""\n',
                }
                root, file = self.tree("bounded", edits, at=f"allowed-{index}")
                found = self.read_set(root, file)
                self.assertFalse(found["whole"], f"{case}: {found['why']}")
                if member:
                    self.assertIn(member, found["members"])
        examined("constructs", [*cases, *allowed])

    def test_holds_judges_every_path_member_tree_and_listing(self):
        root, file = self.tree("bounded")
        files = self.trees["bounded"]["files"]
        helper = files["scripts/helper.py"]
        cases = {
            "an unrelated file changed on both sides": (
                [section("docs/other.md", "# Other\n", "# One\n")],
                [section("docs/other.md", "# Other\n", "# Two\n")],
                True,
            ),
            "a member's section equal on both sides": (
                [section("scripts/helper.py", helper, helper + "# x\n")],
                [section("scripts/helper.py", helper, helper + "# x\n")],
                True,
            ),
            "a member changed": ([], [section("scripts/helper.py", helper, helper + "# x\n")],
                                 False),
            "a data file read by a constant path": (
                [], [section("scripts/data/table.json", "{}\n", "[]\n")], False),
            "a file added to a walked directory": (
                [], [section("scripts/tests/fixtures/m/two.txt", None, "two\n")], False),
            "a file removed from a walked directory": (
                [], [section("scripts/tests/fixtures/m/one.txt", "one\n", None)], False),
            "an unrelated file added on one side only": (
                [], [section("notes/new.txt", None, "new\n")], False),
            "nothing changed": ([], [], True),
        }  # fmt: skip
        for case, (base, head, unchanged) in cases.items():
            with self.subTest(case=case):
                found = self.holds(root, file, base, head)
                self.assertEqual(found["unchanged"], unchanged, found["why"])
                self.assertFalse(found["whole"])
        examined("diffs judged", cases)


class PlantedControls(Census):
    """A2: each known escape of a read set is planted and caught, and the own-file-only rule is
    refused by the same fixtures."""

    def test_a_root_reader_is_whole_tree(self):
        root, file = self.tree("root-reader")
        found = self.read_set(root, file)
        self.assertTrue(found["whole"])
        self.assertIn("scripts/tests/test_t.py", found["why"])
        added = [section("docs/schematics/b.md", None, "# B\n")]
        verdict = self.holds(root, file, [], added)
        self.assertFalse(verdict["unchanged"], verdict["why"])
        self.assertTrue(verdict["whole"])
        same = self.holds(root, file, added, added)
        self.assertTrue(same["unchanged"], same["why"])
        examined("root readers", [found, verdict, same])

    def test_an_added_module_that_shadows_an_import_is_changed(self):
        root, file = self.tree("bounded")
        shadow = "scripts/tests/helper.py"
        cases = {
            "added at the head only": ([], [section(shadow, None, "def load(name):\n    pass\n")]),
            "added on both sides, differently": (
                [section(shadow, None, "def load(name):\n    return 1\n")],
                [section(shadow, None, "def load(name):\n    return 2\n")],
            ),
            "a stdlib name shadowed": (
                [section("scripts/tests/json.py", None, "x = 1\n")],
                [section("scripts/tests/json.py", None, "x = 2\n")],
            ),
        }
        for case, (base, head) in cases.items():
            with self.subTest(case=case):
                verdict = self.holds(root, file, base, head)
                self.assertFalse(verdict["unchanged"], case)
                self.assertIn("scripts/tests/", verdict["why"])
        examined("shadowing modules", cases)

    def test_an_absent_probe_added_at_the_head_is_changed(self):
        root, file = self.tree("bounded")
        probe = "scripts/tests/fixtures/absent.json"
        head = [section(probe, None, "{}\n")]
        verdict = self.holds(root, file, [], head)
        self.assertFalse(verdict["unchanged"])
        self.assertIn(probe, verdict["why"])
        other = self.holds(root, file, [section(probe, None, "[]\n")], head)
        self.assertFalse(other["unchanged"])
        self.assertIn(probe, other["why"])
        same = self.holds(root, file, head, head)
        self.assertTrue(same["unchanged"], same["why"])
        examined("absent probes", [verdict, other, same])

    def test_a_binary_section_is_changed(self):
        root, file = self.tree("bounded")
        picture = [section("scripts/tests/fixtures/m/pic.png", "a", "b", binary=True)]
        verdict = self.holds(root, file, picture, picture)
        self.assertFalse(verdict["unchanged"])
        self.assertIn("binary", verdict["why"])
        text = [section("scripts/tests/fixtures/m/one.txt", "one\n", "uno\n")]
        same = self.holds(root, file, text, text)
        self.assertTrue(same["unchanged"], same["why"])
        examined("binary sections", [verdict, same])

    def test_the_own_file_only_rule_is_refused_by_the_same_fixtures(self):
        reader, reader_file = self.tree("root-reader")
        bounded, bounded_file = self.tree("bounded")
        helper = self.trees["bounded"]["files"]["scripts/helper.py"]
        cases = {
            "a schematic added beside a root reader": (
                reader,
                reader_file,
                [],
                [section("docs/schematics/b.md", None, "# B\n")],
            ),
            "a helper changed under a bounded file": (
                bounded,
                bounded_file,
                [],
                [section("scripts/helper.py", helper, helper + "# x\n")],
            ),
            "a module shadowing an import": (
                bounded,
                bounded_file,
                [],
                [section("scripts/tests/helper.py", None, "x = 1\n")],
            ),
        }
        for case, (root, file, base, head) in cases.items():
            with self.subTest(case=case):
                self.assertTrue(own_file_only(file, base, head), "the rule would carry it")
                verdict = self.holds(root, file, base, head)
                self.assertFalse(verdict["unchanged"], verdict["why"])
        examined("fixtures that refuse the own-file-only rule", cases)


if __name__ == "__main__":
    unittest.main()
