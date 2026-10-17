"""The transition tables of the workspace's product state enums, and the five rules over them
(SPEC-434, ADR-448).

A product state enum is a Rust enum declared in a `.rs` file under `crates/*/src`, outside every
item under `#[cfg(test)]`, whose name ends in one of seven suffixes. Its table in
`config/fsm-tables.json` names its initial states, its terminal states and its moves, and its
states are the enum's own variants, read from its file. FSM-TABLE reads that every product state
enum has a table, and FSM-REACH, FSM-DEAD-END, FSM-TERMINAL and FSM-DETERMINISTIC read each
table's graph. Every reading prints one summary line and one line per hit, in the spelling of
SPEC-433's lines. A rule at `warn` prints and passes, and a rule at `fail` fails on a `new` or a
`stale` hit. A reading that examined nothing, a tree with no `crates/` or no register, and a
table naming a file, an enum or a state that is absent are VOID at every level.

The masker here serves these five rules alone; a lexer the Rust text guards share is #886's.
"""

import collections
import json
import os
import re
from pathlib import Path, PurePosixPath

REGISTER = "config/fsm-tables.json"
# A listed entry's ruling is a file under this directory (R3).
RULINGS = ("docs", "rulings")
RULES = ("FSM-TABLE", "FSM-REACH", "FSM-DEAD-END", "FSM-TERMINAL", "FSM-DETERMINISTIC")
LEVELS = ("warn", "fail")
# The unit each rule's summary line counts what it examined in (R13).
UNITS = {
    "FSM-TABLE": "state-enums",
    "FSM-REACH": "states",
    "FSM-DEAD-END": "states",
    "FSM-TERMINAL": "terminals",
    "FSM-DETERMINISTIC": "pairs",
}
# The ends of a product state enum's name (R4).
SUFFIXES = ("State", "Status", "Phase", "Stage", "Step", "Lifecycle", "Mode")
TABLE_KEYS = ("file", "enum", "initial", "terminal", "moves")
ROW_KEYS = ("from", "to", "on")
LISTED_KEYS = ("file", "key", "ruling")
# What opens a comment or a literal. A literal's prefix (`b`, `c`, `r`, `br`, `cr`) is read back
# from its quote, so a name that ends in one of those letters opens nothing.
OPENING = re.compile(r"""//|/\*|["']""")
COMMENT_EDGE = re.compile(r"/\*|\*/")
QUOTED = re.compile(r'(?:[^"\\]|\\.)*"', re.S)
# A character or byte literal; a quote that opens none is a lifetime's or a label's.
CHARACTER = re.compile(
    r"""'(?:[^'\\\n]|\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]{1,8}\}|[nrt0\\'"]))'"""
)
IDENTIFIER_CHARACTER = re.compile(r"[A-Za-z0-9_]")
UNMASKED = re.compile(r"[^\n]")
CFG_TEST = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
ATTRIBUTE = re.compile(r"#\s*\[")
# An item an outer `#[cfg(test)]` removes: an optional visibility, then an item's keyword (the
# Reference's VisItem and MacroItem). The attribute before a variant, a field or a statement
# removes no item here; R5 reads a variant's own.
ITEM = re.compile(
    r"(?:pub\b\s*(?:\([^)]*\)\s*)?)?"
    r"(?:mod|extern|use|fn|type|struct|enum|union|const|static|trait|impl|unsafe|async|auto"
    r"|macro_rules)\b"
)
ITEM_EDGE = re.compile(r"[()\[\]{};]")
ENUM = re.compile(r"\benum\s+([A-Za-z_][A-Za-z0-9_]*)")
BODY = re.compile(r"[{;]")
BRACKET = re.compile(r"[()\[\]{}]")
# The variant reader's depth: a comma inside a tuple's, a struct's or an attribute's brackets
# separates no two variants.
DEPTH = {"(": 1, ")": -1, "[": 1, "]": -1, "{": 1, "}": -1}
# A unit, tuple or struct variant, its discriminant already cut off (R5).
VARIANT = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)\s*(?:\(.*\)|\{.*\})?\s*", re.S)

