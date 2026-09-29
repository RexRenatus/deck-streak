"""A mutants run's memory scope (SPEC-196 A1 to A8 and A17).

`scripts/memory_scope.py` is loaded by path and driven through `run()` and its keyword-only seams,
never through an option, so no test can set the cap. A `sudo` and a `systemctl` that record their
arguments stand in for the privileged calls, and a control-group tree is planted in a temporary
directory in the way `test_memory_watch.py` plants one. No test starts a real scope.
"""

import contextlib
import importlib.util
import io
import json
import os
import re
import stat
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "memory_scope.py"
WORKFLOWS = REPO / ".github" / "workflows"
CI = WORKFLOWS / "ci.yml"
WEEKLY = WORKFLOWS / "mutation-weekly.yml"
PAGE = 4096
EVENTS = "low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\noom_group_kill 0\n"
SCOPED = re.compile(r"python3 scripts/memory_scope\.py --report (\S+) -- cargo mutants ")
OUTPUT = re.compile(r"--output (\S+)")

# The three commands that run tests, from `cargo mutants` to the end of the step's line, as the
# tree held them before this SPEC, and the four listing lines.
CI_LEG = (
    'cargo mutants --no-shuffle -vV --in-place --in-diff "$RUNNER_TEMP/mutation/git.diff" '
    '--sharding round-robin --shard "$SHARD/$SHARDS" --timeout 300 --build-timeout 600 '
    '--output "$out" || rc=$?'
)
WEEKLY_LEG = (
    'cargo mutants --no-shuffle -vV --in-place ${PACKAGE:+--package "$PACKAGE"} '
    '--sharding round-robin --shard "$SHARD/$SHARDS" --timeout 300 --build-timeout 600 '
    '--output "$RUNNER_TEMP/mutation" || rc=$?'
)
REHEARSAL_LEG = (
    "cargo mutants --no-shuffle -vV --in-place -f crates/kernel/src/clock.rs --timeout 300 "
    '--build-timeout 600 --output "$shard" || rc=$?'
)
LISTINGS = {
    CI: [
        'cargo mutants --no-shuffle --list --json --in-diff "$RUNNER_TEMP/mutation/git.diff" '
        '--timeout 300 --build-timeout 600 > "$RUNNER_TEMP/mutation/listed.json"',
        "cargo mutants --no-shuffle --list --json --timeout 300 --build-timeout 600 "
        '> "$RUNNER_TEMP/mutation/whole.json"',
    ],
    WEEKLY: [
        'cargo mutants --no-shuffle --list --json --in-place ${PACKAGE:+--package "$PACKAGE"} '
        '--timeout 300 --build-timeout 600 > "$RUNNER_TEMP/size/package.json"',
        "cargo mutants --no-shuffle --list --json --in-place --timeout 300 --build-timeout 600 "
        '> "$RUNNER_TEMP/listing/whole.json"',
    ],
}


