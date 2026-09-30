"""Every refusal a missing tool makes is read WHOLE (SPEC-039 A53, ADR-291, issue #431).

THE CLASS. A refusal is decided by its exact text and by the branch that chooses it. A test that
asserts only that a refusal happened, or that its line holds a word of the reason, leaves every
mutant of the text and of the branch alive: the generated mutants of `resolve_tool`, `_backstop`,
`_exit_refusal` and `run_tool` are exactly those. Each member below is one (reason, spawn route)
and asserts the whole `ToolMissing` (its tool and its complete message), never a part of it.

THE POPULATION is generated, not listed. The reasons are the modes of an executable the runner
cannot run (absent by a bare name, absent by a path, not executable, a directory); the spawn
errors are every errno the operating system names, each raised at the spawn by the spawn's own
error type; the exits are every value a process can end with, 0 to 255. Each is crossed with the
three routes that spawn (`resolve_tool` itself, `run_tool`, `run_in_own_group`), and the member
count is printed and asserted, so a route or a reason that leaves the table is a failure.
"""

import errno
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined

sys.path.insert(0, str(REPO / "scripts"))

import mutation_rows as runner  # noqa: E402

TOOL = "refusal-probe-tool"
#: (the reason, how the tool is set up in a directory on the child's PATH or named by a path).
REASONS = {
    "not found on PATH": "absent by name",
    "no such file": "absent by path",
    "not executable": "present without the execute bit",
    "is a directory": "a directory at the name",
}
ROUTES = ("resolve_tool", "run_tool", "run_tool checked", "run_in_own_group")
#: The errnos the operating system names, each one a way a spawn can fail.
ERRNOS = sorted(errno.errorcode)
EXITS = range(256)
REFUSED_EXITS = {
    126: "exit 126: a program it needs cannot be run",
    127: "exit 127: a program it needs is not found",
}


def place(root, reason):
    """Set `TOOL` up in `root` so that resolving it gives `reason`; the command that names it."""
    if reason == "not found on PATH":
        return [TOOL]
    if reason == "no such file":
        return [str(root / "no" / TOOL)]
    target = root / TOOL
    if reason == "is a directory":
        target.mkdir()
    else:
        target.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        target.chmod(0o644)
    return [TOOL]


def go(route, command, env, cwd):
    """Reach the spawn by `route` with `command` and `env`, as the runner's own callers do."""
    if route == "resolve_tool":
        return runner.resolve_tool(command, env)
    if route == "run_tool":
        return runner.run_tool(command, env=env, cwd=cwd, capture_output=True)
    if route == "run_tool checked":
        return runner.run_tool(command, env=env, cwd=cwd, capture_output=True, check=True)
    return runner.run_in_own_group(command, cwd=cwd, env=env, timeout=30)


#: The `PATH` entries a spawn reads in the child's working directory, not the runner's.
RELATIVE_ENTRIES = ("", ".", "rel")
#: What sits at a candidate: nothing, a program that runs, a file without the execute bit, a directory.
STATES = ("absent", "runnable", "not executable", "a directory")
#: The reason a candidate in a state gives when no later candidate runs.
WHY_STATE = {"not executable": "not executable", "a directory": "is a directory"}
#: The routes that hand the spawn a working directory.
CWD_ROUTES = ("run_tool", "run_tool checked", "run_in_own_group")


#: A file the resolver's check passes (a regular file with the execute bit) that the kernel will not
#: run, and the reason its spawn gives. The spawn's own search passes over such a file and starts a
#: LATER candidate, so the file the runner judged would not be the file the spawn ran.
KERNEL_REFUSES = {
    "a bad interpreter line": (b"#!/nonexistent/interpreter-431\n", "No such file or directory"),
    "an empty file": (b"", "Exec format error"),
    "an unknown format": (b"\x7fELF\x02\x01\x01not-an-image\n", "Exec format error"),
    "an interpreter without the execute bit": (None, "Permission denied"),
}
#: Where the file the kernel will not run sits: its PATH entry, and the directory it names.
KERNEL_POSITIONS = ("an absolute entry", "a relative entry", "an empty entry")


