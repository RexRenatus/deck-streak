#!/usr/bin/env python3
"""data-migration-probe: judge a declared data port against the tree that carries it (SPEC-V2-2221).

A repository that moves data from an old system's SQLite database into a new schema states the move
in `data-migration.json` (schema `phx.data-migration.v1`): the source's pinned `user_version` and its
full schema snapshot, the target's sqlx migrations, one action per source table with a count rule,
the tool and its runbook, the parity oracle's goldens, and the tests that prove the dry run,
idempotency, rollback and the count reconciliation. Every class below reads that plan and the files
it names, and nothing else: the check is static, fetches nothing, runs nothing it judges, and is
standard-library Python (3.11+, for `tomllib`) so another repository can vendor it.

    python3 scripts/data-migration-probe.py --root R [--manifest PATH] check <class>
    python3 scripts/data-migration-probe.py list

Every class prints one line per finding, `<class>: <finding>`, then `examined N`. Exit 0 is green,
1 a finding, 2 a usage error, 3 VOID: nothing was examined, or the plan could not be read. A VOID is
never a pass.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import tomllib
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

EXIT_OK = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

SCHEMA = "phx.data-migration.v1"
GOLDEN_SCHEMA = "phx.parity-golden.v1"
DEFAULT_MANIFEST = "data-migration.json"

TOP_KEYS = {
    "schema",
    "source",
    "target",
    "tool",
    "runbook",
    "tables",
    "oracle",
    "proofs",
}
SOURCE_KEYS = {"system", "user_version", "schema"}
TARGET_KEYS = {"migrations", "database"}
TOOL_KEYS = {"path", "command", "apply_flag"}
TABLE_KEYS = {"source", "action", "target", "targets", "count", "reason"}
ORACLE_KEYS = {
    "source_commit",
    "generator",
    "goldens",
    "consumers",
    "functions",
    "edge_classes",
}
PROOF_KEYS = ("dry_run", "idempotency", "rollback", "counts")
GOLDEN_KEYS = {
    "schema",
    "function",
    "source_commit",
    "generator",
    "generator_sha256",
    "inputs",
    "seed",
    "cases",
}
CASE_KEYS = {"input", "output", "class", "diverges", "note"}
ACTIONS = ("copy", "transform", "split", "merge", "drop")
#: The count rules each action may state. `filtered` and `declared` also need a `reason`.
COUNT_RULES = {
    "copy": ("equal",),
    "transform": ("equal", "filtered", "declared"),
    "split": ("sum", "declared"),
    "merge": ("sum", "declared"),
}
REASONED_COUNTS = ("filtered", "declared")
SHA1 = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")
PROOF = re.compile(r"(?P<path>[^:]+)::(?P<name>[A-Za-z_][A-Za-z0-9_]*)")

# An identifier as SQLite accepts it in CREATE TABLE: "double", [bracket], `backtick`, 'single'
# (legacy, and what `.schema` prints for FTS shadow tables), or a bare word.
_IDENT = r"(?:\"(?:[^\"]|\"\")+\"|\[[^\]]+\]|`(?:[^`]|``)+`|'(?:[^']|'')+'|[A-Za-z_][A-Za-z0-9_$]*)"
CREATE_TABLE = re.compile(
    r"\bCREATE\s+(?P<temp>TEMP\s+|TEMPORARY\s+)?(?P<virtual>VIRTUAL\s+)?TABLE\s+"
    r"(?:IF\s+NOT\s+EXISTS\s+)?"
    rf"(?:(?P<schema>{_IDENT})\s*\.\s*)?(?P<name>{_IDENT})",
    re.IGNORECASE,
)
DROP_TABLE = re.compile(
    rf"\bDROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:{_IDENT}\s*\.\s*)?(?P<name>{_IDENT})",
    re.IGNORECASE,
)
RENAME_TABLE = re.compile(
    rf"\bALTER\s+TABLE\s+(?:{_IDENT}\s*\.\s*)?(?P<old>{_IDENT})\s+RENAME\s+TO\s+(?P<new>{_IDENT})",
    re.IGNORECASE,
)
USER_VERSION = re.compile(
    r"\bPRAGMA\s+(?:(?:main|\"main\")\s*\.\s*)?user_version\s*=\s*(?P<value>[+-]?\d+)",
    re.IGNORECASE,
)
# sqlx's migration file grammar: `<VERSION>_<DESCRIPTION>.sql`, or `.up.sql` and `.down.sql` for a
# reversible one. sqlx ignores any other file in the directory (i1492), so this does too.
SQLX_MIGRATION = re.compile(
    r"^(?P<version>\d+)_(?P<description>.+?)(?P<kind>\.up|\.down)?\.sql$"
)
FTS_SHADOWS = (
    "_data",
    "_idx",
    "_content",
    "_docsize",
    "_config",
    "_segments",
    "_segdir",
    "_stat",
)
READ_ONLY_MARKERS = (
    re.compile(r"\.read_only\s*\(\s*true\s*\)"),
    re.compile(r"[?&]mode=ro\b"),
    re.compile(r"[?&]immutable=1\b"),
    re.compile(r"\bSQLITE_OPEN_READ_?ONLY\b"),
    re.compile(r"\bPRAGMA\s+query_only\s*=\s*(?:1|ON|TRUE)\b", re.IGNORECASE),
)
BACKUP_MARKERS = (
    re.compile(r"\bVACUUM\s+(?:\w+\s+)?INTO\b", re.IGNORECASE),
    re.compile(r"(?:^|[\s\"'])\.backup\b"),
    re.compile(r"\bsqlite3_rsync\b"),
    re.compile(r"\blitestream\s+restore\b.*\s-o(?:\s|=)"),
)
INTEGRITY = re.compile(
    r"\b(?:integrity_check|quick_check)\b|-integrity-check\b", re.IGNORECASE
)
FOREIGN_KEYS = re.compile(r"\bforeign_key_check\b", re.IGNORECASE)
DB_SUFFIX = re.compile(r"\.(?:db|sqlite3?|anki2)(?:-wal|-shm|-journal)?$")
COPY_COMMANDS = ("cp", "rsync", "scp")
COMMAND_PREFIXES = ("sudo", "doas", "env", "nice", "ionice", "time", "command", "exec")
SOURCE_SUFFIXES = (".rs", ".py", ".sql", ".sh")
SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    "build",
}
FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(?P<level>#{1,6})\s+(?P<title>.+?)\s*#*\s*$")


class Unreadable(Exception):
    """The plan cannot be read, so no class can judge it: VOID."""


class Refused(Exception):
    """A file this class needs is not what it has to be, named in the finding."""


@dataclass
class Outcome:
    """A class's verdict: its findings and the size of the population it read."""

    findings: list[str] = field(default_factory=list)
    examined: int = 0
    void: str | None = None

    def exit_code(self) -> int:
        if self.void is not None or self.examined == 0:
            return EXIT_VOID
        return EXIT_FINDING if self.findings else EXIT_OK


