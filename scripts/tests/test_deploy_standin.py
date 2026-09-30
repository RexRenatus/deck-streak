"""The deploy tests' host stand-in runs only the commands the tests use (SPEC-298; ADR-298; #502).

`test_deploy_scripts.py` replaces the deploy host with a stand-in script, `HOST`. A test double fails
closed on a command it was not written for, and never runs it:

* the stand-in executes an `argv[0]` only if `HOST_ALLOWED` lists it (default-deny: no list of refused
  commands exists anywhere), and any other `argv[0]` exits non-zero with one line naming it;
* `HOST_ALLOWED` equals the set of `argv[0]` values the tests' calls make, derived here from the
  script under test and from the tests' own environments, and measured from the stand-in's log;
* every call that runs `deploy.sh` or `rollback.sh` goes through `launch`, which refuses an
  environment that does not name `DECKSTREAK_DEPLOY_ELEVATE`; a census over the module's syntax tree
  proves no other call site can start a program.

A test that hands the stand-in a privilege-named `argv[0]` puts a recording shim of that name first
on its own `PATH`, so a broken stand-in reaches the shim and the test fails on the record, never a
real program."""

import ast
import os
import re
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

import test_deploy_scripts as deploy_tests
from _support import REPO, examined

ELEVATE = "DECKSTREAK_DEPLOY_ELEVATE"
SOURCE = Path(deploy_tests.__file__)
DEPLOY = REPO / "deploy" / "deploy.sh"
# The privilege commands the stand-in must refuse without a list of them: each is given as a name
# that is NOT in the allowed set, beside arbitrary names and every spelling of an allowed one.
PRIVILEGE = (
    "sudo",
    "doas",
    "su",
    "pkexec",
    "busctl",
    "systemd-run",
    "runuser",
    "nsenter",
    "chroot",
    "env",
)
ARBITRARY = ("frobnicate", "sh", "python3", "rm", "zsh")


def refusal(name):
    """The one line the stand-in prints for a command it does not run."""
    return f"host stand-in: refusing {name}: not a command the deploy tests use\n"


def elevate_values(tree):
    """Every string the module gives `DECKSTREAK_DEPLOY_ELEVATE`, in a dict or a keyword; a value the
    tree cannot read as a constant is refused, so no environment escapes the derivation."""
    values = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Dict):
            for key, value in zip(node.keys, node.values, strict=True):
                if isinstance(key, ast.Constant) and key.value == ELEVATE:
                    values.append(value)
        if isinstance(node, ast.keyword) and node.arg == ELEVATE:
            values.append(node.value)
    found = []
    for value in values:
        assert isinstance(value, ast.Constant) and isinstance(value.value, str), (
            f"line {value.lineno}: an elevation value that is not a string constant"
        )
        found.append(value.value)
    return found


def derived_argv0():
    """The `argv[0]` values the stand-in receives from the calls the tests make: the word deploy.sh
    puts after the elevation command, when the tests give the setting empty, and the first word of
    each non-empty elevation command the tests give."""
    text = DEPLOY.read_text(encoding="utf-8")
    shape = re.compile(
        re.escape('"${HOST_CMD[@]}" ${ELEVATE_CMD[@]+"${ELEVATE_CMD[@]}"} ') + r"(\S+) -c "
    )
    found = shape.findall(text)
    assert len(found) == 1, f"deploy.sh makes {len(found)} host calls of the shape read: {found}"
    values = elevate_values(ast.parse(SOURCE.read_text(encoding="utf-8")))
    assert values, "no test environment gives the elevation setting"
    argv0 = set()
    for value in values:
        words = value.split()
        argv0.add(words[0] if words else found[0])
    return argv0