def load():
    """The script's module, or None while the file is absent."""
    if not SCRIPT.is_file():
        return None
    spec = importlib.util.spec_from_file_location("memory_scope", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def cap_of(total_kb, page):
    return total_kb * 1024 * 15 // 16 // page * page


def meminfo_text(total_kb):
    return f"MemTotal:       {total_kb} kB\nMemFree:        123456 kB\nSwapTotal:      0 kB\n"


class Plant:
    """One planted machine: a meminfo, a control-group tree, and a sudo and systemctl that record."""

    def __init__(self, root, total_kb=16777216, page=PAGE, text=None):
        self.root = Path(root)
        self.pid = os.getpid()
        self.unit = f"memory-scope-{self.pid}.scope"
        self.page = page
        self.cap = cap_of(total_kb, page)
        self.meminfo = self.root / "meminfo"
        self.meminfo.write_text(meminfo_text(total_kb) if text is None else text, "utf-8")
        self.proc = self.root / "proc-cgroup"
        self.proc.write_text("0::/user.slice/session.scope\n", "utf-8")
        self.cgroup = self.root / "cgroup"
        self.group = self.cgroup / "system.slice" / self.unit
        self.group.mkdir(parents=True)
        self.files = {
            "memory.max": f"{self.cap}\n",
            "memory.swap.max": "0\n",
            "memory.oom.group": "0\n",
            "memory.events": EVENTS,
            "memory.peak": "0\n",
        }
        self.sudo_log = self.root / "sudo-argv"
        self.systemctl_log = self.root / "systemctl-argv"
        self.unit_group = self.unit
        self.sudo_exit = 0
        self.policy = "continue"
        self.marker = self.root / "marker"
        self.report = self.root / "report"
        self.report.mkdir()

    def build(self):
        for name, text in self.files.items():
            (self.group / name).write_text(text, "utf-8")
        cgroup = f"0::/system.slice/{self.unit_group}\n"
        self.stub(
            "sudo",
            f'printf "%s\\n" "$@" >> "{self.sudo_log}"\n'
            + (f"printf %s '{cgroup}' > \"{self.proc}\"\n" if self.sudo_exit == 0 else "")
            + f"exit {self.sudo_exit}\n",
        )
        self.stub(
            "systemctl",
            f'printf "%s\\n" "$@" >> "{self.systemctl_log}"\nprintf "%s\\n" "{self.policy}"\n',
        )
        return self

    def stub(self, name, body):
        path = self.root / f"{name}-stub"
        path.write_text("#!/bin/sh\n" + body, "utf-8")
        path.chmod(path.stat().st_mode | stat.S_IXUSR)

    def seams(self):
        return {
            "meminfo": self.meminfo,
            "page": self.page,
            "sudo": (str(self.root / "sudo-stub"),),
            "systemctl": (str(self.root / "systemctl-stub"),),
            "proc_cgroup": self.proc,
            "cgroup_root": self.cgroup,
            "reads": 2,
        }

    def marker_command(self):
        code = f"open({str(self.marker)!r}, 'w').write('ran')"
        return [sys.executable, "-c", code]

    def go(self, module, command=None):
        """Run the script's `run()`: (exit, stdout, the record or None)."""
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = module.run(command or self.marker_command(), self.report, **self.seams())
        record = self.report / "memory-scope.json"
        return (
            code,
            out.getvalue(),
            json.loads(record.read_text("utf-8")) if record.exists() else None,
        )

    def sudo_argv(self):
        return self.sudo_log.read_text("utf-8").splitlines() if self.sudo_log.exists() else []


class ScriptCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)
        self.module = load()
        if self.module is None:
            self.fail("scripts/memory_scope.py is absent: the memory scope is not built")

    def plant(self, **kwargs):
        return Plant(self.tmp, **kwargs)


