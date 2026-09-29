"""The Python mutation runner: its listing, its map, its judging and its report (SPEC-087 A1 to A12).

Every test plants a git repository in a temporary directory, holding the files a scenario needs
under `scripts/`, their test modules under `scripts/tests/` and a `scripts/mutation-python.json`
map, and runs the repository's own `scripts/mutation_python.py` over it with `--root`. Each
planted test module is what the run under test executes as a child, so what a test plants is what
the runner sees:

- `sites.py` holds one site of each operator, a `not` before a parenthesised operand and, beside
  them, a docstring, an argument's, a return's and an annotated assignment's annotation and an
  f-string each holding operator sites of their own, a bare `return`, a `return None`, an empty
  string, a multi-line string, a `True` and a `False`, at module level, in a function and in a
  method;
- `calc.py` (every mutant killed), `loose.py` (one survivor), `spin.py` (a mutant that loops),
  `quit.py` (a mutant whose child exits without its report) and `span.py` (a multi-line
  expression) are the smallest files that make each outcome;
- `broken.py`, `nothing.py`, `reader.py` and `only_reader.py` plant a control that fails, a
  control that selects no test, and a test that reads its file's bytes.

A restore that fails is planted by a test that replaces the mutated file while the mutant is
installed. AV1: a failed restore writes its report with the run's exit (4) under `exit`;
`exit_code` derives only 0, 1 or 3 from outcomes, and R7's text governs. AV3: each run that may
hang passes a short `--test-seconds` and a subprocess timeout.
"""

import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import tokenize
import unittest
from pathlib import Path

from _support import REPO