# --------------------------------------------------------------------------------------------
# Reading.
# --------------------------------------------------------------------------------------------


def _no_constant(name: str) -> object:
    raise ValueError(f"{name} is not JSON (RFC 8259 section 6)")


def _unique_pairs(pairs: list[tuple[str, object]]) -> dict:
    seen: dict[str, object] = {}
    for key, value in pairs:
        if key in seen:
            raise ValueError(f"the key {key!r} appears twice")
        seen[key] = value
    return seen


def strict_json(text: str) -> object:
    """JSON as RFC 8259 reads it: `NaN`, `Infinity` and a repeated key are refused, not accepted.

    Python's own `json.loads` accepts all three, and a Rust consumer through serde_json refuses the
    first two, so a golden Python wrote could never be read by the port it is meant to prove.
    """
    return json.loads(
        text, parse_constant=_no_constant, object_pairs_hook=_unique_pairs
    )


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def inside(root: Path, relative: str) -> Path | None:
    """`relative` resolved under `root`, or None when it is absolute or climbs out of it."""
    if not isinstance(relative, str) or not relative.strip():
        return None
    pure = PurePosixPath(relative.strip())
    if pure.is_absolute() or ".." in pure.parts:
        return None
    return root / Path(*pure.parts) if pure.parts else None


def normalised(relative: str) -> str:
    """A repository-relative path spelled one way: no `./`, no doubled or trailing slash."""
    return PurePosixPath(relative.strip()).as_posix().removeprefix("./")


def strip_sql_comments(text: str) -> str:
    """SQL without its `--` and `/* */` comments, keeping quoted text intact."""
    out: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        if char in "'\"`[":
            close = "]" if char == "[" else char
            end = index + 1
            while end < length:
                if text[end] == close:
                    if close != "]" and end + 1 < length and text[end + 1] == close:
                        end += 2
                        continue
                    break
                end += 1
            out.append(text[index : end + 1])
            index = end + 1
        elif text.startswith("--", index):
            newline = text.find("\n", index)
            index = length if newline == -1 else newline
        elif text.startswith("/*", index):
            close_at = text.find("*/", index + 2)
            index = length if close_at == -1 else close_at + 2
            out.append(" ")
        else:
            out.append(char)
            index += 1
    return "".join(out)


def strip_code_comments(text: str, suffix: str) -> str:
    """Source without its comments: `//` and `/* */` for Rust, `#` for Python and shell, SQL's own.

    A marker a class looks for must be code, not a sentence about code: `// read_only(true)` in a
    comment opens nothing read-only.
    """
    if suffix == ".sql":
        return strip_sql_comments(text)
    if suffix in (".py", ".sh"):
        kept = []
        for line in text.splitlines():
            kept.append(_strip_hash_comment(line))
        return "\n".join(kept)
    out: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        if char == '"':
            end = index + 1
            while end < length and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            out.append(text[index : end + 1])
            index = end + 1
        elif text.startswith("//", index):
            newline = text.find("\n", index)
            index = length if newline == -1 else newline
        elif text.startswith("/*", index):
            close_at = text.find("*/", index + 2)
            index = length if close_at == -1 else close_at + 2
            out.append(" ")
        else:
            out.append(char)
            index += 1
    return "".join(out)


def _strip_hash_comment(line: str) -> str:
    quote = None
    for index, char in enumerate(line):
        if quote:
            if char == quote:
                quote = None
        elif char in "'\"":
            quote = char
        elif char == "#" and (index == 0 or line[index - 1].isspace()):
            return line[:index]
    return line


def unquote_identifier(raw: str) -> str:
    if len(raw) >= 2 and raw[0] in "\"'`" and raw[-1] == raw[0]:
        return raw[1:-1].replace(raw[0] * 2, raw[0])
    if len(raw) >= 2 and raw[0] == "[" and raw[-1] == "]":
        return raw[1:-1]
    return raw


def fenced_lines(text: str) -> list[str]:
    """The lines inside the document's fenced code blocks, in order: the commands a runbook runs."""
    lines: list[str] = []
    fenced = False
    for line in text.splitlines():
        if FENCE.match(line):
            fenced = not fenced
            continue
        if fenced and line.strip():
            lines.append(line.strip())
    return lines


def sections(text: str) -> list[tuple[str, list[str]]]:
    """Each heading's title and the fenced command lines of its section, up to the next heading."""
    found: list[tuple[str, list[str]]] = []
    fenced = False
    for line in text.splitlines():
        if FENCE.match(line):
            fenced = not fenced
            continue
        if not fenced:
            match = HEADING.match(line)
            if match:
                found.append((match.group("title"), []))
                continue
        if fenced and line.strip() and found:
            found[-1][1].append(line.strip())
    return found


def walk_files(base: Path, suffixes: Sequence[str]) -> list[Path]:
    if base.is_file():
        return [base] if base.suffix in suffixes else []
    if not base.is_dir():
        return []
    found = []
    for path in sorted(base.rglob("*")):
        if any(part in SKIP_DIRS for part in path.relative_to(base).parts):
            continue
        if path.is_file() and path.suffix in suffixes:
            found.append(path)
    return found


# --------------------------------------------------------------------------------------------
# The plan.
# --------------------------------------------------------------------------------------------


@dataclass
class Plan:
    """The repository's `data-migration.json`, read once, and the root its paths resolve under."""

    root: Path
    path: Path
    document: dict

    def section(self, key: str) -> dict:
        value = self.document.get(key)
        return value if isinstance(value, dict) else {}

    def tables(self) -> list[dict]:
        value = self.document.get("tables")
        return (
            [entry for entry in value if isinstance(entry, dict)]
            if isinstance(value, list)
            else []
        )

    def file(self, relative: object) -> Path | None:
        return inside(self.root, relative) if isinstance(relative, str) else None

    def display(self, path: Path) -> str:
        try:
            return path.relative_to(self.root).as_posix()
        except ValueError:
            return str(path)


