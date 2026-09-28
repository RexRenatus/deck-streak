#!/usr/bin/env python3
"""row_target: a mutation row's target, read by its table's declared spelling (SPEC-039 R8).

The rows file spells a row's target two ways: `MUTATIONS` stores a crate directory and a path
inside that crate, and the two script tables store a path from the repository root. The header
(`scripts/mutation-rows.json`) declares each table's spelling on its own face, one line of its `_`
per table:

    target spelling: MUTATIONS is crate-relative, crates/{cell 1}/{cell 2}
    target spelling: SCRIPT_MUTATIONS is repo-rooted, {cell 1}

This module reads those lines and resolves a row by them. It keeps the interface the vendored
mutation-rows pack's probe calls (`declared_spellings`, `row_target`, `UnresolvableTarget`), so
the probe's `mutants-distinct` class judges DeckStreak's rows with DeckStreak's own resolver.

A spelling is read, never guessed: a table the document carries with no declaration is refused by
name, and so is a declaration naming a spelling nobody defined. A resolved target is normalised
lexically, and a row too short to carry the cells its spelling reads is refused.
"""

from __future__ import annotations

import posixpath
from collections.abc import Mapping, Sequence

SPELLING_PREFIX = "target spelling: "
CRATE_RELATIVE = "crate-relative"
REPO_ROOTED = "repo-rooted"
SPELLINGS = (CRATE_RELATIVE, REPO_ROOTED)


class UnresolvableTarget(ValueError):
    """A table declares no target spelling, or a row lacks a cell its spelling reads."""


def declared_spellings(document: Mapping[str, object]) -> dict[str, str]:
    """`{table: spelling}` as the document's `_` declares it, for every table it carries."""
    legend = document.get("_")
    spellings: dict[str, str] = {}
    for line in legend if isinstance(legend, list) else []:
        if not isinstance(line, str) or not line.startswith(SPELLING_PREFIX):
            continue
        table, separator, rest = line[len(SPELLING_PREFIX) :].partition(" is ")
        word = rest.split(",", 1)[0].strip()
        if not separator or not table or word not in SPELLINGS:
            raise UnresolvableTarget(
                f"REFUSING: the line {line!r} declares no spelling this reader knows"
            )
        if table in spellings:
            raise UnresolvableTarget(f"REFUSING: table {table} declares its spelling twice")
        spellings[table] = word
    tables = document.get("tables")
    for table in tables if isinstance(tables, Mapping) else ():
        if table not in spellings:
            raise UnresolvableTarget(f"REFUSING: table {table} declares no target spelling")
    return spellings


def row_target(table: str, row: Sequence[str], spellings: Mapping[str, str]) -> str:
    """A row's target, repository-relative and lexically normalised."""
    spelling = spellings.get(table)
    if spelling is None:
        raise UnresolvableTarget(f"REFUSING: table {table} declares no target spelling")
    identifier = row[0] if row else "<no id>"
    if spelling == CRATE_RELATIVE:
        if len(row) < 3:
            raise UnresolvableTarget(
                f"REFUSING: {table} row {identifier!r} carries no crate and path in cells 1 and 2"
            )
        return posixpath.normpath(f"crates/{row[1]}/{row[2]}")
    if len(row) < 2:
        raise UnresolvableTarget(f"REFUSING: {table} row {identifier!r} carries no path in cell 1")
    return posixpath.normpath(row[1])
