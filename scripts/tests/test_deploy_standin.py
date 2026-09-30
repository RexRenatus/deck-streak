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
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

import _standin_checks as checks
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


def derived_argv0():
    """The `argv[0]` values the stand-in receives from the calls the tests make."""
    return checks.derived_argv0(
        DEPLOY.read_text(encoding="utf-8"), SOURCE.read_text(encoding="utf-8")
    )


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
        source = SOURCE.read_text(encoding="utf-8")
        examined("string constant(s)", range(checks.check_stubs(source)))


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


def tests_texts():
    """Every module beside this one, by name, as text."""
    return {
        p.stem: p.read_text(encoding="utf-8") for p in sorted(Path(__file__).parent.glob("*.py"))
    }


class NoOtherCallSiteStartsAProgram(unittest.TestCase):
    def test_only_the_helper_the_git_wrapper_and_the_scrub_call_start_a_program(self):
        sites = checks.check_census(tests_texts(), deploy_tests.__name__)
        examined("call site(s) that start a program", sites)

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


# --- the killers: a generated population per check, each member planted into a copy of the text
# the check reads; the REAL check must refuse every member, and the failure names each it passed.

STUB_HEAD = '#!/bin/bash\necho "planted $*" >> "$STUB_LOG/planted.log"\n'
# Each route by which a stub's body can run what its arguments name (members.txt's routes; the one
# route that hands the arguments to the shell's evaluator is closed by construction, not planted).
STUB_ROUTES = {
    "a semicolon after the list": '"$@"; true',
    "a brace group": '{ "$@"; }',
    "the last pipeline stage": 'echo x | "$@"',
    "NUL-separated into xargs env": "printf '%s\\0' \"$@\" | xargs -0 env",
    "sh -c of the joined arguments": 'sh -c "$*"',
    "bash -c of the first": 'bash -c "$1"',
    "source of the first": "source $1",
    "the dot builtin on the first": '. "$1"',
    "a command substitution": 'out=$("$@")',
    "a here-string into bash": 'bash <<<"$*"',
    "exec --": 'exec -- "$@"',
    "exec -a": 'exec -a stand "$@"',
    "env -i": 'env -i "$@"',
    "the time keyword": 'time "$@"',
    "the condition of an if": 'if "$@"; then :; fi',
    "an array copy, expanded": 'args=("$@"); "${args[@]}"',
    "a slice from 1": '"${@:1}"',
    "python runpy on the first": (
        "exec python3 -c 'import runpy, sys; runpy.run_path(sys.argv[1])' \"$@\""
    ),
    "exec of the quoted list": 'exec "$@"',
    "the bare quoted list": '"$@"',
    "exec of the unquoted list": "exec $@",
    "the first pipeline stage": '"$@" | cat',
    "the command prefix": 'command "$@"',
    "the nohup prefix": 'nohup "$@"',
}
# Each way a stub's text can be built or installed, round one positive body.
POSITIVE = 'exec "$@"'
STUB_FORMS = {
    "an annotated assignment": f'PLANT: str = r"""{STUB_HEAD}{POSITIVE}\n"""',
    "passed straight to script()": f'_w.script("planted", r"""{STUB_HEAD}{POSITIVE}\n""")',
    "bytes": f'PLANT = b"""{STUB_HEAD}{POSITIVE}\n"""',
    "concatenated from pieces": "PLANT = \"#!/bin/bash\\nexec \" + '\"$' + '@\"\\n'",
    "percent-formatted": 'PLANT = "#!/bin/bash\\nexec %s\\n" % \'"$@"\'',
    "str.format": 'PLANT = "#!/bin/bash\\nexec {}\\n".format(\'"$@"\')',
    "joined lines": 'PLANT = "\\n".join(["#!/bin/bash", \'exec "$\' + \'@"\'])',
    "a placeholder replaced": (
        'PLANT = "#!/bin/bash\\nexec @X@\\n".replace("@X@", \'"$\' + \'@"\')'
    ),
    "an f-string over a built name": (
        '_arg = "$" + "@"\nPLANT = f\'#!/bin/bash\\nexec "{_arg}"\\n\''
    ),
    "a dollar before an unreadable piece": (
        'PLANT = "#!/bin/bash\\nexec \\"$" + _piece() + "\\"\\n"'
    ),
}