def load_plan(root: Path, manifest: str | None) -> Plan:
    path = Path(manifest) if manifest else root / DEFAULT_MANIFEST
    if not path.is_absolute():
        path = root / path
    if not path.is_file():
        raise Unreadable(f"no {DEFAULT_MANIFEST} at {path}")
    try:
        document = strict_json(read_text(path))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise Unreadable(f"{path.name} is not strict JSON: {error}") from error
    if not isinstance(document, dict):
        raise Unreadable(f"{path.name} is not a JSON object")
    return Plan(root=root, path=path, document=document)


def targets_of(entry: dict) -> list[str]:
    if isinstance(entry.get("targets"), list):
        return [name for name in entry["targets"] if isinstance(name, str) and name]
    target = entry.get("target")
    return [target] if isinstance(target, str) and target else []


# --------------------------------------------------------------------------------------------
# Stage `plan`.
# --------------------------------------------------------------------------------------------


def check_manifest(plan: Plan) -> Outcome:
    """The plan is closed and well-typed: every key known, every required key present."""
    outcome = Outcome(examined=1)
    document = plan.document
    add = outcome.findings.append
    if document.get("schema") != SCHEMA:
        add(f"schema is {document.get('schema')!r}, not {SCHEMA!r}")
    for key in sorted(set(document) - TOP_KEYS):
        add(f"unknown key {key!r}")
    for key in ("source", "target", "tool", "oracle", "proofs"):
        if not isinstance(document.get(key), dict):
            add(f"{key} is not an object")
    if not isinstance(document.get("runbook"), str) or not document["runbook"].strip():
        add("runbook names no file")
    source = plan.section("source")
    for key in sorted(set(source) - SOURCE_KEYS):
        add(f"source has an unknown key {key!r}")
    version = source.get("user_version")
    if isinstance(version, bool) or not isinstance(version, int) or version < 0:
        add("source.user_version is not a non-negative integer")
    if plan.file(source.get("schema")) is None:
        add("source.schema names no file inside the repository")
    target = plan.section("target")
    for key in sorted(set(target) - TARGET_KEYS):
        add(f"target has an unknown key {key!r}")
    if plan.file(target.get("migrations")) is None:
        add("target.migrations names no directory inside the repository")
    tool = plan.section("tool")
    for key in sorted(set(tool) - TOOL_KEYS):
        add(f"tool has an unknown key {key!r}")
    if plan.file(tool.get("path")) is None:
        add("tool.path names nothing inside the repository")
    if not isinstance(tool.get("command"), str) or not tool["command"].strip():
        add("tool.command is not the command line that runs the tool")
    flag = tool.get("apply_flag")
    if not isinstance(flag, str) or not re.fullmatch(r"--[a-z0-9][a-z0-9-]*", flag):
        add("tool.apply_flag is not a long flag such as --apply")
    tables = document.get("tables")
    if not isinstance(tables, list) or not tables:
        add("tables is not a non-empty list")
    else:
        seen: set[str] = set()
        for index, entry in enumerate(tables):
            outcome.examined += 1
            where = f"tables[{index}]"
            if not isinstance(entry, dict):
                add(f"{where} is not an object")
                continue
            for key in sorted(set(entry) - TABLE_KEYS):
                add(f"{where} has an unknown key {key!r}")
            name = entry.get("source")
            if not isinstance(name, str) or not name.strip():
                add(f"{where} names no source table")
            else:
                where = f"table {name}"
                if name.casefold() in seen:
                    add(f"{where} is listed twice")
                seen.add(name.casefold())
            action = entry.get("action")
            if action not in ACTIONS:
                add(f"{where}: action {action!r} is none of {', '.join(ACTIONS)}")
                continue
            if action == "drop":
                if (
                    not isinstance(entry.get("reason"), str)
                    or not entry["reason"].strip()
                ):
                    add(f"{where} is dropped with no reason")
                if targets_of(entry):
                    add(f"{where} is dropped and still names a target")
            elif action == "split":
                names = entry.get("targets")
                if (
                    not isinstance(names, list)
                    or len(names) < 2
                    or not all(isinstance(name, str) and name for name in names)
                ):
                    add(f"{where} is split into fewer than two named targets")
            elif (
                not isinstance(entry.get("target"), str) or not entry["target"].strip()
            ):
                add(f"{where} names no target table")
    oracle = plan.section("oracle")
    for key in sorted(set(oracle) - ORACLE_KEYS):
        add(f"oracle has an unknown key {key!r}")
    if not isinstance(oracle.get("source_commit"), str) or not SHA1.fullmatch(
        oracle["source_commit"]
    ):
        add("oracle.source_commit is not a 40-hex commit")
    for key in ("generator", "goldens"):
        if plan.file(oracle.get(key)) is None:
            add(f"oracle.{key} names nothing inside the repository")
    consumers = oracle.get("consumers")
    if (
        not isinstance(consumers, list)
        or not consumers
        or any(plan.file(c) is None for c in consumers)
    ):
        add("oracle.consumers is not a non-empty list of repository paths")
    functions = oracle.get("functions")
    if (
        not isinstance(functions, list)
        or not functions
        or not all(isinstance(name, str) and name.strip() for name in functions)
    ):
        add("oracle.functions is not a non-empty list of function names")
    elif len(set(functions)) != len(functions):
        add("oracle.functions names a function twice")
    classes = oracle.get("edge_classes")
    if classes is not None and (
        not isinstance(classes, dict)
        or not all(
            isinstance(value, list)
            and all(isinstance(item, str) and item for item in value)
            for value in classes.values()
        )
    ):
        add("oracle.edge_classes is not an object of function to class names")
    proofs = plan.section("proofs")
    for key in sorted(set(proofs) - set(PROOF_KEYS)):
        add(f"proofs has an unknown key {key!r}")
    for key in PROOF_KEYS:
        if not isinstance(proofs.get(key), str) or not PROOF.fullmatch(proofs[key]):
            add(f"proofs.{key} is not a path::test_name")
    return outcome


def snapshot_tables(text: str) -> tuple[dict[str, str], int]:
    """The persisted tables a schema snapshot creates, casefolded to their spelling, and its virtual
    tables' count. Internal `sqlite_` tables, TEMP tables and FTS shadow tables are not data the
    plan maps: SQLite maintains them itself."""
    code = strip_sql_comments(text)
    tables: dict[str, str] = {}
    virtual: list[str] = []
    for match in CREATE_TABLE.finditer(code):
        if match.group("temp"):
            continue
        name = unquote_identifier(match.group("name"))
        if name.casefold().startswith("sqlite_"):
            continue
        if match.group("virtual"):
            virtual.append(name.casefold())
        tables.setdefault(name.casefold(), name)
    for table in virtual:
        for shadow in FTS_SHADOWS:
            tables.pop(table + shadow, None)
    return tables, len(virtual)


