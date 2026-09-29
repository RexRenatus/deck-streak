"""A killer the runner ends takes its whole process group with it (SPEC-025's amendment of
2026-09-29, #366).

`mutation_rows.run_killer` ran a killer with `subprocess.run(timeout=...)`, which kills the direct
child alone. A killer's test binary, and a daemon that binary started, kept running with no parent
(three `deckstreakd api` and one `deckstreakd bot` were found long after their runs). These tests
run a killer that starts a grandchild (a `sleep`) and outruns a small bound, and read the
grandchild's pid from a file the killer writes.
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


class ARunThatEndsOnItsOwnIsReadAsItAlwaysWas(Fixture):
    def test_a_passing_and_a_failing_killer_are_counted_from_their_own_output(self):
        passed = self.run_killer("test_passes")
        failed = self.run_killer("test_fails")
        self.assertEqual(passed, runner.Run(1, True))
        self.assertEqual(failed, runner.Run(1, False))


if __name__ == "__main__":
    unittest.main()