Hit = collections.namedtuple("Hit", "path line key")
Table = collections.namedtuple("Table", "path name line states initial terminal moves")
Reading = collections.namedtuple(
    "Reading", "root rule level examined new listed stale files tables"
)


class Void(AssertionError):
    """A reading that proves nothing: it examined nothing, or what it reads is absent (R14)."""


class Refused(AssertionError):
    """A register that is not exactly R1's, R2's and R3's shape, refused at every level."""


class Failed(AssertionError):
    """A rule at `fail` with a `new` or a `stale` hit (R13)."""


# --------------------------------------------------------------------------- the masker (R4)


def blank(text):
    """`text` with every character but a newline turned to a space, so offsets and lines stay."""
    return UNMASKED.sub(" ", text)


def comment_end(text, at):
    """Where the comment opening at `at` ends, or None when none opens there: a line comment at
    its newline, and a block comment after the `*/` that closes it, nested ones counted."""
    if text.startswith("//", at):
        newline = text.find("\n", at)
        return len(text) if newline < 0 else newline
    if not text.startswith("/*", at):
        return None
    depth = 0
    for edge in COMMENT_EDGE.finditer(text, at):
        depth += 1 if edge.group() == "/*" else -1
        if depth == 0:
            return edge.end()
    return len(text)


def raw_prefix(text, at):
    """Whether the character at `at` is a raw string's `r`, alone or after a `b` or a `c`, with no
    name running into it."""
    if at < 0 or text[at] != "r":
        return False
    before = at - 1
    if before >= 0 and text[before] in "bc":
        before -= 1
    return before < 0 or not IDENTIFIER_CHARACTER.match(text, before)


def literal_end(text, at):
    """Where the string, raw string or character literal whose quote is at `at` ends, or None for
    a quote that opens none, a lifetime's or a label's. A raw string's quote follows its `r` and
    its hashes, and the string ends at a quote followed by as many hashes."""
    if text[at] == "'":
        found = CHARACTER.match(text, at)
        return found.end() if found else None
    if text[at] != '"':
        return None
    start = at
    while start > 0 and text[start - 1] == "#":
        start -= 1
    if raw_prefix(text, start - 1):
        hashes = at - start
        close = text.find('"' + "#" * hashes, at + 1)
        return len(text) if close < 0 else close + 1 + hashes
    found = QUOTED.match(text, at + 1)
    return found.end() if found else len(text)


def masked_literals(text):
    """`text` with every comment, string, raw string, byte string and character literal blanked."""
    parts, at = [], 0
    while found := OPENING.search(text, at):
        start = found.start()
        end = comment_end(text, start)
        if end is None:
            end = literal_end(text, start)
        if end is None:
            parts.append(text[at : start + 1])
            at = start + 1
            continue
        parts += [text[at:start], blank(text[start:end])]
        at = end
    parts.append(text[at:])
    return "".join(parts)


def skip_space(text, at, end):
    """The first offset at or after `at`, before `end`, that is not whitespace."""
    while at < end and text[at].isspace():
        at += 1
    return at


def closing(text, at):
    """The offset of the bracket that closes the one at `at`, or the text's end when none does."""
    depth = 0
    for found in BRACKET.finditer(text, at):
        depth += 1 if found.group() in "([{" else -1
        if depth == 0:
            return found.start()
    return len(text)


def skip_attributes(text, at, end):
    """The first offset at or after `at`, before `end`, past whitespace and outer attributes."""
    at = skip_space(text, at, end)
    while ATTRIBUTE.match(text, at, end):
        at = skip_space(text, closing(text, text.index("[", at)) + 1, end)
    return at


def item_end(text, at):
    """Where the item starting at `at` ends: after its `;` outside every bracket, or after the
    brace that closes its first brace outside every bracket."""
    depth = 0
    for edge in ITEM_EDGE.finditer(text, at):
        char = edge.group()
        if char == ";" and depth == 0:
            return edge.end()
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            if depth < 0:
                return edge.start()
            if depth == 0 and char == "}":
                return edge.end()
    return len(text)