def plant(path, state, label):
    """Put a candidate in `state` at `path`; a runnable one prints `label`."""
    if state == "absent":
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    if state == "a directory":
        path.mkdir()
        return
    path.write_text(f"#!/bin/sh\necho {label}\n", encoding="utf-8")
    path.chmod(0o755 if state == "runnable" else 0o644)


def outcome(call):
    """What `call` ended with, as one value: ("returned", its code), or ("raised", the type, the
    text, the tool, the cause's type, the code) for ANY exception, so a test compares it whole."""
    try:
        done = call()
    except Exception as error:  # noqa: BLE001 - every way out is a value to compare
        cause = type(error.__cause__).__name__ if error.__cause__ is not None else None
        return (
            "raised",
            type(error).__name__,
            str(error),
            getattr(error, "tool", None),
            cause,
            getattr(error, "returncode", None),
        )
    return ("returned", getattr(done, "returncode", None))


def executable(path, exit_code=0):
    """An executable script at `path` that ends with `exit_code`."""
    path.write_text(f"#!/bin/sh\nexit {exit_code}\n", encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


class TheRefusalIsReadWhole(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name).resolve()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.env = {"PATH": str(self.bin)}
        self.other = {"PATH": str(self.root / "elsewhere")}

    def test_each_reason_is_refused_whole_at_every_route(self):
        members = [(reason, route) for reason in REASONS for route in ROUTES]
        examined("refusal member(s)", members)
        self.assertEqual(len(members), len(REASONS) * len(ROUTES))
        for reason, route in members:
            with self.subTest(reason=reason, route=route):
                root = self.root / f"{list(REASONS).index(reason)}-{ROUTES.index(route)}"
                root.mkdir()
                bin_dir = root / "bin"
                bin_dir.mkdir()
                command = place(bin_dir, reason)
                if command[0].startswith(str(bin_dir.parent)):
                    command = [str(root / "no" / TOOL)]
                with self.assertRaises(runner.ToolMissing) as raised:
                    go(route, command, {"PATH": str(bin_dir)}, root)
                self.assertEqual(raised.exception.tool, command[0])
                self.assertEqual(str(raised.exception), f"missing tool: {command[0]}: {reason}")

    def test_a_name_holding_a_slash_is_the_path_itself_and_a_bare_name_is_searched(self):
        # A bare name is found on the child's PATH and never looked for in the working directory.
        executable(self.bin / TOOL)
        runner.resolve_tool([TOOL], self.env)
        elsewhere = self.root / "elsewhere"
        elsewhere.mkdir()
        executable(elsewhere / TOOL)
        with self.assertRaises(runner.ToolMissing) as raised:
            runner.resolve_tool([TOOL], {"PATH": str(self.root / "empty")})
        self.assertEqual(str(raised.exception), f"missing tool: {TOOL}: not found on PATH")
        # A name with a slash ignores PATH: the file it names is the one, found or not.
        runner.resolve_tool([str(elsewhere / TOOL)], self.env)
        with self.assertRaises(runner.ToolMissing) as raised:
            runner.resolve_tool([str(self.bin / "sub" / TOOL)], self.env)
        self.assertEqual(
            str(raised.exception), f"missing tool: {self.bin / 'sub' / TOOL}: no such file"
        )
        print("examined 5 name-shape member(s)")

    def test_an_empty_or_dot_path_entry_is_the_working_directory(self):
        here = self.root / "here"
        here.mkdir()
        executable(here / TOOL)
        before = Path.cwd()
        os.chdir(here)
        self.addCleanup(os.chdir, before)
        entries = ("", ".", os.pathsep + str(self.root / "empty"), str(self.root / "gone") + ":")
        for entry in entries:
            with self.subTest(path=entry):
                runner.resolve_tool([TOOL], {"PATH": entry})
        os.chdir(self.root)
        for entry in ("", ".", os.pathsep):
            with self.subTest(path=entry, cwd="without the tool"):
                with self.assertRaises(runner.ToolMissing) as raised:
                    runner.resolve_tool([TOOL], {"PATH": entry})
                self.assertEqual(str(raised.exception), f"missing tool: {TOOL}: not found on PATH")
        print(f"examined {len(entries) + 3} path entry member(s)")

    def test_the_child_environment_decides_and_not_this_process(self):
        executable(self.bin / TOOL)
        with mock.patch.dict(os.environ, {"PATH": str(self.root / "empty")}):
            for route in ROUTES:
                with self.subTest(route=route, source="the given env"):
                    if route in ("run_tool", "run_tool checked", "run_in_own_group"):
                        go(route, [TOOL], self.env, self.root)
                    else:
                        go(route, [TOOL], self.env, self.root)
        with mock.patch.dict(os.environ, {"PATH": str(self.bin)}):
            for route in ROUTES:
                with self.subTest(route=route, source="this process"):
                    with self.assertRaises(runner.ToolMissing) as raised:
                        go(route, [TOOL], self.other, self.root)
                    self.assertEqual(
                        str(raised.exception), f"missing tool: {TOOL}: not found on PATH"
                    )
            # No env given: this process's PATH is the child's.
            runner.resolve_tool([TOOL], None)
            runner.run_tool([TOOL], capture_output=True)
        # An env without a PATH falls back to the default search path.
        runner.resolve_tool(["sh"], {})
        with self.assertRaises(runner.ToolMissing):
            runner.resolve_tool([TOOL], {})

    def test_a_relative_candidate_is_read_in_the_directory_the_child_runs_in(self):
        """The empty, `.` and relative `PATH` entries, and a relative name holding a slash, are read
        by the spawn in the CHILD's working directory (`cwd=`), so resolution reads them there too:
        the file the runner judges is the file the spawn runs, or the refusal names why none runs."""
        slash = "a relative name holding a slash"
        members = [
            (entry, here, there, later, route)
            for entry in RELATIVE_ENTRIES + (slash,)
            for here in STATES
            for there in STATES
            for later in ("absent", "runnable")
            for route in CWD_ROUTES
        ]
        examined("child working directory member(s)", members)
        self.assertEqual(
            len(members), 2 * (len(RELATIVE_ENTRIES) + 1) * len(STATES) ** 2 * len(CWD_ROUTES)
        )
        before = Path.cwd()
        self.addCleanup(os.chdir, before)
        for number, (entry, here, there, later, route) in enumerate(members):
            with self.subTest(
                entry=entry, runner_cwd=here, child_cwd=there, later=later, route=route
            ):
                base = self.root / f"cwd-{number}"
                runner_cwd, child_cwd, later_dir = base / "runner", base / "child", base / "later"
                for directory in (runner_cwd, child_cwd, later_dir):
                    directory.mkdir(parents=True)
                if entry == slash:
                    command, relative = [f"sub/{TOOL}"], Path("sub") / TOOL
                    env, failure = {"PATH": str(later_dir)}, "no such file"
                else:
                    command, relative = [TOOL], Path(entry) / TOOL
                    env, failure = (
                        {"PATH": entry + os.pathsep + str(later_dir)},
                        "not found on PATH",
                    )
                plant(runner_cwd / relative, here, "RUNNER")
                plant(child_cwd / relative, there, "CHILD")
                searched = [(there, "CHILD")]
                # A name holding a slash is never searched on PATH, even where PATH holds it.
                plant(later_dir / (relative if entry == slash else TOOL), later, "LATER")
                if entry != slash:
                    searched.append((later, "LATER"))
                want = ("refused", f"missing tool: {command[0]}: {failure}")
                for state, label in searched:
                    if state == "runnable":
                        want = ("ran", label)
                        break
                    if state in WHY_STATE:
                        want = ("refused", f"missing tool: {command[0]}: {WHY_STATE[state]}")
                os.chdir(runner_cwd)
                try:
                    done = go(route, command, env, child_cwd)
                except runner.ToolMissing as refusal:
                    got = ("refused", str(refusal))
                else:
                    out = done.stdout.decode() if isinstance(done.stdout, bytes) else done.stdout
                    got = ("ran", out.strip())
                finally:
                    os.chdir(before)
                self.assertEqual(got, want)

    def spawned(self, route, env, cwd):
        """What a spawn by `route` ended with: ("ran", what the tool printed) or ("refused", why)."""
        try:
            done = go(route, [TOOL], env, cwd)
        except runner.ToolMissing as refusal:
            return ("refused", str(refusal))
        out = done.stdout.decode() if isinstance(done.stdout, bytes) else done.stdout
        return ("ran", out.strip())

    def test_the_spawn_runs_the_file_it_judged_by_every_route(self):
        """The file the runner judges is the file the spawn runs. A candidate the resolver accepts
        and the kernel will not run, with a runnable copy LATER on PATH, is refused for its own
        reason, and the later copy never runs, at every position and route; and a PATH changed
        between the judgement and the spawn (in the env passed, or in this process's) still runs
        the file judged."""
        members = [
            (kind, where, route)
            for kind in KERNEL_REFUSES
            for where in KERNEL_POSITIONS
            for route in CWD_ROUTES
        ]
        members += [
            ("a PATH changed after the judgement", held, route)
            for held in ("the env passed", "this process's")
            for route in ("run_tool", "run_tool checked")
        ]
        examined("judged-file member(s)", members)
        self.assertEqual(
            len(members), len(KERNEL_REFUSES) * len(KERNEL_POSITIONS) * len(CWD_ROUTES) + 4
        )
        for number, (kind, where, route) in enumerate(members):
            with self.subTest(kind=kind, where=where, route=route):
                base = self.root / f"judged-{number}"
                child, first, later = base / "child", base / "first", base / "later"
                for directory in (child, first, later):
                    directory.mkdir(parents=True)
                plant(later / TOOL, "runnable", "LATER")
                if kind in KERNEL_REFUSES:
                    body, why = KERNEL_REFUSES[kind]
                    if body is None:
                        interpreter = base / "interpreter"
                        interpreter.write_text("#!/bin/sh\n", encoding="utf-8")
                        interpreter.chmod(0o644)
                        body = f"#!{interpreter}\n".encode()
                    entry, directory = {
                        "an absolute entry": (str(first), first),
                        "a relative entry": ("rel", child / "rel"),
                        "an empty entry": ("", child),
                    }[where]
                    directory.mkdir(parents=True, exist_ok=True)
                    (directory / TOOL).write_bytes(body)
                    (directory / TOOL).chmod(0o755)
                    got = self.spawned(route, {"PATH": entry + os.pathsep + str(later)}, child)
                    self.assertEqual(got, ("refused", f"missing tool: {TOOL}: {why}"))
                    continue
                plant(first / TOOL, "runnable", "JUDGED")
                env = {"PATH": str(first)} if where == "the env passed" else None
                real = os.access

                def access(path, mode, *args, env=env, later=later, **kwargs):
                    answer = real(path, mode, *args, **kwargs)
                    if mode == os.X_OK and answer:
                        (env if env is not None else os.environ)["PATH"] = str(later)
                    return answer

                with mock.patch.dict(os.environ, {"PATH": str(first)}):
                    with mock.patch.object(runner.os, "access", access):
                        got = self.spawned(route, env, child)
                self.assertEqual(got, ("ran", "JUDGED"))

    def test_every_errno_at_the_spawn_is_the_same_refusal_naming_the_tool(self):
        members = [(number, route) for number in ERRNOS for route in ("run_tool", "own_group")]
        examined("spawn errno member(s)", members)
        self.assertEqual(len(members), 2 * len(ERRNOS))
        command = [str(executable(self.bin / TOOL)), "second-argument"]
        for number, route in members:
            for given in (command[0], None):
                text = os.strerror(number)
                error = OSError(number, text, given)
                with self.subTest(errno=number, route=route, filename=given):
                    refused = self.spawn_fails(route, command, error)
                    self.assertIsInstance(refused, runner.ToolMissing)
                    self.assertEqual(refused.tool, command[0])
                    self.assertEqual(str(refused), f"missing tool: {command[0]}: {text}")
                    self.assertIs(refused.__cause__, error)

    def test_a_spawn_error_that_names_something_else_is_not_a_refusal(self):
        command = [str(executable(self.bin / TOOL)), "second-argument"]
        for route in ("run_tool", "own_group"):
            for name in (str(self.root), command[1], "", "cwd"):
                error = OSError(errno.ENOENT, os.strerror(errno.ENOENT), name)
                with self.subTest(route=route, filename=name):
                    refused = self.spawn_fails(route, command, error)
                    self.assertIs(refused, error)

    def test_a_spawn_error_without_a_reason_is_the_refusal_that_cannot_be_run(self):
        command = [str(executable(self.bin / TOOL)), "second-argument"]
        for route in ("run_tool", "own_group"):
            for text in (None, ""):
                error = OSError()
                error.strerror, error.filename = text, command[0]
                with self.subTest(route=route, strerror=text):
                    refused = self.spawn_fails(route, command, error)
                    self.assertEqual(str(refused), f"missing tool: {command[0]}: cannot be run")
        error = OSError()
        error.strerror, error.filename = "a stated reason", None
        self.assertEqual(
            str(self.spawn_fails("run_tool", command, error)),
            f"missing tool: {command[0]}: a stated reason",
        )

    def spawn_fails(self, route, command, error):
        """What escapes `route` when the operating system fails the spawn with `error`."""
        if route == "run_tool":
            patched = mock.patch.object(runner.subprocess, "run", side_effect=error)
        else:
            patched = mock.patch.object(runner.subprocess, "Popen", side_effect=error)
        with patched:
            with self.assertRaises((OSError, runner.ToolMissing)) as raised:
                go(
                    route if route == "run_tool" else "run_in_own_group",
                    command,
                    self.env,
                    self.root,
                )
        return raised.exception

    def test_only_the_two_exits_of_a_program_that_cannot_run_are_refusals_at_every_route(self):
        members = [(code, route) for code in EXITS for route in ROUTES[1:]]
        examined("exit member(s)", members)
        self.assertEqual(len(members), len(EXITS) * len(ROUTES[1:]))
        tool = self.bin / TOOL
        for code, route in members:
            executable(tool, code)
            with self.subTest(exit=code, route=route):
                if code in REFUSED_EXITS:
                    with self.assertRaises(runner.ToolMissing) as raised:
                        go(route, [TOOL], self.env, self.root)
                    self.assertEqual(raised.exception.tool, TOOL)
                    self.assertEqual(
                        str(raised.exception), f"missing tool: {TOOL}: {REFUSED_EXITS[code]}"
                    )
                    if route == "run_tool checked":
                        self.assertIsInstance(
                            raised.exception.__cause__, subprocess.CalledProcessError
                        )
                elif route == "run_tool checked" and code:
                    with self.assertRaises(subprocess.CalledProcessError) as raised:
                        go(route, [TOOL], self.env, self.root)
                    self.assertEqual(raised.exception.returncode, code)
                else:
                    done = go(route, [TOOL], self.env, self.root)
                    self.assertEqual(done.returncode, code)

    def test_every_exit_at_every_route_is_one_whole_outcome(self):
        """Each exit at each route ends as ONE value compared whole: the code it returned, or the
        type, text, tool, cause and code of what it raised. A wrong exception, or a refusal
        without its tool, is then an assertion that fails, never an error beside the assertions."""
        members = [(code, route) for code in EXITS for route in ROUTES[1:]]
        examined("whole outcome member(s)", members)
        self.assertEqual(len(members), len(EXITS) * len(ROUTES[1:]))
        tool = self.bin / TOOL
        for code, route in members:
            executable(tool, code)
            with self.subTest(exit=code, route=route):
                if code in REFUSED_EXITS:
                    cause = "CalledProcessError" if route == "run_tool checked" else None
                    text = f"missing tool: {TOOL}: {REFUSED_EXITS[code]}"
                    want = ("raised", "ToolMissing", text, TOOL, cause, None)
                elif route == "run_tool checked" and code:
                    text = str(subprocess.CalledProcessError(code, [TOOL]))
                    want = ("raised", "CalledProcessError", text, None, None, code)
                else:
                    want = ("returned", code)
                self.assertEqual(outcome(lambda: go(route, [TOOL], self.env, self.root)), want)

    def test_the_exit_refusal_is_a_value_of_the_code_and_the_command(self):
        command = [TOOL, "second-argument"]
        table = {
            code: runner._exit_refusal(code, command)
            for code in EXITS  # noqa: SLF001
        }
        self.assertEqual(
            {code for code, made in table.items() if made is not None}, set(REFUSED_EXITS)
        )
        for code, why in REFUSED_EXITS.items():
            self.assertIsInstance(table[code], runner.ToolMissing)
            self.assertEqual(table[code].tool, TOOL)
            self.assertEqual(str(table[code]), f"missing tool: {TOOL}: {why}")
        self.assertEqual(runner.UNRUNNABLE_EXITS, REFUSED_EXITS)


if __name__ == "__main__":
    unittest.main()