RUNNER = REPO / "scripts" / "mutation_python.py"
LISTED = re.compile(r"^mutation-python: listed (\d+)$", re.MULTILINE)
EXAMINED = re.compile(r"^examined (\d+)", re.MULTILINE)
SUBPROCESS_SECONDS = 120
SITES = '''"""Docstring: 1 + 2 and a == b."""
LIMIT = 10
NAME = "abc"
BLANK = ""
TEXT = """multi
line"""
FLAG = True
OFF = False


def arithmetic(a: "1 + 2", b) -> "1 + 2":
    """Doc: 1 + 2."""
    x: "a == b" = a
    f = f"{a + b}"
    x = a + b
    x = a - b
    x = a * b
    x = a / b
    x = a // b
    x = a % b
    x = a | b
    x = a & b
    return


def logic(a, b):
    x = a == b
    x = a != b
    x = a < b
    x = a <= b
    x = a > b
    x = a >= b
    x = a in b
    x = a not in b
    x = a is b
    x = a is not b
    x = a and b
    x = a or b
    x = not (a)
    return None


class Box:
    def method(self, a):
        for _ in a:
            if a:
                break
            continue
        return a
'''
#: (line, column, text) of each mutant `sites.py` lists, in source order.
SITE_MUTANTS = [
    (2, 9, "replace 10 with 11 in <module>"),
    (3, 8, 'replace "abc" with "" in <module>'),
    (4, 9, 'replace "" with "XX" in <module>'),
    (7, 8, "replace True with False in <module>"),
    (8, 7, "replace False with True in <module>"),
    (15, 11, "replace + with - in arithmetic"),
    (16, 11, "replace - with + in arithmetic"),
    (17, 11, "replace * with / in arithmetic"),
    (18, 11, "replace / with * in arithmetic"),
    (19, 11, "replace // with * in arithmetic"),
    (20, 11, "replace % with * in arithmetic"),
    (21, 11, "replace | with & in arithmetic"),
    (22, 11, "replace & with | in arithmetic"),
    (27, 11, "replace == with != in logic"),
    (28, 11, "replace != with == in logic"),
    (29, 11, "replace < with >= in logic"),
    (30, 11, "replace <= with > in logic"),
    (31, 11, "replace > with <= in logic"),
    (32, 11, "replace >= with < in logic"),
    (33, 11, "replace in with not in in logic"),
    (34, 11, "replace not in with in in logic"),
    (35, 11, "replace is with is not in logic"),
    (36, 11, "replace is not with is in logic"),
    (37, 11, "replace and with or in logic"),
    (38, 11, "replace or with and in logic"),
    (39, 9, "replace not with nothing in logic"),
    (47, 17, "replace break with continue in Box.method"),
    (48, 13, "replace continue with break in Box.method"),
    (49, 16, "replace return value with return None in Box.method"),
]
CALC = "def double(x):\n    return x * 2\n"
LOOSE = "def keep(x):\n    return x + 0\n"
SPIN = "def count(n):\n    i = 0\n    while i < n:\n        i = i + 1\n    return i\n"
QUIT = "import os\n\n\ndef value(x):\n    if x == 0:\n        os._exit(0)\n    return x\n"
SPAN = (
    "def total(a, b, c):\n"
    "    return (a +\n"
    "            b +\n"
    "            c)\n"
    "\n"
    "\n"
    "def other(x):\n"
    "    return x * 2\n"
)
BROKEN = "def five():\n    return 5\n"
READER = "def total(a, b):\n    return a + b\n"
HEAD = (
    "import sys\n"
    "import unittest\n"
    "from pathlib import Path\n\n"
    "ROOT = Path(__file__).resolve().parents[2]\n"
    "sys.path.insert(0, str(ROOT / 'scripts'))\n"
    "import {module}\n\n\n"
)
TESTS = {
    "calc": HEAD.format(module="calc") + "class Double(unittest.TestCase):\n"
    "    def test_two_doubles_to_four(self):\n"
    "        self.assertEqual(calc.double(2), 4)\n",
    "loose": HEAD.format(module="loose") + "class Keep(unittest.TestCase):\n"
    "    def test_keeps_five(self):\n"
    "        self.assertEqual(loose.keep(5), 5)\n",
    "spin": HEAD.format(module="spin") + "import subprocess\n\n\n"
    "class Count(unittest.TestCase):\n"
    "    def test_counts_three(self):\n"
    "        sleeper = subprocess.Popen(['sleep', '300'])\n"
    "        with (ROOT / 'pids.log').open('a') as log:\n"
    "            log.write(f'{sleeper.pid}\\n')\n"
    "        self.assertEqual(spin.count(3), 3)\n",
    "quit": HEAD.format(module="quit") + "class Value(unittest.TestCase):\n"
    "    def test_five_is_five(self):\n"
    "        self.assertEqual(quit.value(5), 5)\n",
    "span": HEAD.format(module="span") + "class Total(unittest.TestCase):\n"
    "    def test_total_and_other(self):\n"
    "        self.assertEqual(span.total(1, 2, 3), 6)\n"
    "        self.assertEqual(span.other(2), 4)\n",
    "broken": HEAD.format(module="broken") + "class Five(unittest.TestCase):\n"
    "    def test_five_is_six(self):\n"
    "        with (ROOT / 'calls.log').open('a') as log:\n"
    "            log.write('broken\\n')\n"
    "        self.assertEqual(broken.five(), 6)\n",
    "nothing": "import unittest\n\n\nclass Empty(unittest.TestCase):\n    pass\n",
    "reader": HEAD.format(module="reader") + "class Total(unittest.TestCase):\n"
    "    def test_reads_its_own_bytes(self):\n"
    "        source = (ROOT / 'scripts' / 'reader.py').read_text()\n"
    "        self.assertEqual(source.splitlines()[1], '    return a + b')\n"
    "        self.assertEqual(len(source.splitlines()), 2)\n\n"
    "    def test_zero_is_zero(self):\n"
    "        self.assertEqual(reader.total(0, 0), 0)\n",
    "only_reader": HEAD.format(module="only_reader") + "class Total(unittest.TestCase):\n"
    "    def test_reads_its_own_bytes(self):\n"
    "        source = (ROOT / 'scripts' / 'only_reader.py').read_text()\n"
    "        self.assertEqual(len(source.splitlines()), 2)\n",
}


def git(root, *args):
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=True
    ).stdout


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Tree:
    """A git repository in a temporary directory holding a scenario's population."""

    def __init__(self, test, sources, tests=None, mapping=None, extra=None):
        scratch = tempfile.TemporaryDirectory()
        test.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.write(".gitignore", "*.log\n__pycache__/\n")
        mapped = {}
        for name, text in sources.items():
            self.write(f"scripts/{name}.py", text)
            module = tests.get(name, [name]) if tests is not None else [name]
            mapped[f"scripts/{name}.py"] = {
                "dir": "scripts/tests",
                "modules": [f"test_{m}" for m in module],
            }
            for m in module:
                if TESTS.get(m) and not (self.root / f"scripts/tests/test_{m}.py").exists():
                    self.write(f"scripts/tests/test_{m}.py", TESTS[m])
        for relative, text in (extra or {}).items():
            self.write(relative, text)
        self.write(
            "scripts/mutation-python.json",
            json.dumps(mapped if mapping is None else mapping, indent=2) + "\n",
        )
        git(self.root, "init", "-q", "-b", "dev")
        git(self.root, "config", "user.email", "fixture@example.invalid")
        git(self.root, "config", "user.name", "fixture")
        git(self.root, "add", "-A")
        git(self.root, "commit", "-q", "-m", "the fixture")

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def files(self):
        return sorted(
            p.relative_to(self.root).as_posix()
            for p in self.root.rglob("*")
            if p.is_file() and ".git" not in p.relative_to(self.root).parts
        )

    def runner(self, *args, runner=RUNNER, timeout=SUBPROCESS_SECONDS):
        return subprocess.run(
            [sys.executable, str(runner), *args, "--root", str(self.root)],
            capture_output=True,
            text=True,
            timeout=timeout,
            env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        )

    def report(self, *args, **kwargs):
        target = self.root.parent / f"{self.root.name}-report.json"
        self.addcleanup = None
        done = self.runner("run", *args, "--report", str(target), **kwargs)
        document = json.loads(target.read_text(encoding="utf-8")) if target.exists() else None
        target.unlink(missing_ok=True)
        return done, document


