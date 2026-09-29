"""A killer the runner ends takes its whole process group with it (SPEC-025's amendment of
2026-09-29, #366).

`mutation_rows.run_killer` ran a killer with `subprocess.run(timeout=...)`, which kills the direct
child alone. A killer's test binary, and a daemon that binary started, kept running with no parent
(three `deckstreakd api` and one `deckstreakd bot` were found long after their runs). These tests
run a killer that starts a grandchild (a `sleep`) and outruns a small bound, and read the
grandchild's pid from a file the killer writes.

The amendment of 2026-09-29 (2), issue 409, adds three more edges: a SIGTERM of the runner (to its
pid, and to its whole group as `timeout -s TERM` sends it) ends the killer's group, a timed-out
killer returns at its bound whatever a descendant that left the group still holds, and every path
closes the killer's pipes.
"""

import contextlib
import importlib.util
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import textwrap
import threading
import time
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO

SPEC = importlib.util.spec_from_file_location(
    "mutation_rows", REPO / "scripts" / "mutation_rows.py"
)
runner = importlib.util.module_from_spec(SPEC)
sys.modules["mutation_rows"] = runner
SPEC.loader.exec_module(runner)

#: How long a test waits for the grandchild's pid file, and for the grandchild to be gone.
BOUND = 20.0
#: The killer's own bound in these tests: small, so a hung killer is ended at once.
SMALL = 2

#: A killer module: its test starts a grandchild, records the pid, then hangs past any bound.
HANGING = textwrap.dedent(
    """\
    import os
    import subprocess
    import time
    import unittest
    from pathlib import Path


    class Killer(unittest.TestCase):
        def test_hangs_with_a_grandchild(self):
            grandchild = subprocess.Popen(["sleep", "300"])
            Path(os.environ["LEADER_PID_FILE"]).write_text(str(os.getpid()))
            Path(os.environ["GRANDCHILD_PID_FILE"]).write_text(str(grandchild.pid))
            time.sleep(300)

        def test_passes(self):
            self.assertTrue(True)

        def test_fails(self):
            self.assertTrue(False)
    """
)


#: A killer whose descendant leaves the group (its own session) and holds the output pipes.
ESCAPING = textwrap.dedent(
    """\
    import os
    import subprocess
    import time
    import unittest
    from pathlib import Path


    class Killer(unittest.TestCase):
        def test_hangs_while_an_escapee_holds_the_pipes(self):
            escapee = subprocess.Popen(["sleep", os.environ["ESCAPEE_SECONDS"]], start_new_session=True)
            Path(os.environ["ESCAPEE_PID_FILE"]).write_text(str(escapee.pid))
            time.sleep(300)
    """
)

#: How long the escapee lives: far past the bound and the margin, so a run that waits for it is
#: told apart from one that returns at its bound.
ESCAPEE_SECONDS = 40
#: What a run may take beyond its bound. Ending the group and reaping its leader take
#: milliseconds; ten seconds absorbs a loaded machine and is still well under ESCAPEE_SECONDS,
#: so only a run that waits on the escapee's pipes can exceed SMALL + MARGIN.
MARGIN = 10.0

#: The runner as a process of its own: `main()` and the real `run_killer` on the planted killer,
#: with the row proof replaced by that one killer run (a planted tree would need a repository).
DRIVER = textwrap.dedent(
    """\
    import signal
    import sys
    from pathlib import Path

    sys.path.insert(0, sys.argv[1])
    import mutation_rows as runner


    def prove(root, args):
        killer = runner.Killer(
            "script", "hanging_killer.Killer.test_hangs_with_a_grandchild", "hanging_killer.py", cwd="."
        )
        return 0 if runner.run_killer(root, killer, root).passed else 1


    runner.prove = prove
    signal.signal(signal.SIGINT, signal.default_int_handler)
    sys.exit(runner.main(["prove", "--all", "--root", sys.argv[2]]))
    """
)


def running(pid):
    """Whether process `pid` runs: a zombie awaiting its parent does not."""
    try:
        stat = Path(f"/proc/{pid}/stat").read_text()
    except OSError:
        return False
    return stat.rsplit(")", 1)[1].split()[0] != "Z"


def stop_by_number(pid):
    """SIGKILL to the one pid this test started, if it still runs; never a pattern."""
    with contextlib.suppress(ProcessLookupError):
        os.kill(pid, signal.SIGKILL)


