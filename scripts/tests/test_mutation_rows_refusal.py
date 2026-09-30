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
                root = Path(tempfile.mkdtemp(dir=self.root))
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
