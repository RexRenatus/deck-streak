"""Shared helpers for the repository's own guard tests."""

import os
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def examined(what, items):
    """Print how many items a guard examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


def refuse_link_components(path, root):
    """Refuse, by assertion, a link at any component of `path` below `root`, the root excluded and
    the path's last component included. A guard reads a path as the tree stores it: a link there
    holds the name it points at and no file, so a link at a directory above the file, like one at
    the file, is refused and never followed. Each component is judged before the next is touched,
    so a link that loops or dangles is refused as a link and never read through."""
    root = Path(root)
    try:
        relative = Path(path).relative_to(root)
    except ValueError:
        raise AssertionError(f"{path} is not below {root}") from None
    walked = root
    for part in relative.parts:
        walked = walked / part
        if os.path.islink(walked):
            raise AssertionError(
                f"{walked.relative_to(root).as_posix()} is a link: the tree stores the name it "
                "points at and no file, and a link is never followed"
            )
