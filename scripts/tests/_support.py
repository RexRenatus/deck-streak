"""Shared helpers for the repository's own guard tests."""

from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def examined(what, items):
    """Print how many items a guard examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items