def write_script(path, text):
    path.write_text(text, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


SHIM = '#!/bin/bash\necho "reached $0" >> "$RECORD"\nexit 0\n'


class TheAllowedShapes(unittest.TestCase):
    def test_the_list_lives_in_one_place_and_equals_the_shapes_the_calls_make(self):
        listed = getattr(deploy_tests, "HOST_ALLOWED", None)
        self.assertIsNotNone(listed, "the stand-in's allowed shapes are not listed: HOST_ALLOWED")
        self.assertIsInstance(listed, tuple, "HOST_ALLOWED is one tuple")
        self.assertEqual(len(listed), len(set(listed)), f"a shape is listed twice: {listed}")
        derived = derived_argv0()
        self.assertEqual(set(listed), derived, "the list is not the set of argv[0] the tests make")
        # The stand-in logs each argv[0] it is given: the set it received over a deploy, a second
        # deploy and a rollback is the listed set, so a new shape fails here until it is listed.
        with tempfile.TemporaryDirectory() as tmp:
            w = deploy_tests.World(tmp)
            for tag in ("v1.0.0", "v1.1.0"):
                w.ship(tag)
                done = w.deploy(tag)
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            done = w.rollback("v1.0.0")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            record = w.log / "host-argv0.log"
            self.assertTrue(record.is_file(), "the stand-in logged no argv[0]")
            seen = examined("host call(s)", record.read_text(encoding="utf-8").splitlines())
        self.assertEqual(set(seen), set(listed), "the stand-in received a shape that is not listed")

    def test_the_only_stub_that_runs_its_first_argument_is_the_host_stand_in(self):
        tree = ast.parse(SOURCE.read_text(encoding="utf-8"))
        runs_argv = re.compile(
            r"^\s*(?:(?:exec|command|eval|env|nohup|builtin)\s+)*\"?\$(?:@|\{@\}|1|\{1\})\"?(?=\s|$)",
            re.MULTILINE,
        )
        names = set()
        scanned = 0
        for node in ast.walk(tree):
            if not isinstance(node, ast.Assign):
                continue
            for part in ast.walk(node.value):
                if isinstance(part, ast.Constant) and isinstance(part.value, str):
                    scanned += 1
                    if runs_argv.search(part.value):
                        names |= {t.id for t in node.targets if isinstance(t, ast.Name)}
        examined("string constant(s)", range(scanned))
        self.assertEqual(names, {"HOST"}, f"stubs that run their argv: {sorted(names)}")


class TheStandInRefusesWhatItDoesNotKnow(unittest.TestCase):
    @staticmethod
    def members(allowed):
        names = list(PRIVILEGE) + list(ARBITRARY)
        for word in sorted(allowed):
            names += [
                f"/bin/{word}",
                f"/usr/bin/{word}",
                f"./{word}",
                word.upper(),
                word.capitalize(),
                word + "2",
                word + " ",
                " " + word,
                word + "\t",
                word[:-1],
                word + "*",
                f"[{word[0]}]{word[1:]}",
                f"-{word}",
            ]
        names += ["", "-c", "--", "*", "?"]
        return sorted(set(names))

    def test_a_command_outside_the_list_is_refused_by_name_and_run_by_nothing(self):
        host = getattr(deploy_tests, "HOST", None)
        self.assertIsInstance(host, str, "the stand-in's text is HOST")
        allowed = derived_argv0()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            shims = root / "shims"
            log = root / "log"
            for directory in (shims, log):
                directory.mkdir()
            stand_in = root / "host"
            write_script(stand_in, host)
            record = root / "reached"
            names = self.members(allowed)
            for name in names:
                if name and "/" not in name and name not in (".", ".."):
                    write_script(shims / name, SHIM)
            env = {
                "PATH": f"{shims}:{os.environ['PATH']}",
                "STUB_LOG": str(log),
                "RECORD": str(record),
            }
            for name in examined("argv[0] member(s) outside the list", names):
                argv = [str(stand_in), name, "-c", f"echo reached-direct >> {record}", "x"]
                done = subprocess.run(
                    argv,
                    cwd=shims,
                    env=env,
                    stdin=subprocess.DEVNULL,
                    capture_output=True,
                    text=True,
                    timeout=60,
                    check=False,
                )
                if record.exists():
                    self.fail(f"{name!r}: a program was run: {record.read_text()}")
                self.assertNotEqual(done.returncode, 0, f"{name!r}: the stand-in did not refuse")
                self.assertEqual(done.stdout, "", f"{name!r}: the stand-in printed on stdout")
                self.assertEqual(done.stderr, refusal(name), f"{name!r}: not one line naming it")

    def test_a_listed_command_is_run(self):
        host = getattr(deploy_tests, "HOST", None)
        self.assertIsInstance(host, str, "the stand-in's text is HOST")
        listed = getattr(deploy_tests, "HOST_ALLOWED", None)
        self.assertIsNotNone(listed, "the stand-in's allowed shapes are not listed: HOST_ALLOWED")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "log").mkdir()
            stand_in = root / "host"
            write_script(stand_in, host)
            for word in examined("listed shape(s)", listed):
                record = root / f"ran-{word}"
                done = subprocess.run(
                    [str(stand_in), word, "-c", f'echo ran > "{record}"', "x"],
                    env={"PATH": os.environ["PATH"], "STUB_LOG": str(root / "log")},
                    stdin=subprocess.DEVNULL,
                    capture_output=True,
                    text=True,
                    timeout=60,
                    check=False,
                )
                self.assertEqual(done.returncode, 0, f"{word}: {done.stderr}")
                self.assertTrue(record.is_file(), f"{word}: the listed command was not run")