class TheScope(ScriptCase):
    def test_the_cap_is_fifteen_sixteenths_of_the_machine_in_whole_pages_and_never_given(self):
        shapes = [
            ("first", 16777216, 4096, "MemTotal:       16777216 kB\nMemFree: 1 kB\n"),
            ("last", 8388608, 4096, "MemFree: 1 kB\nSwapTotal: 0 kB\nMemTotal:  8388608 kB\n"),
            ("between", 7340032, 4096, "Buffers: 2 kB\nMemTotal: 7340032 kB\nCached: 9 kB\n"),
            ("page that does not divide", 16777213, 65536, "MemTotal: 16777213 kB\n"),
            ("small", 1000, 4096, "MemTotal: 1000 kB\n"),
        ]
        for label, total, page, text in shapes:
            with self.subTest(shape=label):
                with tempfile.TemporaryDirectory() as directory:
                    plant = Plant(directory, total_kb=total, page=page, text=text).build()
                    self.assertEqual(plant.cap % page, 0, label)
                    self.assertEqual(self.module.cap_of(text, page), plant.cap)
                    code, out, record = plant.go(self.module)
                    self.assertEqual(code, 0, out)
                    self.assertIn(
                        f"MemoryMax\nt\n{plant.cap}\n", "\n".join(plant.sudo_argv()) + "\n"
                    )
                    self.assertTrue(record["in_force"], record)
                    self.assertEqual(plant.marker.read_text("utf-8"), "ran")
        with tempfile.TemporaryDirectory() as directory:
            plant = Plant(directory, text="MemFree: 1 kB\nSwapTotal: 0 kB\n").build()
            code, out, record = plant.go(self.module)
            self.assertEqual(code, 78, out)
            self.assertIn("memory-scope: REFUSED:", out)
            self.assertIn("MemTotal", out)
            self.assertIs(record["in_force"], False, record)
            self.assertFalse(plant.marker.exists(), "the command ran without a cap")
            self.assertEqual(plant.sudo_argv(), [], "the manager was called with no cap to give")
        parser = self.module.build_parser()
        self.assertEqual(sorted(parser._option_string_actions), ["--help", "--report", "-h"])
        self.assertIs(parser.allow_abbrev, False)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as refused:
            self.module.main(["--cap", "1", "--report", str(self.tmp), "--", "true"])
        self.assertEqual(refused.exception.code, 2)

    def test_the_scope_holds_the_scripts_own_process_with_swap_forbidden_and_oom_policy_continue(
        self,
    ):
        plant = self.plant().build()
        code, out, _ = plant.go(self.module)
        self.assertEqual(code, 0, out)
        pid, cap, unit = str(plant.pid), str(plant.cap), plant.unit
        tail = [
            "--non-interactive", "busctl", "call", "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1", "org.freedesktop.systemd1.Manager", "StartTransientUnit",
            "ssa(sv)a(sa(sv))", unit, "fail", "5", "PIDs", "au", "1", pid, "MemoryMax", "t", cap,
            "MemorySwapMax", "t", "0", "OOMPolicy", "s", "continue", "CollectMode", "s",
            "inactive-or-failed", "0",
        ]  # fmt: skip
        self.assertEqual(plant.sudo_argv(), tail)
        self.assertEqual(
            self.module.scope_call(unit, plant.pid, plant.cap, ("sudo",)), ["sudo", *tail]
        )
        for word in plant.marker_command():
            self.assertNotIn(word, plant.sudo_argv())
        self.assertEqual(plant.marker.read_text("utf-8"), "ran")

    def test_a_scope_whose_cap_is_not_in_force_runs_nothing(self):
        arms = [
            ("unit", "control group", lambda p: setattr(p, "unit_group", "memory-scope-1.scope")),
            ("max", "memory.max", lambda p: p.files.update({"memory.max": f"{p.cap - PAGE}\n"})),
            ("swap", "memory.swap.max", lambda p: p.files.update({"memory.swap.max": "max\n"})),
            ("group", "memory.oom.group", lambda p: p.files.update({"memory.oom.group": "1\n"})),
            (
                "killed",
                "memory.events",
                lambda p: p.files.update(
                    {"memory.events": EVENTS.replace("oom_kill 0", "oom_kill 1")}
                ),
            ),
            (
                "no kill key",
                "memory.events",
                lambda p: p.files.update({"memory.events": "low 0\nhigh 0\nmax 0\noom 0\n"}),
            ),
            ("policy", "OOMPolicy", lambda p: setattr(p, "policy", "stop")),
            ("manager", "manager", lambda p: setattr(p, "sudo_exit", 1)),
        ]
        for label, word, bend in arms:
            with self.subTest(arm=label):
                with tempfile.TemporaryDirectory() as directory:
                    plant = Plant(directory)
                    bend(plant)
                    plant.build()
                    code, out, record = plant.go(self.module)
                    self.assertEqual(code, 78, out)
                    line = next(
                        (x for x in out.splitlines() if x.startswith("memory-scope: REFUSED:")), ""
                    )
                    self.assertIn(word, line, out)
                    self.assertIs(record["in_force"], False, record)
                    self.assertIn(word, record["reason"], record)
                    self.assertFalse(
                        plant.marker.exists(), "the command ran under a cap not in force"
                    )

    def test_the_command_runs_with_the_callers_identity_and_environment(self):
        plant = self.plant().build()
        out_file = self.tmp / "identity.json"
        code_text = (
            "import json, os\n"
            "json.dump({'env': dict(os.environ), 'cwd': os.getcwd(), 'uid': os.getuid(),\n"
            "           'gid': os.getgid(), 'groups': sorted(os.getgroups())},\n"
            f"          open({str(out_file)!r}, 'w'))\n"
        )
        with mock.patch.dict(os.environ, {"SCOPE_PROBE": "kept", "PYTHONCOERCECLOCALE": "0"}):
            code, out, _ = plant.go(self.module, [sys.executable, "-c", code_text])
            here = {"env": dict(os.environ), "cwd": os.getcwd(), "uid": os.getuid(),
                    "gid": os.getgid(), "groups": sorted(os.getgroups())}  # fmt: skip
        self.assertEqual(code, 0, out)
        seen = json.loads(out_file.read_text("utf-8"))
        for key in ("cwd", "uid", "gid", "groups"):
            self.assertEqual(seen[key], here[key], key)
        self.assertEqual(seen["env"], here["env"])

    def test_the_commands_exit_status_is_the_scripts(self):
        for wanted in (0, 2, 3, 4):
            with self.subTest(exit=wanted):
                with tempfile.TemporaryDirectory() as directory:
                    plant = Plant(directory).build()
                    code, out, _ = plant.go(self.module, ["sh", "-c", f"exit {wanted}"])
                    self.assertEqual(code, wanted, out)
        with tempfile.TemporaryDirectory() as directory:
            plant = Plant(directory).build()
            code, out, _ = plant.go(self.module, ["sh", "-c", "kill -KILL $$"])
            self.assertEqual(code, 137, out)

    def test_the_record_is_begun_before_the_command_and_finished_after(self):
        plant = self.plant().build()
        seen = self.tmp / "record-seen.json"
        events = plant.group / "memory.events"
        peak = plant.group / "memory.peak"
        record = plant.report / "memory-scope.json"
        code_text = (
            "import shutil\n"
            f"shutil.copy({str(record)!r}, {str(seen)!r})\n"
            f"open({str(events)!r}, 'w').write('low 0\\nhigh 0\\nmax 3\\noom 1\\noom_kill 1\\n')\n"
            f"open({str(peak)!r}, 'w').write('{plant.cap // 2}\\n')\n"
        )
        code, out, done = plant.go(self.module, [sys.executable, "-c", code_text])
        self.assertEqual(code, 0, out)
        begun = json.loads(seen.read_text("utf-8"))
        self.assertEqual((begun["state"], begun["in_force"]), ("running", True), begun)
        self.assertEqual(done["state"], "done")
        self.assertIs(done["in_force"], True)
        self.assertEqual((done["oom"], done["oom_kill"], done["max"]), (1, 1, 3), done)
        self.assertEqual(done["peak_percent"], 50)
        self.assertIn(
            "memory-scope: peak 50% of the cap; the kernel stopped 1 process(es) at the cap",
            out.splitlines(),
        )
        for text in (out, record.read_text("utf-8")):
            self.assertNotIn(str(plant.cap), text)
            self.assertNotIn(str(plant.cap // 2), text)


def workflow_commands():
    """(file, line number, line) for every `cargo mutants` occurrence in every workflow."""
    found = []
    for path in sorted(WORKFLOWS.glob("*.yml")):
        for number, line in enumerate(path.read_text("utf-8").splitlines(), 1):
            if "cargo mutants" in line and not line.lstrip().startswith("#"):
                found.append((path.name, number, line))
    return examined("cargo mutants commands", found)


def refusal(line):
    """Why a `cargo mutants` line is not run as SPEC-196 R6 says, or None when it is."""
    if "--list" in line:
        return "a listing is wrapped" if "memory_scope.py" in line else None
    wrapped = SCOPED.search(line)
    if wrapped is None:
        return "a run that runs tests is not inside the scope"
    output = OUTPUT.search(line)
    if output is None or output.group(1) != wrapped.group(1):
        return "the report is not the run's --output"
    return None


class TheWorkflows(unittest.TestCase):
    def test_every_mutants_run_that_runs_tests_is_inside_the_scope(self):
        refused = [
            f"{name}:{number}: {why}"
            for name, number, line in workflow_commands()
            if (why := refusal(line))
        ]
        self.assertEqual(refused, [])
        good = 'python3 scripts/memory_scope.py --report "$o" -- cargo mutants -vV --output "$o"'
        planted = {
            "a report that is not the output": good.replace('--report "$o"', '--report "$p"'),
            "an unwrapped test run": 'cargo mutants -vV --output "$o"',
            "a wrapped listing": 'python3 scripts/memory_scope.py --report "$o" -- cargo mutants --list',
        }
        self.assertIsNone(refusal(good), "the planted good line is refused")
        for label, line in planted.items():
            with self.subTest(plant=label):
                self.assertIsNotNone(refusal(line), label)

    def test_the_mutants_arguments_are_the_ones_before_the_scope(self):
        pins = [(CI, CI_LEG), (WEEKLY, WEEKLY_LEG), (WEEKLY, REHEARSAL_LEG)]
        pins += [(path, line) for path, lines in LISTINGS.items() for line in lines]
        for path, literal in examined("pinned commands", pins):
            with self.subTest(file=path.name, command=literal[:60]):
                self.assertEqual(path.read_text("utf-8").count(literal), 1)

    def test_the_rehearsal_counts_what_it_counted_before(self):
        text = WEEKLY.read_text("utf-8")
        start = text.index("\n  rehearsal:\n")
        after = re.search(r"\n  [a-z][a-z-]*:\n", text[start + 1 :])
        job = text[start : start + 1 + after.start()] if after else text[start:]
        for literal in (
            REHEARSAL_LEG,
            "-f crates/kernel/src/clock.rs",
            'examined = cargo["caught"] + cargo["missed"] + cargo["timeout"]',
        ):
            with self.subTest(literal=literal[:50]):
                self.assertEqual(job.count(literal), 1)


if __name__ == "__main__":
    unittest.main()