class Fixture(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp(prefix="killer-group-"))
        self.addCleanup(shutil.rmtree, self.directory, ignore_errors=True)
        (self.directory / "hanging_killer.py").write_text(HANGING)
        self.pid_file = self.directory / "grandchild.pid"
        self.leader_file = self.directory / "leader.pid"
        patcher = mock.patch.dict(
            os.environ,
            {"GRANDCHILD_PID_FILE": str(self.pid_file), "LEADER_PID_FILE": str(self.leader_file)},
        )
        patcher.start()
        self.addCleanup(patcher.stop)
        self.grandchild = None
        self.addCleanup(self.stop_grandchild)

    def stop_grandchild(self):
        """Ends, by number, what a red run would leave: the killer and its grandchild."""
        for pid in (self.grandchild, self.leader()):
            if pid is not None:
                stop_by_number(pid)

    def leader(self):
        try:
            return int(self.leader_file.read_text())
        except (OSError, ValueError):
            return None

    def within_bound(self, action):
        """`action`'s outcome, or a failure when it does not return: a runner that leaves the
        killer's group alive waits on the pipes that group holds, so the run is bounded here."""
        outcome = {}

        def call():
            try:
                outcome["value"] = action()
            except BaseException as raised:  # noqa: BLE001 - carried to the test's thread
                outcome["raised"] = raised

        thread = threading.Thread(target=call, daemon=True)
        thread.start()
        thread.join(BOUND)
        if thread.is_alive():
            self.fail("the run did not return: the killer's group was left running")
        if "raised" in outcome:
            raise outcome["raised"]
        return outcome["value"]

    def killer(self, name):
        return runner.Killer(
            "script", f"hanging_killer.Killer.{name}", "hanging_killer.py", cwd="."
        )

    def run_killer(self, name):
        return runner.run_killer(self.directory, self.killer(name), self.directory)

    def wait_for_grandchild(self):
        """The grandchild's pid, once the killer has written it."""
        deadline = time.monotonic() + BOUND
        while time.monotonic() < deadline:
            try:
                self.grandchild = int(self.pid_file.read_text())
                return self.grandchild
            except (OSError, ValueError):
                time.sleep(0.05)
        raise AssertionError("the killer never wrote its grandchild's pid")

    def wait_until_gone(self, pid):
        deadline = time.monotonic() + BOUND
        while running(pid) and time.monotonic() < deadline:
            time.sleep(0.05)
        return not running(pid)


class ATimedOutKillerLeavesNothingRunning(Fixture):
    def test_the_grandchild_of_a_timed_out_killer_is_gone_when_the_run_returns(self):
        seen_alive = []

        def watch():
            pid = self.wait_for_grandchild()
            seen_alive.append(running(pid))

        watcher = threading.Thread(target=watch)
        watcher.start()
        with (
            mock.patch.object(runner, "BUILD_SECONDS", 0),
            mock.patch.object(runner, "TEST_SECONDS", SMALL),
        ):
            result = self.within_bound(lambda: self.run_killer("test_hangs_with_a_grandchild"))
        watcher.join()

        # The outcome is what it always was, and the grandchild was alive before the bound.
        self.assertEqual(result, runner.Run(0, False, "it timed out"))
        self.assertEqual(seen_alive, [True], "the grandchild was running before the timeout")
        self.assertTrue(
            self.wait_until_gone(self.grandchild),
            f"the grandchild {self.grandchild} of a timed-out killer is still running",
        )

    def test_an_interrupted_run_ends_the_killers_group_too(self):
        def interrupted(*_args, **_kwargs):
            self.wait_for_grandchild()
            raise KeyboardInterrupt

        with mock.patch.object(subprocess.Popen, "communicate", side_effect=interrupted):
            with self.assertRaises(KeyboardInterrupt):
                self.within_bound(lambda: self.run_killer("test_hangs_with_a_grandchild"))
        self.assertTrue(
            self.wait_until_gone(self.grandchild),
            f"the grandchild {self.grandchild} survived an interrupted run",
        )