def python_trace_text(source):
    """The text of the in-tree stub PYTHON_TRACE, read from the tree."""
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id == "PYTHON_TRACE" for t in node.targets
        ):
            return ast.literal_eval(node.value)
    raise AssertionError("PYTHON_TRACE is not in the module")


def stub_members(source):
    members = []
    for label, line in STUB_ROUTES.items():
        body = f'\nPLANT = r"""{STUB_HEAD}{line}\n"""\n'
        members.append((f"route: {label}", source + body))
    for label, text in STUB_FORMS.items():
        members.append((f"form: {label}", source + "\n" + text + "\n"))
    traced = python_trace_text(source)
    members.append(("the in-tree PYTHON_TRACE route", source + f'\nPLANT = r"""{traced}"""\n'))
    return members


def report(kind, passed, total):
    return f"{len(passed)} of {total} {kind} member(s) passed the check: {passed}"


class TheStubScanFindsEveryRoute(unittest.TestCase):
    def test_every_route_by_which_a_stub_runs_its_arguments_is_refused(self):
        source = SOURCE.read_text(encoding="utf-8")
        checks.check_stubs(source)
        harmless = source + f'\nPLANT = r"""{STUB_HEAD}exit 0\n"""\n'
        checks.check_stubs(harmless)
        members = list(examined("stub member(s)", stub_members(source)))
        passed = []
        for label, text in members:
            try:
                checks.check_stubs(text)
            except AssertionError:
                continue
            passed.append(label)
        self.assertGreaterEqual(len(members), 20, "the population shrank")
        self.assertEqual(passed, [], report("stub", passed, len(members)))


# One extra host call, spelled one way, placed after the call deploy.sh makes.
HOST_CALL_PLANTS = {
    "the host array unquoted, then the elevation, then sh -c": (
        '${HOST_CMD[@]} ${ELEVATE_CMD[@]+"${ELEVATE_CMD[@]}"} sh -c "$script" deck-streak-host'
    ),
    "a heredoc into sh -s": (
        '"${HOST_CMD[@]}" ${ELEVATE_CMD[@]+"${ELEVATE_CMD[@]}"} sh -s <<\'PLANT\'\n:\nPLANT'
    ),
    "the host variable, no array": '$DECKSTREAK_DEPLOY_HOST sh -c "$script" deck-streak-host',
    "the host array, no elevation": '"${HOST_CMD[@]}" sh -c "$script" deck-streak-host',
    "a literal privilege word": '"${HOST_CMD[@]}" sudo -n bash -c "$script" deck-streak-host',
    "the elevation array fully quoted": '"${HOST_CMD[@]}" "${ELEVATE_CMD[@]}" sh -c "$script"',
    "a function wrapper called with python3": (
        'plant_host() { "${HOST_CMD[@]}" "$@"; }\nplant_host python3 render.py'
    ),
}
# One statement that gives the elevation setting the value `sudo`, in the module the tests read.
ELEVATION_PLANTS = {
    "a dict keyed by the ELEVATE name": '_e = {ELEVATE: "sudo"}',
    "a subscript with the constant key": '_e = {}\n_e["DECKSTREAK_DEPLOY_ELEVATE"] = "sudo"',
    "a subscript with the ELEVATE name": "_e = {}\n_e[ELEVATE] = 'sudo'",
    "a bytes NAME=value entry": '_e = [b"DECKSTREAK_DEPLOY_ELEVATE=sudo"]',
    "a str NAME=value entry": '_e = ["DECKSTREAK_DEPLOY_ELEVATE=sudo"]',
    "a dict key built by concatenation": '_e = {"DECKSTREAK_DEPLOY_" + "ELEVATE": "sudo"}',
    "setdefault": '_e = {}\n_e.setdefault("DECKSTREAK_DEPLOY_ELEVATE", "sudo")',
    "a line in a sourced file": (
        '_held.write_text("DECKSTREAK_DEPLOY_ELEVATE=sudo\\n", encoding="utf-8")'
    ),
    "an assignment into os.environ": 'os.environ["DECKSTREAK_DEPLOY_ELEVATE"] = "sudo"',
}