def mutants_of(report, path=None):
    return [
        m for entry in report["files"] if path in (None, entry["path"]) for m in entry["mutants"]
    ]


def by_text(report, path):
    return {m["mutant"]: m for m in mutants_of(report, path)}


class TheRunnerListsItsMutants(unittest.TestCase):
    def test_the_runner_lists_exactly_the_operator_sets_mutants(self):
        """A1"""
        tree = Tree(self, {"sites": SITES}, tests={"sites": []})
        before = digest(tree.root / "scripts/sites.py")
        done = tree.runner("list", "--all", "--out", str(tree.root / "listing.json"))
        self.assertEqual(done.returncode, 0, done.stderr)
        expected = [
            f"scripts/sites.py:{line}:{column}: {text}" for line, column, text in SITE_MUTANTS
        ]
        listed = done.stdout.splitlines()
        self.assertEqual(listed[0], f"mutation-python: listed {len(expected)}")
        self.assertEqual(listed[1:], expected)
        document = json.loads((tree.root / "listing.json").read_text(encoding="utf-8"))
        self.assertEqual([m["name"] for m in document["mutants"]], expected)
        self.assertEqual(digest(tree.root / "scripts/sites.py"), before)
        self.assertEqual(git(tree.root, "status", "--porcelain"), "?? listing.json\n")
        again = tree.runner("list", "--all")
        self.assertEqual(again.stdout, done.stdout)

    def test_every_listed_mutant_parses_and_a_comment_changes_no_listing(self):
        """A1 (each parsing; comments)"""
        sys.path.insert(0, str(REPO / "scripts"))
        self.addCleanup(sys.path.remove, str(REPO / "scripts"))
        import mutation_python

        mutants = mutation_python.list_source(SITES)
        self.assertEqual(len(mutants), len(SITE_MUTANTS))
        for mutant in mutants:
            compile(mutant.apply(SITES), "sites.py", "exec")
        commented = (
            SITES.replace("x = a + b", "x = a + b  # a == b, 1 + 2 and not x").replace(
                "return None\n", "return None  # return value\n"
            )
            + "\n# x == 1 and y\n"
        )
        self.assertEqual(
            [(m.line, m.column, m.text) for m in mutation_python.list_source(commented)],
            [(m.line, m.column, m.text) for m in mutants],
        )

    def test_the_plan_selects_the_changed_lines_mutants_and_the_shards_partition_them(self):
        """A10"""
        tree = Tree(self, {"span": SPAN, "calc": CALC})

        def selected(lines, applies=True, klass="scripts", path="scripts/span.py"):
            plan = {
                "classes": {"scripts": {"applies": applies}, "oracle": {"applies": False}},
                "files": [{"path": path, "class": klass, "code": lines}],
            }
            target = tree.write("plan.json", json.dumps(plan))
            done = tree.runner("list", "--plan", str(target))
            self.assertEqual(done.returncode, 0, done.stderr)
            names = done.stdout.splitlines()[1:]
            self.assertEqual(LISTED.search(done.stdout).group(1), str(len(names)))
            return sorted(re.sub(r"^\S+:\d+:\d+: replace ", "", n) for n in names)

        every = ["+ with - in total", "+ with - in total", "return value with return None in total"]
        self.assertEqual(selected([1]), [])
        self.assertEqual(selected([2]), sorted(every))
        self.assertEqual(selected([3]), sorted(every))
        self.assertEqual(selected([4]), sorted(every[1:]))
        self.assertEqual(selected([5]), [])
        self.assertEqual(len(selected([8])), 3)
        self.assertEqual(
            selected([2, 3, 4, 8]),
            sorted(
                [
                    *every,
                    "* with / in other",
                    "2 with 3 in other",
                    "return value with return None in other",
                ]
            ),
        )
        self.assertEqual(selected([3], applies=False), [])
        self.assertEqual(selected([3], path="scripts/elsewhere.py"), [])
        self.assertEqual(selected([2], klass="other"), [])
        whole = tree.runner("list", "--all").stdout.splitlines()[1:]
        self.assertEqual(len(whole), 9)
        self.assertEqual(
            tree.runner("list", "--file", "scripts/calc.py").stdout.splitlines()[0],
            "mutation-python: listed 3",
        )
        self.assertEqual(
            tree.runner("list", "--all", "--shard", "0/1").stdout.splitlines()[1:], whole
        )
        parts = [
            tree.runner("list", "--all", "--shard", f"{k}/4").stdout.splitlines()[1:]
            for k in range(4)
        ]
        for k, part in enumerate(parts):
            self.assertEqual(part, whole[k::4])
        self.assertEqual(sorted(sum(parts, [])), sorted(whole))
        empty = tree.write(
            "empty.json",
            json.dumps(
                {
                    "classes": {"scripts": {"applies": True}},
                    "files": [{"path": "scripts/span.py", "class": "scripts", "code": [5]}],
                }
            ),
        )
        done, report = tree.report("--plan", str(empty))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(report["counts"]["examined"], 0)
        self.assertRegex(done.stdout, r"(?m)^examined 0$")