def masked_tests(text):
    """Masked `text` with every item under an outer `#[cfg(test)]` blanked as well: the attribute,
    the attributes after it and the item, to the `;` or the closing brace that ends it."""
    parts, at = [], 0
    for found in CFG_TEST.finditer(text):
        if found.start() < at:
            continue
        item = skip_attributes(text, found.end(), len(text))
        if not ITEM.match(text, item):
            continue
        end = item_end(text, item)
        parts += [text[at : found.start()], blank(text[found.start() : end])]
        at = end
    parts.append(text[at:])
    return "".join(parts)


def masked(text):
    """Rust source `text` as R4 reads it: comments, literals and `#[cfg(test)]` items blanked."""
    return masked_tests(masked_literals(text))


def line_of(text, at):
    """The line of the offset `at` in `text`, from 1."""
    return text.count("\n", 0, at) + 1


# ------------------------------------------------------------- the enum and variant reader (R5)


def enums_in(text):
    """Every enum masked `text` declares, by name: the line of its `enum` and the bounds of its
    body. A name declared twice keeps its first declaration."""
    found = {}
    for match in ENUM.finditer(text):
        body = BODY.search(text, match.end())
        if body and body.group() == "{" and match.group(1) not in found:
            line = line_of(text, match.start())
            found[match.group(1)] = (line, body.end(), closing(text, body.start()))
    return found


def pieces(text, start, end):
    """The bounds of each comma-separated piece of `text[start:end]` outside every bracket."""
    depth, begin = 0, start
    for at in range(start, end):
        depth += DEPTH.get(text[at], 0)
        if text[at] == "," and depth == 0:
            yield begin, at
            begin = at + 1
    yield begin, end


def discriminant(text, at, end):
    """Where a variant's `= <discriminant>` begins in `text[at:end]` outside every bracket, or
    `end` when it carries none."""
    depth = 0
    for index in range(at, end):
        depth += DEPTH.get(text[index], 0)
        if text[index] == "=" and depth == 0:
            return index
    return end


def states_of(text, start, end):
    """The states of the enum whose body is masked `text[start:end]` (R5), each with its line: the
    name of each top-level variant, after its outer attributes, unless one of them is
    `#[cfg(test)]`. A variant the reader cannot read is VOID."""
    states = []
    for begin, finish in pieces(text, start, end):
        at = skip_attributes(text, begin, finish)
        if CFG_TEST.search(text, begin, at):
            continue
        head = text[at : discriminant(text, at, finish)].strip()
        if not head:
            continue
        variant = VARIANT.fullmatch(head)
        if not variant:
            raise Void(f"the variant at line {line_of(text, at)} is not one the reader reads")
        states.append((variant.group(1), line_of(text, at)))
    return states


def is_product(path):
    """Whether `path` names a product file: a `.rs` file under `crates/*/src`, at any depth."""
    parts = PurePosixPath(path).parts
    return (
        len(parts) > 3
        and parts[0] == "crates"
        and parts[2] == "src"
        and parts[-1].endswith(".rs")
        and ".." not in parts
    )


def table_states(root, path, name):
    """The line of the enum `name` in the product file `path` under `root`, and its states with
    their lines; VOID when the file is absent or declares no such enum (R6)."""
    source = Path(root) / path
    if not (is_product(path) and source.is_file()):
        raise Void(f"the table of {name} names {path}, a product file that is absent")
    text = masked(source.read_text(encoding="utf-8"))
    enums = enums_in(text)
    if name not in enums:
        raise Void(f"the table of {name} names {path}, which declares no enum {name}")
    line, start, end = enums[name]
    return line, states_of(text, start, end)


# --------------------------------------------------------------------- the register (R1-R3)


def is_name(value):
    """Whether `value` is a non-empty name."""
    return isinstance(value, str) and bool(value.strip())


def is_names(value):
    """Whether `value` is a list of non-empty names."""
    return isinstance(value, list) and all(is_name(each) for each in value)


def need(table, key, where):
    """The value of `key` in `table`, refused when the table has none."""
    if key not in table:
        raise Refused(f"{where} has no {key!r}")
    return table[key]


