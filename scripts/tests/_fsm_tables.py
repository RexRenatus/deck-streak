"""The transition tables of the workspace's product state enums, and the five rules over them
(SPEC-434, ADR-448).

STUBS, for the red commit: every input is kept and the behaviour each criterion reads is missing.
The product walk reads only the top level of each crate's `src`; the enum reader keeps the items
under `#[cfg(test)]` and reads no suffix but `State`; the variant reader splits on the commas
outside parentheses, so a struct variant's fields and a `#[cfg(test)]` variant are read as
states; no table and no reading is VOID; the register reader takes any shape; every listed entry
is admitted and none reads `stale`; the graph rules return no hit, and FSM-REACH examines only the
initial states; FSM-DEAD-END, FSM-TERMINAL and FSM-DETERMINISTIC print no summary line; `fail`
behaves as `warn`; and a hit line carries the text of the line it names.
"""

import collections
import json
import re
from pathlib import Path

REGISTER = "config/fsm-tables.json"
RULES = ("FSM-TABLE", "FSM-REACH", "FSM-DEAD-END", "FSM-TERMINAL", "FSM-DETERMINISTIC")
# The unit each rule's summary line counts what it examined in (R13).
UNITS = {
    "FSM-TABLE": "state-enums",
    "FSM-REACH": "states",
    "FSM-DEAD-END": "states",
    "FSM-TERMINAL": "terminals",
    "FSM-DETERMINISTIC": "pairs",
}
OPENING = re.compile(r"""//|/\*|["']""")
COMMENT_EDGE = re.compile(r"/\*|\*/")
QUOTED = re.compile(r'(?:[^"\\]|\\.)*"', re.S)
CHARACTER = re.compile(
    r"""'(?:[^'\\\n]|\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]{1,8}\}|[nrt0\\'"]))'"""
)
IDENTIFIER_CHARACTER = re.compile(r"[A-Za-z0-9_]")
UNMASKED = re.compile(r"[^\n]")
ENUM = re.compile(r"\benum\s+([A-Za-z_][A-Za-z0-9_]*)")
BODY = re.compile(r"[{;]")
BRACKET = re.compile(r"[()\[\]{}]")
NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
ATTRIBUTE_TEXT = re.compile(r"#\s*\[[^\]]*\]")

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


def blank(text):
    """`text` with every character but a newline turned to a space, so offsets and lines stay."""
    return UNMASKED.sub(" ", text)


def comment_end(text, at):
    """Where the comment opening at `at` ends, or None when none opens there."""
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
    """Whether the character at `at` is a raw string's `r`, with no name running into it."""
    if at < 0 or text[at] != "r":
        return False
    before = at - 1
    if before >= 0 and text[before] in "bc":
        before -= 1
    return before < 0 or not IDENTIFIER_CHARACTER.match(text, before)


def literal_end(text, at):
    """Where the literal whose quote is at `at` ends, or None for a quote that opens none."""
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
    """`text` with every comment and literal blanked."""
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


def masked(text):
    """Stub: comments and literals blanked, and every `#[cfg(test)]` item kept."""
    return masked_literals(text)


def line_of(text, at):
    """The line of the offset `at` in `text`, from 1."""
    return text.count("\n", 0, at) + 1


def closing(text, at):
    """The offset of the bracket that closes the one at `at`, or the text's end when none does."""
    depth = 0
    for found in BRACKET.finditer(text, at):
        depth += 1 if found.group() in "([{" else -1
        if depth == 0:
            return found.start()
    return len(text)


def enums_in(text):
    """Every enum masked `text` declares, by name: its line and the bounds of its body."""
    found = {}
    for match in ENUM.finditer(text):
        body = BODY.search(text, match.end())
        if body and body.group() == "{" and match.group(1) not in found:
            line = line_of(text, match.start())
            found[match.group(1)] = (line, body.end(), closing(text, body.start()))
    return found


def states_of(text, start, end):
    """Stub: the first name of each piece between the commas outside parentheses, attributes
    dropped."""
    states, depth, begin = [], 0, start
    for at in range(start, end + 1):
        char = text[at] if at < end else ","
        depth += {"(": 1, ")": -1}.get(char, 0)
        if char == "," and depth == 0:
            piece = ATTRIBUTE_TEXT.sub(" ", text[begin:at])
            name = NAME.search(piece)
            if name:
                states.append((name.group(), line_of(text, begin + name.start())))
            begin = at + 1
    return states


def table_states(root, path, name):
    """Stub: the enum's line and states, and no VOID for an absent file or enum."""
    source = Path(root) / path
    if not source.is_file():
        return 0, []
    text = masked(source.read_text(encoding="utf-8"))
    enums = enums_in(text)
    if name not in enums:
        return 0, []
    line, start, end = enums[name]
    return line, states_of(text, start, end)