class TheMapCensusRefuses(unittest.TestCase):
    def census(self, mapping, sources=None, extra=None):
        tree = Tree(self, sources or {"calc": CALC}, mapping=mapping, extra=extra)
        return tree, tree.runner("census")

    def test_the_map_census_refuses_an_omitted_file_a_missing_module_and_a_stranger(self):
        """A2"""
        good = {"scripts/calc.py": {"dir": "scripts/tests", "modules": ["test_calc"]}}
        tree, done = self.census(good, extra={"scripts/tests/test_calc.py": TESTS["calc"]})
        self.assertEqual((done.returncode, done.stdout.strip().splitlines()[-1]), (0, "examined 1"))
        cases = {
            "omitted": ({}, "scripts/calc.py: the map omits this population file"),
            "stranger": (
                {**good, "scripts/ghost.py": {"dir": "scripts/tests", "modules": []}},
                "scripts/ghost.py: a key outside the population",
            ),
            "absent": (
                {"scripts/calc.py": {"dir": "scripts/tests", "modules": ["test_nope"]}},
                "the module test_nope has no file in scripts/tests",
            ),
            "twice": (
                {
                    "scripts/calc.py": {
                        "dir": "scripts/tests",
                        "modules": ["test_calc", "test_calc"],
                    }
                },
                "the module test_calc is named twice",
            ),
            "key": (
                {"scripts/calc.py": {"dir": "scripts/tests", "modules": [], "skip": True}},
                "the entry carries 'skip'",
            ),
        }
        for name, (mapping, reason) in cases.items():
            with self.subTest(name):
                tree, done = self.census(
                    mapping, extra={"scripts/tests/test_calc.py": TESTS["calc"]}
                )
                self.assertEqual(done.returncode, 1)
                self.assertIn("mutation-python: REFUSED: ", done.stdout)
                self.assertIn(reason, done.stdout)

    def test_the_committed_map_passes_and_prints_its_count(self):
        """A2 (the committed map)"""
        done = subprocess.run(
            [sys.executable, str(RUNNER), "census"], capture_output=True, text=True, check=False
        )
        self.assertEqual(done.returncode, 0, done.stdout)
        named = json.loads((REPO / "scripts/mutation-python.json").read_text(encoding="utf-8"))
        self.assertEqual(EXAMINED.search(done.stdout).group(1), str(len(named)))
        self.assertGreater(len(named), 0)
        expected = sorted(p.relative_to(REPO).as_posix() for p in (REPO / "scripts").glob("*.py"))
        self.assertEqual(
            sorted(k for k in named if k != "tools/parity-oracle/generate.py"), expected
        )
        self.assertIn("tools/parity-oracle/generate.py", named)
        self.assertIn("scripts/mutation_python.py", named)