def read_snapshot(plan: Plan) -> tuple[Path | None, str | None, str | None]:
    """The snapshot's path and text, or why it could not be read."""
    path = plan.file(plan.section("source").get("schema"))
    if path is None:
        return None, None, "source.schema names no file"
    if not path.is_file():
        return path, None, f"source.schema {plan.display(path)} does not exist"
    try:
        return path, read_text(path), None
    except (OSError, UnicodeDecodeError) as error:
        return path, None, f"cannot read {plan.display(path)}: {error}"


def tool_sources(plan: Plan) -> list[Path]:
    path = plan.file(plan.section("tool").get("path"))
    return walk_files(path, (".rs", ".py")) if path is not None else []


def code_text(path: Path) -> str:
    try:
        return strip_code_comments(read_text(path), path.suffix)
    except (OSError, UnicodeDecodeError):
        return ""


def check_source_pinned(plan: Plan) -> Outcome:
    """The snapshot pins the source's `user_version`, the plan agrees, and the tool reads it."""
    outcome = Outcome()
    declared = plan.section("source").get("user_version")
    path, text, why = read_snapshot(plan)
    outcome.examined += 1
    if why:
        outcome.findings.append(why)
    else:
        assert text is not None and path is not None
        pins = {
            int(match.group("value"))
            for match in USER_VERSION.finditer(strip_sql_comments(text))
        }
        if not pins:
            outcome.findings.append(
                f"{plan.display(path)} states no PRAGMA user_version, so the schema it records "
                "is pinned to no version"
            )
        elif len(pins) > 1:
            outcome.findings.append(
                f"{plan.display(path)} states user_version {sorted(pins)}: one snapshot, one version"
            )
        elif (
            isinstance(declared, int)
            and not isinstance(declared, bool)
            and pins != {declared}
        ):
            outcome.findings.append(
                f"{plan.display(path)} is at user_version {pins.pop()}, the plan pins {declared}"
            )
    sources = tool_sources(plan)
    outcome.examined += len(sources)
    if not sources:
        outcome.findings.append(
            "the tool has no Rust or Python source to read its guard from"
        )
    elif not any(
        re.search(r"\buser_version\b", code_text(source)) for source in sources
    ):
        outcome.findings.append(
            "the tool never reads user_version, so a source at another version is migrated blind"
        )
    return outcome


def check_tables_covered(plan: Plan) -> Outcome:
    """Every table the source schema creates is mapped exactly once, and nothing else is."""
    outcome = Outcome()
    _, text, why = read_snapshot(plan)
    if why:
        outcome.examined = 1
        outcome.findings.append(why)
        return outcome
    assert text is not None
    tables, _ = snapshot_tables(text)
    outcome.examined = len(tables)
    if not tables:
        outcome.void = "the source schema snapshot creates no table"
        return outcome
    mapped: dict[str, int] = {}
    for entry in plan.tables():
        name = entry.get("source")
        if isinstance(name, str) and name.strip():
            mapped[name.casefold()] = mapped.get(name.casefold(), 0) + 1
    for key, spelled in sorted(tables.items()):
        if key not in mapped:
            outcome.findings.append(
                f"table {spelled} is in the source schema and the plan maps it nowhere"
            )
        elif mapped[key] > 1:
            outcome.findings.append(f"table {spelled} is mapped {mapped[key]} times")
    for key in sorted(set(mapped) - set(tables)):
        outcome.findings.append(
            f"the plan maps {key}, which the source schema does not create"
        )
    return outcome


def replay_migrations(directory: Path) -> tuple[set[str], int]:
    """The tables a sqlx migrations directory leaves behind: every up or simple migration in version
    order, creating, dropping and renaming. A `.down.sql` is a rollback, not a step, and a file that
    is not `<VERSION>_<DESCRIPTION>.sql` is one sqlx never runs."""
    steps: list[tuple[int, str, Path]] = []
    for path in directory.iterdir():
        match = SQLX_MIGRATION.match(path.name)
        if not path.is_file() or match is None or match.group("kind") == ".down":
            continue
        steps.append((int(match.group("version")), path.name, path))
    tables: set[str] = set()
    for _, _, path in sorted(steps):
        code = strip_sql_comments(read_text(path))
        events: list[tuple[int, str, str, str]] = []
        for match in CREATE_TABLE.finditer(code):
            if not match.group("temp"):
                events.append(
                    (
                        match.start(),
                        "create",
                        unquote_identifier(match.group("name")).casefold(),
                        "",
                    )
                )
        for match in DROP_TABLE.finditer(code):
            events.append(
                (
                    match.start(),
                    "drop",
                    unquote_identifier(match.group("name")).casefold(),
                    "",
                )
            )
        for match in RENAME_TABLE.finditer(code):
            events.append(
                (
                    match.start(),
                    "rename",
                    unquote_identifier(match.group("old")).casefold(),
                    unquote_identifier(match.group("new")).casefold(),
                )
            )
        for _, kind, name, new in sorted(events):
            if kind == "create":
                tables.add(name)
            elif kind == "drop":
                tables.discard(name)
            elif name in tables:
                tables.discard(name)
                tables.add(new)
    return tables, len(steps)


def check_targets_exist(plan: Plan) -> Outcome:
    """Every target the plan names is a table the target's migrations leave behind."""
    outcome = Outcome()
    directory = plan.file(plan.section("target").get("migrations"))
    wanted = [
        (entry.get("source"), name)
        for entry in plan.tables()
        if entry.get("action") != "drop"
        for name in targets_of(entry)
    ]
    outcome.examined = len(wanted)
    if not wanted:
        outcome.void = "the plan names no target table"
        return outcome
    if directory is None or not directory.is_dir():
        outcome.findings.append("target.migrations is not a directory of migrations")
        return outcome
    try:
        tables, steps = replay_migrations(directory)
    except (OSError, UnicodeDecodeError) as error:
        outcome.findings.append(f"cannot read the target migrations: {error}")
        return outcome
    if steps == 0:
        outcome.findings.append(
            f"{plan.display(directory)} holds no <VERSION>_<DESCRIPTION>.sql migration sqlx would run"
        )
        return outcome
    for source, name in wanted:
        if name.casefold() not in tables:
            outcome.findings.append(
                f"table {source} maps to {name}, which the target migrations do not leave behind"
            )
    return outcome


