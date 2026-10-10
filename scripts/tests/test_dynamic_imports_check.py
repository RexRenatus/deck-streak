"""A changed `scripts/tests` module that loads code is held to the census register before the push
(SPEC-406 A1 to A8; ADR-420).

`scripts/dynamic-imports-check.py` reads, from the commit `HEAD` names, each test module the push
adds or modifies and the `DYNAMIC_IMPORTS` register of `test_ci_workflows.py`, both as text. These
tests plant small git repositories in temporary directories and run the check over them; the real
register is parsed, never imported, and no directory is walked.
"""

import ast
import contextlib
import importlib.util
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "dynamic-imports-check.py"
REGISTER = REPO / "scripts" / "tests" / "test_ci_workflows.py"
TEMPLATE = REPO / "docs" / "specs" / "_TEMPLATE.md"
IDENTITY = (
    "-c",
    "user.name=fixture",
    "-c",
    "user.email=fixture" + "@" + "example.invalid",
    "-c",
    "commit.gpgsign=false",
)
LOADER_SOURCE = "import importlib.util\n\nSPEC = importlib.util.spec_from_file_location('x', 'y')\n"
PLAIN_SOURCE = "VALUE = 1\n"
NEW = "scripts/tests/test_new.py"
OLD = "scripts/tests/test_old.py"
REGISTER_PATH = "scripts/tests/test_ci_workflows.py"