def check_listed(root, rule, listed):
    """One listed entry (R3): its hit's file and key, and a ruling that is a file under
    `docs/rulings/` in the tree."""
    if not isinstance(listed, dict) or set(listed) - set(LISTED_KEYS):
        raise Refused(f"a listed entry of {rule} holds a key other than file, key and ruling")
    if not (is_name(listed.get("file")) and is_name(listed.get("key"))):
        raise Refused(f"a listed entry of {rule} names no file and key")
    ruling = listed.get("ruling")
    if not is_name(ruling):
        raise Refused(f"a listed entry of {rule} carries no ruling")
    parts = PurePosixPath(ruling).parts
    if parts[: len(RULINGS)] != RULINGS or ".." in parts:
        raise Refused(f"a listed entry of {rule} names {ruling}, outside docs/rulings/")
    if not (Path(root) / ruling).is_file():
        raise Refused(f"a listed entry of {rule} names {ruling}, which does not exist")


def check_rule(root, rule, entry):
    """One rule's entry (R1): exactly a level of `warn` or `fail` and its listed entries."""
    if not isinstance(entry, dict) or set(entry) != {"level", "listed"}:
        raise Refused(f"{rule} holds exactly a level and its listed entries, and does not")
    if entry["level"] not in LEVELS:
        raise Refused(f"{rule}'s level {entry['level']!r} is neither warn nor fail")
    if not isinstance(entry["listed"], list):
        raise Refused(f"{rule}'s listed entries are not a list")
    for listed in entry["listed"]:
        check_listed(root, rule, listed)


def check_table(index, table):
    """One table (R2): exactly its keys, a non-empty initial set, rows of a state, a state and an
    event with no row twice, and a reason exactly when it has no moves."""
    where = f"table {index + 1}"
    if not isinstance(table, dict):
        raise Refused(f"{where} is not an object")
    unknown = set(table) - {*TABLE_KEYS, "no_moves"}
    if unknown:
        raise Refused(f"{where} holds an unknown key: {', '.join(sorted(unknown))}")
    file = need(table, "file", where)
    enum = need(table, "enum", where)
    initial = need(table, "initial", where)
    terminal = need(table, "terminal", where)
    moves = need(table, "moves", where)
    if not (is_name(file) and is_name(enum)):
        raise Refused(f"{where} names no file and enum")
    if not (is_names(initial) and is_names(terminal) and isinstance(moves, list)):
        raise Refused(f"{where}'s initial and terminal are not lists of names, or its moves a list")
    if not initial:
        raise Refused(f"{where} has an empty initial set")
    rows = []
    for move in moves:
        if not (isinstance(move, dict) and set(move) == set(ROW_KEYS)):
            raise Refused(f"{where} holds a row that is not exactly from, to and on")
        row = (move["from"], move["to"], move["on"])
        if not all(is_name(each) for each in row):
            raise Refused(f"{where} holds a row with an empty name")
        if row in rows:
            raise Refused(f"{where} holds the row {' '.join(row)} twice")
        rows.append(row)
    if not moves and not is_name(table.get("no_moves")):
        raise Refused(f"{where} has no moves and no reason in no_moves")
    if moves and "no_moves" in table:
        raise Refused(f"{where} has moves and a no_moves")


def register(root):
    """The register under `root`, refused unless it is exactly R1's, R2's and R3's shape; VOID when
    it is absent (R14)."""
    path = Path(root) / REGISTER
    if not path.is_file():
        raise Void(f"the register {REGISTER} is absent")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as error:
        raise Refused(f"the register is not JSON: {error}") from None
    if not isinstance(data, dict) or set(data) != {"rules", "tables"}:
        raise Refused("the register holds exactly rules and tables, and this one does not")
    rules = data["rules"]
    if not isinstance(rules, dict) or set(rules) != set(RULES):
        raise Refused("the register's rules are not exactly " + ", ".join(RULES))
    for rule in RULES:
        check_rule(root, rule, rules[rule])
    if not isinstance(data["tables"], list):
        raise Refused("the register's tables are not a list")
    seen = set()
    for index, table in enumerate(data["tables"]):
        check_table(index, table)
        key = (table["file"], table["enum"])
        if key in seen:
            raise Refused(f"two tables name {table['enum']} in {table['file']}")
        seen.add(key)
    return data