class TheRunnerJudgesEachMutant(unittest.TestCase):
    def test_the_runner_refuses_a_dirty_tree_before_it_mutates(self):
        """A3"""
        tree = Tree(self, {"calc": CALC})
        tree.write("scripts/calc.py", CALC + "# a tracked change\n")
        before = digest(tree.root / "scripts/calc.py")
        done, report = tree.report("--all", "--test-seconds", "5")
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIsNone(report)
        self.assertEqual(digest(tree.root / "scripts/calc.py"), before)
        clean = Tree(self, {"calc": CALC})
        for args in (
            [],
            ["--all", "--file", "scripts/calc.py"],
            ["--all", "--shard", "2/2"],
            ["--all", "--shard", "0/0"],
            ["--all", "--shard", "x"],
        ):
            with self.subTest(args=args):
                done, report = clean.report(*args)
                self.assertEqual(done.returncode, 2, done.stdout)
                self.assertIsNone(report)
        self.assertEqual(git(clean.root, "status", "--porcelain"), "")

    def test_a_red_or_empty_control_voids_its_file(self):
        """A4"""
        tree = Tree(self, {"broken": BROKEN, "nothing": "def z():\n    return 1\n", "loose": LOOSE})
        done, report = tree.report("--all", "--failfast", "--test-seconds", "20")
        self.assertEqual(done.returncode, 3, done.stdout)
        for path in ("scripts/broken.py", "scripts/nothing.py"):
            entry = next(e for e in report["files"] if e["path"] == path)
            self.assertTrue(entry["void"], path)
            self.assertIn(f"VOID: {path}", done.stdout)
            self.assertTrue(all(m["outcome"] == "void" for m in entry["mutants"]))
        self.assertIn("survived", {m["outcome"] for m in mutants_of(report, "scripts/loose.py")})
        self.assertEqual((tree.root / "calls.log").read_text().splitlines(), ["broken"])

    def test_a_test_that_fails_on_the_sentinel_is_named_and_never_a_killer(self):
        """A5"""
        tree = Tree(self, {"reader": READER, "only_reader": READER})
        done, report = tree.report("--file", "scripts/reader.py", "--test-seconds", "20")
        self.assertEqual(done.returncode, 1, done.stdout)
        entry = report["files"][0]
        self.assertEqual(len(entry["byte_readers"]), 1)
        self.assertIn("test_reads_its_own_bytes", entry["byte_readers"][0])
        outcomes = by_text(report, "scripts/reader.py")
        self.assertEqual(outcomes["replace + with - in total"]["outcome"], "survived")
        for mutant in mutants_of(report):
            self.assertFalse(any("test_reads_its_own_bytes" in k for k in mutant["killers"]))
        done, report = tree.report("--file", "scripts/only_reader.py", "--test-seconds", "20")
        self.assertEqual(done.returncode, 3, done.stdout)
        self.assertIn("VOID: scripts/only_reader.py", done.stdout)
        self.assertTrue(report["files"][0]["void"])

    def test_a_killed_mutant_names_its_killers(self):
        """A6"""
        many = (
            HEAD.format(module="calc") + "class Double(unittest.TestCase):\n"
            "    def test_a(self):\n        self.assertEqual(calc.double(2), 4)\n\n"
            "    def test_b(self):\n        self.assertEqual(calc.double(3), 6)\n\n"
            "    def test_c(self):\n"
            "        for i in (1, 2):\n"
            "            with self.subTest(i=i):\n"
            "                self.assertEqual(calc.double(i), 2 * i)\n"
        )
        tree = Tree(self, {"calc": CALC}, extra={"scripts/tests/test_calc.py": many})
        done, report = tree.report("--all", "--test-seconds", "20")
        self.assertEqual(done.returncode, 0, done.stdout)
        first = by_text(report, "scripts/calc.py")[
            "replace return value with return None in double"
        ]
        self.assertEqual(first["outcome"], "killed")
        for name in ("test_a", "test_b", "test_c"):
            self.assertTrue(any(name in k for k in first["killers"]), name)
        self.assertTrue(any("(i=1)" in k for k in first["killers"]), first["killers"])
        done, report = tree.report("--all", "--failfast", "--test-seconds", "20")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertTrue(report["failfast"])
        self.assertTrue(all(len(m["killers"]) == 1 for m in mutants_of(report)))

    def test_an_import_that_raises_is_killed_by_a_test_named_for_its_module(self):
        """A6 (import failure)"""
        boom = "LIMIT = 2\nassert LIMIT == 2\n"
        module = (
            HEAD.format(module="boom")
            + "class Ok(unittest.TestCase):\n    def test_ok(self):\n        self.assertEqual(boom.LIMIT, 2)\n"
        )
        tree = Tree(self, {"boom": boom}, extra={"scripts/tests/test_boom.py": module})
        done, report = tree.report("--all", "--test-seconds", "20")
        self.assertEqual(done.returncode, 0, done.stdout)
        for mutant in mutants_of(report):
            self.assertEqual(mutant["outcome"], "killed", mutant)
            self.assertTrue(any("test_boom" in k for k in mutant["killers"]), mutant)

    def test_a_mutant_of_the_runner_is_killed_and_a_by_name_import_reads_the_tree(self):
        """A6 (the runner itself; the child's sys.path and sys.modules, AV2)"""
        rows = "def label():\n    return 'rows'\n"
        by_name = (
            "import sys\nimport unittest\nfrom pathlib import Path\n\n"
            "sys.path.append(str(Path(__file__).resolve().parents[1]))\n"
            "import mutation_rows\n\n\n"
            "class ByName(unittest.TestCase):\n"
            "    def test_the_tree_file_is_loaded(self):\n"
            "        self.assertEqual(mutation_rows.label(), 'rows')\n"
        )
        tree = Tree(
            self,
            {"mutation_rows": rows},
            tests={"mutation_rows": ["rows_by_name"]},
            extra={"scripts/tests/test_rows_by_name.py": by_name},
        )
        done, report = tree.report("--all", "--test-seconds", "20")
        self.assertEqual(done.returncode, 0, done.stdout)
        killed = mutants_of(report)
        self.assertGreater(len(killed), 0)
        for mutant in killed:
            self.assertTrue(any("test_the_tree_file_is_loaded" in k for k in mutant["killers"]))
        self.assertEqual(report["files"][0]["control"]["failures"], 0)

        text = RUNNER.read_text(encoding="utf-8")
        line = next(n for n, t in enumerate(text.splitlines(), 1) if t.startswith("if __name__ =="))
        own = (
            "import subprocess\nimport sys\nimport unittest\nfrom pathlib import Path\n\n"
            "ROOT = Path(__file__).resolve().parents[2]\n\n\n"
            "class Own(unittest.TestCase):\n"
            "    def test_the_runner_prints_its_census(self):\n"
            "        done = subprocess.run(\n"
            "            [sys.executable, str(ROOT / 'scripts' / 'mutation_python.py'), 'census',\n"
            "             '--root', str(ROOT)], capture_output=True, text=True)\n"
            "        self.assertIn('examined', done.stdout)\n"
        )
        rows_text = (REPO / "scripts/mutation_rows.py").read_text(encoding="utf-8")
        tree = Tree(
            self,
            {"mutation_python": text, "mutation_rows": rows_text},
            tests={"mutation_python": ["own"], "mutation_rows": []},
            extra={"scripts/tests/test_own.py": own},
        )
        plan = tree.write(
            "plan.json",
            json.dumps(
                {
                    "classes": {"scripts": {"applies": True}},
                    "files": [
                        {"path": "scripts/mutation_python.py", "class": "scripts", "code": [line]}
                    ],
                }
            ),
        )
        done, report = tree.report(
            "--plan",
            str(plan),
            "--test-seconds",
            "60",
            runner=tree.root / "scripts/mutation_python.py",
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        negated = by_text(report, "scripts/mutation_python.py")["replace == with != in <module>"]
        self.assertEqual(negated["outcome"], "killed")

    def test_a_mutant_whose_tests_hang_is_void_never_killed(self):
        """A7"""
        tree = Tree(self, {"spin": SPIN, "quit": QUIT})
        began = time.monotonic()
        done, report = tree.report("--all", "--test-seconds", "3", timeout=60)
        self.assertLess(time.monotonic() - began, 55)
        self.assertEqual(done.returncode, 3, done.stdout)
        spin = by_text(report, "scripts/spin.py")
        self.assertEqual(spin["replace + with - in count"]["outcome"], "timeout")
        self.assertEqual(spin["replace + with - in count"]["killers"], [])
        quit_ = by_text(report, "scripts/quit.py")
        self.assertEqual(quit_["replace == with != in value"]["outcome"], "void")
        pids = [int(p) for p in (tree.root / "pids.log").read_text().split()]
        self.assertGreater(len(pids), 2)
        for _ in range(50):
            alive = []
            for pid in pids:
                try:
                    os.kill(pid, 0)
                    state = Path(f"/proc/{pid}/stat").read_text().split(")")[-1].split()[0]
                    if state != "Z":
                        alive.append(pid)
                except (ProcessLookupError, FileNotFoundError):
                    pass
            if not alive:
                break
            time.sleep(0.1)
        self.assertEqual(alive, [], "a child's process outlived the run")

    def test_a_mutant_that_does_not_parse_is_unviable_and_runs_no_test(self):
        """A8"""
        sys.path.insert(0, str(REPO / "scripts"))
        self.addCleanup(sys.path.remove, str(REPO / "scripts"))
        import mutation_python

        with tempfile.TemporaryDirectory() as scratch:
            target = Path(scratch) / "t.py"
            target.write_text("x = 1\n", encoding="utf-8")

            def never():
                raise AssertionError("a test ran for a mutant that does not parse")

            outcome = mutation_python.judge_mutant(
                target, target.read_bytes(), "def broken(:\n", never
            )
            self.assertEqual(outcome, ("unviable", []))
            self.assertEqual(target.read_text(encoding="utf-8"), "x = 1\n")
        report = {
            "files": [
                {"path": "t.py", "void": None, "mutants": [{"outcome": "unviable", "name": "n"}]}
            ]
        }
        counts = mutation_python.count(report)
        self.assertEqual((counts["unviable"], counts["examined"]), (1, 0))
        self.assertEqual(mutation_python.exit_code(report), 0)

    def test_a_failed_restore_is_the_runs_exit_and_no_outcome_derives_it(self):
        """AV1: exit_code derives 0, 1 or 3 only"""
        sys.path.insert(0, str(REPO / "scripts"))
        self.addCleanup(sys.path.remove, str(REPO / "scripts"))
        import mutation_python

        def report(*outcomes, **more):
            mutants = [{"outcome": o, "name": o} for o in outcomes]
            return {"files": [{"path": "t.py", "void": None, "mutants": mutants}], **more}

        self.assertEqual(mutation_python.exit_code(report("killed", exit=4)), 0)
        self.assertEqual(mutation_python.exit_code(report("killed", "survived")), 1)
        self.assertEqual(mutation_python.exit_code(report("uncovered")), 1)
        self.assertEqual(mutation_python.exit_code(report("survived", "timeout")), 3)
        self.assertEqual(mutation_python.exit_code(report("void")), 3)
        void_file = report("killed")
        void_file["files"][0]["void"] = "the control failed"
        self.assertEqual(mutation_python.exit_code(void_file), 3)
        for outcomes in ((), ("killed",), ("survived",), ("timeout", "survived"), ("unviable",)):
            self.assertIn(mutation_python.exit_code(report(*outcomes, exit=4)), (0, 1, 3))

    def test_every_mutant_is_restored_byte_for_byte_and_a_failed_restore_stops_the_run(self):
        """A9"""
        tree = Tree(self, {"spin": SPIN, "loose": LOOSE})
        before = {p: digest(tree.root / p) for p in tree.files()}
        done, report = tree.report("--all", "--test-seconds", "3", timeout=90)
        self.assertEqual(done.returncode, 3, done.stdout)
        seen = {m["outcome"] for m in mutants_of(report)}
        self.assertTrue({"killed", "survived", "timeout"} <= seen, seen)
        for path, was in before.items():
            self.assertEqual(digest(tree.root / path), was, path)
        self.assertEqual(
            sorted(set(tree.files()) - set(before)), ["pids.log"], "the tree gained a file"
        )
        self.assertEqual(git(tree.root, "status", "--porcelain"), "")
        for kind, plant in (
            ("a directory", "path.unlink()\n            path.mkdir()\n"),
            ("a link to /dev/null", "path.unlink()\n            path.symlink_to('/dev/null')\n"),
        ):
            with self.subTest(kind):
                swap = (
                    HEAD.format(module="calc") + "class Double(unittest.TestCase):\n"
                    "    def test_two_doubles_to_four(self):\n"
                    "        with (ROOT / 'ran.log').open('a') as log:\n"
                    "            log.write('x\\n')\n"
                    "        if calc.double(2) != 4:\n"
                    "            path = ROOT / 'scripts' / 'calc.py'\n            " + plant + ""
                    "        self.assertEqual(calc.double(2), 4)\n"
                )
                tree = Tree(self, {"calc": CALC}, extra={"scripts/tests/test_calc.py": swap})
                done, report = tree.report("--all", "--test-seconds", "20", timeout=60)
                self.assertEqual(done.returncode, 4, done.stdout)
                self.assertIn("scripts/calc.py", done.stdout)
                self.assertEqual(report["exit"], 4)
                self.assertIn("scripts/calc.py", report["restore_failed"])
                self.assertEqual(len((tree.root / "ran.log").read_text().splitlines()), 3)
                self.assertEqual(len(mutants_of(report)), 0)

    def test_a_file_with_no_test_module_reads_every_mutant_uncovered(self):
        """A11"""
        tree = Tree(self, {"calc": CALC}, tests={"calc": []})
        done, report = tree.report("--all", "--test-seconds", "20")
        self.assertEqual(done.returncode, 1, done.stdout)
        found = mutants_of(report)
        self.assertEqual(len(found), 3)
        self.assertTrue(all(m["outcome"] == "uncovered" for m in found))
        self.assertEqual(report["counts"]["examined"], 3)
        self.assertRegex(done.stdout, r"(?m)^examined 3$")
        self.assertEqual([p for p in tree.files() if p.endswith(".log")], [])


class TheReportCarriesWhatItSays(unittest.TestCase):
    def test_the_report_counts_examined_and_the_exit_reads_its_outcomes(self):
        """A12"""
        tree = Tree(self, {"calc": CALC, "loose": LOOSE, "spin": SPIN, "quit": QUIT})
        done, report = tree.report("--all", "--test-seconds", "3", "--failfast", timeout=120)
        self.assertEqual(done.returncode, 3, done.stdout)
        self.assertEqual(report["schema"], "deckstreak.mutation-python.v1")
        self.assertEqual(
            (report["selection"], report["shard"], report["failfast"]), ("all", None, True)
        )
        self.assertEqual(report["exit"], 3)
        entry = next(e for e in report["files"] if e["path"] == "scripts/calc.py")
        self.assertEqual(entry["modules"], ["test_calc"])
        self.assertEqual(entry["control"]["ran"], 1)
        self.assertEqual(entry["control"]["failures"], 0)
        self.assertGreaterEqual(entry["control"]["seconds"], 0)
        self.assertEqual(entry["bound"], 3)
        self.assertEqual(entry["byte_readers"], [])
        self.assertIsNone(entry["void"])
        mutant = by_text(report, "scripts/calc.py")["replace * with / in double"]
        self.assertEqual(
            (
                mutant["name"],
                mutant["file"],
                mutant["line"],
                mutant["end_line"],
                mutant["column"],
                mutant["operator"],
            ),
            (
                "scripts/calc.py:2:14: replace * with / in double",
                "scripts/calc.py",
                2,
                2,
                14,
                "arithmetic",
            ),
        )
        found = mutants_of(report)
        counts = report["counts"]
        for outcome in ("killed", "survived", "uncovered", "unviable", "timeout", "void"):
            self.assertEqual(counts[outcome], sum(m["outcome"] == outcome for m in found), outcome)
        self.assertEqual(
            counts["examined"], counts["killed"] + counts["survived"] + counts["uncovered"]
        )
        self.assertGreaterEqual(counts["timeout"], 1)
        self.assertGreaterEqual(counts["void"], 1)
        self.assertRegex(done.stdout, rf"(?m)^examined {counts['examined']}$")
        clean = Tree(self, {"calc": CALC, "loose": LOOSE})
        done, report = clean.report("--file", "scripts/calc.py", "--test-seconds", "20")
        self.assertEqual(done.returncode, 0, done.stdout)
        done, report = clean.report("--file", "scripts/loose.py", "--test-seconds", "20")
        self.assertEqual(done.returncode, 1, done.stdout)

    def test_the_bound_is_five_times_the_control_and_never_under_sixty(self):
        """A12 (the bound)"""
        tree = Tree(self, {"calc": CALC})
        for seconds, bound in (("5", 60), ("12", 60), ("20", 100)):
            done, report = tree.report("--file", "scripts/calc.py", "--control-seconds", seconds)
            self.assertEqual(done.returncode, 0, done.stdout)
            self.assertEqual(report["files"][0]["bound"], bound, seconds)
        done, report = tree.report(
            "--file", "scripts/calc.py", "--control-seconds", "20", "--test-seconds", "7"
        )
        self.assertEqual(report["files"][0]["bound"], 7)


def runner_module():
    """The runner's file loaded fresh, so a constant a row mutates in place is read as it stands."""
    spec = importlib.util.spec_from_file_location("mutation_python_constants", RUNNER)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class TheRunnersConstantsAreNamedWhole(unittest.TestCase):
    """Each constant a run's output or bound rests on, named by its whole value (SPEC-087 R15)."""

    def test_the_sentinel_is_the_exact_text_a_byte_reader_is_run_against(self):
        self.assertEqual(runner_module().SENTINEL, "\n# mutation-python sentinel\n")

    def test_a_control_without_a_bound_may_run_fifteen_minutes(self):
        self.assertEqual(runner_module().CONTROL_SECONDS, 900.0)

    def test_the_outcomes_are_in_the_order_the_report_counts_them(self):
        self.assertEqual(
            runner_module().OUTCOMES,
            ("killed", "survived", "uncovered", "unviable", "timeout", "void"),
        )

    def test_a_newline_is_a_line_feed_a_carriage_return_or_both(self):
        newline = runner_module().NEWLINE
        self.assertEqual(newline.split("a\r\nb\rc\nd"), ["a", "b", "c", "d"])

    def test_the_quiet_tokens_are_the_six_kinds_that_carry_no_code(self):
        self.assertEqual(
            runner_module().QUIET,
            frozenset(
                {
                    tokenize.COMMENT,
                    tokenize.NL,
                    tokenize.NEWLINE,
                    tokenize.INDENT,
                    tokenize.DEDENT,
                    tokenize.ENDMARKER,
                }
            ),
        )


if __name__ == "__main__":
    unittest.main()
