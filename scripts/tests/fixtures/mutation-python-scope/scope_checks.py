"""The scope fixture's checks (SPEC-420 A8 to A16), named so that discovery never collects them.

`EXECUTES` is the table the runner's tests read: for each function of `scope_target`, the checks
whose processes execute its line. `test_in_child` executes `in_child` only in the child process it
starts. `test_reads_text` executes nothing of the target and reads its text, so a mutant of
`textual` that `test_textual_runs` lets pass is killed only by the whole suite.

`SCOPE_CASE` adds the one behaviour a fallback needs: `test_case` starts a shell, a child that
ends by `os._exit`, a child that replaces its image, or a thread it leaves running; `tool` takes
every monitoring tool id at import; `ghost` defines a check only while a trace is being taken.
"""

import os
import subprocess
import sys
import threading
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import scope_target  # noqa: E402

CASE = os.environ.get("SCOPE_CASE", "")
EXECUTES = {
    "in_process": ["test_in_process"],
    "shared": ["test_shared_one", "test_shared_two"],
    "in_child": ["test_in_child"],
    "at_class_body": [],
    "at_set_up_class": [],
    "unreached": [],
    "textual": ["test_textual_runs"],
}
#: The child that executes `in_child`, with the target's directory as its one argument.
CHILD = (
    "import sys\n"
    "sys.path.insert(0, sys.argv[1])\n"
    "import scope_target\n"
    "print(scope_target.in_child(5, 3))\n"
)
#: A child that ends without its exit handlers, and one that replaces its own image.
QUIT = "import os\nos._exit(0)\n"
EXEC = "import os, sys\nos.execv(sys.executable, [sys.executable, '-c', 'pass'])\n"
HELD = threading.Event()

if CASE == "tool":
    for tool in range(6):
        sys.monitoring.free_tool_id(tool)
        sys.monitoring.use_tool_id(tool, "scope-fixture")


def child(program, *args):
    return subprocess.run(
        [sys.executable, "-c", program, *args],
        capture_output=True,
        text=True,
        check=True,
        timeout=60,
    )


class Lines(unittest.TestCase):
    BODY = scope_target.at_class_body(7, 3)

    @staticmethod
    def setUpClass():
        scope_target.at_set_up_class(7, 3)

    def test_case(self):
        if CASE == "shell":
            subprocess.run(["sh", "-c", ":"], check=True, timeout=60)
        elif CASE == "quit":
            child(QUIT)
        elif CASE == "exec":
            child(EXEC)
        elif CASE == "thread":
            threading.Thread(target=HELD.wait, daemon=True).start()

    def test_in_child(self):
        self.assertEqual(child(CHILD, str(SCRIPTS)).stdout, "2\n")

    def test_in_process(self):
        self.assertEqual(scope_target.in_process(5, 3), 8)

    def test_reads_text(self):
        text = Path(scope_target.__file__).read_text(encoding="utf-8")
        self.assertIn("    return a & b\n", text)

    def test_shared_one(self):
        self.assertEqual(scope_target.shared(5, 3), 15)

    def test_shared_two(self):
        self.assertEqual(scope_target.shared(2, 4), 8)

    def test_textual_runs(self):
        scope_target.textual(6, 3)

    def test_width(self):
        self.assertEqual(scope_target.WIDTH, 2)


if CASE == "ghost" and os.environ.get("MUTATION_PYTHON_TRACE_DIR"):

    class Ghost(unittest.TestCase):
        def test_ghost(self):
            scope_target.unreached(2, 3)