def load_tables(root, raw):
    """Each table of the register, keyed by its file and enum, with its enum's states (R5); VOID
    when it names a state its enum lacks (R6)."""
    tables = {}
    for table in raw:
        path, name = table["file"], table["enum"]
        line, states = table_states(root, path, name)
        known = {state for state, _line in states}
        moves = [(move["from"], move["to"], move["on"]) for move in table["moves"]]
        named = [
            *table["initial"],
            *table["terminal"],
            *(row[at] for row in moves for at in (0, 1)),
        ]
        absent = sorted({state for state in named if state not in known})
        if absent:
            raise Void(f"the table of {name} in {path} names {', '.join(absent)}, which it lacks")
        tables[(path, name)] = Table(
            path, name, line, states, table["initial"], table["terminal"], moves
        )
    return tables


# ------------------------------------------------------------------- the walk and the rules


def product_files(root):
    """Every product file under `root`, by its path from `root` (R4)."""
    crates = Path(root) / "crates"
    return sorted(
        path.relative_to(root).as_posix() for path in crates.glob("*/src/**/*.rs") if path.is_file()
    )


def listed_independently(root):
    """The product files under `root` by a second walk, `os.walk` from each crate's `src`, which
    FSM-TABLE's walk is compared with (R14)."""
    crates = Path(root) / "crates"
    if not crates.is_dir():
        return []
    found = []
    for crate in sorted(os.listdir(crates)):
        for directory, _directories, files in os.walk(crates / crate / "src"):
            found += [Path(directory, name) for name in files if name.endswith(".rs")]
    return found


def walk(initial, moves):
    """A table's reached set (R7): its initial states, then, breadth first, every state a row
    leads to from a reached one."""
    reached = list(dict.fromkeys(initial))
    queue = collections.deque(reached)
    while queue:
        state = queue.popleft()
        for source, target, _on in moves:
            if source == state and target not in reached:
                reached.append(target)
                queue.append(target)
    return set(reached)


def rules_over(states, initial, terminal, moves):
    """The four graph rules over one table (R7-R12): for each, the states or pairs it examines and
    its hits. `states` lists the enum's states, `initial` and `terminal` name some of them, and
    each move is a `(from, to, on)` row."""
    terminal = set(terminal)
    reached = walk(initial, moves)
    leaving = {source for source, _target, _on in moves}
    unreached = [state for state in states if state not in reached]
    inner = [state for state in states if state in reached and state not in terminal]
    ends = [state for state in states if state in reached and state in terminal]
    targets = collections.defaultdict(set)
    for source, target, on in moves:
        targets[(source, on)].add(target)
    split = sorted(pair for pair, found in targets.items() if len(found) > 1)
    return {
        "FSM-REACH": (list(states), unreached),
        "FSM-DEAD-END": (inner, [state for state in inner if state not in leaving]),
        "FSM-TERMINAL": (ends, [state for state in ends if state in leaving]),
        "FSM-DETERMINISTIC": (sorted(targets), split),
    }


def site(rule, table, unit):
    """Where one examined state, or one state and event pair, is: the variant's line for a state
    (R8-R10), and the enum's for a pair (R11)."""
    if rule == "FSM-DETERMINISTIC":
        state, event = unit
        return Hit(table.path, table.line, f"{table.name}::{state} on {event}")
    return Hit(table.path, dict(table.states)[unit], f"{table.name}::{unit}")


def graph_rule(rule, tables):
    """One graph rule (R8-R11) over every table: each state or pair it examines, and its hits."""
    examined, hits = [], []
    for key in sorted(tables):
        table = tables[key]
        states = [state for state, _line in table.states]
        units, found = rules_over(states, table.initial, table.terminal, table.moves)[rule]
        examined += [site(rule, table, unit) for unit in units]
        hits += [site(rule, table, unit) for unit in found]
    return examined, hits


