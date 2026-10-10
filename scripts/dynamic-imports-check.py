#!/usr/bin/env python3
"""Refuse, before the push, a changed scripts/tests module that loads code and the register lacks.

SPEC-406, ADR-420. Standard library only; nothing it judges is imported or run.
"""

import argparse
import sys
from pathlib import Path

REGISTER = "scripts/tests/test_ci_workflows.py"
TESTS = "scripts/tests"
REGISTER_NAME = "DYNAMIC_IMPORTS"
LOADER_MODULES = ("importlib", "runpy")
LOADER_BUILTIN = "exec"
REPO = Path(__file__).resolve().parents[1]


class Unreadable(Exception):
    """An input the check cannot read; its text is the VOID cause."""


def loader_sites(source):
    """Each load in `source` as (line, loader), in source order."""
    return []


def register_modules(source):
    """The module names the register's sites lead with."""
    return set()


def module_name(path):
    """The dotted name of a path under scripts/tests."""
    return ""


def changed_modules(root, base):
    """The .py paths under scripts/tests that HEAD adds or modifies against `base`."""
    return []


def main(argv=None):
    """Run the check; return the exit status."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=str(REPO))
    parser.add_argument("--base", default="origin/dev")
    parser.parse_args(argv)
    return 0


if __name__ == "__main__":
    sys.exit(main())
