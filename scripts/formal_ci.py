#!/usr/bin/env python3
"""The formal entries' checks that CI runs (SPEC-614, ADR-636): the inert seam of the red commit.

It defines every function and verb the tests import, finds nothing and examines nothing, and
reads no file.
"""

import sys


def span(path, text, anchor):
    """The span of the item `anchor` names in `text`, or None with the reason."""
    return None, "not read"


def covers(root):
    """Every cover's finding lines, as (examined, findings, listed)."""
    return 0, [], []


def audit_source(root):
    """The lines of the audit file and the declarations it names."""
    return [], []


def audit(root, output):
    """The declarations examined and the findings over Lean's output."""
    return 0, []


def main(argv=None):
    return 0


if __name__ == "__main__":
    sys.exit(main())