def check_count_rules(plan: Plan) -> Outcome:
    """Every migrated table states the count invariant its action allows, with a reason where the
    counts are meant to differ."""
    outcome = Outcome()
    for entry in plan.tables():
        action = entry.get("action")
        if action not in COUNT_RULES:
            continue
        outcome.examined += 1
        name = entry.get("source")
        count = entry.get("count")
        allowed = COUNT_RULES[action]
        if count is None:
            outcome.findings.append(f"table {name} ({action}) states no count rule")
        elif count not in allowed:
            outcome.findings.append(
                f"table {name} ({action}) states count {count!r}; a {action} allows {', '.join(allowed)}"
            )
        elif count in REASONED_COUNTS and (
            not isinstance(entry.get("reason"), str) or not entry["reason"].strip()
        ):
            outcome.findings.append(
                f"table {name} expects {count} counts and gives no reason the counts differ"
            )
    if outcome.examined == 0:
        outcome.void = "the plan migrates no table"
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `oracle`.
# --------------------------------------------------------------------------------------------


@dataclass
class Golden:
    path: Path
    document: dict | None
    error: str | None


def golden_files(plan: Plan) -> tuple[Path | None, list[Path]]:
    directory = plan.file(plan.section("oracle").get("goldens"))
    if directory is None or not directory.is_dir():
        return directory, []
    return directory, sorted(
        path for path in directory.rglob("*.json") if path.is_file()
    )


def load_goldens(plan: Plan) -> tuple[Path | None, list[Golden]]:
    directory, paths = golden_files(plan)
    goldens = []
    for path in paths:
        try:
            document = strict_json(read_text(path))
        except (OSError, UnicodeDecodeError, ValueError) as error:
            goldens.append(Golden(path, None, str(error)))
            continue
        if not isinstance(document, dict):
            goldens.append(Golden(path, None, "is not a JSON object"))
        else:
            goldens.append(Golden(path, document, None))
    return directory, goldens


def cases_of(golden: Golden) -> list[dict]:
    cases = golden.document.get("cases") if golden.document else None
    return (
        [case for case in cases if isinstance(case, dict)]
        if isinstance(cases, list)
        else []
    )


def declared_functions(plan: Plan) -> list[str]:
    functions = plan.section("oracle").get("functions")
    return (
        [name for name in functions if isinstance(name, str)]
        if isinstance(functions, list)
        else []
    )


def readable(goldens: list[Golden]) -> list[Golden]:
    return [golden for golden in goldens if golden.document is not None]


def check_oracle_goldens(plan: Plan) -> Outcome:
    """Every golden is strict JSON in the golden schema, and every declared function has one."""
    outcome = Outcome()
    directory, goldens = load_goldens(plan)
    functions = declared_functions(plan)
    outcome.examined = len(goldens) + len(functions)
    if directory is None or not directory.is_dir():
        outcome.findings.append("oracle.goldens is not a directory")
        outcome.examined = max(outcome.examined, 1)
        return outcome
    covered: set[str] = set()
    for golden in goldens:
        where = plan.display(golden.path)
        if golden.error:
            outcome.findings.append(f"{where} {golden.error}")
            continue
        document = golden.document
        assert document is not None
        if document.get("schema") != GOLDEN_SCHEMA:
            outcome.findings.append(
                f"{where} has schema {document.get('schema')!r}, not {GOLDEN_SCHEMA!r}"
            )
        for key in sorted(set(document) - GOLDEN_KEYS):
            outcome.findings.append(f"{where} has an unknown key {key!r}")
        function = document.get("function")
        if function not in functions:
            outcome.findings.append(
                f"{where} is for {function!r}, which the plan's oracle does not name"
            )
        else:
            covered.add(function)
        cases = document.get("cases")
        if not isinstance(cases, list) or not cases:
            outcome.findings.append(f"{where} carries no cases")
            continue
        for index, case in enumerate(cases):
            if (
                not isinstance(case, dict)
                or "input" not in case
                or "output" not in case
            ):
                outcome.findings.append(
                    f"{where} case {index} is not an object with an input and an output"
                )
                continue
            for key in sorted(set(case) - CASE_KEYS):
                outcome.findings.append(
                    f"{where} case {index} has an unknown key {key!r}"
                )
    for function in functions:
        if function not in covered:
            outcome.findings.append(f"function {function} has no golden")
    if outcome.examined == 0:
        outcome.void = "the oracle names no function and holds no golden"
    return outcome


def check_oracle_provenance(plan: Plan) -> Outcome:
    """Every golden records the pinned source commit and the committed generator's digest."""
    outcome = Outcome()
    oracle = plan.section("oracle")
    _, goldens = load_goldens(plan)
    goldens = readable(goldens)
    outcome.examined = len(goldens)
    if not goldens:
        outcome.void = "the oracle holds no readable golden"
        return outcome
    generator_path = plan.file(oracle.get("generator"))
    digest = None
    if generator_path is None or not generator_path.is_file():
        outcome.findings.append(
            f"the generator {oracle.get('generator')!r} does not exist"
        )
    else:
        digest = hashlib.sha256(generator_path.read_bytes()).hexdigest()
    pinned = oracle.get("source_commit")
    generator = (
        normalised(oracle["generator"])
        if isinstance(oracle.get("generator"), str)
        else None
    )
    for golden in goldens:
        document = golden.document
        assert document is not None
        where = plan.display(golden.path)
        commit = document.get("source_commit")
        if commit != pinned:
            outcome.findings.append(
                f"{where} was generated from {commit!r}; the plan pins {pinned!r}"
            )
        named = document.get("generator")
        if not isinstance(named, str) or normalised(named) != generator:
            outcome.findings.append(
                f"{where} names generator {named!r}, not the plan's {generator!r}"
            )
        recorded = document.get("generator_sha256")
        if not isinstance(recorded, str) or not SHA256.fullmatch(recorded):
            outcome.findings.append(f"{where} records no generator_sha256")
        elif digest is not None and recorded != digest:
            outcome.findings.append(
                f"{where} records generator {recorded[:12]}, the committed generator is "
                f"{digest[:12]}: regenerate the goldens"
            )
    return outcome


def check_oracle_consumed(plan: Plan) -> Outcome:
    """Every golden is read by a test of the port: its file name or the goldens directory is named in
    a consumer's code, not in a comment."""
    outcome = Outcome()
    oracle = plan.section("oracle")
    directory, goldens = load_goldens(plan)
    outcome.examined = len(goldens)
    if not goldens:
        outcome.void = "the oracle holds no golden to be read"
        return outcome
    consumers = (
        oracle.get("consumers") if isinstance(oracle.get("consumers"), list) else []
    )
    texts: list[str] = []
    for relative in consumers:
        path = plan.file(relative)
        if path is None or not path.is_file():
            outcome.findings.append(f"consumer {relative!r} does not exist")
            continue
        text = code_text(path)
        if not re.search(r"#\s*\[\s*(?:[\w:]+::)?test\b|\bdef\s+test_", text):
            outcome.findings.append(f"consumer {relative} holds no test")
        texts.append(text)
    relative_dir = plan.display(directory) if directory is not None else ""
    spellings = {relative_dir, "/".join(relative_dir.split("/")[-2:])} - {""}
    for golden in goldens:
        name = golden.path.name
        read = any(
            name in text or any(spelling in text for spelling in spellings)
            for text in texts
        )
        if not read:
            outcome.findings.append(
                f"{plan.display(golden.path)} is read by no consumer"
            )
    return outcome


