"""A planted module for SPEC-633 A1: it passes only while barrier_b.py runs at the same time."""

import os
import time
import unittest
from pathlib import Path

HERE, OTHER = "a", "b"


class Barrier(unittest.TestCase):
    def test_meets_the_other_module(self):
        meeting = Path(os.environ["PYTHON_STAGE_BARRIER"])
        (meeting / HERE).touch()
        wait = float(os.environ.get("PYTHON_STAGE_BARRIER_WAIT", "10"))
        deadline = time.monotonic() + wait
        while not (meeting / OTHER).exists():
            if time.monotonic() > deadline:
                self.fail(f"barrier timeout: module {OTHER} did not arrive within {wait:g} s")
            time.sleep(0.05)