def derivation_members(deploy, source):
    call = next(
        line for line in deploy.splitlines() if line.lstrip().startswith('"${HOST_CMD[@]}"')
    )
    indent = call[: len(call) - len(call.lstrip())]
    members = []
    for label, plant in HOST_CALL_PLANTS.items():
        extra = "\n".join(indent + line for line in plant.splitlines())
        members.append((f"host call: {label}", (deploy.replace(call, call + "\n" + extra), source)))
    for label, plant in ELEVATION_PLANTS.items():
        members.append((f"elevation value: {label}", (deploy, source + "\n" + plant + "\n")))
    return members


class TheDerivationReadsEveryHostCallAndEveryValue(unittest.TestCase):
    def test_every_host_call_and_every_elevation_value_adds_to_the_set_or_is_refused(self):
        deploy = DEPLOY.read_text(encoding="utf-8")
        source = SOURCE.read_text(encoding="utf-8")
        base = checks.derived_argv0(deploy, source)
        self.assertEqual(base, set(deploy_tests.HOST_ALLOWED), "the control is not the list")
        members = list(examined("derivation member(s)", derivation_members(deploy, source)))
        passed = []
        for label, (planted_deploy, planted_source) in members:
            try:
                got = checks.derived_argv0(planted_deploy, planted_source)
            except AssertionError:
                continue
            if got == base:
                passed.append(label)
        self.assertGreaterEqual(len(members), 16, "the population shrank")
        self.assertEqual(passed, [], report("derivation", passed, len(members)))


def census_members(texts):
    source = texts[deploy_tests.__name__]
    standin = texts[Path(__file__).stem]
    starts = {
        "subprocess.getstatusoutput": 'subprocess.getstatusoutput("bash deploy/deploy.sh")',
        "os.posix_spawn": 'os.posix_spawn("/bin/bash", ["bash", "deploy/deploy.sh"], {})',
        "os.posix_spawnp": 'os.posix_spawnp("bash", ["bash", "deploy/deploy.sh"], {})',
        "pty.spawn after a plain import": 'import pty\n        pty.spawn(["bash", "deploy/deploy.sh"])',
        "asyncio.create_subprocess_exec": (
            "import asyncio\n        asyncio.create_subprocess_exec('bash', 'deploy/deploy.sh')"
        ),
        "asyncio.create_subprocess_shell": (
            "import asyncio\n        asyncio.create_subprocess_shell('bash deploy/deploy.sh')"
        ),
        "getattr of the module": 'getattr(subprocess, "run")(["bash", "deploy/deploy.sh"])',
        "functools.partial of the starter": (
            'import functools\n        functools.partial(subprocess.run, ["bash", "x"])()'
        ),
        "the starter bound to a local name": (
            'start = subprocess.run\n        start(["bash", "deploy/deploy.sh"])'
        ),
        "importlib.import_module": (
            'import importlib\n        importlib.import_module("subprocess").run(["bash", "x"])'
        ),
        "__import__": '__import__("subprocess").run(["bash", "deploy/deploy.sh"])',
        "a git alias through World.git": (
            'w.git("config", "alias.go", "!bash deploy/deploy.sh", cwd=w.tmp)\n'
            '        w.git("go", cwd=w.tmp)'
        ),
    }
    members = []
    for label, start in starts.items():
        planted = f"\n\nclass PlantedStart:\n    def test_it(self):\n        {start}\n"
        members.append((f"start: {label}", {**texts, deploy_tests.__name__: source + planted}))
    anchor = '        assert done.returncode == 0, f"git {args}: {done.stderr}"\n'
    assert source.count(anchor) == 1, "the git wrapper's anchor moved"
    second = source.replace(
        anchor, anchor + '        subprocess.run(["bash", "deploy/deploy.sh"])\n'
    )
    members.append(
        ("start: a second start inside World.git", {**texts, deploy_tests.__name__: second})
    )
    planted = "\n\nclass PlantedStart:\n    def test_it(self):\n        subprocess.run(['bash'])\n"
    members.append(
        ("start: written in the standin module", {**texts, Path(__file__).stem: standin + planted})
    )
    importer = (
        "import subprocess\nfrom test_deploy_scripts import launch\n\n"
        "subprocess.run(['bash', 'deploy/deploy.sh'])\n"
    )
    members.append(
        ("start: a new module that imports the deploy tests", {**texts, "test_zz_new": importer})
    )
    named = 'import importlib\n\nimportlib.import_module("test_deploy_scripts")\n'
    members.append(
        (
            "start: a module that names the deploy tests without importing",
            {**texts, "test_zz_named": named},
        )
    )
    return members


