#!/usr/bin/env python3
"""privacy-gdpr-probe.py -- judges a repository's personal data against the GDPR and Telegram's rules.

SPEC-V2-2222. A repository declares what personal data it keeps in `privacy.json` (schema
`phx.privacy.v1`): each category of data, where it is stored, why, on which Article 6(1) basis,
for how long, and how it is exported and erased. This script proves that declaration against the
tree itself -- the SQL migrations, the export, erase and purge code, the Litestream and journald
configuration, the privacy policy, the default settings and every file the repository publishes:

    inventory-valid         block     the inventory parses and names only files that exist
    schema-covered          block     every column of a table tied to a person, and every Telegram
                                      storage key the Mini App writes, is declared
    retention-bounded       block     every category keeps its data for a bounded time (Art. 5(1)(e))
    purpose-limited         block     consent can be withdrawn, legitimate interests are balanced, and
                                      Telegram data never trains a model (Bot Developer Terms s4.3)
    export-complete         block     the export reads every exportable table, in JSON, CSV or XML
    erase-complete          block     the erase deletes every erasable table and storage key (Art. 17)
    erase-effective         block     deleted rows are not recoverable: secure_delete, VACUUM, and a
                                      personal full-text index's own secure-delete
    policy-published        block     the policy names each category's basis and retention, the
                                      backup and log windows, and a reachable entry point (Art. 13)
    purge-automated         block     a timer purges every table kept for a fixed period
    copies-bounded          block     Litestream snapshots and journald logs age out inside the
                                      declared windows
    telegram-minimised      block     no raw launch data is stored; optional Telegram fields are
                                      justified; a phone number is asked for only if declared
    defaults-private        block     nothing is shared, published or tracked by default (Art. 25(2))
    logs-pseudonymised      advisory  no log call records a name, username, contact or launch data
    public-scrub            block     no published file carries an address, a cloud id, a secret
                                      name, a chat id or personal data (persona-core's deny list plus
                                      this pack's public-repository shapes)
    private-list-committed  block     no private deny list, and nothing under a declared private
                                      path, is published

It is VENDORABLE on purpose: standard library only (Python >= 3.10; TOML settings need 3.11), no
import from phoenix-v2. The vocabulary (GDPR lists, personal column names, exposure keys) lives in
the pack's `vocabulary.json`, and the public-repository shapes live in the pack's `deny-list.json`
in persona-core's deny schema, loaded by persona-core's own `load_deny()`: this script composes with
persona-core's scrubber and never copies it.

`privacy.json` is found anywhere under `--root` (dot-directories, node_modules and target skipped),
or named by `--manifest`. Each inventory is judged against the tree under its own directory. Every
class prints one line per finding, `<class>: <finding>`, and ends with `examined N`. Exit 0 is
green, 1 a finding, 2 a usage error, and 3 VOID: nothing was examined or an input could not be
read, which is never a pass. A finding names a rule and a line, never a denied value.
"""

import argparse
import fnmatch
import importlib.util
import ipaddress
import json
import os
import re
import subprocess
import sys
import unicodedata
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path

try:
    import tomllib
except (
    ModuleNotFoundError
):  # Python 3.10: TOML settings are then unreadable, never skipped.
    tomllib = None

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

MANIFEST_NAME = "privacy.json"
SCHEMA = "phx.privacy.v1"
VOCABULARY_SCHEMA = "phx.privacy.vocabulary.v1"
DENY_SCHEMA = "phx.persona.deny.v1"
HERE = Path(__file__).resolve().parent
DEFAULT_PACK_DIR = HERE.parent / "skills" / "packs" / "privacy-gdpr"
DEFAULT_PERSONA_CORE = HERE / "persona-core-probe.py"
DENY_ENV = "PERSONA_CORE_DENY_LIST"
SKIPPED_DIRECTORIES = frozenset({"node_modules", "target"})
MAX_SCAN_BYTES = 5_000_000

CLASSES = (
    ("inventory-valid", "inventory", "block"),
    ("schema-covered", "inventory", "block"),
    ("retention-bounded", "inventory", "block"),
    ("purpose-limited", "inventory", "block"),
    ("export-complete", "rights", "block"),
    ("erase-complete", "rights", "block"),
    ("erase-effective", "rights", "block"),
    ("policy-published", "rights", "block"),
    ("purge-automated", "design", "block"),
    ("copies-bounded", "design", "block"),
    ("telegram-minimised", "design", "block"),
    ("defaults-private", "design", "block"),
    ("logs-pseudonymised", "design", "advisory"),
    ("public-scrub", "public", "block"),
    ("private-list-committed", "public", "block"),
)
CLASS_NAMES = tuple(name for name, _stage, _severity in CLASSES)

TOP_KEYS = frozenset(
    {
        "schema",
        "categories",
        "sql",
        "code",
        "export",
        "erase",
        "purge",
        "backups",
        "logs",
        "policy",
        "config",
        "public_by_design",
        "public",
        "private",
    }
)
TOP_REQUIRED = (
    "schema",
    "categories",
    "sql",
    "export",
    "erase",
    "backups",
    "logs",
    "policy",
)
CATEGORY_KEYS = frozenset(
    {
        "id",
        "data",
        "source",
        "stores",
        "purpose",
        "lawful_basis",
        "retention",
        "export",
        "erase",
        "withdraw",
        "balancing",
        "justify",
    }
)
CATEGORY_REQUIRED = (
    "id",
    "data",
    "source",
    "stores",
    "purpose",
    "lawful_basis",
    "retention",
    "export",
    "erase",
)
BLOCK_KEYS = {
    "export": (frozenset({"code", "format"}), ("code", "format")),
    "erase": (frozenset({"code"}), ("code",)),
    "purge": (frozenset({"code", "timer"}), ("code", "timer")),
    "backups": (frozenset({"config", "retention"}), ("config", "retention")),
    "logs": (frozenset({"config", "retention"}), ("config", "retention")),
    "policy": (frozenset({"file", "entry"}), ("file", "entry")),
    "public": (frozenset({"allow"}), ()),
}
RETENTION_KEYS = frozenset({"period", "after", "until"})

SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
TABLE_STORE = re.compile(
    r"^(?P<table>[A-Za-z_][A-Za-z0-9_]*)\.(?P<column>\*|[A-Za-z_][A-Za-z0-9_]*)$"
)
CLIENT_STORE = re.compile(r"^(?P<kind>[a-z]+):(?P<key>\*|[A-Za-z0-9_-]{1,128})$")
DURATION = re.compile(
    r"^P(?!$)(?:(?P<Y>\d+)Y)?(?:(?P<M>\d+)M)?(?:(?P<W>\d+)W)?(?:(?P<D>\d+)D)?"
    r"(?:T(?=\d)(?:(?P<h>\d+)H)?(?:(?P<m>\d+)M)?(?:(?P<s>\d+)S)?)?$"
)
DAY = 86_400
ISO_SECONDS = {
    "Y": 365.25 * DAY,
    "M": 30.44 * DAY,
    "W": 7 * DAY,
    "D": DAY,
    "h": 3_600,
    "m": 60,
    "s": 1,
}
ISO_WORDS = {
    "Y": "year",
    "M": "month",
    "W": "week",
    "D": "day",
    "h": "hour",
    "m": "minute",
    "s": "second",
}
GO_DURATION = re.compile(r"(\d+(?:\.\d+)?)(ns|us|µs|ms|s|m|h|d)")
GO_SECONDS = {
    "ns": 1e-9,
    "us": 1e-6,
    "µs": 1e-6,
    "ms": 1e-3,
    "s": 1,
    "m": 60,
    "h": 3_600,
    "d": DAY,
}
SYSTEMD_SPAN = re.compile(r"(\d+(?:\.\d+)?)\s*([A-Za-zµ]*)")
SYSTEMD_SECONDS = {
    "": 1,
    "usec": 1e-6,
    "us": 1e-6,
    "µs": 1e-6,
    "msec": 1e-3,
    "ms": 1e-3,
    "seconds": 1,
    "second": 1,
    "sec": 1,
    "s": 1,
    "minutes": 60,
    "minute": 60,
    "min": 60,
    "m": 60,
    "hours": 3_600,
    "hour": 3_600,
    "hr": 3_600,
    "h": 3_600,
    "days": DAY,
    "day": DAY,
    "d": DAY,
    "weeks": 7 * DAY,
    "week": 7 * DAY,
    "w": 7 * DAY,
    "months": 30.44 * DAY,
    "month": 30.44 * DAY,
    "M": 30.44 * DAY,
    "years": 365.25 * DAY,
    "year": 365.25 * DAY,
    "y": 365.25 * DAY,
}
LITESTREAM_DEFAULT_SECONDS = 24 * 3_600
TIMER_SCHEDULES = (
    "OnCalendar",
    "OnUnitActiveSec",
    "OnActiveSec",
    "OnBootSec",
    "OnStartupSec",
)

