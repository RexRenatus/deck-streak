#!/usr/bin/env python3
"""row_target: a mutation row's target, read by its table's declared spelling (SPEC-039 R8).

Red stub: the interface the vendored mutation-probe.py calls, resolving nothing yet.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence


class UnresolvableTarget(ValueError):
    """A table declares no target spelling, or a row lacks a cell its spelling reads."""


def declared_spellings(document: Mapping[str, object]) -> dict[str, str]:
    return {}


def row_target(table: str, row: Sequence[str], spellings: Mapping[str, str]) -> str:
    raise UnresolvableTarget(f"REFUSING: table {table} declares no target spelling")