def check_oracle_synthetic(plan: Plan) -> Outcome:
    """Every golden's inputs are synthetic and seeded, so the public tree carries no production row."""
    outcome = Outcome()
    _, goldens = load_goldens(plan)
    goldens = readable(goldens)
    outcome.examined = len(goldens)
    if not goldens:
        outcome.void = "the oracle holds no readable golden"
        return outcome
    for golden in goldens:
        document = golden.document
        assert document is not None
        where = plan.display(golden.path)
        if document.get("inputs") != "synthetic":
            outcome.findings.append(
                f"{where} declares inputs {document.get('inputs')!r}, not 'synthetic'"
            )
        seed = document.get("seed")
        if isinstance(seed, bool) or not isinstance(seed, int):
            outcome.findings.append(
                f"{where} records no integer seed its inputs were generated from"
            )
    return outcome


def check_oracle_divergences(plan: Plan) -> Outcome:
    """A case where the port is meant to differ from the source is declared and cites an ADR that
    exists and names the function."""
    outcome = Outcome()
    _, goldens = load_goldens(plan)
    for golden in readable(goldens):
        function = str(golden.document.get("function", "")) if golden.document else ""
        short = function.rsplit(".", 1)[-1]
        for index, case in enumerate(cases_of(golden)):
            outcome.examined += 1
            if "diverges" not in case:
                continue
            where = f"{plan.display(golden.path)} case {index}"
            diverges = case["diverges"]
            if not isinstance(diverges, dict):
                outcome.findings.append(f"{where} diverges with no reason and no ADR")
                continue
            if (
                not isinstance(diverges.get("reason"), str)
                or not diverges["reason"].strip()
            ):
                outcome.findings.append(f"{where} diverges with no reason")
            adr = plan.file(diverges.get("adr"))
            if adr is None or not adr.is_file():
                outcome.findings.append(
                    f"{where} cites ADR {diverges.get('adr')!r}, which does not exist"
                )
                continue
            try:
                text = read_text(adr)
            except (OSError, UnicodeDecodeError) as error:
                outcome.findings.append(f"{where} cites an unreadable ADR: {error}")
                continue
            if function not in text and (not short or short not in text):
                outcome.findings.append(
                    f"{where} cites {plan.display(adr)}, which never names {function}"
                )
    if outcome.examined == 0:
        outcome.void = "the oracle holds no golden case"
    return outcome


def check_oracle_edge_classes(plan: Plan) -> Outcome:
    """Each function's goldens cover the trap classes the plan declares for it (advisory)."""
    outcome = Outcome()
    oracle = plan.section("oracle")
    functions = declared_functions(plan)
    outcome.examined = len(functions)
    if not functions:
        outcome.void = "the oracle names no function"
        return outcome
    declared = oracle.get("edge_classes")
    if not isinstance(declared, dict) or not declared:
        outcome.findings.append(
            "the oracle declares no edge_classes: name the trap classes each function's inputs cover "
            "(ties, negatives, the rollover boundary)"
        )
        return outcome
    _, goldens = load_goldens(plan)
    seen: dict[str, set[str]] = {}
    for golden in readable(goldens):
        function = golden.document.get("function") if golden.document else None
        for case in cases_of(golden):
            if isinstance(case.get("class"), str):
                seen.setdefault(str(function), set()).add(case["class"])
    for function in sorted(set(declared) - set(functions)):
        outcome.findings.append(
            f"edge_classes names {function}, which the oracle does not declare"
        )
    for function in functions:
        wanted = declared.get(function)
        if not isinstance(wanted, list) or not wanted:
            outcome.findings.append(f"function {function} declares no edge class")
            continue
        for name in wanted:
            if name not in seen.get(function, set()):
                outcome.findings.append(
                    f"function {function} has no golden case of class {name!r}"
                )
    return outcome


def _has_float(value: object) -> bool:
    if isinstance(value, float):
        return True
    if isinstance(value, dict):
        return any(_has_float(item) for item in value.values())
    if isinstance(value, list):
        return any(_has_float(item) for item in value)
    return False


def _serde_features(table: object) -> set[str] | None:
    """The features a Cargo dependency table gives serde_json, or None when it names no serde_json."""
    if not isinstance(table, dict) or "serde_json" not in table:
        return None
    spec = table["serde_json"]
    if isinstance(spec, dict):
        features = spec.get("features")
        found = (
            {item for item in features if isinstance(item, str)}
            if isinstance(features, list)
            else set()
        )
        if spec.get("workspace") is True:
            found.add("__workspace__")
        return found
    return set()


def crate_manifest(root: Path, start: Path) -> Path | None:
    directory = start.parent
    while True:
        candidate = directory / "Cargo.toml"
        if candidate.is_file():
            return candidate
        if directory == root or root not in directory.parents:
            return None
        directory = directory.parent


def check_oracle_float_roundtrip(plan: Plan) -> Outcome:
    """A golden holding a float is read through serde_json's `float_roundtrip` (advisory)."""
    outcome = Outcome()
    _, goldens = load_goldens(plan)
    goldens = readable(goldens)
    outcome.examined = len(goldens)
    if not goldens:
        outcome.void = "the oracle holds no readable golden"
        return outcome
    floating = [
        golden
        for golden in goldens
        if _has_float(
            [[case.get("input"), case.get("output")] for case in cases_of(golden)]
        )
    ]
    if not floating:
        return outcome
    root_features: set[str] = set()
    root_manifest = plan.root / "Cargo.toml"
    if root_manifest.is_file():
        try:
            workspace = tomllib.loads(read_text(root_manifest)).get("workspace", {})
            root_features = _serde_features(workspace.get("dependencies")) or set()
        except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError):
            root_features = set()
    consumers = plan.section("oracle").get("consumers")
    for relative in consumers if isinstance(consumers, list) else []:
        path = plan.file(relative)
        if path is None or not path.is_file():
            continue
        manifest = crate_manifest(plan.root, path)
        if manifest is None:
            outcome.findings.append(
                f"consumer {relative} belongs to no crate with a Cargo.toml"
            )
            continue
        try:
            document = tomllib.loads(read_text(manifest))
        except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
            outcome.findings.append(f"{plan.display(manifest)} is not TOML: {error}")
            continue
        features: set[str] = set()
        found = False
        for table in ("dependencies", "dev-dependencies"):
            named = _serde_features(document.get(table))
            if named is not None:
                found = True
                features |= named
        if "__workspace__" in features:
            features |= root_features
        if not found or "float_roundtrip" not in features:
            outcome.findings.append(
                f"{len(floating)} golden(s) hold floats and {plan.display(manifest)} reads them "
                "without serde_json's float_roundtrip, so an exact comparison can differ by one ulp"
            )
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `run`.
# --------------------------------------------------------------------------------------------