class ASignalledRunnerEndsTheKillersGroup(Fixture):
    def start_runner(self):
        """The runner, in a group of its own, once its killer and the killer's grandchild run."""
        (self.directory / "driver.py").write_text(DRIVER)
        self.runner = subprocess.Popen(
            [
                sys.executable,
                str(self.directory / "driver.py"),
                str(REPO / "scripts"),
                str(self.directory),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            process_group=0,
        )
        self.addCleanup(stop_by_number, self.runner.pid)
        self.wait_for_grandchild()
        self.assertTrue(running(self.grandchild), "the grandchild runs before the signal")
        self.assertTrue(running(self.leader()), "the killer runs before the signal")

    def assert_the_killers_group_is_gone(self, how):
        leader = self.leader()
        self.assertTrue(self.wait_until_gone(leader), f"the killer {leader} still runs after {how}")
        self.assertTrue(
            self.wait_until_gone(self.grandchild),
            f"the grandchild {self.grandchild} still runs after {how}",
        )

    def test_sigterm_to_the_runner_ends_the_killers_group(self):
        self.start_runner()
        os.kill(self.runner.pid, signal.SIGTERM)
        self.assert_the_killers_group_is_gone("a SIGTERM of the runner")
        self.assertEqual(self.runner.wait(BOUND), 128 + signal.SIGTERM)

    def test_sigterm_to_the_runners_group_ends_the_killers_group(self):
        self.start_runner()
        os.killpg(self.runner.pid, signal.SIGTERM)
        self.assert_the_killers_group_is_gone("a SIGTERM of the runner's group")

    def test_sigint_to_the_runner_ends_the_killers_group(self):
        self.start_runner()
        os.kill(self.runner.pid, signal.SIGINT)
        self.assert_the_killers_group_is_gone("a SIGINT of the runner")


class AHeldPipeDoesNotHoldATimedOutKiller(Fixture):
    def setUp(self):
        super().setUp()
        (self.directory / "escaping_killer.py").write_text(ESCAPING)
        self.escapee_file = self.directory / "escapee.pid"
        patcher = mock.patch.dict(
            os.environ,
            {"ESCAPEE_PID_FILE": str(self.escapee_file), "ESCAPEE_SECONDS": str(ESCAPEE_SECONDS)},
        )
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_a_timed_out_killer_returns_at_its_bound_whatever_an_escapee_holds(self):
        killer = runner.Killer(
            "script",
            "escaping_killer.Killer.test_hangs_while_an_escapee_holds_the_pipes",
            "escaping_killer.py",
            cwd=".",
        )
        outcome = []
        started = time.monotonic()
        with (
            mock.patch.object(runner, "BUILD_SECONDS", 0),
            mock.patch.object(runner, "TEST_SECONDS", SMALL),
        ):
            thread = threading.Thread(
                target=lambda: outcome.append(
                    runner.run_killer(self.directory, killer, self.directory)
                ),
                daemon=True,
            )
            thread.start()
            thread.join(SMALL + MARGIN)
        elapsed = time.monotonic() - started
        try:
            self.grandchild = int(self.escapee_file.read_text())
        except (OSError, ValueError):
            self.fail("the killer never wrote its escapee's pid")
        self.assertFalse(
            thread.is_alive(),
            f"the run was still waiting on the escapee's pipes after {elapsed:.1f}s",
        )
        self.assertEqual(outcome, [runner.Run(0, False, "it timed out")])
        self.assertLess(elapsed, SMALL + MARGIN)
        # The escapee left the group, so ending it is not the runner's to do.
        self.assertTrue(running(self.grandchild), "the escapee was outside the killer's group")


class APipeIsClosedOnEveryPath(Fixture):
    def setUp(self):
        super().setUp()
        self.made = []
        made = self.made

        class Recording(subprocess.Popen):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                made.append(self)

        patcher = mock.patch.object(runner.subprocess, "Popen", Recording)
        patcher.start()
        self.addCleanup(patcher.stop)

    def assert_both_pipes_closed(self):
        self.assertEqual(len(self.made), 1, "one Popen was recorded")
        process = self.made[0]
        self.assertTrue(process.stdout.closed, "the stdout pipe was left open")
        self.assertTrue(process.stderr.closed, "the stderr pipe was left open")

    def test_a_run_that_ends_on_its_own_closes_both_pipes(self):
        self.assertEqual(self.run_killer("test_passes"), runner.Run(1, True))
        self.assert_both_pipes_closed()

    def test_a_timed_out_run_closes_both_pipes(self):
        with (
            mock.patch.object(runner, "BUILD_SECONDS", 0),
            mock.patch.object(runner, "TEST_SECONDS", SMALL),
        ):
            result = self.within_bound(lambda: self.run_killer("test_hangs_with_a_grandchild"))
        self.assertEqual(result, runner.Run(0, False, "it timed out"))
        self.assert_both_pipes_closed()

    def interrupted_with(self, raised):
        def interrupt(*_args, **_kwargs):
            self.wait_for_grandchild()
            raise raised

        with mock.patch.object(subprocess.Popen, "communicate", side_effect=interrupt):
            with self.assertRaises(type(raised)):
                self.within_bound(lambda: self.run_killer("test_hangs_with_a_grandchild"))
        self.assert_both_pipes_closed()

    def test_an_interrupted_run_closes_both_pipes(self):
        self.interrupted_with(KeyboardInterrupt())

    def test_a_run_ended_by_the_sigterm_exit_closes_both_pipes(self):
        self.interrupted_with(SystemExit(128 + signal.SIGTERM))


class ARunThatEndsOnItsOwnIsReadAsItAlwaysWas(Fixture):
    def test_a_passing_and_a_failing_killer_are_counted_from_their_own_output(self):
        passed = self.run_killer("test_passes")
        failed = self.run_killer("test_fails")
        self.assertEqual(passed, runner.Run(1, True))
        self.assertEqual(failed, runner.Run(1, False))


if __name__ == "__main__":
    unittest.main()