IDENT = r'(?:"[^"]+"|`[^`]+`|\[[^\]]+\]|[A-Za-z_][A-Za-z0-9_$]*)'
QUALIFIED = rf"(?:{IDENT}\.)?{IDENT}"
CREATE_TABLE = re.compile(
    rf"^CREATE\s+(?:TEMP(?:ORARY)?\s+)?TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(?P<name>{QUALIFIED})\s*"
    r"(?:\((?P<body>.*)\)[^()]*|AS\s+.*)$",
    re.IGNORECASE | re.DOTALL,
)
CREATE_VIRTUAL = re.compile(
    rf"^CREATE\s+VIRTUAL\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(?P<name>{QUALIFIED})\s+USING\s+"
    r"(?P<module>\w+)\s*(?:\((?P<args>.*)\))?\s*$",
    re.IGNORECASE | re.DOTALL,
)
ALTER_ADD = re.compile(
    rf"^ALTER\s+TABLE\s+(?P<name>{QUALIFIED})\s+ADD\s+(?:COLUMN\s+)?(?P<column>{IDENT})",
    re.IGNORECASE,
)
ALTER_RENAME_TABLE = re.compile(
    rf"^ALTER\s+TABLE\s+(?P<name>{QUALIFIED})\s+RENAME\s+TO\s+(?P<new>{IDENT})",
    re.IGNORECASE,
)
ALTER_RENAME_COLUMN = re.compile(
    rf"^ALTER\s+TABLE\s+(?P<name>{QUALIFIED})\s+RENAME\s+(?:COLUMN\s+)?(?P<old>{IDENT})\s+TO\s+"
    rf"(?P<new>{IDENT})",
    re.IGNORECASE,
)
ALTER_DROP_COLUMN = re.compile(
    rf"^ALTER\s+TABLE\s+(?P<name>{QUALIFIED})\s+DROP\s+(?:COLUMN\s+)?(?P<column>{IDENT})",
    re.IGNORECASE,
)
DROP_TABLE = re.compile(
    rf"^DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?P<name>{QUALIFIED})", re.IGNORECASE
)
CONSTRAINT_WORDS = frozenset({"CONSTRAINT", "PRIMARY", "FOREIGN", "UNIQUE", "CHECK"})
FTS_MODULES = frozenset({"fts3", "fts4", "fts5"})

STORAGE_WRITE = re.compile(
    r"\b(?P<kind>CloudStorage|cloudStorage|DeviceStorage|deviceStorage|SecureStorage|secureStorage)"
    r"\s*\.\s*setItem\s*\(\s*(?P<arg>[^,)]*)"
)
STORAGE_REMOVE = re.compile(r"\bremoveItems?\s*\(")
REQUEST_CONTACT = re.compile(r"\brequestContact\s*\(")
SECURE_DELETE_ON = re.compile(
    r"secure_delete[\"'`]?\s*[=,(:]\s*[\"'`]?\s*(?:on|1|true|yes)\b", re.IGNORECASE
)
VACUUM = re.compile(r"\bVACUUM\b", re.IGNORECASE)
FTS_SECURE_DELETE = re.compile(
    r"'secure-delete'\s*,\s*1\b|\"secure-delete\"\s*,\s*1\b", re.IGNORECASE
)
FTS_MERGE = re.compile(
    r"'(?:optimize|rebuild)'|\"(?:optimize|rebuild)\"", re.IGNORECASE
)
LOG_CALL = re.compile(
    r"\b(?:tracing::|log::)?(?:trace|debug|info|warn|error|event)!\s*\("
    r"|\bconsole\s*\.\s*(?:log|info|warn|error|debug|trace)\s*\("
    r"|\b(?:logging|logger|log)\s*\.\s*(?:debug|info|warning|warn|error|exception|critical)\s*\("
)
IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
PLACEHOLDER = re.compile(r"\$\{([^}]*)\}|\{([A-Za-z_][A-Za-z0-9_.]*)(?::[^}]*)?\}")

C_LIKE = frozenset(
    {
        ".rs",
        ".ts",
        ".tsx",
        ".js",
        ".jsx",
        ".mjs",
        ".cjs",
        ".mts",
        ".cts",
        ".go",
        ".java",
        ".kt",
        ".swift",
        ".c",
        ".h",
        ".cc",
        ".cpp",
        ".cs",
        ".svelte",
        ".vue",
        ".astro",
    }
)
HASH_LIKE = frozenset(
    {".py", ".sh", ".bash", ".yml", ".yaml", ".toml", ".conf", ".ini", ".env"}
)
MARKUP_LIKE = frozenset({".svelte", ".vue", ".astro", ".html", ".md"})


class Void(Exception):
    """An input the class needs could not be read: the class is VOID, never green."""


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


@dataclass
class Inventory:
    """One privacy.json: its path, the tree it judges, and its data, or why it did not load."""

    path: Path
    base: Path
    data: "dict | None" = None
    error: "str | None" = None

    def usable(self) -> bool:
        return isinstance(self.data, dict) and self.data.get("schema") == SCHEMA

    def categories(self) -> list:
        value = self.data.get("categories") if isinstance(self.data, dict) else None
        return (
            [c for c in value if isinstance(c, dict)] if isinstance(value, list) else []
        )

    def block(self, key: str) -> dict:
        value = self.data.get(key) if isinstance(self.data, dict) else None
        return value if isinstance(value, dict) else {}

    def strings(self, value: object) -> list:
        return (
            [item for item in value if isinstance(item, str)]
            if isinstance(value, list)
            else []
        )


@dataclass(frozen=True)
class Table:
    name: str
    columns: tuple
    module: "str | None"
    options: tuple
    where: str
    line: int


@dataclass
class Context:
    root: Path
    inventories: list
    vocabulary: dict
    pack_dir: Path
    persona_core: Path
    deny_list: "Path | None"
    _schemas: dict = field(default_factory=dict)
    _texts: dict = field(default_factory=dict)

    def usable(self) -> list:
        return [inv for inv in self.inventories if inv.usable()]

    def show(self, path: Path) -> str:
        try:
            return path.resolve().relative_to(self.root.resolve()).as_posix()
        except ValueError:
            return str(path)

    def text(self, path: Path) -> str:
        key = path.resolve()
        if key not in self._texts:
            try:
                self._texts[key] = path.read_text(encoding="utf-8", errors="replace")
            except OSError as error:
                raise Void(
                    f"{self.show(path)}: unreadable ({error.strerror or error})"
                ) from error
        return self._texts[key]

    def code(self, path: Path) -> str:
        return strip_comments(self.text(path), path.suffix.lower())

    def schema(self, inv: Inventory) -> dict:
        key = inv.path.resolve()
        if key not in self._schemas:
            self._schemas[key] = parse_schema(
                self, globbed(inv, inv.strings(inv.data.get("sql")))
            )
        return self._schemas[key]


# --- reading: the vocabulary, the inventories and the trees they name ---------------------------


def load_vocabulary(pack_dir: Path) -> dict:
    path = pack_dir / "vocabulary.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Void(f"{path}: the vocabulary cannot be read ({error})") from error
    if not isinstance(data, dict) or data.get("schema") != VOCABULARY_SCHEMA:
        raise Void(f"{path}: schema is not {VOCABULARY_SCHEMA}")
    return data


def discover(root: Path) -> list:
    found = []
    for directory, subdirectories, files in os.walk(root):
        subdirectories[:] = sorted(
            name
            for name in subdirectories
            if not name.startswith(".") and name not in SKIPPED_DIRECTORIES
        )
        if MANIFEST_NAME in files:
            found.append(Path(directory) / MANIFEST_NAME)
    return sorted(found)