def load():
    spec = importlib.util.spec_from_file_location("dynamic_imports_check", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(module)
    return module


def register_text(*modules):
    """A planted register naming each module, in the shape the real one has."""
    sites = ", ".join(f'("{name}", "load", "x", 1)' for name in modules)
    return f'DYNAMIC_IMPORTS = {{**allowed("planted", {sites})}}\n'


def git(root, *args):
    return subprocess.run(
        ["git", *IDENTITY, "-C", str(root), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def write(root, files):
    for name, text in files.items():
        path = Path(root) / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def commit(root, message):
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", message)
    return git(root, "rev-parse", "HEAD")


def plant(root, files):
    """A repository whose first commit holds `files`; returns that commit, the base."""
    git(root, "init", "-q")
    write(root, {"README.md": "planted\n", **files})
    return commit(root, "base")


def run_program(root, base):
    done = subprocess.run(
        [sys.executable, str(SCRIPT), "--root", str(root), "--base", base],
        capture_output=True,
        text=True,
        check=False,
    )
    return done.returncode, done.stdout.splitlines()


class DynamicImportsCheck(unittest.TestCase):
    def setUp(self):
        self.check = load()

    def run_main(self, *args):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = self.check.main(list(args))
        return code, out.getvalue().splitlines()

    def test_a_planted_loader_module_missing_from_the_register_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = plant(tmp, {REGISTER_PATH: register_text("test_known")})
            write(tmp, {NEW: LOADER_SOURCE})
            commit(tmp, "head")
            code, lines = run_program(tmp, base)
        self.assertEqual(code, 1)
        self.assertIn(
            "dynamic-imports: REFUSED: scripts/tests/test_new.py: loads code by importlib at "
            "line 3; DYNAMIC_IMPORTS lists no site of test_new",
            lines,
        )
        self.assertEqual(lines[-1], "dynamic-imports: REFUSED")

    def test_the_same_planted_module_registered_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = plant(tmp, {REGISTER_PATH: register_text("test_known", "test_new")})
            write(tmp, {NEW: LOADER_SOURCE})
            commit(tmp, "head")
            code, lines = self.run_main("--root", tmp, "--base", base)
        self.assertEqual(code, 0)
        self.assertEqual(
            lines,
            [
                "dynamic-imports: register: 2 module(s) in DYNAMIC_IMPORTS at HEAD",
                "dynamic-imports: examined 1 changed scripts/tests module(s), 1 loader-style",
                "dynamic-imports: OK",
            ],
        )

    def test_every_loader_spelling_is_found_and_loader_text_in_a_string_is_not(self):
        spellings = [
            (
                "importlib.util",
                "import importlib.util\nx = importlib.util.find_spec('m')\n",
                [(2, "importlib")],
            ),
            (
                "importlib itself",
                "import importlib\nimportlib.import_module('m')\n",
                [(2, "importlib")],
            ),
            (
                "from importlib import util",
                "from importlib import util\nutil.find_spec('m')\n",
                [(2, "importlib")],
            ),
            (
                "from importlib.util import as",
                "from importlib.util import spec_from_file_location as find\nfind('x', 'y')\n",
                [(2, "importlib")],
            ),
            (
                "import importlib.util as",
                "import importlib.util as iu\niu.find_spec('m')\n",
                [(2, "importlib")],
            ),
            ("runpy", "import runpy\nrunpy.run_path('p')\n", [(2, "runpy")]),
            (
                "from runpy import",
                "from runpy import run_module\nrun_module('m')\n",
                [(2, "runpy")],
            ),
            ("from runpy import as", "from runpy import run_path as go\ngo('p')\n", [(2, "runpy")]),
            ("bare exec", "exec('x = 1')\n", [(1, "exec")]),
            (
                "import inside a function",
                "def f():\n    import importlib\n    return importlib.import_module('m')\n",
                [(3, "importlib")],
            ),
            (
                "two loads in order",
                "import runpy\nexec('a')\nrunpy.run_path('p')\n",
                [(2, "exec"), (3, "runpy")],
            ),
            (
                "two loads on one line",
                "import runpy\nrunpy.run_path(runpy.run_module('m'))\n",
                [(2, "runpy"), (2, "runpy")],
            ),
            (
                "exec after importlib on one line",
                "import importlib\nimportlib.import_module(exec('m'))\n",
                [(2, "importlib"), (2, "exec")],
            ),
            ("a string", "S = 'importlib.util.spec_from_file_location'\n", []),
            ("a comment", "# importlib.util.spec_from_file_location\nx = 1\n", []),
            ("an unused import", "import importlib\n", []),
            ("an unused from import", "from runpy import run_path\n", []),
            ("a module that defines exec", "def exec(x):\n    return x\n\nexec(1)\n", []),
            ("a module that binds exec by assignment", "exec = print\nexec(1)\n", []),
            ("a module that takes exec as an argument", "def f(exec):\n    return exec(1)\n", []),
            ("a module that imports exec", "from shlex import split as exec\nexec('a')\n", []),
            ("a module that defines a class exec", "class exec:\n    pass\n\nexec()\n", []),
            (
                "a process whose argument says exec",
                "import subprocess\nsubprocess.run(['python3', '-c', 'exec(1)'])\n",
                [],
            ),
            ("exec after an unrelated import", "import json\nexec('a')\n", [(2, "exec")]),
            (
                "exec after an unrelated from import",
                "from json import dumps\nexec('a')\n",
                [(2, "exec")],
            ),
            ("exec bound by import as", "import json as exec\nexec('a')\n", []),
            ("exec bound by from import as", "from json import dumps as exec\nexec('a')\n", []),
            ("a star import", "from runpy import *\nrun_path('p')\n", []),
            ("a sibling named runpy", "from .runpy import run_path\nrun_path('p')\n", []),
            ("a relative import", "from . import importlib\nimportlib.x()\n", []),
            ("an unrelated module", "import json\njson.dumps(1)\n", []),
        ]
        for spelling, source, expected in examined("loader spellings", spellings):
            with self.subTest(spelling):
                self.assertEqual(self.check.loader_sites(source), expected)

    def test_the_real_register_lists_this_module_and_the_census_counts_what_the_check_reads(self):
        modules = self.check.register_modules(REGISTER.read_text(encoding="utf-8"))
        for name in examined(
            "register modules wanted",
            [
                "_support",
                "test_backup_units",
                "test_threat_model",
                "test_audit_web",
                "test_dynamic_imports_check",
            ],
        ):
            self.assertIn(name, modules)
        own = Path(__file__).read_text(encoding="utf-8")
        sites = self.check.loader_sites(own)
        self.assertEqual([loader for _line, loader in sites], ["importlib", "importlib"])
        self.assertIn("test_dynamic_imports_check", modules)
        census = {}
        tree = ast.parse(REGISTER.read_text(encoding="utf-8"))
        for node in tree.body:
            if isinstance(node, ast.Assign) and isinstance(node.targets[0], ast.Name):
                census[node.targets[0].id] = node.value
        vetted = [key.value for key in census["VETTED_MODULES"].keys]
        bare = {item.value for item in census["BARE_DYNAMIC"].args[0].elts}
        for key in examined("vetted modules", vetted):
            for name in self.check.LOADER_MODULES:
                self.assertNotEqual(key, name)
                self.assertFalse(key.startswith(name + "."), key)
        self.assertIn(self.check.LOADER_BUILTIN, bare)
        self.assertEqual(self.check.REPO, REPO)
        self.assertEqual(self.check.REGISTER, "scripts/tests/test_ci_workflows.py")
        self.assertEqual(self.check.TESTS, "scripts/tests")
        self.assertEqual(self.check.REGISTER_NAME, "DYNAMIC_IMPORTS")

    def test_a_register_or_a_module_the_check_cannot_read_stops_the_run(self):
        good = register_text("test_known")
        pair = 'DYNAMIC_IMPORTS = {**allowed("r", ("%s", "l", "t", 1))}\n'
        cases = [
            ("no assignment", {REGISTER_PATH: "X = 1\n"}, [], "no DYNAMIC_IMPORTS assignment"),
            (
                "two assignments",
                {REGISTER_PATH: "X = 1\n" + pair % "a" + pair % "b"},
                [],
                "assigned a second time at line 3",
            ),
            ("not a dict", {REGISTER_PATH: "DYNAMIC_IMPORTS = [1]\n"}, [], "is not a dict display"),
            (
                "a literal key",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = {('a', 'q', 't'): (1, 'r')}\n"},
                [],
                "literal key at line 1",
            ),
            (
                "another helper",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = {**other('r', ('a', 'l', 't', 1))}\n"},
                [],
                "entry at line 1 is not an allowed group",
            ),
            (
                "a call that is not a name",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = {**x.allowed('r', ('a', 'l', 't', 1))}\n"},
                [],
                "is not an allowed group",
            ),
            (
                "a module that is not a string",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = {**allowed('r', (1, 'l', 't', 1))}\n"},
                [],
                "site at line 1 has no string module",
            ),
            (
                "a site that is not a tuple",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = {**allowed('r', 'a')}\n"},
                [],
                "has no string module",
            ),
            (
                "an empty tuple site",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = {**allowed('r', ())}\n"},
                [],
                "has no string module",
            ),
            ("an empty register", {REGISTER_PATH: "DYNAMIC_IMPORTS = {}\n"}, [], "names no module"),
            (
                "a group with no site",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = {**allowed('r')}\n"},
                [],
                "names no module",
            ),
            (
                "a register that does not parse",
                {REGISTER_PATH: "DYNAMIC_IMPORTS = (:\n"},
                [],
                "scripts/tests/test_ci_workflows.py does not parse",
            ),
            (
                "a changed module that does not parse",
                {REGISTER_PATH: good},
                [(NEW, "def (:\n")],
                "scripts/tests/test_new.py does not parse",
            ),
            ("no register file", {}, [(NEW, PLAIN_SOURCE)], "git show failed"),
        ]
        for name, base_files, head_files, cause in examined("unreadable inputs", cases):
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                base = plant(tmp, base_files)
                write(tmp, {"head.txt": "x\n", **dict(head_files)})
                commit(tmp, "head")
                code, lines = self.run_main("--root", tmp, "--base", base)
                self.assertEqual(code, 2)
                self.assertTrue(lines[-1].startswith("dynamic-imports: VOID: "), lines[-1])
                self.assertIn(cause, lines[-1])
        with tempfile.TemporaryDirectory() as tmp:
            plant(tmp, {REGISTER_PATH: good})
            code, lines = self.run_main("--root", tmp, "--base", "refs/heads/no-such-branch")
        self.assertEqual(code, 2)
        self.assertTrue(lines[-1].startswith("dynamic-imports: VOID: "), lines[-1])
        self.assertIn("git diff failed", lines[-1])

    def test_the_check_judges_what_head_commits_by_the_names_head_gives(self):
        with tempfile.TemporaryDirectory() as tmp:  # (a) an uncommitted register entry
            base = plant(tmp, {REGISTER_PATH: register_text("test_known")})
            write(tmp, {NEW: LOADER_SOURCE})
            commit(tmp, "head")
            write(tmp, {REGISTER_PATH: register_text("test_known", "test_new")})
            code, lines = self.run_main("--root", tmp, "--base", base)
            self.assertEqual(code, 1, lines)
            self.assertEqual(lines[-1], "dynamic-imports: REFUSED")
        with tempfile.TemporaryDirectory() as tmp:  # (b) a registered loader, renamed
            base = plant(tmp, {REGISTER_PATH: register_text("test_old"), OLD: LOADER_SOURCE})
            git(tmp, "mv", OLD, NEW)
            commit(tmp, "head")
            code, lines = self.run_main("--root", tmp, "--base", base)
            self.assertEqual(code, 1, lines)
            self.assertTrue(any("lists no site of test_new" in line for line in lines), lines)
        with tempfile.TemporaryDirectory() as tmp:  # (c) a deletion
            base = plant(tmp, {REGISTER_PATH: register_text("test_old"), OLD: LOADER_SOURCE})
            git(tmp, "rm", "-q", OLD)
            commit(tmp, "head")
            code, lines = self.run_main("--root", tmp, "--base", base)
            self.assertEqual(code, 0, lines)
            self.assertEqual(lines[-1], "dynamic-imports: NOT-APPLICABLE")
        with tempfile.TemporaryDirectory() as tmp:  # (d) a package and a module in it
            base = plant(tmp, {REGISTER_PATH: register_text("pkg")})
            write(
                tmp,
                {
                    "scripts/tests/pkg/__init__.py": LOADER_SOURCE,
                    "scripts/tests/pkg/test_deep.py": LOADER_SOURCE,
                },
            )
            commit(tmp, "head")
            code, lines = self.run_main("--root", tmp, "--base", base)
            self.assertEqual(code, 1, lines)
            refused = [line for line in lines if "REFUSED:" in line]
            self.assertEqual(len(refused), 1, lines)
            self.assertIn("scripts/tests/pkg/test_deep.py", refused[0])
            self.assertIn("lists no site of pkg.test_deep", refused[0])
        with tempfile.TemporaryDirectory() as tmp:  # (e) a load only in the work tree
            base = plant(tmp, {REGISTER_PATH: register_text("test_known")})
            write(tmp, {NEW: PLAIN_SOURCE})
            commit(tmp, "head")
            write(tmp, {NEW: LOADER_SOURCE})
            code, lines = self.run_main("--root", tmp, "--base", base)
            self.assertEqual(code, 0, lines)
            self.assertIn(
                "dynamic-imports: examined 1 changed scripts/tests module(s), 0 loader-style",
                lines,
            )
        with tempfile.TemporaryDirectory() as tmp:  # (f) only .py paths under scripts/tests
            base = plant(tmp, {REGISTER_PATH: register_text("test_known")})
            write(
                tmp,
                {
                    "scripts/tests/notes.txt": LOADER_SOURCE,
                    "scripts/other.py": LOADER_SOURCE,
                    "scripts/tests_extra/test_x.py": LOADER_SOURCE,
                },
            )
            commit(tmp, "head")
            code, lines = self.run_main("--root", tmp, "--base", base)
            self.assertEqual(code, 0, lines)
            self.assertEqual(lines[-1], "dynamic-imports: NOT-APPLICABLE")

    def test_a_push_that_changes_no_test_module_is_not_applicable_and_still_reads_the_register(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmp:
            base = plant(tmp, {REGISTER_PATH: register_text("test_known")})
            write(tmp, {"docs/x.md": "text\n", "scripts/tests/fixtures/plan.json": "{}\n"})
            commit(tmp, "head")
            code, lines = self.run_main("--root", tmp, "--base", base)
            self.assertEqual(code, 0)
            self.assertEqual(
                lines,
                [
                    "dynamic-imports: register: 1 module(s) in DYNAMIC_IMPORTS at HEAD",
                    "dynamic-imports: examined 0 changed scripts/tests module(s), 0 loader-style",
                    "dynamic-imports: NOT-APPLICABLE",
                ],
            )
            git(tmp, "update-ref", "refs/remotes/origin/dev", base)
            write(tmp, {NEW: LOADER_SOURCE})
            commit(tmp, "loader")
            code, lines = self.run_main("--root", tmp)
            self.assertEqual(code, 1, lines)
            self.assertTrue(any("REFUSED: scripts/tests/test_new.py" in line for line in lines))

    def test_the_spec_template_manifest_section_names_the_census_module_and_the_check(self):
        text = TEMPLATE.read_text(encoding="utf-8")
        section = text.split("## 4. File manifest", 1)[1].split("\n## ", 1)[0]
        for wanted in examined(
            "manifest names",
            [
                "scripts/tests/test_ci_workflows.py",
                "DYNAMIC_IMPORTS",
                "scripts/dynamic-imports-check.py",
            ],
        ):
            self.assertIn(wanted, section)


if __name__ == "__main__":
    unittest.main()
