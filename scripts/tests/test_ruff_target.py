"""SPEC-626: ruff reads the tree as Python 3.12 code, the Python its tests need and CI runs
(ADR-648).

The test parses the committed `ruff.toml` and holds its `target-version` to `py312`, so a change
of target is a reviewed decision and never a drift. It reads one file and starts no process.
"""

import tomllib
import unittest

from _support import REPO

CONFIG = REPO / "ruff.toml"
#: The Python the tree's tests need and CI runs (SPEC-626 R1).
TARGET = "py312"


class RuffTarget(unittest.TestCase):
    def test_ruff_targets_python_312(self):
        settings = tomllib.loads(CONFIG.read_text(encoding="utf-8"))
        self.assertEqual(
            settings.get("target-version"),
            TARGET,
            "ruff.toml must target py312, the Python the tests need and CI runs",
        )


if __name__ == "__main__":
    unittest.main()