def load_inventory(path: Path) -> Inventory:
    inventory = Inventory(path=path, base=path.parent)
    try:
        inventory.data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        inventory.error = f"unreadable ({error.strerror or error})"
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        inventory.error = f"not JSON ({error})"
    return inventory


def globbed(inv: Inventory, patterns: Iterable) -> list:
    found: dict = {}
    for pattern in patterns:
        if not isinstance(pattern, str) or not pattern or Path(pattern).is_absolute():
            continue
        if ".." in Path(pattern).parts:
            continue
        for path in inv.base.glob(pattern):
            if path.is_file():
                found[path.resolve()] = path
    return [found[key] for key in sorted(found)]


def named_file(inv: Inventory, relative: object) -> "Path | None":
    if not isinstance(relative, str) or not relative or Path(relative).is_absolute():
        return None
    if ".." in Path(relative).parts:
        return None
    return inv.base / relative


def published_files(base: Path) -> list:
    """What a repository publishes: git's tracked and unignored files, else every file walked."""
    try:
        inside = subprocess.run(
            ["git", "-C", str(base), "rev-parse", "--is-inside-work-tree"],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        if inside.returncode == 0 and inside.stdout.strip() == "true":
            listed = subprocess.run(
                [
                    "git",
                    "-C",
                    str(base),
                    "ls-files",
                    "-z",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                    "--",
                    ".",
                ],
                capture_output=True,
                timeout=120,
                check=False,
            )
            if listed.returncode == 0:
                names = listed.stdout.decode("utf-8", errors="replace").split("\0")
                files = {(base / name) for name in names if name}
                return sorted(path for path in files if path.is_file())
    except (OSError, subprocess.TimeoutExpired):
        pass
    found = []
    for directory, subdirectories, files in os.walk(base):
        subdirectories[:] = sorted(
            name
            for name in subdirectories
            if name != ".git" and name not in SKIPPED_DIRECTORIES
        )
        found += [Path(directory) / name for name in files]
    return sorted(found)


def strip_comments(text: str, suffix: str) -> str:
    """The text with every comment blanked to spaces, strings kept, and line numbers unchanged."""
    if suffix == ".sql":
        line_marks, block, quotes = ("--",), ("/*", "*/"), ("'", '"', "`")
    elif suffix in C_LIKE:
        line_marks, block, quotes = ("//",), ("/*", "*/"), ('"', "'", "`")
    elif suffix in HASH_LIKE:
        line_marks, block, quotes = ("#",), None, ('"', "'")
    else:
        line_marks, block, quotes = (), None, ()
    markup = suffix in MARKUP_LIKE
    rust = suffix == ".rs"
    out = []
    i = 0
    length = len(text)
    while i < length:
        if markup and text.startswith("<!--", i):
            end = text.find("-->", i + 4)
            end = length if end < 0 else end + 3
            out.append(_blank(text[i:end]))
            i = end
            continue
        if block and text.startswith(block[0], i):
            end = text.find(block[1], i + len(block[0]))
            end = length if end < 0 else end + len(block[1])
            out.append(_blank(text[i:end]))
            i = end
            continue
        mark = next((m for m in line_marks if text.startswith(m, i)), None)
        if mark is not None and not (
            mark == "#" and suffix == ".conf" and i > 0 and text[i - 1] not in "\n \t"
        ):
            end = text.find("\n", i)
            end = length if end < 0 else end
            out.append(" " * (end - i))
            i = end
            continue
        char = text[i]
        if char in quotes:
            if rust and char == "'":
                literal = re.match(r"'(?:\\.|[^\\'\n])'", text[i:])
                if literal is None:
                    out.append(char)
                    i += 1
                    continue
            end = i + 1
            while end < length and text[end] != char:
                if text[end] == "\\" and suffix != ".sql":
                    end += 1
                elif text[end] == "\n" and char != "`" and suffix != ".sql":
                    break
                end += 1
            end = min(end + 1, length)
            out.append(text[i:end])
            i = end
            continue
        out.append(char)
        i += 1
    return "".join(out)


def _blank(span: str) -> str:
    return "".join("\n" if ch == "\n" else " " for ch in span)


def unquote(name: str) -> str:
    name = name.strip()
    parts = re.findall(r'"[^"]+"|`[^`]+`|\[[^\]]+\]|[^.]+', name)
    last = parts[-1] if parts else name
    if last[:1] in {'"', "`", "["}:
        last = last[1:-1]
    return last.strip().lower()


def split_top(text: str, separator: str) -> list:
    """Split on `separator` outside quotes and parentheses; offsets kept for line numbers."""
    parts = []
    depth = 0
    quote = None
    start = 0
    for index, char in enumerate(text):
        if quote:
            if char == quote:
                quote = None
            continue
        if char in "'\"`":
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth = max(0, depth - 1)
        elif char == separator and depth == 0:
            parts.append((start, text[start:index]))
            start = index + 1
    parts.append((start, text[start:]))
    return parts


def parse_schema(ctx: Context, files: list) -> dict:
    """{table: Table} after running every migration in path order."""
    tables: dict = {}
    for path in files:
        text = ctx.code(path)
        where = ctx.show(path)
        for offset, statement in split_top(text, ";"):
            body = statement.strip()
            if not body:
                continue
            line = (
                text.count("\n", 0, offset + len(statement) - len(statement.lstrip()))
                + 1
            )
            flat = " ".join(body.split())
            match = CREATE_VIRTUAL.match(flat)
            if match:
                columns, options = [], []
                for _start, argument in split_top(match["args"] or "", ","):
                    argument = argument.strip()
                    if not argument:
                        continue
                    if "=" in argument:
                        key, _, value = argument.partition("=")
                        options.append((key.strip().lower(), unquote(value)))
                    else:
                        columns.append(unquote(argument.split()[0]))
                name = unquote(match["name"])
                tables[name] = Table(
                    name,
                    tuple(columns),
                    match["module"].lower(),
                    tuple(options),
                    where,
                    line,
                )
                continue
            match = CREATE_TABLE.match(flat)
            if match:
                columns = []
                for _start, definition in split_top(match["body"] or "", ","):
                    words = definition.split()
                    if not words or words[0].upper() in CONSTRAINT_WORDS:
                        continue
                    columns.append(unquote(words[0]))
                name = unquote(match["name"])
                tables[name] = Table(name, tuple(columns), None, (), where, line)
                continue
            match = ALTER_RENAME_COLUMN.match(flat)
            if (
                match
                and unquote(match["name"]) in tables
                and match["old"].upper() != "TO"
            ):
                table = tables[unquote(match["name"])]
                old, new = unquote(match["old"]), unquote(match["new"])
                columns = tuple(new if c == old else c for c in table.columns)
                tables[table.name] = Table(
                    table.name,
                    columns,
                    table.module,
                    table.options,
                    table.where,
                    table.line,
                )
                continue
            match = ALTER_RENAME_TABLE.match(flat)
            if match and unquote(match["name"]) in tables:
                table = tables.pop(unquote(match["name"]))
                new = unquote(match["new"])
                tables[new] = Table(
                    new, table.columns, table.module, table.options, where, line
                )
                continue
            match = ALTER_ADD.match(flat)
            if match and unquote(match["name"]) in tables:
                table = tables[unquote(match["name"])]
                column = unquote(match["column"])
                tables[table.name] = Table(
                    table.name,
                    (*table.columns, column),
                    table.module,
                    table.options,
                    where,
                    line,
                )
                continue
            match = ALTER_DROP_COLUMN.match(flat)
            if match and unquote(match["name"]) in tables:
                table = tables[unquote(match["name"])]
                column = unquote(match["column"])
                tables[table.name] = Table(
                    table.name,
                    tuple(c for c in table.columns if c != column),
                    table.module,
                    table.options,
                    table.where,
                    table.line,
                )
                continue
            match = DROP_TABLE.match(flat)
            if match:
                tables.pop(unquote(match["name"]), None)
    return tables


# --- shared judgements ------------------------------------------------------------------------------


def stores(category: dict) -> list:
    value = category.get("stores")
    return [s for s in value if isinstance(s, str)] if isinstance(value, list) else []


def table_stores(category: dict) -> list:
    """[(table, column)] for every `table.column` or `table.*` store, lowercased."""
    found = []
    for store in stores(category):
        match = TABLE_STORE.match(store)
        if match:
            found.append((match["table"].lower(), match["column"].lower()))
    return found


def client_stores(category: dict, vocabulary: dict) -> list:
    kinds = set(vocabulary.get("client_stores", []))
    found = []
    for store in stores(category):
        match = CLIENT_STORE.match(store)
        if match and match["kind"] in kinds:
            found.append((match["kind"], match["key"]))
    return found


def category_tables(category: dict) -> list:
    return sorted({table for table, _column in table_stores(category)})


def is_personal(
    table: Table,
    declared: set,
    vocabulary: dict,
    tables: dict,
    seen: "set | None" = None,
) -> bool:
    if table.name in declared or table.name in vocabulary.get("personal_tables", []):
        return True
    marks = set(vocabulary.get("user_link_columns", [])) | set(
        vocabulary.get("identifier_columns", [])
    )
    if marks & set(table.columns):
        return True
    seen = (seen or set()) | {table.name}
    for key, value in table.options:
        if key == "content" and value in tables and value not in seen:
            if is_personal(tables[value], declared, vocabulary, tables, seen):
                return True
    return False


def duration_seconds(text: object) -> "float | None":
    if not isinstance(text, str):
        return None
    match = DURATION.match(text)
    if not match:
        return None
    return sum(int(v) * ISO_SECONDS[k] for k, v in match.groupdict().items() if v)


def duration_words(text: str) -> list:
    """The ways a policy may state an ISO duration: the ISO text, and "N unit(s)" for one unit."""
    match = DURATION.match(text)
    if not match:
        return []
    phrases = [text]
    parts = [(k, int(v)) for k, v in match.groupdict().items() if v]
    if len(parts) == 1:
        unit, count = parts[0]
        word = ISO_WORDS[unit]
        phrases.append(f"{count} {word}" + ("" if count == 1 else "s"))
    return phrases


def references(texts: Iterable, table: str, verbs: str) -> bool:
    pattern = re.compile(
        rf"\b(?:{verbs})\s+[\"`\[]?(?:main\.)?{re.escape(table)}\b[\"`\]]?",
        re.IGNORECASE,
    )
    return any(pattern.search(text) for text in texts)


def code_texts(ctx: Context, inv: Inventory, key: str) -> list:
    paths = globbed(inv, inv.strings(inv.block(key).get("code")))
    return [ctx.code(path) for path in paths]


def line_of(text: str, index: int) -> int:
    return text.count("\n", 0, index) + 1


# --- the classes ------------------------------------------------------------------------------------


def check_inventory_valid(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    findings = []
    for inv in ctx.inventories:
        where = ctx.show(inv.path)
        if inv.error:
            findings.append(f"{where}: {inv.error}")
            continue
        data = inv.data
        if not isinstance(data, dict):
            findings.append(f"{where}: the inventory is not a JSON object")
            continue
        if data.get("schema") != SCHEMA:
            findings.append(f"{where}: schema is {data.get('schema')!r}, not {SCHEMA}")
            continue
        for key in sorted(set(data) - TOP_KEYS):
            findings.append(f"{where}: unknown key {key}")
        for key in TOP_REQUIRED:
            if key not in data:
                findings.append(f"{where}: {key} is missing")
        for key, (allowed, required) in BLOCK_KEYS.items():
            if key not in data:
                continue
            block = data[key]
            if not isinstance(block, dict):
                findings.append(f"{where}: {key} is not an object")
                continue
            for extra in sorted(set(block) - allowed):
                findings.append(f"{where}: {key} has an unknown key {extra}")
            for need in required:
                if need not in block:
                    findings.append(f"{where}: {key}.{need} is missing")
        for key in ("sql", "code", "config", "private"):
            if key in data and not (
                isinstance(data[key], list)
                and data[key]
                and all(isinstance(item, str) and item for item in data[key])
            ):
                findings.append(f"{where}: {key} is not a non-empty list of globs")
        for key in ("sql", "config"):
            for pattern in inv.strings(data.get(key)):
                if not globbed(inv, [pattern]):
                    findings.append(f"{where}: {key} glob {pattern} matches no file")
        for block_key, file_key in (
            ("export", "code"),
            ("erase", "code"),
            ("purge", "code"),
        ):
            for pattern in inv.strings(inv.block(block_key).get(file_key)):
                if not globbed(inv, [pattern]):
                    findings.append(f"{where}: {block_key} {pattern} does not exist")
        for block_key, file_key in (
            ("purge", "timer"),
            ("backups", "config"),
            ("logs", "config"),
            ("policy", "file"),
        ):
            value = inv.block(block_key).get(file_key)
            if value is None:
                continue
            path = named_file(inv, value)
            if path is None or not path.is_file():
                findings.append(f"{where}: {block_key} {value} does not exist")
        for block_key in ("backups", "logs"):
            value = inv.block(block_key).get("retention")
            if value is not None and duration_seconds(value) is None:
                findings.append(
                    f"{where}: {block_key}.retention {value!r} is not an ISO 8601 duration"
                )
        entries = inv.block("policy").get("entry")
        if "entry" in inv.block("policy") and not (
            isinstance(entries, list) and entries
        ):
            findings.append(f"{where}: policy.entry is not a non-empty list")
        for entry in entries if isinstance(entries, list) else []:
            if not isinstance(entry, dict) or set(entry) != {"file", "text"}:
                findings.append(f"{where}: a policy entry is not {{file, text}}")
                continue
            path = named_file(inv, entry.get("file"))
            if path is None or not path.is_file():
                findings.append(
                    f"{where}: policy entry {entry.get('file')} does not exist"
                )
        allow = inv.block("public").get("allow")
        if "allow" in inv.block("public") and not (
            isinstance(allow, list) and all(isinstance(a, str) and a for a in allow)
        ):
            findings.append(f"{where}: public.allow is not a list of strings")
        by_design = data.get("public_by_design")
        if by_design is not None and not (
            isinstance(by_design, dict)
            and all(isinstance(v, str) and v.strip() for v in by_design.values())
        ):
            findings.append(
                f"{where}: public_by_design is not a map of setting to reason"
            )
        categories = data.get("categories")
        if not isinstance(categories, list) or not categories:
            findings.append(f"{where}: categories is not a non-empty list")
            continue
        seen: set = set()
        for index, category in enumerate(categories):
            if not isinstance(category, dict):
                findings.append(f"{where}: category {index} is not an object")
                continue
            label = category.get("id", index)
            for key in sorted(set(category) - CATEGORY_KEYS):
                findings.append(f"{where}: category {label}: unknown key {key}")
            for key in CATEGORY_REQUIRED:
                if key not in category:
                    findings.append(f"{where}: category {label}: {key} is missing")
            category_id = category.get("id")
            if not isinstance(category_id, str) or not SLUG.match(category_id):
                findings.append(f"{where}: category {label}: id is not a slug")
            elif category_id in seen:
                findings.append(f"{where}: duplicate category id {category_id}")
            else:
                seen.add(category_id)
            for key in ("data", "purpose"):
                if key in category and not (
                    isinstance(category[key], str) and category[key].strip()
                ):
                    findings.append(f"{where}: category {label}: {key} is empty")
            source = category.get("source")
            if "source" in category and source not in vocabulary["sources"]:
                findings.append(
                    f"{where}: category {label}: source {source} is not a known source"
                )
            basis = category.get("lawful_basis")
            if "lawful_basis" in category and basis not in vocabulary["lawful_bases"]:
                findings.append(
                    f"{where}: category {label}: lawful_basis {basis} is not an Article 6(1) basis"
                )
            store_list = category.get("stores")
            if "stores" in category and not (
                isinstance(store_list, list) and store_list
            ):
                findings.append(
                    f"{where}: category {label}: stores is not a non-empty list"
                )
            for store in store_list if isinstance(store_list, list) else []:
                client = CLIENT_STORE.match(store) if isinstance(store, str) else None
                if isinstance(store, str) and (
                    TABLE_STORE.match(store)
                    or (client and client["kind"] in vocabulary["client_stores"])
                ):
                    continue
                findings.append(
                    f"{where}: category {label}: store {store} is not table.column, table.* or "
                    "cloudstorage:key"
                )
            retention = category.get("retention")
            if "retention" in category and not (
                isinstance(retention, dict) and set(retention) <= RETENTION_KEYS
            ):
                findings.append(
                    f"{where}: category {label}: retention is not {{period, after}} or {{until}}"
                )
            export = category.get("export")
            if (
                "export" in category
                and export is not True
                and not (
                    isinstance(export, dict)
                    and set(export) == {"exempt"}
                    and isinstance(export["exempt"], str)
                    and export["exempt"].strip()
                )
            ):
                findings.append(
                    f"{where}: category {label}: export is neither true nor {{exempt: why}}"
                )
            erase = category.get("erase")
            if (
                "erase" in category
                and erase not in vocabulary["erase_modes"]
                and not (isinstance(erase, dict) and set(erase) == {"retain", "why"})
            ):
                findings.append(
                    f"{where}: category {label}: erase is not delete, anonymise or {{retain, why}}"
                )
            justify = category.get("justify")
            if justify is not None and not (
                isinstance(justify, dict)
                and all(isinstance(v, str) and v.strip() for v in justify.values())
            ):
                findings.append(
                    f"{where}: category {label}: justify is not a map of store to reason"
                )
    return Outcome(len(ctx.inventories), tuple(findings))


def check_schema_covered(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    findings = []
    examined = 0
    for inv in ctx.usable():
        where = ctx.show(inv.path)
        tables = ctx.schema(inv)
        wildcard: set = set()
        columns: set = set()
        clients: set = set()
        for category in inv.categories():
            for table, column in table_stores(category):
                (wildcard.add(table) if column == "*" else columns.add((table, column)))
            clients |= set(client_stores(category, vocabulary))
        declared = wildcard | {table for table, _column in columns}
        for table in tables.values():
            examined += 1
            if not is_personal(table, declared, vocabulary, tables):
                continue
            if table.name in wildcard:
                continue
            if not table.columns:
                findings.append(
                    f"{table.where}:{table.line}: {table.name} is personal data and its columns are "
                    f"unknown; declare {table.name}.*"
                )
            for column in table.columns:
                if (table.name, column) not in columns:
                    findings.append(
                        f"{table.where}:{table.line}: {table.name}.{column} is personal data that no "
                        "category declares"
                    )
        for table in sorted(declared - set(tables)):
            findings.append(
                f"{where}: store {table}.* names {table}, which no migration creates"
            )
        for table, column in sorted(columns):
            if table in tables and column not in tables[table].columns:
                findings.append(
                    f"{where}: store {table}.{column} names a column no migration creates"
                )
        for path in globbed(inv, inv.strings(inv.data.get("code"))):
            text = ctx.code(path)
            examined += 1
            for match in STORAGE_WRITE.finditer(text):
                kind = match["kind"].lower()
                argument = match["arg"].strip()
                literal = re.match(r"""^(['"`])([^'"`]+)\1$""", argument)
                at = f"{ctx.show(path)}:{line_of(text, match.start())}"
                if (kind, "*") in clients:
                    continue
                if literal is None:
                    findings.append(
                        f"{at}: {kind}.setItem writes a dynamic key; declare {kind}:* in a category"
                    )
                elif (kind, literal.group(2)) not in clients:
                    findings.append(
                        f"{at}: {kind}:{literal.group(2)} is written but no category declares it"
                    )
    return Outcome(examined, tuple(findings))


def check_retention_bounded(ctx: Context) -> Outcome:
    events = ctx.vocabulary["retention_events"]
    findings = []
    examined = 0
    for inv in ctx.usable():
        where = ctx.show(inv.path)
        for category in inv.categories():
            examined += 1
            retention = category.get("retention")
            label = category.get("id")
            if not isinstance(retention, dict):
                findings.append(f"{where}: category {label}: retention is missing")
                continue
            period, after, until = (
                retention.get(k) for k in ("period", "after", "until")
            )
            if until is not None and period is None and after is None:
                if until not in events:
                    findings.append(
                        f"{where}: category {label}: retention until {until!r} is no event this "
                        f"pack knows ({', '.join(events)})"
                    )
                continue
            if period is None or until is not None:
                findings.append(
                    f"{where}: category {label}: retention is neither a period nor an event, so it "
                    "is unbounded (Art. 5(1)(e))"
                )
                continue
            if duration_seconds(period) is None:
                findings.append(
                    f"{where}: category {label}: retention period {period!r} is not an ISO 8601 "
                    "duration, so it is unbounded (Art. 5(1)(e))"
                )
            if after is not None and after not in events:
                findings.append(
                    f"{where}: category {label}: retention after {after!r} is no event this pack knows"
                )
    return Outcome(examined, tuple(findings))


def check_purpose_limited(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    training = re.compile(vocabulary["training_words"], re.IGNORECASE)
    findings = []
    examined = 0
    for inv in ctx.usable():
        where = ctx.show(inv.path)
        for category in inv.categories():
            examined += 1
            label = category.get("id")
            basis = category.get("lawful_basis")
            if basis == "consent" and not (
                isinstance(category.get("withdraw"), str)
                and category["withdraw"].strip()
            ):
                findings.append(
                    f"{where}: category {label}: consent needs a withdraw path the learner can use "
                    "(Art. 7(3))"
                )
            if basis == "legitimate-interests" and not (
                isinstance(category.get("balancing"), str)
                and category["balancing"].strip()
            ):
                findings.append(
                    f"{where}: category {label}: legitimate interests need a balancing test "
                    "(Art. 6(1)(f))"
                )
            if category.get("source") in vocabulary["telegram_sources"]:
                text = " ".join(
                    str(category.get(key, "")) for key in ("purpose", "data")
                )
                if training.search(text):
                    findings.append(
                        f"{where}: category {label}: Telegram data may not be used for training "
                        "models or building datasets (Bot Developer Terms s4.3)"
                    )
    return Outcome(examined, tuple(findings))


def check_export_complete(ctx: Context) -> Outcome:
    formats = ctx.vocabulary["export_formats"]
    findings = []
    examined = 0
    for inv in ctx.usable():
        where = ctx.show(inv.path)
        export = inv.block("export")
        form = export.get("format")
        if form not in formats:
            findings.append(
                f"{where}: export format {form} is not structured, commonly used and "
                f"machine-readable ({', '.join(formats)}; Art. 20(1))"
            )
        texts = code_texts(ctx, inv, "export")
        for category in inv.categories():
            examined += 1
            if category.get("export") is not True:
                continue
            for table in category_tables(category):
                if not references(texts, table, r"FROM|JOIN"):
                    findings.append(
                        f"{where}: category {category.get('id')}: the export never reads {table} "
                        "(Art. 15(3))"
                    )
    return Outcome(examined, tuple(findings))


def check_erase_complete(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    findings = []
    examined = 0
    for inv in ctx.usable():
        where = ctx.show(inv.path)
        texts = code_texts(ctx, inv, "erase")
        for category in inv.categories():
            examined += 1
            label = category.get("id")
            erase = category.get("erase")
            if isinstance(erase, dict):
                if erase.get("retain") not in vocabulary["retain_exceptions"] or not (
                    isinstance(erase.get("why"), str) and erase["why"].strip()
                ):
                    findings.append(
                        f"{where}: category {label}: retain {erase.get('retain')!r} is not an "
                        f"Article 17(3) exception ({', '.join(vocabulary['retain_exceptions'])})"
                    )
                continue
            if erase not in vocabulary["erase_modes"]:
                continue
            verbs = (
                r"DELETE\s+FROM"
                if erase == "delete"
                else r"DELETE\s+FROM|UPDATE(?:\s+OR\s+\w+)?"
            )
            for table in category_tables(category):
                if not references(texts, table, verbs):
                    findings.append(
                        f"{where}: category {label}: the erase never "
                        f"{'deletes from' if erase == 'delete' else 'deletes or anonymises'} {table} "
                        "(Art. 17(1))"
                    )
            for kind, key in client_stores(category, vocabulary):
                removes = [t for t in texts if STORAGE_REMOVE.search(t)]
                if key == "*":
                    ok = bool(removes)
                else:
                    quoted = re.compile(rf"""(['"`]){re.escape(key)}\1""")
                    ok = any(quoted.search(t) for t in removes)
                if not ok:
                    findings.append(
                        f"{where}: category {label}: the erase never calls removeItem for "
                        f"{kind}:{key}"
                    )
    return Outcome(examined, tuple(findings))


def check_erase_effective(ctx: Context) -> Outcome:
    findings = []
    examined = 0
    for inv in ctx.usable():
        examined += 1
        where = ctx.show(inv.path)
        erase_texts = code_texts(ctx, inv, "erase")
        paths = globbed(
            inv, inv.strings(inv.data.get("code")) + inv.strings(inv.data.get("sql"))
        )
        texts = erase_texts + [ctx.code(path) for path in paths]
        secure = any(SECURE_DELETE_ON.search(t) for t in texts)
        vacuum = any(VACUUM.search(t) for t in erase_texts)
        if not (secure or vacuum):
            findings.append(
                f"{where}: no code turns SQLite's secure_delete on and the erase runs no VACUUM, so "
                "erased rows stay recoverable from free pages (EDPB 4/2019 para 82)"
            )
        tables = ctx.schema(inv)
        declared = {t for c in inv.categories() for t in category_tables(c)}
        for table in tables.values():
            if table.module not in FTS_MODULES:
                continue
            if not is_personal(table, declared, ctx.vocabulary, tables):
                continue
            examined += 1
            statements = [s for t in erase_texts for _o, s in split_top(t, ";")]
            named = re.compile(rf"\b{re.escape(table.name)}\b", re.IGNORECASE)
            guard = FTS_SECURE_DELETE if table.module == "fts5" else FTS_MERGE
            if not any(
                named.search(s) and (guard.search(s) or FTS_MERGE.search(s))
                for s in statements
            ):
                findings.append(
                    f"{table.where}:{table.line}: {table.name} is a personal full-text index; the "
                    "erase never sets its 'secure-delete' option or runs 'optimize' or 'rebuild', so "
                    "erased rows stay readable in the index"
                )
    return Outcome(examined, tuple(findings))


def _policy_phrases(retention: object, events: dict) -> list:
    if not isinstance(retention, dict):
        return []
    if "period" in retention:
        return duration_words(str(retention["period"]))
    until = retention.get("until")
    if isinstance(until, str):
        return [until, events.get(until, until)]
    return []


def check_policy_published(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    bases = vocabulary["lawful_bases"]
    events = vocabulary["retention_events"]
    findings = []
    examined = 0
    for inv in ctx.usable():
        policy = inv.block("policy")
        path = named_file(inv, policy.get("file"))
        if path is None or not path.is_file():
            raise Void(f"{ctx.show(inv.path)}: policy.file cannot be read")
        where = ctx.show(path)
        lines = ctx.text(path).splitlines()
        lowered = [line.casefold() for line in lines]
        for category in inv.categories():
            examined += 1
            label = category.get("id")
            if not isinstance(label, str):
                continue
            token = re.compile(rf"(?<![\w-]){re.escape(label)}(?![\w-])")
            basis = category.get("lawful_basis")
            basis_words = [
                w.casefold() for w in (basis, bases.get(basis)) if isinstance(w, str)
            ]
            phrases = [
                p.casefold() for p in _policy_phrases(category.get("retention"), events)
            ]
            if not any(
                token.search(line)
                and any(w in lowered[i] for w in basis_words)
                and any(p in lowered[i] for p in phrases)
                for i, line in enumerate(lines)
            ):
                findings.append(
                    f"{where}: no line names category {label} with its lawful basis "
                    f"({basis}) and its retention ({' or '.join(phrases) or 'none declared'}) "
                    "(Art. 13(1)(c), 13(2)(a))"
                )
        for key, word in (("backups", "backup"), ("logs", "log")):
            retention = inv.block(key).get("retention")
            phrases = (
                [p.casefold() for p in duration_words(str(retention))]
                if retention
                else []
            )
            examined += 1
            if not any(
                word in line and any(p in line for p in phrases) for line in lowered
            ):
                findings.append(
                    f"{where}: no line tells the learner how long {key} keep their data "
                    f"({' or '.join(phrases) or 'none declared'}); say what happens to {key}"
                )
        for entry in (
            policy.get("entry") if isinstance(policy.get("entry"), list) else []
        ):
            if not isinstance(entry, dict):
                continue
            examined += 1
            target = named_file(inv, entry.get("file"))
            text = entry.get("text")
            if (
                target is None
                or not target.is_file()
                or not isinstance(text, str)
                or not text
            ):
                findings.append(f"{where}: a privacy entry point cannot be read")
            elif text not in ctx.text(target):
                findings.append(
                    f"{ctx.show(target)}: the privacy entry {text} is not in it, so the policy is "
                    "not reachable there (Telegram Bot Developer Terms s4)"
                )
    return Outcome(examined, tuple(findings))


def ini_values(text: str, section: str) -> dict:
    """{key: [values]} of one [section] of a systemd-style file."""
    found: dict = {}
    current = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line[0] in "#;":
            continue
        if line.startswith("[") and line.endswith("]"):
            current = line[1:-1].strip()
            continue
        if current == section and "=" in line:
            key, _, value = line.partition("=")
            found.setdefault(key.strip(), []).append(value.strip())
    return found


def check_purge_automated(ctx: Context) -> Outcome:
    findings = []
    examined = 0
    for inv in ctx.usable():
        where = ctx.show(inv.path)
        periodic = [
            c
            for c in inv.categories()
            if isinstance(c.get("retention"), dict) and "period" in c["retention"]
        ]
        examined += len(inv.categories())
        if not periodic:
            continue
        purge = inv.block("purge")
        if not purge:
            for category in periodic:
                findings.append(
                    f"{where}: category {category.get('id')} keeps data for "
                    f"{category['retention'].get('period')} but no purge is declared "
                    "(EDPB 4/2019 para 82: automate deletion)"
                )
            continue
        timer = named_file(inv, purge.get("timer"))
        if timer is None or not timer.is_file():
            findings.append(
                f"{where}: the purge timer {purge.get('timer')} cannot be read"
            )
        else:
            values = ini_values(ctx.text(timer), "Timer")
            if not any(v for key in TIMER_SCHEDULES for v in values.get(key, [])):
                findings.append(
                    f"{ctx.show(timer)}: the timer has no OnCalendar= or monotonic schedule, so the "
                    "purge never runs"
                )
        texts = code_texts(ctx, inv, "purge")
        for category in periodic:
            for table in category_tables(category):
                if not references(texts, table, r"DELETE\s+FROM"):
                    findings.append(
                        f"{where}: category {category.get('id')}: the purge never deletes from "
                        f"{table} (DELETE FROM {table})"
                    )
    return Outcome(examined, tuple(findings))


def go_seconds(text: str) -> "float | None":
    text = text.strip().strip("\"'")
    if not text or GO_DURATION.sub("", text):
        return None
    return sum(float(n) * GO_SECONDS[u] for n, u in GO_DURATION.findall(text))


def systemd_seconds(text: str) -> "float | None":
    text = text.strip()
    if not text:
        return None
    total = 0.0
    position = 0
    for match in SYSTEMD_SPAN.finditer(text):
        if text[position : match.start()].strip():
            return None
        unit = match.group(2)
        if unit not in SYSTEMD_SECONDS:
            return None
        total += float(match.group(1)) * SYSTEMD_SECONDS[unit]
        position = match.end()
    return None if text[position:].strip() else total


def litestream(text: str) -> tuple:
    """(interval, retention, [line of each replica-level retention]) of a Litestream config."""
    interval = retention = None
    replica_lines = []
    stack: list = []
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.split("#", 1)[0].rstrip()
        if not line.strip():
            continue
        match = re.match(
            r"^(?P<indent>\s*)(?P<dash>-\s+)?(?P<key>[A-Za-z0-9_-]+)\s*:\s*(?P<value>.*)$",
            line,
        )
        if not match:
            continue
        indent = len(match["indent"]) + (len(match["dash"]) if match["dash"] else 0)
        while stack and stack[-1][0] >= indent:
            stack.pop()
        path = [key for _indent, key in stack] + [match["key"]]
        value = match["value"].strip()
        if path == ["snapshot", "interval"]:
            interval = value
        elif path == ["snapshot", "retention"]:
            retention = value
        elif match["key"] == "retention" and "dbs" in path:
            replica_lines.append(number)
        if not value:
            stack.append((indent, match["key"]))
    return interval, retention, replica_lines


def human(seconds: float) -> str:
    return f"{seconds / 3_600:g}h" if seconds < 3 * DAY else f"{seconds / DAY:g} days"


def check_copies_bounded(ctx: Context) -> Outcome:
    findings = []
    examined = 0
    for inv in ctx.usable():
        backups = inv.block("backups")
        path = named_file(inv, backups.get("config"))
        declared = duration_seconds(backups.get("retention"))
        if path is None or not path.is_file() or declared is None:
            raise Void(
                f"{ctx.show(inv.path)}: backups.config or backups.retention cannot be read"
            )
        examined += 1
        where = ctx.show(path)
        interval_text, retention_text, replica_lines = litestream(ctx.text(path))
        for number in replica_lines:
            findings.append(
                f"{where}:{number}: a replica-level retention is ignored since Litestream 0.5; set "
                "the global snapshot.retention"
            )
        interval = (
            LITESTREAM_DEFAULT_SECONDS
            if interval_text is None
            else go_seconds(interval_text)
        )
        retention = (
            LITESTREAM_DEFAULT_SECONDS
            if retention_text is None
            else go_seconds(retention_text)
        )
        if interval is None or retention is None:
            findings.append(
                f"{where}: snapshot.interval or snapshot.retention is not a duration"
            )
        elif interval + retention > declared:
            findings.append(
                f"{where}: Litestream keeps snapshots up to {human(interval + retention)} (interval "
                f"plus retention), past the declared backups retention {backups['retention']}"
            )
        logs = inv.block("logs")
        path = named_file(inv, logs.get("config"))
        declared = duration_seconds(logs.get("retention"))
        if path is None or not path.is_file() or declared is None:
            raise Void(
                f"{ctx.show(inv.path)}: logs.config or logs.retention cannot be read"
            )
        examined += 1
        where = ctx.show(path)
        values = ini_values(ctx.text(path), "Journal").get("MaxRetentionSec", [])
        limit = systemd_seconds(values[-1]) if values else None
        if not values or not limit:
            findings.append(
                f"{where}: journald sets no MaxRetentionSec, so logs are kept until the disk fills, "
                f"past the declared logs retention {logs['retention']}"
            )
        elif limit > declared:
            findings.append(
                f"{where}: MaxRetentionSec={values[-1]} keeps logs past the declared logs retention "
                f"{logs['retention']}"
            )
    return Outcome(examined, tuple(findings))


def check_telegram_minimised(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    raw = set(vocabulary["raw_launch_columns"])
    optional = set(vocabulary["telegram_optional_columns"])
    phones = set(vocabulary["phone_store_names"])
    findings = []
    examined = 0
    for inv in ctx.usable():
        tables = ctx.schema(inv)
        declared = {t for c in inv.categories() for t in category_tables(c)}
        owners: dict = {}
        for category in inv.categories():
            for table, column in table_stores(category):
                owners.setdefault((table, column), category)
        for table in tables.values():
            examined += 1
            for column in table.columns:
                if column in raw:
                    findings.append(
                        f"{table.where}:{table.line}: {table.name}.{column} stores raw launch data; "
                        "validate initData, keep only what a category needs, and drop the rest "
                        "(Art. 5(1)(c), Bot Developer Terms s4.3)"
                    )
                if column in optional and is_personal(
                    table, declared, vocabulary, tables
                ):
                    owner = owners.get((table.name, column)) or owners.get(
                        (table.name, "*")
                    )
                    justify = owner.get("justify") if owner else None
                    reason = (
                        justify.get(f"{table.name}.{column}")
                        if isinstance(justify, dict)
                        else None
                    )
                    if not (isinstance(reason, str) and reason.strip()):
                        findings.append(
                            f"{table.where}:{table.line}: {table.name}.{column} is an optional "
                            "Telegram field; its category must justify it in justify"
                        )
        names = [c for cat in inv.categories() for _table, c in table_stores(cat)]
        names += [
            k for cat in inv.categories() for _kind, k in client_stores(cat, vocabulary)
        ]
        phone_declared = any(any(p in n.lower() for p in phones) for n in names)
        for path in globbed(inv, inv.strings(inv.data.get("code"))):
            examined += 1
            text = ctx.code(path)
            for match in REQUEST_CONTACT.finditer(text):
                if not phone_declared:
                    findings.append(
                        f"{ctx.show(path)}:{line_of(text, match.start())}: requestContact asks for a "
                        "phone number that no category declares"
                    )
    return Outcome(examined, tuple(findings))


def _leaves(value: object, path: tuple) -> Iterable:
    if isinstance(value, dict):
        for key, inner in value.items():
            yield from _leaves(inner, (*path, str(key)))
    else:
        yield path, value


def check_defaults_private(ctx: Context) -> Outcome:
    vocabulary = ctx.vocabulary
    exposure = set(vocabulary["exposure_keys"])
    exposed_values = set(vocabulary["exposed_values"])
    findings = []
    examined = 0
    for inv in ctx.usable():
        where = ctx.show(inv.path)
        examined += 1
        by_design = inv.data.get("public_by_design")
        by_design = by_design if isinstance(by_design, dict) else {}
        files = globbed(inv, inv.strings(inv.data.get("config")))
        if not files:
            findings.append(
                f"{where}: no default settings file is named in config; Article 25(2) needs the "
                "defaults readable"
            )
            continue
        for path in files:
            examined += 1
            try:
                if path.suffix == ".toml":
                    if tomllib is None:
                        raise Void(f"{ctx.show(path)}: TOML needs Python 3.11 or later")
                    data = tomllib.loads(ctx.text(path))
                elif path.suffix == ".json":
                    data = json.loads(ctx.text(path))
                else:
                    raise Void(
                        f"{ctx.show(path)}: only .toml and .json settings are read"
                    )
            except (ValueError, TypeError) as error:
                findings.append(
                    f"{ctx.show(path)}: the settings do not parse ({error})"
                )
                continue
            for keys, value in _leaves(data, ()):
                segments = {k.lower().replace("-", "_") for k in keys}
                if not segments & exposure:
                    continue
                on = value is True or (
                    isinstance(value, str) and value.strip().lower() in exposed_values
                )
                dotted = ".".join(keys)
                if on and not (
                    isinstance(by_design.get(dotted), str) and by_design[dotted].strip()
                ):
                    findings.append(
                        f"{ctx.show(path)}: {dotted} is on by default; Article 25(2) wants it off "
                        "until the learner turns it on, or a reason in public_by_design"
                    )
    return Outcome(examined, tuple(findings))


def _call_arguments(text: str, open_index: int) -> str:
    """The argument text of a call whose '(' is at open_index, strings blanked but placeholders kept."""
    depth = 0
    out = []
    i = open_index
    while i < len(text) and i < open_index + 4_000:
        char = text[i]
        if char in "\"'`":
            end = text.find(char, i + 1)
            end = len(text) if end < 0 else end
            literal = text[i + 1 : end]
            out.append(
                " ".join(
                    m.group(1) or m.group(2) or ""
                    for m in PLACEHOLDER.finditer(literal)
                )
                + " "
            )
            i = end + 1
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                break
        out.append(char)
        i += 1
    return "".join(out)


def check_logs_pseudonymised(ctx: Context) -> Outcome:
    identifiers = set(ctx.vocabulary["log_identifiers"])
    findings = []
    examined = 0
    for inv in ctx.usable():
        for path in globbed(inv, inv.strings(inv.data.get("code"))):
            examined += 1
            text = ctx.code(path)
            for match in LOG_CALL.finditer(text):
                arguments = _call_arguments(text, match.end() - 1)
                named = sorted(
                    {t.lower() for t in IDENTIFIER.findall(arguments)} & identifiers
                )
                if named:
                    findings.append(
                        f"{ctx.show(path)}:{line_of(text, match.start())}: a log call records "
                        f"{', '.join(named)}; log a pseudonymous id instead"
                    )
    return Outcome(examined, tuple(findings))


def load_core(path: Path) -> object:
    spec = importlib.util.spec_from_file_location("persona_core_probe", path)
    if spec is None or spec.loader is None:
        raise Void(f"persona-core's probe cannot be loaded from {path}")
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except (OSError, SyntaxError, ImportError) as error:
        raise Void(
            f"persona-core's probe cannot be loaded from {path} ({error})"
        ) from error
    if not callable(getattr(module, "load_deny", None)):
        raise Void(f"persona-core's probe at {path} has no load_deny()")
    return module


def deny_rules(ctx: Context) -> dict:
    """persona-core's public and private deny lists merged with this pack's public shapes."""
    core = load_core(ctx.persona_core)
    try:
        theirs = core.load_deny(None, ctx.deny_list)
        mine = core.load_deny(ctx.pack_dir, None)
    except ValueError as error:
        raise Void(f"a deny list cannot be read ({error})") from error
    patterns = []
    for origin, row_id, pattern in list(theirs["patterns"]) + list(mine["patterns"]):
        anchored = (
            re.compile(pattern.pattern, pattern.flags | re.MULTILINE)
            if "^" in pattern.pattern or "$" in pattern.pattern
            else None
        )
        patterns.append((origin, row_id, pattern, anchored))
    return {
        "patterns": patterns,
        "literals": list(theirs["literals"]) + list(mine["literals"]),
        "own_ip_rows": {
            row_id for _o, row_id, _p in mine["patterns"] if row_id == "ipv6"
        },
    }


def _address(text: str) -> "ipaddress.IPv4Address | ipaddress.IPv6Address | None":
    try:
        return ipaddress.ip_address(text.strip("[]"))
    except ValueError:
        return None


def _denied(
    match: "re.Match", line: str, row_id: str, rules: dict, networks: list, allow: list
) -> bool:
    """Whether one match is a finding: an address outside the allowed networks, never a dotted
    number that continues (an OID or a version), never an allowed literal."""
    found = match.group(0)
    address = _address(found)
    if row_id in rules["own_ip_rows"] and address is None:
        return False
    if address is not None and any(
        address in n for n in networks if n.version == address.version
    ):
        return False
    if (
        address is not None
        and address.version == 4
        and (
            re.match(r"\.\d", line[match.end() : match.end() + 2])
            or re.search(r"\d\.$", line[max(0, match.start() - 2) : match.start()])
        )
    ):
        return False
    return not any(a in found or found in a for a in allow)


def scrub_text(text: str, rules: dict, networks: list, allow: list) -> list:
    """[(line, label)] for one text, judged line by line on NFKC text as persona-core's scrubber is.

    A pattern is run over the lines only when a whole-text search can match it: over the lines
    joined by NUL (so a lookaround at a line's edge sees a non-space, non-word character, as at the
    edge of a string) or over the text in MULTILINE mode (so `^` and `$` see every line). One of the
    two finds every match the line scan can, so the pre-search skips work and never a finding."""
    folded = unicodedata.normalize("NFKC", text)
    lines = folded.split("\n")
    joined = "\0".join(lines)
    hits = []
    for origin, row_id, pattern, anchored in rules["patterns"]:
        if not pattern.search(joined) and not (
            anchored is not None and anchored.search(folded)
        ):
            continue
        for number, line in enumerate(lines, 1):
            for match in pattern.finditer(line):
                if _denied(match, line, row_id, rules, networks, allow):
                    hits.append((number, f"{origin} pattern {row_id}"))
                    break
    lowered = folded.casefold()
    for index, (_origin, literal) in enumerate(rules["literals"]):
        if literal not in lowered or any(literal in a.casefold() for a in allow):
            continue
        for number, line in enumerate(lines, 1):
            if literal in line.casefold():
                hits.append((number, f"private literal {index}"))
    return sorted(hits)


def check_public_scrub(ctx: Context) -> Outcome:
    rules = deny_rules(ctx)
    networks = [ipaddress.ip_network(n) for n in ctx.vocabulary["allowed_networks"]]
    findings = []
    examined = 0
    for inv in ctx.usable():
        allow = list(inv.strings(inv.block("public").get("allow")))
        for path in published_files(inv.base):
            try:
                raw = path.read_bytes()
            except OSError as error:
                raise Void(
                    f"{ctx.show(path)}: unreadable ({error.strerror or error})"
                ) from error
            if len(raw) > MAX_SCAN_BYTES or b"\0" in raw[:8192]:
                continue
            examined += 1
            where = ctx.show(path)
            text = raw.decode("utf-8", errors="replace")
            for number, label in scrub_text(text, rules, networks, allow):
                findings.append(f"{where}:{number}: {label}")
    return Outcome(examined, tuple(findings))


def check_private_list_committed(ctx: Context) -> Outcome:
    findings = []
    examined = 0
    for inv in ctx.usable():
        private = inv.strings(inv.data.get("private"))
        for path in published_files(inv.base):
            examined += 1
            relative = path.relative_to(inv.base).as_posix()
            for pattern in private:
                if fnmatch.fnmatchcase(relative, pattern):
                    findings.append(
                        f"{ctx.show(path)}: it is under the private path {pattern}, and it is published"
                    )
                    break
            if path.suffix != ".json":
                continue
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, UnicodeDecodeError, json.JSONDecodeError):
                continue
            literals = data.get("literals") if isinstance(data, dict) else None
            if (
                isinstance(data, dict)
                and data.get("schema") == DENY_SCHEMA
                and literals
            ):
                count = len(literals) if isinstance(literals, list) else 1
                findings.append(
                    f"{ctx.show(path)}: a deny list with {count} private literals is published; "
                    "the private list stays outside the repository"
                )
    return Outcome(examined, tuple(findings))


CHECKS = {
    "inventory-valid": check_inventory_valid,
    "schema-covered": check_schema_covered,
    "retention-bounded": check_retention_bounded,
    "purpose-limited": check_purpose_limited,
    "export-complete": check_export_complete,
    "erase-complete": check_erase_complete,
    "erase-effective": check_erase_effective,
    "policy-published": check_policy_published,
    "purge-automated": check_purge_automated,
    "copies-bounded": check_copies_bounded,
    "telegram-minimised": check_telegram_minimised,
    "defaults-private": check_defaults_private,
    "logs-pseudonymised": check_logs_pseudonymised,
    "public-scrub": check_public_scrub,
    "private-list-committed": check_private_list_committed,
}


# --- the command line -------------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="privacy-gdpr-probe.py",
        description="Judge a repository's personal data against its privacy.json (SPEC-V2-2222).",
    )
    parser.add_argument("--root", default=".", help="the tree to judge (default: .)")
    parser.add_argument(
        "--manifest",
        action="append",
        default=[],
        help="a privacy.json to judge instead of discovering them; repeatable",
    )
    parser.add_argument(
        "--deny-list",
        help=f"persona-core's private deny list (default: ${DENY_ENV}, if set)",
    )
    parser.add_argument(
        "--persona-core", help="persona-core's probe (default: beside this script)"
    )
    parser.add_argument(
        "--pack-dir",
        help="this pack's data directory (default: skills/packs/privacy-gdpr)",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASS_NAMES)
    commands.add_parser(
        "classes", help="list the classes with their stage and severity"
    )
    return parser


def main(argv: "Sequence[str] | None" = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "classes":
        for name, stage, severity in CLASSES:
            print(f"{name} {stage} {severity}")
        return EXIT_GREEN
    root = Path(args.root)
    if not root.is_dir():
        print(
            f"privacy-gdpr-probe: --root is not a directory: {args.root}",
            file=sys.stderr,
        )
        return EXIT_USAGE
    manifests = [Path(m) for m in args.manifest]
    for manifest in manifests:
        if not manifest.is_file():
            print(
                f"privacy-gdpr-probe: --manifest does not exist: {manifest}",
                file=sys.stderr,
            )
            return EXIT_USAGE
    private = args.deny_list or os.environ.get(DENY_ENV) or None
    name = args.name
    try:
        pack_dir = Path(args.pack_dir) if args.pack_dir else DEFAULT_PACK_DIR
        ctx = Context(
            root=root,
            inventories=[load_inventory(p) for p in (manifests or discover(root))],
            vocabulary=load_vocabulary(pack_dir),
            pack_dir=pack_dir,
            persona_core=Path(args.persona_core)
            if args.persona_core
            else DEFAULT_PERSONA_CORE,
            deny_list=Path(private) if private else None,
        )
        if not ctx.inventories:
            raise Void(f"no {MANIFEST_NAME} under {root}")
        if name != "inventory-valid" and not ctx.usable():
            raise Void(f"no {MANIFEST_NAME} under {root} parses as {SCHEMA}")
        outcome = CHECKS[name](ctx)
    except Void as error:
        print(f"{name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    if outcome.examined == 0:
        print(f"{name}: VOID: examined nothing")
        print("examined 0")
        return EXIT_VOID
    print(f"examined {outcome.examined}")
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


if __name__ == "__main__":
    sys.exit(main())
