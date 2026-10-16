"""The four state-machine lints' one helper (SPEC-433 R1 to R11, ADR-447 D1 to D5).

Each rule is a test module of its own (`test_fsm_check_list.py`, `test_fsm_wildcard.py`,
`test_fsm_unreachable.py`, `test_fsm_uncovered.py`), and their shared behaviour is tested in
`test_fsm_lints.py`. This module holds what they share. It reads committed files and writes
nothing.

This is the red commit's helper: it keeps every input the tests hand it (the data types, the three
walks, the register's file and the entries' keys) and none of the behaviour. Each rule returns a
reading whose file count is its walk's and which holds nothing else, never VOID and printing
nothing; the register is parsed and nothing in it is refused; and `report` prints nothing.
"""

import json
from dataclasses import dataclass, field
from pathlib import Path

#: Where the register lives, and where a ruling one of its entries cites must live (R1, R2).
REGISTER = "config/fsm-lints.json"
RULINGS = "docs/rulings/"

CHECK_LIST = "FSM-CHECK-LIST"
WILDCARD = "FSM-WILDCARD"
UNREACHABLE = "FSM-UNREACHABLE"
UNCOVERED = "FSM-UNCOVERED"
#: The fields that key each rule's ratchet entries: a column, or a site by path and item (R2).
ENTRY_KEY = {
    CHECK_LIST: ("table", "column"),
    WILDCARD: ("path", "enum", "fn"),
    UNREACHABLE: ("path", "fn", "macro"),
    UNCOVERED: ("path", "enum"),
}
#: R11: a SQLite that cannot run this cannot build the schema the migrations declare.
STRICT_PROBE = "CREATE TABLE fsm_strict_probe (probe TEXT) STRICT"


class Refused(AssertionError):
    """The register breaks R1 or R2, which fails at every level."""


class Void(AssertionError):
    """A reading that judged nothing, or could not judge what it was given (R11), at every level."""


@dataclass(frozen=True)
class Hit:
    """One site a rule names: its kind (`new`, `listed` or `stale`), where it is, and its key."""

    rule: str
    kind: str
    path: str
    line: int
    key: tuple


@dataclass
class Reading:
    """One rule's reading of one tree: how many files its walk found, how many units it examined,
    its hits, and what it read on the way (the pairs it compared, the domain columns of the final
    schema, the matches on a state enum, the state enums and the cover lines)."""

    rule: str
    level: str
    unit: str
    files: int
    examined: int = 0
    hits: list = field(default_factory=list)
    pairs: list = field(default_factory=list)
    columns: dict = field(default_factory=dict)
    matches: list = field(default_factory=list)
    enums: list = field(default_factory=list)
    covers: list = field(default_factory=list)


def product_files(root):
    """R6: every `.rs` file under `crates/*/src`, at any depth, in path order."""
    return sorted(path for path in (Path(root) / "crates").glob("*/src/**/*.rs") if path.is_file())


def migration_files(root):
    """R3: every `migrations/*.sql`, in filename order."""
    return sorted(path for path in (Path(root) / "migrations").glob("*.sql") if path.is_file())


def formal_files(root):
    """R9: every `.tla` and `.lean` module under `formal/`, at any depth, in path order."""
    return sorted(
        path
        for path in (Path(root) / "formal").rglob("*")
        if path.suffix in (".tla", ".lean") and path.is_file()
    )


def read_register(root):
    """The register at `root`, parsed (R1)."""
    return json.loads((Path(root) / REGISTER).read_text(encoding="utf-8"))


def entry_key(name, entry):
    """The key of one of `name`'s ratchet entries (R2)."""
    return tuple(entry[part] for part in ENTRY_KEY[name])


def admit(root, name, entries, seed):
    """The keys of `name`'s ratchet entries (R2)."""
    return {entry_key(name, entry) for entry in entries}


def check_list(root, rule, seed):
    """FSM-CHECK-LIST (R3 to R5)."""
    return Reading(CHECK_LIST, rule["level"], "columns", len(migration_files(root)))


def wildcard(root, rule, seed):
    """FSM-WILDCARD (R6, R7)."""
    return Reading(WILDCARD, rule["level"], "matches", len(product_files(root)))


def unreachable(root, rule, seed):
    """FSM-UNREACHABLE (R8)."""
    return Reading(UNREACHABLE, rule["level"], "arms", len(product_files(root)))


def uncovered(root, rule, seed):
    """FSM-UNCOVERED (R9)."""
    return Reading(UNCOVERED, rule["level"], "enums", len(formal_files(root)))


def report(reading):
    """Print the reading's lines and apply its level (R10). The printed lines are returned."""
    return []
