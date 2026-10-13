"""The leg control's checks (SPEC-420 R9), named so that discovery never collects them.

`test_direct` kills the mutant of the line it executes in the runner's own process. `test_in_child`
kills the mutant of a line that only the child process it starts executes, so the runner must see
that child to keep the check. `test_unobserved` executes its line and asserts nothing, so the
mutant there survives every honest run.
"""

import subprocess
import sys
import unittest
from pathlib import Path

import control_target

HERE = Path(__file__).resolve().parent
#: The child's program: the target imported from the directory given as its one argument.
CHILD = (
    "import sys\n"
    "sys.path.insert(0, sys.argv[1])\n"
    "import control_target\n"
    "box = []\n"
    "control_target.in_child(box, 2, 3)\n"
    "print(box[0])\n"
)


class Control(unittest.TestCase):
    def test_direct(self):
        box = []
        control_target.direct(box, 2, 3)
        self.assertEqual(box, [5])

    def test_in_child(self):
        done = subprocess.run(
            [sys.executable, "-c", CHILD, str(HERE)],
            cwd=HERE,
            capture_output=True,
            text=True,
            check=True,
            timeout=60,
        )
        self.assertEqual(done.stdout, "6\n")

    def test_unobserved(self):
        control_target.unobserved([], 2, 3)