def register(root):
    """Stub: the register as it parses, any shape taken, and a rule it does not name at warn."""
    path = Path(root) / REGISTER
    data = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    rules = {rule: {"level": "warn", "listed": []} for rule in RULES}
    rules.update(data.get("rules", {}))
    return {"rules": rules, "tables": data.get("tables", [])}


def load_tables(root, raw):
    """Stub: each table with its enum's states, and no VOID for a state its enum lacks."""
    tables = {}
    for table in raw:
        path, name = table["file"], table["enum"]
        line, states = table_states(root, path, name)
        moves = [(move["from"], move["to"], move["on"]) for move in table["moves"]]
        tables[(path, name)] = Table(
            path, name, line, states, table["initial"], table["terminal"], moves
        )
    return tables


def product_files(root):
    """Stub: the `.rs` files at the top level of each crate's `src`."""
    crates = Path(root) / "crates"
    return sorted(
        path.relative_to(root).as_posix() for path in crates.glob("*/src/*.rs") if path.is_file()
    )


def rules_over(states, initial, terminal, moves):
    """Stub: what each graph rule examines, FSM-REACH only the initial states, and no hit."""
    reached = set(initial)
    inner = [state for state in states if state in reached and state not in terminal]
    ends = [state for state in states if state in reached and state in terminal]
    pairs = sorted({(source, on) for source, _target, on in moves})
    return {
        "FSM-REACH": (list(initial), []),
        "FSM-DEAD-END": (inner, []),
        "FSM-TERMINAL": (ends, []),
        "FSM-DETERMINISTIC": (pairs, []),
    }


def site(rule, table, unit):
    """Where one examined state, or one state and event pair, is."""
    if rule == "FSM-DETERMINISTIC":
        state, event = unit
        return Hit(table.path, table.line, f"{table.name}::{state} on {event}")
    return Hit(table.path, dict(table.states).get(unit, 0), f"{table.name}::{unit}")


def graph_rule(rule, tables):
    """One graph rule over every table: each state or pair it examines, and its hits."""
    examined, hits = [], []
    for key in sorted(tables):
        table = tables[key]
        states = [state for state, _line in table.states]
        units, found = rules_over(states, table.initial, table.terminal, table.moves)[rule]
        examined += [site(rule, table, unit) for unit in units]
        hits += [site(rule, table, unit) for unit in found]
    return examined, hits


def table_rule(root, files, tables):
    """Stub: the enums named `...State` the product files declare, and a hit for each no table
    names."""
    examined, hits = [], []
    for path in files:
        text = masked((root / path).read_text(encoding="utf-8"))
        for name, (line, _start, _end) in sorted(enums_in(text).items()):
            if not name.endswith("State"):
                continue
            examined.append(Hit(path, line, name))
            if (path, name) not in tables:
                hits.append(Hit(path, line, name))
    return examined, hits


def read(root, rule, listing=None):
    """Stub: one rule's reading, never VOID, every listed entry admitted and none `stale`."""
    root = Path(root)
    data = register(root)
    tables = load_tables(root, data["tables"])
    files = None
    if rule == "FSM-TABLE":
        files = product_files(root)
        examined, hits = table_rule(root, files, tables)
    else:
        examined, hits = graph_rule(rule, tables)
    entry = data["rules"][rule]
    keys = {(listed.get("file"), listed.get("key")) for listed in entry["listed"]}
    new = [hit for hit in hits if (hit.path, hit.key) not in keys]
    old = [hit for hit in hits if (hit.path, hit.key) in keys]
    count = None if files is None else len(files)
    return Reading(root, rule, entry["level"], examined, new, old, [], count, len(tables))


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
    """Stub: one hit's line, with the text of the line it names."""
    where = hit.path + ":" + str(hit.line)
    text = (reading.root / hit.path).read_text(encoding="utf-8").split("\n")[hit.line - 1]
    return " ".join(["FSM-LINT", reading.rule, reading.level, kind, where, hit.key, text.strip()])


def report(reading):
    """Stub: the summary line for FSM-TABLE and FSM-REACH alone, one line per hit, and `fail`
    read as `warn`."""
    lines = [summary(reading)] if reading.rule in ("FSM-TABLE", "FSM-REACH") else []
    for kind, group in (("new", reading.new), ("listed", reading.listed), ("stale", reading.stale)):
        lines += [hit_line(reading, kind, hit) for hit in sorted(group)]
    for line in lines:
        print(line)
    return lines
