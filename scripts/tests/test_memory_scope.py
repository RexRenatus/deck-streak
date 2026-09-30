"""A mutants run's memory scope (SPEC-196 A1 to A8, A17 and A18).

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
DIRECTORY = object()
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


KEYS = ("oom", "oom_kill", "max")
AFTER = {"low": 0, "high": 0, "max": 3, "oom": 1, "oom_kill": 1, "oom_group_kill": 0}
BEFORE = dict.fromkeys(AFTER, 0)
EVENTS_NOT_WHOLE_AFTER = "memory.events is not whole after the command"
PEAK_REASON = "memory.peak holds no count after the command"
REASONS_AFTER = {
    "bad": EVENTS_NOT_WHOLE_AFTER,
    "empty": "memory.events holds no oom count after the command",
    "no oom": "memory.events holds no oom count after the command",
    "no oom_kill": "memory.events holds no oom_kill count after the command",
    "no max": "memory.events holds no max count after the command",
}
REASONS_BEFORE = {
    "bad": "memory.events is not whole",
    "empty": "memory.events holds no oom count",
    "no oom": "memory.events holds no oom count",
    "no oom_kill": "memory.events holds no oom_kill count",
}
UNREADABLE_BEFORE = {
    "memory.max": "memory.max is not what the scope was asked for",
    "memory.swap.max": "memory.swap.max is not what the scope was asked for",
    "memory.oom.group": "memory.oom.group is not what the scope was asked for",
    "meminfo": None,
}


def event_bytes(counts, replace=None, drop=None):
    """memory.events as the kernel writes it for `counts`, one line replaced or one left out."""
    replace = replace or {}
    return b"".join(
        replace.get(name, f"{name} {value}\n".encode())
        for name, value in counts.items()
        if name != drop
    )


# The shapes one key's line can take (the verifier's generated axes for SPEC-196 R13): whole, and
# duplicated with the same value, duplicated 1 then 0, duplicated 0 then 1, a non-ASCII digit
# twice over, and a line holding a byte that is not UTF-8.
SHAPES = {
    "whole": lambda k, v: f"{k} {v}\n".encode(),
    "duplicated, same value": lambda k, v: f"{k} {v}\n{k} {v}\n".encode(),
    "duplicated, 1 then 0": lambda k, v: f"{k} 1\n{k} 0\n".encode(),
    "duplicated, 0 then 1": lambda k, v: f"{k} 0\n{k} 1\n".encode(),
    "superscript two": lambda k, v: f"{k} ²\n".encode(),
    "Arabic-Indic one": lambda k, v: f"{k} ١\n".encode(),
    "a byte that is not UTF-8": lambda k, v: f"{k} {v}\n".encode() + b"\xff\n",
}
EVENT_STATES = {
    f"{key}: {label}": (
        "whole" if label == "whole" else "bad",
        lambda counts, key=key, shape=shape: event_bytes(counts, {key: shape(key, counts[key])}),
    )
    for key in KEYS
    for label, shape in SHAPES.items()
}
EVENT_STATES.update(
    {
        "absent": ("bad", lambda counts: None),
        "unreadable": ("bad", lambda counts: DIRECTORY),
        "garbage": ("bad", lambda counts: b"no counts here\n"),
        "empty": ("empty", lambda counts: b""),
        "no oom": ("no oom", lambda counts: event_bytes(counts, drop="oom")),
        "no oom_kill": ("no oom_kill", lambda counts: event_bytes(counts, drop="oom_kill")),
        "no max": ("no max", lambda counts: event_bytes(counts, drop="max")),
        "trailing blanks": (
            "bad",
            lambda counts: event_bytes(counts, {"oom": f"oom {counts['oom']} \n".encode()}),
        ),
        "CRLF": ("bad", lambda counts: event_bytes(counts).replace(b"\n", b"\r\n")),
        "not a number": (
            "bad",
            lambda counts: event_bytes(counts, {"oom_kill": b"oom_kill many\n"}),
        ),
        "negative": ("bad", lambda counts: event_bytes(counts, {"oom": b"oom -1\n"})),
        "twenty-one digits": (
            "bad",
            lambda counts: event_bytes(counts, {"oom": b"oom %d\n" % 10**20}),
        ),
        "low duplicated": ("bad", lambda counts: event_bytes(counts) + b"low 0\n"),
    }
)
AFTER_EVENTS = EVENT_STATES
# A `no max` file is whole enough before the command, which reads only oom and oom_kill.
BEFORE_EVENTS = {k: v for k, v in EVENT_STATES.items() if v[0] != "no max"}
AFTER_PEAKS = {
    "whole": (True, lambda cap: b"%d\n" % (cap // 2)),
    "whole, no final newline": (True, lambda cap: b"%d" % (cap // 2)),
    "superscript two": (False, lambda cap: "²\n".encode()),
    "Arabic-Indic one": (False, lambda cap: "١\n".encode()),
    "not UTF-8": (False, lambda cap: b"4096\xff\n"),
    "absent": (False, lambda cap: None),
    "unreadable": (False, lambda cap: DIRECTORY),
    "garbage": (False, lambda cap: b"lots\n"),
    "empty": (False, lambda cap: b""),
    "negative": (False, lambda cap: b"-5\n"),
    "plus sign": (False, lambda cap: b"+5\n"),
    "trailing blank": (False, lambda cap: b"4096 \n"),
    "CRLF": (False, lambda cap: b"4096\r\n"),
    "two lines": (False, lambda cap: b"4096\n4096\n"),
    "leading blank": (False, lambda cap: b" 4096\n"),
}


def place(target, data):
    """Replace `target` with bytes, a directory (DIRECTORY) or nothing (None)."""
    target.unlink()
    if data is DIRECTORY:
        target.mkdir()
    elif data is not None:
        target.write_bytes(data)


def rewriting(plant, files):
    """A command that replaces the named control-group files (bytes, DIRECTORY or None) and exits 3."""
    steps = ["import os"]
    for name, data in files.items():
        target = str(plant.group / name)
        steps.append(f"os.unlink({target!r})")
        if data is DIRECTORY:
            steps.append(f"os.mkdir({target!r})")
        elif data is not None:
            steps.append(f"open({target!r}, 'wb').write({data!r})")
    steps.append("raise SystemExit(3)")
    return [sys.executable, "-c", "\n".join(steps)]


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

    def test_a_count_read_after_the_command_that_is_missing_or_unreadable_is_not_in_force(self):
        events = {
            "present": (EVENTS, True),
            "absent": (None, False),
            "unreadable": (DIRECTORY, False),
            "garbage": ("no counts here\n", False),
            "no oom line": ("low 0\nhigh 0\nmax 0\noom_kill 0\n", False),
            "no max line": ("low 0\nhigh 0\noom 0\noom_kill 0\n", False),
            "no oom_kill line": ("low 0\nhigh 0\nmax 0\noom 0\n", False),
            "oom_kill not a number": ("max 0\noom 0\noom_kill many\n", False),
        }
        peaks = {
            "present": ("0\n", True),
            "absent": (None, False),
            "unreadable": (DIRECTORY, False),
            "garbage": ("lots\n", False),
            "empty": ("", False),
            "negative": ("-5\n", False),
        }
        members = [(e, p) for e in events for p in peaks]
        self.assertEqual(len(members), len(events) * len(peaks))
        examined("counts read after the command", members)
        for event_shape, peak_shape in members:
            with self.subTest(events=event_shape, peak=peak_shape):
                with tempfile.TemporaryDirectory() as directory:
                    plant = Plant(directory).build()
                    steps = ["import os, shutil"]
                    for name, (text, _) in (
                        ("memory.events", events[event_shape]),
                        ("memory.peak", peaks[peak_shape]),
                    ):
                        target = str(plant.group / name)
                        steps.append(f"os.unlink({target!r})")
                        if text is DIRECTORY:
                            steps.append(f"os.mkdir({target!r})")
                        elif text is not None:
                            steps.append(f"open({target!r}, 'w').write({text!r})")
                    steps.append("raise SystemExit(3)")
                    command = [sys.executable, "-c", "\n".join(steps)]
                    code, out, record = plant.go(self.module, command)
                    self.assertEqual(code, 3, out)
                    self.assertEqual(record["state"], "done", record)
                    whole = events[event_shape][1] and peaks[peak_shape][1]
                    self.assertIs(record["in_force"], whole, record)
                    if whole:
                        self.assertIsNone(record["reason"], record)
                    else:
                        self.assertTrue(record["reason"], record)
                        self.assertIn("NOT IN FORCE", out)

    def test_the_counts_read_after_the_command_are_whole_or_the_record_is_not_in_force(self):
        members = [(e, p) for e in AFTER_EVENTS for p in AFTER_PEAKS]
        self.assertEqual(len(members), len(AFTER_EVENTS) * len(AFTER_PEAKS))
        self.assertEqual(len(members), 34 * 15)
        examined("counts read after the command, whole or not", members)
        for event_label, peak_label in members:
            with self.subTest(events=event_label, peak=peak_label):
                with tempfile.TemporaryDirectory() as directory:
                    plant = Plant(directory).build()
                    kind, events_of = AFTER_EVENTS[event_label]
                    whole_peak, peak_of = AFTER_PEAKS[peak_label]
                    files = {"memory.events": events_of(AFTER), "memory.peak": peak_of(plant.cap)}
                    code, out, record = self.run_or_fail(plant, rewriting(plant, files))
                    self.assertEqual(code, 3, out)
                    if kind == "whole" and whole_peak:
                        wanted = {
                            "in_force": True, "state": "done", "reason": None, "oom": 1,
                            "oom_kill": 1, "max": 3, "peak_percent": 50,
                        }  # fmt: skip
                        line = (
                            "memory-scope: peak 50% of the cap; "
                            "the kernel stopped 1 process(es) at the cap"
                        )
                    else:
                        reason = PEAK_REASON if kind == "whole" else REASONS_AFTER[kind]
                        wanted = {
                            "in_force": False, "state": "done", "reason": reason, "oom": 0,
                            "oom_kill": 0, "max": 0, "peak_percent": 0,
                        }  # fmt: skip
                        line = f"memory-scope: NOT IN FORCE after the command: {reason}"
                    self.assertEqual(record, wanted, out)
                    self.assertIn(line, out.splitlines())

    def test_the_counts_read_before_the_command_are_whole_or_nothing_runs(self):
        members = [(label, kind, events_of) for label, (kind, events_of) in BEFORE_EVENTS.items()]
        members += [(label, "file", None) for label in UNREADABLE_BEFORE]
        examined("control-group files read before the command", members)
        self.assertEqual(len(members), 33 + 4)
        for label, kind, events_of in members:
            with self.subTest(read=label):
                with tempfile.TemporaryDirectory() as directory:
                    plant = Plant(directory).build()
                    reason = UNREADABLE_BEFORE.get(label)
                    if events_of is not None:
                        reason = REASONS_BEFORE.get(kind)
                        place(plant.group / "memory.events", events_of(BEFORE))
                    elif label == "meminfo":
                        plant.meminfo.write_bytes(b"MemTotal: \xff kB\n")
                    else:
                        (plant.group / label).write_bytes(b"\xff\n")
                    code, out, record = self.run_or_fail(plant, None)
                    if kind == "whole":
                        wanted = {
                            "in_force": True, "state": "done", "reason": None, "oom": 0,
                            "oom_kill": 0, "max": 0, "peak_percent": 0,
                        }  # fmt: skip
                        self.assertEqual((code, record), (0, wanted), out)
                        self.assertEqual(plant.marker.read_text("utf-8"), "ran")
                        continue
                    self.assertEqual(code, 78, out)
                    self.assertFalse(plant.marker.exists(), "the command ran under a bad read")
                    if label == "meminfo":
                        head = "no cap can be measured: MemTotal is unreadable ("
                        self.assertTrue(str(record["reason"]).startswith(head), record)
                        reason = record["reason"]
                    wanted = {
                        "in_force": False, "state": "done", "reason": reason, "oom": 0,
                        "oom_kill": 0, "max": 0, "peak_percent": 0,
                    }  # fmt: skip
                    self.assertEqual(record, wanted, out)
                    self.assertIn(f"memory-scope: REFUSED: {reason}", out.splitlines())

    def test_a_counter_is_at_most_twenty_ascii_digits(self):
        top = 2**64 - 1
        members = [
            ("twenty digits in memory.events", {"oom": b"oom %d\n" % top}, b"4096\n"),
            ("twenty digits in memory.peak", {}, b"%d\n" % top),
            ("twenty-one digits in memory.peak", {}, b"%d\n" % 10**20),
        ]
        examined("counters at the width of a kernel counter", members)
        for label, replace, peak in members:
            with self.subTest(counter=label):
                with tempfile.TemporaryDirectory() as directory:
                    plant = Plant(directory).build()
                    files = {"memory.events": event_bytes(AFTER, replace), "memory.peak": peak}
                    code, out, record = self.run_or_fail(plant, rewriting(plant, files))
                    self.assertEqual(code, 3, out)
                    if label == "twenty-one digits in memory.peak":
                        wanted = {
                            "in_force": False, "state": "done", "reason": PEAK_REASON, "oom": 0,
                            "oom_kill": 0, "max": 0, "peak_percent": 0,
                        }  # fmt: skip
                    else:
                        wanted = {
                            "in_force": True, "state": "done", "reason": None,
                            "oom": top if replace else 1, "oom_kill": 1, "max": 3,
                            "peak_percent": (4096 if replace else top) * 100 // plant.cap,
                        }  # fmt: skip
                    self.assertEqual(record, wanted, out)

    def run_or_fail(self, plant, command):
        """`plant.go`, with a crash of the script reported as a failed assertion."""
        try:
            return plant.go(self.module, command)
        except Exception as error:  # noqa: BLE001
            self.fail(f"the script crashed: {type(error).__name__}: {error}")


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