class TheFallbackIsNeverReachedByTheTests(unittest.TestCase):
    def test_an_empty_elevation_setting_runs_the_host_step_with_no_privilege_command(self):
        with tempfile.TemporaryDirectory() as tmp:
            w = deploy_tests.World(tmp)
            shims = Path(tmp) / "shims"
            shims.mkdir()
            record = Path(tmp) / "reached"
            for name in PRIVILEGE:
                write_script(shims / name, SHIM)
            w.ship("v1.0.0")
            done = w.deploy(
                "v1.0.0",
                PATH=f"{shims}:{w.env['PATH']}",
                RECORD=str(record),
                DECKSTREAK_DEPLOY_ELEVATE="",
            )
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertFalse(record.exists(), f"a privilege command was reached: {record}")
            argv0 = w.log / "host-argv0.log"
            self.assertTrue(argv0.is_file(), "the stand-in logged no argv[0]")
            seen = examined("host call(s)", argv0.read_text(encoding="utf-8").splitlines())
            self.assertEqual(set(seen), derived_argv0(), "a host step ran behind a command")


class EveryEnvironmentNamesTheSetting(unittest.TestCase):
    UNNAMED_SHAPES = 0

    @staticmethod
    def near_misses():
        """Names one edit from the setting, generated from its text, none of which is it."""
        names = {ELEVATE.lower(), ELEVATE + "2", "X" + ELEVATE, ELEVATE[:-1], ELEVATE[1:]}
        for i in range(len(ELEVATE)):
            names.add(ELEVATE[:i] + ELEVATE[i + 1 :])
            names.add(ELEVATE[:i] + "X" + ELEVATE[i + 1 :])
        names.discard(ELEVATE)
        return sorted(names)

    def test_a_call_whose_environment_leaves_the_setting_unset_is_refused_by_name(self):
        launch = getattr(deploy_tests, "launch", None)
        self.assertTrue(callable(launch), "no one helper starts a deploy script: launch")
        with tempfile.TemporaryDirectory() as tmp:
            absent = str(Path(tmp) / "absent-program")
            held = Path(tmp) / "held.bash"
            held.write_text("OTHER=1\n# " + ELEVATE + "=x\n", encoding="utf-8")
            unset = [{}, {"PATH": "/usr/bin"}, {b"PATH": b"/usr/bin"}, [], [b"A=b"]]
            unset.append([ELEVATE.encode()])
            for name in self.near_misses():
                unset.append({name: ""})
                unset.append([name.encode() + b"="])
            for environment in examined("environment(s) that leave the setting unset", unset):
                with self.assertRaises(AssertionError, msg=repr(environment)) as caught:
                    launch([absent], environment)
                self.assertIn(ELEVATE, str(caught.exception), "the refusal does not name it")
            with self.assertRaises(AssertionError, msg="a sourced file that names another"):
                launch([absent], [], sourced=held)
            named = [
                {ELEVATE: ""},
                {ELEVATE: "x"},
                {ELEVATE.encode(): b""},
                [ELEVATE.encode() + b"="],
                [b"A=b", ELEVATE.encode() + b"=x"],
            ]
            for environment in examined("environment(s) that name the setting", named):
                with self.assertRaises(FileNotFoundError, msg=repr(environment)):
                    launch([absent], environment)
            held.write_text(ELEVATE + "=''\n", encoding="utf-8")
            with self.assertRaises(FileNotFoundError, msg="a sourced file that names it"):
                launch([absent], [], sourced=held)

    def test_the_shared_environment_names_the_setting(self):
        launch = getattr(deploy_tests, "launch", None)
        self.assertTrue(callable(launch), "no one helper starts a deploy script: launch")
        tree = ast.parse(SOURCE.read_text(encoding="utf-8"))
        shared = [
            node.value
            for node in ast.walk(tree)
            if isinstance(node, ast.Assign)
            and any(
                isinstance(t, ast.Attribute) and t.attr == "env" and isinstance(t.value, ast.Name)
                for t in node.targets
            )
            and isinstance(node.value, ast.Dict)
        ]
        self.assertEqual(len(shared), 1, "the tests build one shared environment")
        keys = {k.value for k in shared[0].keys if isinstance(k, ast.Constant)}
        self.assertIn(ELEVATE, keys, "the shared environment does not name the setting")
        with tempfile.TemporaryDirectory() as tmp:
            w = deploy_tests.World(tmp)
            self.assertIn(ELEVATE, w.env, "a world's environment does not name the setting")
            with self.assertRaises(FileNotFoundError):
                launch([str(Path(tmp) / "absent-program")], w.env)


