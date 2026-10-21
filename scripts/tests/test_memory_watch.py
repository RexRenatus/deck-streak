"""The memory watch pages on a new OOM kill or MemoryMax event and logs MemoryHigh throttling, each
once (SPEC-031 A6, R5; ADR-031, ADR-032).

The watch runs as its unit runs it, over a synthetic copy of the kernel's cgroup v2 tree
(`scripts/tests/fixtures/cgroup/`) given with `--cgroup-root`, and `$STATE_DIRECTORY` is a temporary
directory, so no run reads the host. Each DeckStreak unit's `memory.max` in the fixture is its
MemoryMax= from deploy/host-budget.json, so the watch's 90% line is DeckStreak's own budget
(ADR-032). The fixture also holds a unit that is not DeckStreak's, whose events are never ours.
"""

import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "deploy" / "scripts" / "memory-watch.sh"
FIXTURE = REPO / "scripts" / "tests" / "fixtures" / "cgroup"
BUDGET = REPO / "deploy" / "host-budget.json"
API = "deck-streak-api.service"
BOT = "deck-streak-bot.service"
UNRELATED = "unrelated.service"
JOB_TEMPLATE = "deck-streak-job"
# systemd escapes the template's dashes in the slice it makes for the template's instances.
JOB_SLICE = "system-deck\\x2dstreak\\x2djob.slice"
UNITS = {"K": 1024, "M": 1024**2, "G": 1024**3}


def size(text):
    """A systemd byte size such as 128M, in bytes."""
    match = re.fullmatch(r"(\d+)([KMG]?)", text)
    if match is None:
        raise AssertionError(f"{text!r} is not a byte size")
    return int(match.group(1)) * UNITS.get(match.group(2), 1)