class TheCensusFindsEveryStart(unittest.TestCase):
    def test_every_start_spelling_is_found_or_refused(self):
        texts = tests_texts()
        checks.check_census(texts, deploy_tests.__name__)
        members = list(examined("start member(s)", census_members(texts)))
        passed = []
        for label, planted in members:
            try:
                checks.check_census(planted, deploy_tests.__name__)
            except AssertionError:
                continue
            passed.append(label)
        self.assertGreaterEqual(len(members), 16, "the population shrank")
        self.assertEqual(passed, [], report("launch census", passed, len(members)))


class TheLaunchRefusalJudgesWhatTheProgramSees(unittest.TestCase):
    def test_an_environment_the_program_will_see_unset_is_refused(self):
        launch = deploy_tests.launch
        named = {ELEVATE: ""}
        with tempfile.TemporaryDirectory() as tmp:
            absent = str(Path(tmp) / "absent-program")
            files = {}
            for label, body in (
                ("a dead branch", f"if false; then\n{ELEVATE}=x\nfi\n"),
                ("set, then unset", f"{ELEVATE}=x\nunset {ELEVATE}\n"),
            ):
                path = Path(tmp) / f"{len(files)}.bash"
                path.write_text(body, encoding="utf-8")
                files[label] = path
            members = [
                (
                    "received names it; env is a mapping without it",
                    [absent],
                    named,
                    {"PATH": "/usr/bin"},
                    None,
                ),
                (
                    "received names it; env is a bytes mapping without it",
                    [absent],
                    named,
                    {b"PATH": b"/usr/bin"},
                    None,
                ),
                ("received names it; env=None, so the child inherits", [absent], named, None, None),
                (
                    "the sourced file names it only in a dead branch",
                    [absent],
                    [],
                    {},
                    files["a dead branch"],
                ),
                (
                    "the sourced file sets it, then unsets it",
                    [absent],
                    [],
                    {},
                    files["set, then unset"],
                ),
            ]
            members = list(examined("launch member(s)", members))
            admitted = []
            for label, argv, received, env, sourced in members:
                try:
                    launch(argv, received, env=env, sourced=sourced)
                except AssertionError:
                    continue
                except FileNotFoundError:
                    admitted.append(label)
                    continue
                admitted.append(label + " (started something)")
            self.assertGreaterEqual(len(members), 5, "the population shrank")
            self.assertEqual(admitted, [], report("launch", admitted, len(members)))


if __name__ == "__main__":
    unittest.main()