def read_runbook(plan: Plan) -> tuple[str | None, str | None]:
    path = plan.file(plan.document.get("runbook"))
    if path is None:
        return None, "runbook names no file"
    if not path.is_file():
        return None, f"runbook {plan.display(path)} does not exist"
    try:
        return read_text(path), None
    except (OSError, UnicodeDecodeError) as error:
        return None, f"cannot read the runbook: {error}"


def tokens(line: str) -> list[str]:
    return re.findall(r"\S+", line)


def is_tool_line(line: str, command: str) -> bool:
    return bool(command) and command in line


def applies(line: str, flag: str) -> bool:
    return any(
        token == flag or token.startswith(flag + "=")
        for token in tokens(line.strip("`"))
    )


def first_apply(lines: list[str], command: str, flag: str) -> int | None:
    for index, line in enumerate(lines):
        if is_tool_line(line, command) and applies(line, flag):
            return index
    return None


def check_proof(plan: Plan, key: str, outcome: Outcome) -> None:
    """The proof the plan names is a test function defined in the named file."""
    proof = plan.section("proofs").get(key)
    outcome.examined += 1
    match = PROOF.fullmatch(proof) if isinstance(proof, str) else None
    if match is None:
        outcome.findings.append(f"proofs.{key} names no path::test_name")
        return
    path = plan.file(match.group("path"))
    if path is None or not path.is_file():
        outcome.findings.append(
            f"proofs.{key} names {match.group('path')}, which does not exist"
        )
        return
    name = re.escape(match.group("name"))
    text = code_text(path)
    if path.suffix == ".py":
        defined = re.search(rf"^\s*(?:async\s+)?def\s+{name}\s*\(", text, re.MULTILINE)
    else:
        defined = re.search(rf"\bfn\s+{name}\s*[<(]", text)
    if not defined:
        outcome.findings.append(
            f"proofs.{key}: {plan.display(path)} defines no test {match.group('name')}"
        )


def check_dry_run_default(plan: Plan) -> Outcome:
    """The runbook runs the tool without its apply flag before it runs it with it, and a test proves a
    run without the flag writes nothing."""
    outcome = Outcome()
    tool = plan.section("tool")
    command = tool.get("command") if isinstance(tool.get("command"), str) else ""
    flag = tool.get("apply_flag") if isinstance(tool.get("apply_flag"), str) else ""
    text, why = read_runbook(plan)
    outcome.examined += 1
    if why:
        outcome.findings.append(why)
    else:
        assert text is not None
        lines = [line for line in fenced_lines(text) if is_tool_line(line, command)]
        applying = [index for index, line in enumerate(lines) if applies(line, flag)]
        if not command or not flag:
            outcome.findings.append(
                "the plan names no tool.command or tool.apply_flag to read the runbook by"
            )
        elif not lines:
            outcome.findings.append(
                f"the runbook never runs {command!r} in a code block"
            )
        elif not applying:
            outcome.findings.append(f"the runbook never runs the tool with {flag}")
        elif applying[0] == 0:
            outcome.findings.append(
                f"the runbook's first run of the tool already passes {flag}: dry-run first"
            )
    check_proof(plan, "dry_run", outcome)
    return outcome


def check_source_read_only(plan: Plan) -> Outcome:
    """The tool opens its source read-only, in code: an aborted port leaves the source untouched."""
    outcome = Outcome()
    sources = tool_sources(plan)
    outcome.examined = len(sources)
    if not sources:
        outcome.void = "the tool has no Rust or Python source"
        return outcome
    if not any(
        marker.search(code_text(path))
        for path in sources
        for marker in READ_ONLY_MARKERS
    ):
        outcome.findings.append(
            "the tool opens its source with no read-only mode (read_only(true), mode=ro, immutable=1, "
            "SQLITE_OPEN_READONLY or PRAGMA query_only)"
        )
    return outcome


def check_backup_before_apply(plan: Plan) -> Outcome:
    """Before the runbook's first apply: a consistent backup, then an integrity check of it."""
    outcome = Outcome()
    tool = plan.section("tool")
    command = tool.get("command") if isinstance(tool.get("command"), str) else ""
    flag = tool.get("apply_flag") if isinstance(tool.get("apply_flag"), str) else ""
    text, why = read_runbook(plan)
    outcome.examined = 1
    if why:
        outcome.findings.append(why)
        return outcome
    assert text is not None
    lines = fenced_lines(text)
    outcome.examined = len(lines) or 1
    applied = first_apply(lines, command, flag)
    if applied is None:
        outcome.findings.append(
            f"the runbook never runs the tool with {flag or 'its apply flag'}"
        )
        return outcome
    backup = next(
        (
            index
            for index, line in enumerate(lines[:applied])
            if any(m.search(line) for m in BACKUP_MARKERS)
        ),
        None,
    )
    if backup is None:
        outcome.findings.append(
            "no backup precedes the first apply: VACUUM INTO, .backup, sqlite3_rsync or "
            "litestream restore -o"
        )
        return outcome
    if not any(INTEGRITY.search(line) for line in lines[backup:applied]):
        outcome.findings.append(
            "the backup is never integrity-checked before the first apply"
        )
    return outcome