class Watch:
    """Runs of the watch over one copy of the fixture's cgroup tree and one state directory."""

    def __init__(self, scratch):
        root = Path(scratch)
        self.root, self.state = root / "cgroup", root / "state"
        shutil.copytree(FIXTURE, self.root)
        self.state.mkdir()
        self.slice = self.root / "system.slice"

    def run(self):
        done = subprocess.run(
            [str(SCRIPT), "--cgroup-root", str(self.root)],
            env={
                "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
                "STATE_DIRECTORY": str(self.state),
            },
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        return done.returncode, done.stdout.splitlines() + done.stderr.splitlines()

    def cgroup(self, unit):
        found = [path for path in self.slice.rglob(unit) if path.is_dir()]
        if len(found) != 1:
            raise AssertionError(f"{len(found)} cgroup(s) named {unit}")
        return found[0]

    def bump(self, unit, counter, by=1):
        """Raises one counter of a unit's memory.events, as the kernel does on an event."""
        events = self.cgroup(unit) / "memory.events"
        lines = []
        for line in events.read_text(encoding="utf-8").splitlines():
            key, value = line.split()
            lines.append(f"{key} {int(value) + by if key == counter else value}")
        events.write_text("\n".join(lines) + "\n", encoding="utf-8")

    def use(self, unit, current):
        (self.cgroup(unit) / "memory.current").write_text(f"{current}\n", encoding="utf-8")

    def remake(self, unit, parent=None):
        """Replaces a unit's cgroup with a new one holding the same files, as a restart does. The
        new directory is made before the old one goes, so it can never reuse the old's inode."""
        old = self.cgroup(unit) if parent is None else parent / unit
        fresh = old.parent / f".{unit}.new"
        shutil.copytree(old, fresh)
        shutil.rmtree(old)
        fresh.rename(old)
        return old


def paged(lines, unit):
    return [line for line in lines if line.startswith(f"<3>{unit}:")]


def logged(lines, unit):
    return [line for line in lines if line.startswith(f"<4>{unit}:")]


class TheWatchPagesOncePerEvent(unittest.TestCase):
    def test_the_watch_pages_on_a_new_oom_kill_and_logs_a_new_high_event(self):
        with tempfile.TemporaryDirectory() as scratch:
            watch = Watch(scratch)
            # The first run is each unit's baseline: nothing it finds is new.
            code, lines = watch.run()
            self.assertEqual(code, 0, lines)
            self.assertEqual([line for line in lines if line.startswith("<3>")], [], lines)
            # A new OOM kill in the API pages, naming the unit and the counter.
            watch.bump(API, "oom_kill")
            watch.bump(UNRELATED, "oom_kill")
            code, lines = watch.run()
            self.assertEqual(code, 1, "a new oom_kill paged nobody:\n" + "\n".join(lines))
            api = paged(lines, API)
            self.assertEqual(len(api), 1, lines)
            self.assertIn("oom_kill", api[0])
            # The next run finds nothing new: the event paged once.
            code, lines = watch.run()
            self.assertEqual(code, 0, lines)
            self.assertEqual(paged(lines, API), [], lines)
            # A new high event in the bot is MemoryHigh throttling it: logged, and paging no one.
            watch.bump(BOT, "high", by=3)
            code, lines = watch.run()
            self.assertEqual(code, 0, lines)
            bot = logged(lines, BOT)
            self.assertEqual(len(bot), 1, lines)
            self.assertIn("high", bot[0])
            self.assertEqual([line for line in lines if line.startswith("<3>")], [], lines)
            # A unit that is not DeckStreak's was never read, whatever its counters did.
            self.assertEqual([line for line in lines if UNRELATED in line], [])

    def test_a_new_max_event_pages_and_the_ninety_percent_line_logs_once(self):
        with tempfile.TemporaryDirectory() as scratch:
            watch = Watch(scratch)
            watch.run()
            watch.bump(API, "max")
            code, lines = watch.run()
            self.assertEqual(code, 1, lines)
            self.assertIn("max", paged(lines, API)[0])
            # Usage past 90% of memory.max is logged when it crosses, and not again while it stays.
            ceiling = int((watch.cgroup(API) / "memory.max").read_text(encoding="utf-8"))
            watch.use(API, ceiling * 9 // 10 + 4096)
            code, lines = watch.run()
            self.assertEqual(code, 0, lines)
            crossed = logged(lines, API)
            self.assertEqual(len(crossed), 1, lines)
            self.assertIn("90%", crossed[0])
            code, lines = watch.run()
            self.assertEqual(logged(lines, API), [], lines)
            # Back below it, then past it again: a new crossing, logged again.
            watch.use(API, ceiling // 2)
            code, lines = watch.run()
            self.assertEqual(logged(lines, API), [], lines)
            watch.use(API, ceiling - 1)
            code, lines = watch.run()
            self.assertEqual(len(logged(lines, API)), 1, lines)

    def test_a_cgroup_made_since_the_last_run_counts_every_event_as_new(self):
        with tempfile.TemporaryDirectory() as scratch:
            watch = Watch(scratch)
            watch.bump(API, "oom_kill")
            watch.run()
            # A restart made a new cgroup, whose one OOM kill equals the count remembered for the
            # old one: it is still a new event.
            watch.remake(API)
            code, lines = watch.run()
            self.assertEqual(code, 1, lines)
            self.assertEqual(len(paged(lines, API)), 1, lines)
            # A job's instance sits in its template's slice: found, its first sight a baseline, and
            # its next run's cgroup new.
            job = f"{JOB_TEMPLATE}@sync.service"
            cgroup = watch.slice / JOB_SLICE / job
            shutil.copytree(watch.cgroup(BOT), cgroup)
            watch.bump(job, "max")
            code, lines = watch.run()
            self.assertEqual((code, paged(lines, job)), (0, []), lines)
            watch.remake(job, parent=cgroup.parent)
            code, lines = watch.run()
            self.assertEqual(code, 1, lines)
            self.assertEqual(len(paged(lines, job)), 1, lines)

    def test_a_units_first_sight_is_its_baseline(self):
        with tempfile.TemporaryDirectory() as scratch:
            watch = Watch(scratch)
            # What a unit already counts when the watch first sees it happened before the watch.
            watch.bump(API, "oom_kill")
            watch.bump(API, "max")
            code, lines = watch.run()
            self.assertEqual((code, paged(lines, API)), (0, []), lines)
            code, lines = watch.run()
            self.assertEqual((code, paged(lines, API)), (0, []), lines)
            # Its next event is new, and pages.
            watch.bump(API, "oom_kill")
            code, lines = watch.run()
            self.assertEqual(code, 1, lines)
            self.assertIn("rose from 1 to 2", paged(lines, API)[0])

    def test_a_watch_that_finds_no_unit_pages_once(self):
        with tempfile.TemporaryDirectory() as scratch:
            watch = Watch(scratch)
            kept = Path(scratch) / "kept"
            kept.mkdir()
            for unit in (API, BOT):
                watch.cgroup(unit).rename(kept / unit)
            # Only a unit that is not DeckStreak's: the watch sees nothing, and says so once.
            code, lines = watch.run()
            self.assertEqual(code, 1, lines)
            self.assertEqual(
                [line for line in lines if line.startswith("<3>no DeckStreak unit's cgroup")],
                [line for line in lines if line.startswith("<3>")],
            )
            self.assertEqual(len([line for line in lines if line.startswith("<3>")]), 1, lines)
            code, lines = watch.run()
            self.assertEqual(code, 0, lines)
            self.assertEqual(len([line for line in lines if line.startswith("<4>still no")]), 1)
            # A unit seen again ends the episode, and the next blindness pages again.
            (kept / API).rename(watch.slice / API)
            code, lines = watch.run()
            self.assertEqual(code, 0, lines)
            (watch.slice / API).rename(kept / API)
            code, lines = watch.run()
            self.assertEqual(code, 1, lines)

    def test_the_ninety_percent_line_is_crossed_only_past_it(self):
        with tempfile.TemporaryDirectory() as scratch:
            watch = Watch(scratch)
            ceiling = int((watch.cgroup(API) / "memory.max").read_text(encoding="utf-8"))
            watch.use(API, ceiling * 9 // 10)
            code, lines = watch.run()
            self.assertEqual((code, logged(lines, API)), (0, []), lines)
            watch.use(API, ceiling * 9 // 10 + 1)
            code, lines = watch.run()
            self.assertEqual(len(logged(lines, API)), 1, lines)

    def test_a_unit_without_a_ceiling_or_readable_events_is_passed_over(self):
        with tempfile.TemporaryDirectory() as scratch:
            watch = Watch(scratch)
            # No ceiling: no 90% line, however much it uses.
            (watch.cgroup(API) / "memory.max").write_text("max\n", encoding="utf-8")
            watch.use(API, 10**12)
            # Events it cannot read: said once per run, and the other units are still watched.
            (watch.cgroup(BOT) / "memory.events").unlink()
            code, lines = watch.run()
            self.assertEqual(code, 0, lines)
            self.assertEqual(logged(lines, API), [], lines)
            self.assertEqual(logged(lines, BOT), [f"<4>{BOT}: its memory.events cannot be read"])
            watch.bump(API, "oom_kill")
            code, lines = watch.run()
            self.assertEqual((code, len(paged(lines, API))), (1, 1), lines)

    def test_the_fixture_ceilings_are_the_host_budgets(self):
        # The 90% line is the unit's MemoryMax=, which deploy/host-budget.json decides (ADR-032).
        budget = json.loads(BUDGET.read_text(encoding="utf-8"))["units"]
        units = examined(
            "DeckStreak cgroup(s) in the fixture",
            sorted(path for path in (FIXTURE / "system.slice").glob("deck-streak-*.service")),
        )
        for cgroup in units:
            ceiling = int((cgroup / "memory.max").read_text(encoding="utf-8"))
            self.assertEqual(ceiling, size(budget[cgroup.name]["memory_max"]), cgroup.name)


if __name__ == "__main__":
    unittest.main()