class NoOtherCallSiteStartsAProgram(unittest.TestCase):
    STARTERS = {
        "subprocess": {"run", "Popen", "call", "check_call", "check_output", "getoutput"},
        "os": {"system", "popen"},
    }
    ALLOWED = {
        "launch": "every deploy script, through the helper that refuses an unnamed environment",
        "World.git": "git, in a synthetic repository",
        "NoDeployScriptNamesAPrivateValue.test_no_deploy_script_names_a_private_value": (
            "the public scrub, on a copy of a file"
        ),
    }

    def starters(self, tree):
        """Each call that can start a program, by the function that holds it."""
        found = {}

        def visit(node, scope):
            if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
                scope = [*scope, node.name]
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
                base = node.func.value
                if isinstance(base, ast.Name) and base.id in self.STARTERS:
                    named = node.func.attr in self.STARTERS[base.id]
                    if named or (base.id == "os" and node.func.attr.startswith(("exec", "spawn"))):
                        found.setdefault(".".join(scope), []).append(node.lineno)
            for child in ast.iter_child_nodes(node):
                visit(child, scope)

        visit(tree, [])
        return found

    def test_only_the_helper_the_git_wrapper_and_the_scrub_call_start_a_program(self):
        tree = ast.parse(SOURCE.read_text(encoding="utf-8"))
        imports = [
            node
            for node in ast.walk(tree)
            if (isinstance(node, ast.ImportFrom) and node.module in ("subprocess", "os", "pty"))
            or (isinstance(node, ast.Import) and any(a.asname for a in node.names))
        ]
        self.assertEqual(
            [n.lineno for n in imports], [], "a name that starts a program is imported bare"
        )
        found = self.starters(tree)
        self.assertEqual(
            set(found),
            set(self.ALLOWED),
            "a call site that starts a program is not the helper, the git wrapper or the scrub",
        )
        examined(
            "call site(s) that start a program", [s for lines in found.values() for s in lines]
        )

    def test_the_helper_names_the_setting_before_it_starts_anything(self):
        tree = ast.parse(SOURCE.read_text(encoding="utf-8"))
        helper = [
            node
            for node in ast.walk(tree)
            if isinstance(node, ast.FunctionDef) and node.name == "launch"
        ]
        self.assertEqual(len(helper), 1, "no one helper starts a deploy script: launch")
        named = [
            n.lineno
            for n in ast.walk(helper[0])
            if isinstance(n, ast.Call)
            and isinstance(n.func, ast.Name)
            and n.func.id == "setting_names"
        ]
        starts = [
            n.lineno
            for n in ast.walk(helper[0])
            if isinstance(n, ast.Call)
            and isinstance(n.func, ast.Attribute)
            and n.func.attr == "Popen"
        ]
        raises = [n for n in ast.walk(helper[0]) if isinstance(n, ast.Raise)]
        self.assertEqual(len(named), 1, "the helper reads the environment's names once")
        self.assertEqual(len(starts), 1, "the helper starts one program")
        self.assertTrue(raises, "the helper refuses")
        self.assertLess(named[0], starts[0], "the helper starts a program before it checks")
        self.assertLess(
            min(r.lineno for r in raises), starts[0], "the refusal comes after the start"
        )


if __name__ == "__main__":
    unittest.main()