def check_integrity_after(plan: Plan) -> Outcome:
    """After the apply, both integrity_check and foreign_key_check run: the first misses FK errors."""
    outcome = Outcome()
    tool = plan.section("tool")
    command = tool.get("command") if isinstance(tool.get("command"), str) else ""
    flag = tool.get("apply_flag") if isinstance(tool.get("apply_flag"), str) else ""
    sources = tool_sources(plan)
    code = "\n".join(code_text(path) for path in sources)
    text, why = read_runbook(plan)
    after: list[str] = []
    if text is not None:
        lines = fenced_lines(text)
        applied = first_apply(lines, command, flag)
        after = lines[applied + 1 :] if applied is not None else []
    outcome.examined = len(sources) + len(after) + (1 if text is not None else 0)
    if outcome.examined == 0:
        outcome.void = why or "neither the tool nor the runbook could be read"
        return outcome
    evidence = code + "\n" + "\n".join(after)
    if not INTEGRITY.search(evidence):
        outcome.findings.append(
            "nothing runs PRAGMA integrity_check on the target after the apply"
        )
    if not FOREIGN_KEYS.search(evidence):
        outcome.findings.append(
            "nothing runs PRAGMA foreign_key_check after the apply, and integrity_check does not find "
            "foreign-key errors"
        )
    return outcome


def check_idempotency_proved(plan: Plan) -> Outcome:
    outcome = Outcome()
    check_proof(plan, "idempotency", outcome)
    return outcome


def check_rollback_proved(plan: Plan) -> Outcome:
    """A test proves the rollback, and the runbook's rollback section says how to run it."""
    outcome = Outcome()
    check_proof(plan, "rollback", outcome)
    text, why = read_runbook(plan)
    outcome.examined += 1
    if why:
        outcome.findings.append(why)
        return outcome
    assert text is not None
    rollback = [
        lines
        for title, lines in sections(text)
        if "rollback" in title.lower() or "roll back" in title.lower()
    ]
    if not rollback:
        outcome.findings.append("the runbook has no rollback section")
    elif not any(rollback):
        outcome.findings.append("the runbook's rollback section runs no command")
    return outcome


def check_counts_proved(plan: Plan) -> Outcome:
    outcome = Outcome()
    check_proof(plan, "counts", outcome)
    return outcome


def copies_a_database(line: str) -> bool:
    words = tokens(line)
    while words and (words[0] in COMMAND_PREFIXES or "=" in words[0]):
        words = words[1:]
    if not words or words[0].rsplit("/", 1)[-1] not in COPY_COMMANDS:
        return False
    return any(
        DB_SUFFIX.search(word.strip("\"'"))
        for word in words[1:]
        if not word.startswith("-")
    )


def check_no_hot_file_copy(plan: Plan) -> Outcome:
    """No runbook command or shell script copies a SQLite file with cp, rsync or scp (advisory)."""
    outcome = Outcome()
    text, _ = read_runbook(plan)
    where_lines: list[tuple[str, str]] = []
    if text is not None:
        where_lines += [("the runbook", line) for line in fenced_lines(text)]
        outcome.examined += 1
    for base in ("deploy", "scripts", "tools", "migration"):
        for path in walk_files(plan.root / base, (".sh",)):
            outcome.examined += 1
            try:
                body = strip_code_comments(read_text(path), ".sh")
            except (OSError, UnicodeDecodeError):
                continue
            where_lines += [
                (plan.display(path), line) for line in body.splitlines() if line.strip()
            ]
    if outcome.examined == 0:
        outcome.void = "no runbook and no shell script to read"
        return outcome
    for where, line in where_lines:
        for part in re.split(r"&&|\|\||;|\|", line):
            if copies_a_database(part.strip()):
                outcome.findings.append(
                    f"{where} copies a database file: {part.strip()} (use VACUUM INTO, .backup or "
                    "sqlite3_rsync, or stop the writer and keep the -wal with it)"
                )
    return outcome


# --------------------------------------------------------------------------------------------
# The command line.
# --------------------------------------------------------------------------------------------

CLASSES: dict[str, tuple[str, Callable[[Plan], Outcome]]] = {
    "manifest": ("plan", check_manifest),
    "source-pinned": ("plan", check_source_pinned),
    "tables-covered": ("plan", check_tables_covered),
    "targets-exist": ("plan", check_targets_exist),
    "count-rules": ("plan", check_count_rules),
    "oracle-goldens": ("oracle", check_oracle_goldens),
    "oracle-provenance": ("oracle", check_oracle_provenance),
    "oracle-consumed": ("oracle", check_oracle_consumed),
    "oracle-synthetic": ("oracle", check_oracle_synthetic),
    "oracle-divergences": ("oracle", check_oracle_divergences),
    "oracle-edge-classes": ("oracle", check_oracle_edge_classes),
    "oracle-float-roundtrip": ("oracle", check_oracle_float_roundtrip),
    "dry-run-default": ("run", check_dry_run_default),
    "source-read-only": ("run", check_source_read_only),
    "backup-before-apply": ("run", check_backup_before_apply),
    "integrity-after": ("run", check_integrity_after),
    "idempotency-proved": ("run", check_idempotency_proved),
    "rollback-proved": ("run", check_rollback_proved),
    "counts-proved": ("run", check_counts_proved),
    "no-hot-file-copy": ("run", check_no_hot_file_copy),
}


def emit(lines: Iterable[str]) -> None:
    for line in lines:
        print(line)


def run_class(name: str, root: Path, manifest: str | None) -> int:
    _, judge = CLASSES[name]
    try:
        plan = load_plan(root, manifest)
    except Unreadable as error:
        emit([f"{name}: VOID: {error}", "examined 0"])
        return EXIT_VOID
    outcome = judge(plan)
    lines = [f"{name}: {finding}" for finding in outcome.findings]
    if outcome.void is not None:
        lines.append(f"{name}: VOID: {outcome.void}")
        outcome.examined = 0
    lines.append(f"examined {outcome.examined}")
    emit(lines)
    return outcome.exit_code()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="data-migration-probe.py",
        description="Judge a declared data port (data-migration.json) against its tree.",
    )
    parser.add_argument("--root", default=".", help="the repository root to judge")
    parser.add_argument(
        "--manifest", help=f"the plan, relative to --root (default {DEFAULT_MANIFEST})"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="print every class and its stage")
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", help="the class to run")
    try:
        args = parser.parse_args(argv)
    except SystemExit as exit_:
        return EXIT_USAGE if exit_.code else EXIT_OK
    if args.command == "list":
        emit(f"{stage} {name}" for name, (stage, _) in CLASSES.items())
        return EXIT_OK
    if args.name not in CLASSES:
        print(
            f"data-migration-probe: no class {args.name!r}; `list` names them",
            file=sys.stderr,
        )
        return EXIT_USAGE
    root = Path(args.root)
    if not root.is_dir():
        print(
            f"data-migration-probe: --root {args.root} is not a directory",
            file=sys.stderr,
        )
        return EXIT_USAGE
    return run_class(args.name, root.resolve(), args.manifest)


if __name__ == "__main__":
    sys.exit(main())