def table_rule(root, files, tables):
    """FSM-TABLE (R6): every product state enum the product files declare, and a hit for each
    that no table names, at its `enum`'s line."""
    examined, hits = [], []
    for path in files:
        text = masked((root / path).read_text(encoding="utf-8"))
        for name, (line, _start, _end) in sorted(enums_in(text).items()):
            if not name.endswith(SUFFIXES):
                continue
            examined.append(Hit(path, line, name))
            if (path, name) not in tables:
                hits.append(Hit(path, line, name))
    return examined, hits


def classified(root, rule, entry, examined, hits, files, tables):
    """The reading: each hit `new` or `listed` by the rule's listed entries, and each listed entry
    no hit matches `stale`, at its site's line when the reading examined it (R3, R13)."""
    keys = {(listed["file"], listed["key"]) for listed in entry["listed"]}
    found = {(hit.path, hit.key) for hit in hits}
    sites = {(unit.path, unit.key): unit.line for unit in examined}
    new = [hit for hit in hits if (hit.path, hit.key) not in keys]
    old = [hit for hit in hits if (hit.path, hit.key) in keys]
    stale = [
        Hit(listed["file"], sites.get((listed["file"], listed["key"]), 0), listed["key"])
        for listed in entry["listed"]
        if (listed["file"], listed["key"]) not in found
    ]
    return Reading(root, rule, entry["level"], examined, new, old, stale, files, tables)


def read(root, rule, listing=None):
    """One rule's reading of the tree at `root`. VOID (R14) when the tree has no `crates/` or no
    register, on a table's VOID (R6), when it examined nothing, and, for FSM-TABLE, when its
    walk's count of product files differs from `listing`'s, `listed_independently` unless a
    caller gives another; a malformed register is refused (R1-R3)."""
    root = Path(root)
    if not (root / "crates").is_dir():
        raise Void(f"FSM-LINT {rule} VOID: the tree has no crates/")
    try:
        data = register(root)
        tables = load_tables(root, data["tables"])
    except Void as void:
        raise Void(f"FSM-LINT {rule} VOID: {void}") from None
    files = None
    if rule == "FSM-TABLE":
        files = product_files(root)
        independent = (listing or listed_independently)(root)
        if len(files) != len(independent):
            raise Void(
                f"FSM-LINT {rule} VOID: its walk read {len(files)} product file(s) and an "
                f"independent listing {len(independent)}"
            )
        examined, hits = table_rule(root, files, tables)
    else:
        examined, hits = graph_rule(rule, tables)
    if not examined:
        raise Void(f"FSM-LINT {rule} VOID: examined 0 {UNITS[rule]}")
    entry = data["rules"][rule]
    count = None if files is None else len(files)
    return classified(root, rule, entry, examined, hits, count, len(tables))


# ------------------------------------------------------------------------- the lines (R13)


def summary(reading):
    """A reading's summary line."""
    counts = [
        f"level={reading.level}",
        f"examined={len(reading.examined)}",
        UNITS[reading.rule],
        f"hits={len(reading.new) + len(reading.listed)}",
        f"new={len(reading.new)}",
        f"listed={len(reading.listed)}",
        f"stale={len(reading.stale)}",
    ]
    return " ".join(["FSM-LINT", reading.rule, *counts])


def hit_line(reading, kind, hit):
    """One hit's line: its rule, level and kind, its path and line, and its key, and never the
    text of the line it names."""
    where = hit.path + ":" + str(hit.line)
    fields = ["FSM-LINT", reading.rule, reading.level, kind, where, hit.key]
    return " ".join(fields)


def report(reading):
    """Print a reading's summary line and one line per hit, and return them; at `fail`, a `new` or
    a `stale` hit then fails (R13)."""
    lines = [summary(reading)]
    for kind, group in (("new", reading.new), ("listed", reading.listed), ("stale", reading.stale)):
        lines += [hit_line(reading, kind, hit) for hit in sorted(group)]
    for line in lines:
        print(line)
    if reading.level == "fail" and (reading.new or reading.stale):
        raise Failed(
            f"{reading.rule} is at fail and reads {len(reading.new)} new and "
            f"{len(reading.stale)} stale hit(s)"
        )
    return lines
