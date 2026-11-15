"""The four state-machine lints' one helper (SPEC-433 R1 to R11, ADR-447 D1 to D5).

Each rule is a test module of its own (`test_fsm_check_list.py`, `test_fsm_wildcard.py`,
`test_fsm_unreachable.py`, `test_fsm_uncovered.py`), and their shared behaviour is tested in
`test_fsm_lints.py`. This module holds what they share: the register's reader and its admission,
the three walks, the Rust masker, the item and match reader, the schema reader, the four rules and
the line printer. It reads committed files and writes nothing: the one database it opens is an
in-memory SQLite database, one per reading.

A rule function returns a `Reading` and prints nothing, so a fixture's hits never reach a job's
log. `report` prints a reading's lines and applies its level: at warn nothing a hit says fails, and
at fail a `new` or `stale` hit fails the test (R10). A reading that examined nothing, a walk over
an absent directory, a migration that does not execute and a pair naming what is absent are VOID
at every level (R11), and a register that breaks R1 or R2 is refused at every level.
"""

import json
import re
from dataclasses import dataclass, field
from pathlib import Path

try:
    import sqlite3
except ImportError:  # R11: a Python without sqlite3 reads VOID, never an import error
    sqlite3 = None

#: Where the register lives, and where a ruling one of its entries cites must live (R1, R2).
REGISTER = "config/fsm-lints.json"
RULINGS = "docs/rulings/"

CHECK_LIST = "FSM-CHECK-LIST"
WILDCARD = "FSM-WILDCARD"
UNREACHABLE = "FSM-UNREACHABLE"
UNCOVERED = "FSM-UNCOVERED"
#: The four rules, each with the keys its object in the register may hold (R1).
RULE_KEYS = {
    CHECK_LIST: ("level", "pairs", "unpaired"),
    WILDCARD: ("level", "listed"),
    UNREACHABLE: ("level", "listed"),
    UNCOVERED: ("level", "listed"),
}
LEVELS = ("warn", "fail")
#: Each list's entries: the fields an entry must hold, then the fields it may hold (R1, R4).
ENTRY_FIELDS = {
    (CHECK_LIST, "pairs"): (
        ("table", "column", "enum", "file", "spelling"),
        ("unstored",),
    ),
    (CHECK_LIST, "unpaired"): (("table", "column", "why"), ("ruling",)),
    (WILDCARD, "listed"): (("path", "enum", "fn"), ("ruling",)),
    (UNREACHABLE, "listed"): (("path", "fn", "macro"), ("ruling",)),
    (UNCOVERED, "listed"): (("path", "enum"), ("ruling",)),
}
#: The fields that key each rule's ratchet entries: a column, or a site by path and item (R2).
ENTRY_KEY = {
    CHECK_LIST: ("table", "column"),
    WILDCARD: ("path", "enum", "fn"),
    UNREACHABLE: ("path", "fn", "macro"),
    UNCOVERED: ("path", "enum"),
}
KINDS = ("new", "listed", "stale")

#: R5: a text domain column with one of these names, or ending in `_` and one, is a state column.
STATE_NAMES = (
    "state",
    "status",
    "outcome",
    "verdict",
    "phase",
    "stage",
    "step",
    "mode",
)
#: R6: an enum declared in product source whose name ends in one of these is a state enum.
ENUM_SUFFIXES = (
    "State",
    "Status",
    "Phase",
    "Stage",
    "Step",
    "Lifecycle",
    "Mode",
)
#: R8: the macros an arm calls to claim that it is never reached.
MACROS = (
    "unreachable",
    "panic",
    "todo",
    "unimplemented",
)
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


@dataclass(frozen=True)
class Enum:
    """An enum declared in product source: its file, name, line and variants."""

    path: str
    name: str
    line: int
    variants: tuple


@dataclass(frozen=True)
class Arm:
    """One arm of a match: its pattern's text, guard included, and where it and its value sit."""

    pattern: str
    start: int
    value: int
    end: int


@dataclass(frozen=True)
class Match:
    """One match: where its keyword sits, its arms, and whether `matches!` wrote it."""

    at: int
    arms: tuple
    macro: bool


@dataclass(frozen=True)
class Source:
    """One product file: its text, the text masked, its string literals, and what it declares."""

    path: str
    text: str
    masked: str
    strings: dict
    fns: list
    impls: list
    matches: list
    enums: list


# ------------------------------------------------------------------------------------- walks


def tree_dir(root, name):
    """`root/name`, refused as VOID when the tree holds no such directory (R11)."""
    folder = Path(root) / name
    if not folder.is_dir():
        raise Void(name + "/ is absent, so there is nothing to read")
    return folder


def product_files(root):
    """R6: every `.rs` file under `crates/*/src`, at any depth, in path order."""
    folder = tree_dir(root, "crates")
    return sorted(path for path in folder.glob("*/src/**/*.rs") if path.is_file())


def migration_files(root):
    """R3: every `migrations/*.sql`, in filename order."""
    folder = tree_dir(root, "migrations")
    return sorted(path for path in folder.glob("*.sql") if path.is_file())


def formal_files(root):
    """R9: every `.tla` and `.lean` module under `formal/`, at any depth, in path order."""
    folder = tree_dir(root, "formal")
    return sorted(
        path for path in folder.rglob("*") if path.suffix in (".tla", ".lean") and path.is_file()
    )


def relative(root, path):
    return path.relative_to(root).as_posix()


def line_of(text, at):
    """The line, counted from 1, that position `at` of `text` stands on."""
    return text.count("\n", 0, at) + 1


# ---------------------------------------------------------------------------------- register


def read_register(root):
    """The register at `root`, refused unless it holds exactly the four rules, each at a known
    level with only its own keys, and each entry with exactly its own fields (R1)."""
    path = Path(root) / REGISTER
    try:
        register = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        raise Refused(REGISTER + " does not read as JSON") from None
    if not isinstance(register, dict) or sorted(register) != ["rules"]:
        raise Refused(REGISTER + " holds something beside its one `rules` object")
    rules = register["rules"]
    if not isinstance(rules, dict):
        raise Refused(REGISTER + "'s `rules` is not an object")
    unknown = sorted(set(rules) - set(RULE_KEYS))
    if unknown:
        raise Refused(REGISTER + " names an unknown rule: " + ", ".join(unknown))
    missing = sorted(set(RULE_KEYS) - set(rules))
    if missing:
        raise Refused(REGISTER + " lacks a rule: " + ", ".join(missing))
    for name, keys in RULE_KEYS.items():
        body = rules[name]
        if not isinstance(body, dict):
            raise Refused(name + " is not an object")
        stray = sorted(set(body) - set(keys))
        if stray:
            raise Refused(name + " holds an unknown key: " + ", ".join(stray))
        level = body.get("level")
        if level not in LEVELS:
            raise Refused(name + " has the level " + repr(level) + ", which is not warn or fail")
        for key in keys[1:]:
            entries = body.get(key, [])
            if not isinstance(entries, list):
                raise Refused(name + " " + key + " is not a list")
            for entry in entries:
                refuse_entry(name, key, entry)
    return register


def refuse_entry(name, key, entry):
    """Refuse an entry of `name`'s list `key` unless it holds exactly its own fields (R1, R4)."""
    where = name + " " + key + " entry " + json.dumps(entry, sort_keys=True)
    required, optional = ENTRY_FIELDS[(name, key)]
    if not isinstance(entry, dict):
        raise Refused(where + " is not an object")
    fields = set(entry)
    if set(required) - fields or fields - set(required) - set(optional):
        raise Refused(where + " does not hold exactly " + ", ".join(required + optional))
    named = [entry[part] for part in required + optional if part in entry]
    if key != "pairs" and not all(isinstance(value, str) for value in named):
        raise Refused(where + " holds a value that is not text")
    if key == "pairs":
        spelling = entry["spelling"]
        if spelling != "snake_case" and not (
            isinstance(spelling, dict)
            and sorted(spelling) == ["fn"]
            and isinstance(spelling["fn"], str)
        ):
            raise Refused(where + ' spells by neither {"fn": <name>} nor "snake_case"')
        for each in entry.get("unstored", []):
            if not isinstance(each, dict) or sorted(each) != ["variant", "why"]:
                raise Refused(where + " names an unstored variant without its variant and why")


def entry_key(name, entry):
    """The key of one of `name`'s ratchet entries (R2)."""
    return tuple(entry[part] for part in ENTRY_KEY[name])


def admit(root, name, entries, seed):
    """The keys of `name`'s ratchet entries (R2). An entry with no `ruling` must be in its test
    module's frozen `seed`; one outside the seed must cite a ruling, a file under docs/rulings/
    that the tree holds. Any other entry refuses the register at every level. Deleting an entry
    owes nothing."""
    seeded = set(seed)
    admitted = set()
    for entry in entries:
        key = entry_key(name, entry)
        ruling = entry.get("ruling")
        if ruling is None:
            if key not in seeded:
                raise Refused(
                    name + " lists " + " ".join(key) + " outside its seed, with no ruling"
                )
        elif not ruled(root, ruling):
            raise Refused(
                name + " lists " + " ".join(key) + " by " + ruling + ", no file under " + RULINGS
            )
        admitted.add(key)
    return admitted


def ruled(root, ruling):
    """Whether `ruling` names a file under docs/rulings/ that the tree holds."""
    if not ruling.startswith(RULINGS):
        return False
    rulings = (Path(root) / RULINGS).resolve()
    cited = (Path(root) / ruling).resolve()
    return rulings in cited.parents and cited.is_file()


# ------------------------------------------------------------------------------ Rust masker

#: Where a comment or a literal may open: line and block comments, raw and prefixed strings,
#: plain strings, byte characters, and a quote that opens a character literal or a lifetime.
LEXEME = re.compile(r"""//|/\*|(?<![\w])(?:br|cr|r)#*"|(?<![\w])[bc]"|"|(?<![\w])b'|'""")
#: The body of a plain string after its opening quote, through its closing quote.
STRING_BODY = re.compile(r'(?:[^"\\]|\\.)*"', re.S)
#: The body of a character literal after its opening quote, through its closing quote (the Rust
#: Reference's CHAR_LITERAL). A quote no such body follows opens a lifetime or a label.
CHAR_BODY = re.compile(r"(?:[^'\\\n]|\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]{1,6}\}|.))'")
NOT_NEWLINE = re.compile(r"[^\n]")
BLANK = re.compile(r"\s*")
WORD = re.compile(r"\w+")


def blank(text):
    """`text` as spaces, its newlines kept."""
    return NOT_NEWLINE.sub(" ", text)


def skip_blank(masked, at):
    return BLANK.match(masked, at).end()


def mask(text):
    """`text` with every comment blanked (line, block, nested and doc) and the inside of every
    string, raw string, byte string and character literal blanked (R6). Each keeps its length and
    newlines, so a position in one is the same position in the other. Also each string literal's
    start mapped to its end, which a spelling is read from (R4)."""
    out = []
    strings = {}
    at = 0
    while True:
        found = LEXEME.search(text, at)
        if found is None:
            out.append(text[at:])
            break
        start = found.start()
        out.append(text[at:start])
        token = found.group()
        if token == "//":
            end = text.find("\n", start)
            end = len(text) if end < 0 else end
            out.append(blank(text[start:end]))
        elif token == "/*":
            end = block_comment_end(text, start)
            out.append(blank(text[start:end]))
        elif token.endswith('"'):
            end = string_end(text, start, token)
            out.append('"' + blank(text[start + 1 : end - 1]) + '"')
            strings[start] = end
        else:
            body = CHAR_BODY.match(text, found.end())
            if body is None:  # a lifetime or a label: no literal opens here
                out.append(token)
                at = found.end()
                continue
            end = body.end()
            out.append("'" + blank(text[start + 1 : end - 1]) + "'")
        at = end
    return "".join(out), strings


def block_comment_end(text, start):
    """The end of the block comment that opens at `start`. Block comments nest (the Rust
    Reference's BLOCK_COMMENT), so each `/*` inside one needs its own `*/`."""
    depth, at = 0, start
    while at < len(text):
        if text.startswith("/*", at):
            depth, at = depth + 1, at + 2
        elif text.startswith("*/", at):
            depth, at = depth - 1, at + 2
            if depth == 0:
                return at
        else:
            at += 1
    return len(text)


def string_end(text, start, token):
    """The end of the string literal `token` opens at `start`: a raw string ends at a quote and as
    many `#` as it opened with, any other string at its first quote no backslash escapes."""
    opened = token.rstrip('#"')
    after = start + len(token)
    if opened.endswith("r"):
        hashes = len(token) - len(opened) - 1
        close = text.find('"' + "#" * hashes, after)
        return len(text) if close < 0 else close + 1 + hashes
    body = STRING_BODY.match(text, after)
    return len(text) if body is None else body.end()


CFG_TEST = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
#: A file whose leading inner attributes include `#![cfg(test)]` is test code from its first line.
TEST_FILE = re.compile(r"\A(?:\s|#\s*!\s*\[[^\]]*\])*#\s*!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
#: The words an item may open with, after any attributes; anything else under an attribute is a
#: field, an arm or a statement, which ends at its comma.
ITEM_WORDS = frozenset(
    {
        "async",
        "const",
        "default",
        "enum",
        "extern",
        "fn",
        "impl",
        "macro_rules",
        "mod",
        "pub",
        "static",
        "struct",
        "trait",
        "type",
        "union",
        "unsafe",
        "use",
    }
)


def bracket_pairs(masked):
    """Each opening bracket's position mapped to its closing bracket's, in masked text."""
    pairs, stack = {}, []
    for at, char in enumerate(masked):
        if char in "([{":
            stack.append(at)
        elif char in ")]}" and stack:
            pairs[stack.pop()] = at
    return pairs


def skip_attributes(masked, pairs, at):
    """The position after the blanks and outer attributes from `at`."""
    while True:
        at = skip_blank(masked, at)
        if not masked.startswith("#", at):
            return at
        open_at = skip_blank(masked, at + 1)
        if not masked.startswith("[", open_at) or open_at not in pairs:
            return at
        at = pairs[open_at] + 1


def item_end(masked, pairs, at):
    """Where the item an attribute ending at `at` stands on ends: an item at its block's close or
    its `;`; a field, an arm or a statement at its `,` or `;`, or where the list holding it
    closes."""
    at = skip_attributes(masked, pairs, at)
    word = WORD.match(masked, at)
    item = word is not None and word.group() in ITEM_WORDS
    end = len(masked)
    while at < end:
        char = masked[at]
        if char in "([":
            at = pairs.get(at, end - 1) + 1
            continue
        if char == "{":
            close = pairs.get(at)
            if close is None:
                return end
            if item:
                return close + 1
            at = close + 1
            continue
        if char == ";":
            return at + 1
        if char in ")]}" or (char == "," and not item):
            return at
        at += 1
    return end


def blank_test_items(masked):
    """`masked` with every item under `#[cfg(test)]` blanked, its attribute included (R6). A file
    whose leading inner attribute is `#![cfg(test)]` is blanked whole."""
    if TEST_FILE.match(masked):
        return blank(masked)
    pairs = bracket_pairs(masked)
    spans = []
    for found in CFG_TEST.finditer(masked):
        if spans and found.start() < spans[-1][1]:
            continue  # inside an item already blanked
        spans.append((found.start(), item_end(masked, pairs, found.end())))
    out, at = [], 0
    for start, end in spans:
        out.append(masked[at:start])
        out.append(blank(masked[start:end]))
        at = end
    out.append(masked[at:])
    return "".join(out)


def masked_source(text):
    """R6's masked product source: comments and literals masked, then `#[cfg(test)]` items."""
    masked, strings = mask(text)
    return blank_test_items(masked), strings


# --------------------------------------------------------------------- items and matches

ENUM = re.compile(r"\benum\s+([A-Za-z_]\w*)")
IMPL = re.compile(r"\bimpl\b")
FN = re.compile(r"\bfn\s+([A-Za-z_]\w*)")
MATCH = re.compile(r"\bmatch\b")
MATCHES = re.compile(r"\bmatches\s*!\s*\(")
TRAIT_FOR = re.compile(r"\bfor\b(?!\s*<)")
WHERE = re.compile(r"\bwhere\b")
IDENT = re.compile(r"[A-Za-z_]\w*")
ATTRIBUTES = re.compile(r"\A(?:#\s*\[[^\]]*\]\s*)*")
#: A value that is an expression with a block: its comma is optional (the Rust Reference's
#: MatchArms), so it ends where its last block closes.
BLOCK_LIKE = re.compile(r"(?:'\w+\s*:\s*)?(?:(if|match|loop|while|for|unsafe|async)\b|\{)")
ELSE = re.compile(r"else\b")
IF = re.compile(r"if\b")


def header_end(masked, pairs, at):
    """The first `{` from `at` outside any parenthesis or bracket, or None where a `;` comes
    first: the block a header (an item's, a match's) opens."""
    end = len(masked)
    while at < end:
        char = masked[at]
        if char in "([":
            at = pairs.get(at, end - 1) + 1
        elif char == "{":
            return at if at in pairs else None
        elif char == ";":
            return None
        else:
            at += 1
    return None


def split_top(masked, pairs, start, end):
    """[(start, end)] of the comma-separated parts from `start` to `end`, outside any bracket."""
    parts, at, begin = [], start, start
    while at < end:
        char = masked[at]
        if char in "([{":
            at = pairs.get(at, end - 1) + 1
            continue
        if char == ",":
            parts.append((begin, at))
            begin = at + 1
        at += 1
    if masked[begin:end].strip():
        parts.append((begin, end))
    return parts


def item_start(masked, at):
    """Whether the keyword at `at` opens an item: only blanks, `unsafe` or `default` stand
    between it and the previous item's end or its block's start. An `impl` in a type does not."""
    before = masked[:at].rstrip()
    for word in ("default", "unsafe"):
        if before.endswith(word) and not before[: -len(word)][-1:].isalnum():
            before = before[: -len(word)].rstrip()
    return not before or before[-1] in ";{}]"


def strip_generics(header):
    """`header` without the generic parameters it opens with."""
    text = header.lstrip()
    if not text.startswith("<"):
        return text
    depth = 0
    for at, char in enumerate(text):
        if char == "<":
            depth += 1
        elif char == ">" and text[at - 1] != "-":
            depth -= 1
            if depth == 0:
                return text[at + 1 :]
    return ""


def impl_type(header):
    """The type an `impl` block's header names (R6): the one after `for` in
    `impl <Trait> for <Type>`, else the one `impl <Type>` names, its path and generics aside."""
    header = WHERE.split(strip_generics(header), maxsplit=1)[0]
    split = TRAIT_FOR.split(header, maxsplit=1)
    if len(split) == 2:
        target = split[1]
    else:
        target = split[0]
    names = IDENT.findall(target.split("<")[0])
    return names[-1] if names else None


def enclosing(spans, at):
    """The name of the innermost (open, close, name) span holding position `at`, or None."""
    best = None
    for open_at, close, name in spans:
        if open_at < at < close and (best is None or open_at > best[0]):
            best = (open_at, name)
    return None if best is None else best[1]


def fns_in(masked, pairs):
    """[(open, close, name)] of each fn's body."""
    found = []
    for item in FN.finditer(masked):
        brace = header_end(masked, pairs, item.end())
        if brace is not None:
            found.append((brace, pairs[brace], item.group(1)))
    return found


def impls_in(masked, pairs):
    """[(open, close, type)] of each `impl` block, by the type it implements for (R6)."""
    found = []
    for item in IMPL.finditer(masked):
        if not item_start(masked, item.start()):
            continue
        brace = header_end(masked, pairs, item.end())
        if brace is None:
            continue
        name = impl_type(masked[item.end() : brace])
        if name is not None:
            found.append((brace, pairs[brace], name))
    return found


def enums_in(path, masked, pairs):
    """Every enum `masked` declares, with its variants."""
    found = []
    for item in ENUM.finditer(masked):
        brace = header_end(masked, pairs, item.end())
        if brace is None:
            continue
        variants = []
        for start, end in split_top(masked, pairs, brace + 1, pairs[brace]):
            part = masked[start:end].strip()
            part = part[ATTRIBUTES.match(part).end() :]
            name = IDENT.match(part)
            if name is not None:
                variants.append(name.group())
        found.append(Enum(path, item.group(1), line_of(masked, item.start()), tuple(variants)))
    return found


def matches_in(masked, pairs):
    """Every match in `masked`, each with its arms. A `matches!` is read as the match it expands
    to: one arm for its pattern, then the `_` arm its expansion adds, marked `macro` (R7)."""
    found = []
    for item in MATCH.finditer(masked):
        brace = header_end(masked, pairs, item.end())
        if brace is not None:
            arms = arms_in(masked, pairs, brace + 1, pairs[brace])
            found.append(Match(item.start(), tuple(arms), False))
    for item in MATCHES.finditer(masked):
        open_at = item.end() - 1
        close = pairs.get(open_at)
        if close is None:
            continue
        parts = split_top(masked, pairs, open_at + 1, close)
        if len(parts) < 2:
            continue
        start, end = parts[1]
        arms = (
            Arm(masked[start:end].strip(), start, close, close),
            Arm("_", close, close, close),
        )
        found.append(Match(item.start(), arms, True))
    return sorted(found, key=lambda match: match.at)


def authored(matches):
    """The matches an author wrote: a `matches!` names the variants it wants by design (R7)."""
    return [match for match in matches if not match.macro]


def arms_in(masked, pairs, start, end):
    """The arms of the match block from `start` to `end`: each a pattern with its guard, an
    arrow, and a value."""
    arms, at = [], start
    while True:
        at = skip_attributes(masked, pairs, at)
        if at >= end:
            break
        arrow = arrow_at(masked, pairs, at, end)
        if arrow is None:
            break
        value = skip_blank(masked, arrow + 2)
        value_end = expression_end(masked, pairs, value, end)
        arms.append(Arm(masked[at:arrow].strip(), at, value, value_end))
        if value_end <= at:
            break
        at = skip_blank(masked, value_end)
        if at < end and masked[at] == ",":
            at += 1
    return arms


def arrow_at(masked, pairs, at, end):
    """The first `=>` from `at` outside any bracket, before `end`, or None."""
    while at < end:
        char = masked[at]
        if char in "([{":
            at = pairs.get(at, end - 1) + 1
        elif masked.startswith("=>", at):
            return at
        else:
            at += 1
    return None


def expression_end(masked, pairs, at, end):
    """Where the arm value that starts at `at` ends: a value with a block where its last block
    closes, unless a method call or `?` continues it, and any other value at its first comma
    outside a bracket."""
    head = BLOCK_LIKE.match(masked, at)
    if head is not None:
        close = block_like_end(masked, pairs, at, head)
        if close is not None and close <= end:
            after = skip_blank(masked, close)
            if after >= end or masked[after] not in ".?":
                return close
            at = after
    while at < end:
        char = masked[at]
        if char in "([{":
            at = pairs.get(at, end - 1) + 1
        elif char == ",":
            return at
        else:
            at += 1
    return end


def block_like_end(masked, pairs, at, head):
    """The end of the expression with a block that `head` opens at `at`, or None."""
    if head.group(1) is None:
        brace = head.end() - 1
        return pairs[brace] + 1 if brace in pairs else None
    brace = header_end(masked, pairs, head.end())
    if brace is None:
        return None
    close = pairs[brace] + 1
    while head.group(1) == "if":
        after = skip_blank(masked, close)
        if not ELSE.match(masked, after):
            break
        after = skip_blank(masked, after + 4)
        if IF.match(masked, after):
            brace = header_end(masked, pairs, after + 2)
        elif masked.startswith("{", after) and after in pairs:
            brace = after
        else:
            break
        if brace is None:
            break
        close = pairs[brace] + 1
        if brace == after:
            break
    return close


def read_source(root, path):
    """One product file read and masked, with its fns, impls, matches and enums."""
    text = path.read_text(encoding="utf-8")
    masked, strings = masked_source(text)
    pairs = bracket_pairs(masked)
    where = relative(root, path)
    return Source(
        where,
        text,
        masked,
        strings,
        fns_in(masked, pairs),
        impls_in(masked, pairs),
        matches_in(masked, pairs),
        enums_in(where, masked, pairs),
    )


def product_sources(root):
    return [read_source(root, path) for path in product_files(root)]


def state_enums(sources):
    """R6: every enum declared in product source whose name ends in a state suffix."""
    return [
        enum for source in sources for enum in source.enums if enum.name.endswith(ENUM_SUFFIXES)
    ]


# ------------------------------------------------------------------------------ FSM-WILDCARD

GUARD = re.compile(r"\bif\b")
VARIANT_PATH = re.compile(r"\b([A-Z]\w*|Self)\s*::\s*([A-Z]\w*)")
WILDCARD_PATTERN = re.compile(r"_")
BINDING = re.compile(r"(?:ref\s+)?(?:mut\s+)?(?!(?:true|false)\b)(?:[a-z]\w*|_\w+)")
AT_WILDCARD = re.compile(r"(?:ref\s+)?(?:mut\s+)?(?:[a-z]\w*|_\w+)\s*@\s*_")


def catch_all(pattern):
    """Whether an arm's whole top-level pattern, its guard aside, matches every value: `_`, a
    lone lower-case binding (with `ref`, `mut` or neither) or `<binding> @ _` (R7). A `_` in a
    tuple or a wrapper is not one."""
    head = GUARD.split(pattern, maxsplit=1)[0].strip()
    return bool(
        WILDCARD_PATTERN.fullmatch(head) or BINDING.fullmatch(head) or AT_WILDCARD.fullmatch(head)
    )


def resolve(enums, name, variant, path):
    """The state enum named `name` that has `variant`, the one in `path` first: two state enums
    may share a name and are told apart by their variants (ADR-447 D2)."""
    found = [enum for enum in enums if enum.name == name and variant in enum.variants]
    same = [enum for enum in found if enum.path == path]
    return (same or found or [None])[0]


def state_enum_of(pattern, impl_name, path, enums):
    """The state enum an arm's pattern names a variant of, by `<Enum>::<Variant>` anywhere in it
    or by `Self::<Variant>` inside an impl of the enum (R6), or None."""
    head = GUARD.split(pattern, maxsplit=1)[0]
    for qualifier, variant in VARIANT_PATH.findall(head):
        name = impl_name if qualifier == "Self" else qualifier
        enum = None if name is None else resolve(enums, name, variant, path)
        if enum is not None:
            return enum
    return None


def wildcard(root, rule, seed):
    """FSM-WILDCARD (R6, R7): each catch-all arm of a match on a state enum, in product source."""
    sources = product_sources(root)
    reading = Reading(WILDCARD, rule["level"], "matches", len(sources))
    listed = admit(root, WILDCARD, rule.get("listed", []), seed)
    enums = state_enums(sources)
    seen = set()
    for source in sources:
        for match in authored(source.matches):
            impl_name = enclosing(source.impls, match.at)
            enum = None
            for arm in match.arms:
                enum = state_enum_of(arm.pattern, impl_name, source.path, enums)
                if enum is not None:
                    break
            if enum is None:
                continue
            fn = enclosing(source.fns, match.at) or ""
            reading.matches.append((source.path, line_of(source.text, match.at), enum.name, fn))
            for arm in match.arms:
                if not catch_all(arm.pattern):
                    continue
                key = (source.path, enum.name, fn)
                seen.add(key)
                site_kind = "listed" if key in listed else "new"
                line = line_of(source.text, arm.start)
                reading.hits.append(
                    Hit(
                        WILDCARD,
                        site_kind,
                        source.path,
                        line,
                        ("enum=" + enum.name, "fn=" + fn),
                    )
                )
    for path, name, fn in sorted(listed - seen):
        reading.hits.append(Hit(WILDCARD, "stale", path, 0, ("enum=" + name, "fn=" + fn)))
    reading.examined = len(reading.matches)
    return finish(reading)


# --------------------------------------------------------------------------- FSM-UNREACHABLE

PANICKING = re.compile(r"\b(?:(?:std|core)\s*::\s*)?(" + "|".join(MACROS) + r")\s*!")


def value_head(masked, arm):
    """Where an arm's value starts, or its block's first statement when the value is a block."""
    at = arm.value
    if masked.startswith("{", at):
        at = skip_blank(masked, at + 1)
    return at


def unreachable(root, rule, seed):
    """FSM-UNREACHABLE (R8): each arm whose value calls a panicking macro, bare or as its block's
    first statement, in product source."""
    sources = product_sources(root)
    reading = Reading(UNREACHABLE, rule["level"], "arms", len(sources))
    listed = admit(root, UNREACHABLE, rule.get("listed", []), seed)
    seen = set()
    for source in sources:
        heads = {}
        for match in authored(source.matches):
            for arm in match.arms:
                reading.examined += 1
                heads[value_head(source.masked, arm)] = arm
        for found in PANICKING.finditer(source.masked):
            arm = heads.get(found.start())
            if arm is None:
                continue
            fn = enclosing(source.fns, found.start()) or ""
            key = (source.path, fn, found.group(1))
            seen.add(key)
            arm_kind = "listed" if key in listed else "new"
            line = line_of(source.text, found.start())
            reading.hits.append(
                Hit(
                    UNREACHABLE,
                    arm_kind,
                    source.path,
                    line,
                    ("fn=" + fn, "macro=" + key[2]),
                )
            )
    for path, fn, macro in sorted(listed - seen):
        reading.hits.append(Hit(UNREACHABLE, "stale", path, 0, ("fn=" + fn, "macro=" + macro)))
    return finish(reading)


# ----------------------------------------------------------------------------- FSM-UNCOVERED

COVER = re.compile(r"@phx covers\s+(\S+)\s+anchor=(\S+)")


def covered_by(covers, path, name):
    """Whether a cover line names the enum's own file at the enum's own anchor (R9)."""
    return any(
        cover_path == path and anchor == name for _module, _line, cover_path, anchor in covers
    )


def uncovered(root, rule, seed):
    """FSM-UNCOVERED (R9): each product state enum no formal entry anchors."""
    sources = product_sources(root)
    formal_modules = formal_files(root)
    reading = Reading(UNCOVERED, rule["level"], "enums", len(formal_modules))
    listed = admit(root, UNCOVERED, rule.get("listed", []), seed)
    for module in formal_modules:
        for number, text in enumerate(module.read_text(encoding="utf-8").splitlines(), 1):
            found = COVER.search(text)
            if found is not None:
                reading.covers.append(
                    (relative(root, module), number, found.group(1), found.group(2))
                )
    present = set()
    for enum in state_enums(sources):
        key = (enum.path, enum.name)
        present.add(key)
        reading.enums.append(key)
        covered = covered_by(reading.covers, enum.path, enum.name)
        if key in listed:
            enum_kind = "stale" if covered else "listed"
        elif covered:
            continue
        else:
            enum_kind = "new"
        reading.hits.append(Hit(UNCOVERED, enum_kind, enum.path, enum.line, ("enum=" + enum.name,)))
    for path, name in sorted(listed - present):
        reading.hits.append(Hit(UNCOVERED, "stale", path, 0, ("enum=" + name,)))
    reading.examined = len(reading.enums)
    return finish(reading)


# ---------------------------------------------------------------------------- FSM-CHECK-LIST

#: SQL's lexemes, the literal first: a `--` or `/* */` comment is masked, a single-quoted literal
#: is one spelling with each doubled `''` one quote, and a quoted name is a name (R3).
SQL_LEXEME = re.compile(
    r"(?P<text>'(?:[^']|'')*'?)"
    r"|(?P<line>--[^\n]*)"
    r"|(?P<block>/\*.*?(?:\*/|\Z))"
    r'|(?P<quoted>"(?:[^"]|"")*"?|`[^`]*`?|\[[^\]]*\]?)'
    r"|(?P<word>[A-Za-z_]\w*)"
    r"|(?P<number>-?\d+(?:\.\d+)?)"
    r"|(?P<mark>\S)",
    re.S,
)
INNER_CAPITAL = re.compile(r"(?<!^)(?=[A-Z])")
STATE_ENDINGS = tuple("_" + name for name in STATE_NAMES)
ESCAPE = re.compile(r"\\(?:x([0-9A-Fa-f]{2})|u\{([0-9A-Fa-f_]+)\}|\n\s*|(.))", re.S)
ESCAPED = {"n": "\n", "r": "\r", "t": "\t", "0": "\0", "\\": "\\", "'": "'", '"': '"'}


def sql_tokens(sql):
    """[(kind, text, at)] of `sql`'s tokens, its comments dropped: a literal's text is its
    spelling, and a quoted name's text is the name."""
    tokens = []
    for found in SQL_LEXEME.finditer(sql):
        kind, text = found.lastgroup, found.group()
        if kind in ("line", "block"):
            continue
        if kind == "text":
            text = text[1:-1].replace("''", "'")
        elif kind == "quoted":
            kind, text = "word", text[1:-1]
        tokens.append((kind, text, found.start()))
    return tokens


def closing_paren(tokens, open_at):
    depth = 0
    for at in range(open_at, len(tokens)):
        if tokens[at][:2] == ("mark", "("):
            depth += 1
        elif tokens[at][:2] == ("mark", ")"):
            depth -= 1
            if depth == 0:
                return at
    return None


def whole_body(body):
    """(column, kind, literals, at) when a CHECK's whole body is `<column> IN (<literal>, ...)`,
    else None: a CHECK that joins an IN list to anything else is a condition, not a domain (R3)."""
    keyword = next(
        (at for at, token in enumerate(body) if token[0] == "word" and token[1].upper() == "IN"),
        None,
    )
    if keyword != 1:
        return None
    column = body[keyword - 1]
    if column[0] != "word" or keyword + 1 >= len(body) or body[keyword + 1][:2] != ("mark", "("):
        return None
    listed_values, kinds, at = [], set(), keyword + 2
    while at < len(body) and body[at][0] in ("text", "number"):
        token_text = body[at][1]
        listed_values.append(token_text)
        kinds.add(body[at][0])
        at += 1
        if at < len(body) and body[at][:2] == ("mark", ","):
            at += 1
            continue
        break
    if not listed_values or at >= len(body) or body[at][:2] != ("mark", ")"):
        return None
    if at != len(body) - 1:
        return None
    kind = "text" if kinds == {"text"} else "integer" if kinds == {"number"} else "mixed"
    return column[1].lower(), kind, tuple(listed_values), column[2]


def domain_at(tokens, at):
    """The domain of the CHECK whose keyword is token `at`, or None."""
    if tokens[at][0] != "word" or tokens[at][1].upper() != "CHECK":
        return None
    if at + 1 >= len(tokens) or tokens[at + 1][:2] != ("mark", "("):
        return None
    close = closing_paren(tokens, at + 1)
    return None if close is None else whole_body(tokens[at + 2 : close])


def domains(sql):
    """[(column, kind, literals, at)]: every whole-body column domain in `sql` (R3)."""
    tokens = sql_tokens(sql)
    found = []
    for at in range(len(tokens)):
        domain = domain_at(tokens, at)
        if domain is not None:
            found.append(domain)
    return found


def table_name_at(tokens, at):
    """The index of the table name a `CREATE TABLE` or `ALTER TABLE` at token `at` names."""
    words = [token[1].upper() if token[0] == "word" else None for token in tokens[at : at + 8]]
    step = 1
    while step < len(words) and words[step] in ("TEMP", "TEMPORARY"):
        step += 1
    if step >= len(words) or words[step] != "TABLE":
        return None
    step += 1
    if words[step : step + 3] == ["IF", "NOT", "EXISTS"]:
        step += 3
    name = at + step
    if name + 2 < len(tokens) and tokens[name + 1][:2] == ("mark", "."):
        name += 2
    return name if name < len(tokens) and tokens[name][0] == "word" else None


def written(text):
    """[(table, column, literals, line)]: each domain a migration's own text writes, under the
    table its CREATE TABLE or ALTER TABLE names, a rename in the same file followed."""
    tokens = sql_tokens(text)
    renamed, found, table = {}, [], None
    for at, token in enumerate(tokens):
        word = token[1].upper() if token[0] == "word" else None
        if word in ("CREATE", "ALTER"):
            name = table_name_at(tokens, at)
            if name is not None:
                table = tokens[name][1].lower()
                after = [each[1].upper() for each in tokens[name + 1 : name + 3]]
                if word == "ALTER" and after == ["RENAME", "TO"] and name + 3 < len(tokens):
                    renamed[table] = tokens[name + 3][1].lower()
        domain = domain_at(tokens, at)
        if domain is not None:
            found.append((table, domain[0], domain[2], line_of(text, domain[3])))
    resolved = []
    for table, column, literals, line in found:
        hops = 0
        while table in renamed and hops < len(renamed):
            table, hops = renamed[table], hops + 1
        resolved.append((table, column, literals, line))
    return resolved


def column_site(writes, table, column, literals):
    """(path, line) of the last migration whose own text writes the column's list, the write
    under the same table first."""
    for path, found in reversed(writes):
        same = [each for each in found if each[1:3] == (column, literals)]
        if same:
            exact = [each for each in same if each[0] == table]
            return path, (exact or same)[0][3]
    return "migrations", 0


def strict_supported():
    """Whether this SQLite creates a STRICT table, which an older one refuses as an error."""
    probe = sqlite3.connect(":memory:")
    try:
        probe.execute(STRICT_PROBE)
    except sqlite3.Error:
        return False
    finally:
        probe.close()
    return True


def final_schema(files):
    """{table: its final CREATE text}: every migration executed in filename order into one
    in-memory SQLite database, then `sqlite_master` read, so SQLite itself applies every rebuild,
    rename and added column (R3)."""
    if sqlite3 is None:
        raise Void("this Python has no sqlite3, so the migrations' schema cannot be built")
    if not strict_supported():
        raise Void("this SQLite cannot create a STRICT table, so it cannot build the schema")
    database = sqlite3.connect(":memory:")
    try:
        for path in files:
            try:
                database.executescript(path.read_text(encoding="utf-8"))
            except sqlite3.Error as error:
                raise Void(path.name + " does not execute: " + str(error)) from None
        rows = database.execute(
            "SELECT name, sql FROM sqlite_master WHERE type = 'table'"
        ).fetchall()
    finally:
        database.close()
    return {name.lower(): sql or "" for name, sql in rows}


def state_column(column, kind):
    """R5: a text domain column whose name is, or ends in `_` and, one of STATE_NAMES. An integer
    list is a flag, never a state column."""
    name = column.lower()
    return kind == "text" and (name in STATE_NAMES or name.endswith(STATE_ENDINGS))


def snake_case(name):
    """`NoNewCards` as `no_new_cards`: lower case, an underscore before each inner capital (R4)."""
    return INNER_CAPITAL.sub("_", name).lower()


def literal_text(source, arm):
    """The text of the string literal that is an arm's whole value, or None."""
    value = source.masked[arm.value : arm.end].rstrip()
    end = source.strings.get(arm.value)
    if end is None or end != arm.value + len(value):
        return None
    raw = source.text[arm.value : end]
    opened = raw[: raw.index('"')]
    prefix = opened.rstrip("#")
    if prefix.endswith("r"):
        hashes = len(opened) - len(prefix)
        return raw[len(opened) + 1 : len(raw) - 1 - hashes]
    return ESCAPE.sub(unescape, raw[len(opened) + 1 : -1])


def unescape(found):
    if found.group(1) is not None:
        return chr(int(found.group(1), 16))
    if found.group(2) is not None:
        return chr(int(found.group(2).replace("_", ""), 16))
    if found.group(3) is None:
        return ""
    return ESCAPED.get(found.group(3), found.group(3))


def void(why):
    raise Void(why)


def spelled(source, spelled_enum, pair):
    """The spellings a pair's enum stores (R4): the literal each arm `Self::<Variant> => "<text>"`
    of the pair's fn in the enum's impl gives, or each variant's name in snake case, less every
    variant the pair names `unstored`."""
    unstored = {each["variant"] for each in pair.get("unstored", [])}
    variants = [variant for variant in spelled_enum.variants if variant not in unstored]
    if pair["spelling"] == "snake_case":
        return {snake_case(variant) for variant in variants}
    name = pair["spelling"]["fn"]
    impls = [(start, end) for start, end, owner in source.impls if owner == spelled_enum.name]
    bodies = [
        (start, end)
        for start, end, fn in source.fns
        if fn == name and any(low < start < high for low, high in impls)
    ]
    if not bodies:
        void(pair["file"] + " has no fn " + name + " in an impl of " + spelled_enum.name)
    spellings = set()
    for match in authored(source.matches):
        if not any(start < match.at < end for start, end in bodies):
            continue
        for arm in match.arms:
            named = [
                variant
                for qualifier, variant in VARIANT_PATH.findall(arm.pattern)
                if qualifier in ("Self", spelled_enum.name)
            ]
            if not named:
                continue
            literal = literal_text(source, arm)
            if literal is None:
                void(
                    pair["file"]
                    + ": an arm of "
                    + name
                    + " spells no literal, at line "
                    + str(line_of(source.text, arm.start))
                )
            for variant in named:
                if variant in variants:
                    spellings.add(literal)
    if not spellings:
        void(pair["file"] + ": " + name + " spells no variant of " + spelled_enum.name)
    return spellings


def check_list(root, rule, seed):
    """FSM-CHECK-LIST (R3 to R5): each pair's spellings against its column's CHECK list, and each
    state column neither paired nor listed, over the schema SQLite builds."""
    files = migration_files(root)
    reading = Reading(CHECK_LIST, rule["level"], "columns", len(files))
    unpaired = admit(root, CHECK_LIST, rule.get("unpaired", []), seed)
    tables = final_schema(files)
    writes = [(relative(root, path), written(path.read_text(encoding="utf-8"))) for path in files]
    for table in sorted(tables):
        for column, kind, literals, _at in domains(tables[table]):
            reading.columns[(table, column)] = (kind, literals)
    reading.examined = len(reading.columns)
    paired = set()
    for pair in rule.get("pairs", []):
        table, column = pair["table"].lower(), pair["column"].lower()
        if table not in tables:
            void(
                CHECK_LIST + " pairs " + table + "." + column + ", and no migration makes " + table
            )
        target = Path(root) / pair["file"]
        if not target.is_file():
            void(
                CHECK_LIST + " pairs " + table + "." + column + " with " + pair["file"] + ", absent"
            )
        source = read_source(root, target)
        paired_enum = next((each for each in source.enums if each.name == pair["enum"]), None)
        if paired_enum is None:
            void(pair["file"] + " declares no enum " + pair["enum"])
        spellings = spelled(source, paired_enum, pair)
        stored_domain = reading.columns.get((table, column))
        if stored_domain is None:
            void(CHECK_LIST + " pairs " + table + "." + column + ", which has no CHECK list")
        stored = set(stored_domain[1])
        lacks = sorted(spellings - stored)
        unspelled = sorted(stored - spellings)
        reading.pairs.append((table, column))
        paired.add((table, column))
        if lacks or unspelled:
            key = ("column=" + table + "." + column,)
            if lacks:
                key += ("lacks=" + ",".join(lacks),)
            if unspelled:
                key += ("unspelled=" + ",".join(unspelled),)
            path, line = column_site(writes, table, column, stored_domain[1])
            reading.hits.append(Hit(CHECK_LIST, "new", path, line, key))
    states = set()
    for (table, column), (kind, literals) in sorted(reading.columns.items()):
        if not state_column(column, kind):
            continue
        states.add((table, column))
        if (table, column) in paired:
            continue
        state_kind = "listed" if (table, column) in unpaired else "new"
        path, line = column_site(writes, table, column, literals)
        reading.hits.append(
            Hit(CHECK_LIST, state_kind, path, line, ("column=" + table + "." + column,))
        )
    for table, column in sorted(unpaired):
        if (table, column) in paired or (table, column) not in states:
            reading.hits.append(
                Hit(
                    CHECK_LIST,
                    "stale",
                    "migrations",
                    0,
                    ("column=" + table + "." + column,),
                )
            )
    return finish(reading)


# ---------------------------------------------------------------------- levels and lines


def finish(reading):
    """The reading, its hits in site order, refused as VOID when it examined nothing (R11)."""
    reading.hits.sort(key=lambda hit: (hit.path, hit.line, hit.key))
    if reading.examined == 0 or reading.files == 0:
        raise Void(
            reading.rule
            + " examined 0 "
            + reading.unit
            + " over "
            + str(reading.files)
            + " file(s), so it judged nothing"
        )
    return reading


def lines(reading):
    """R10: the reading's summary line, then one line per hit naming its site and key; no line
    carries the text of the line it names."""
    count = {kind: 0 for kind in KINDS}
    for hit in reading.hits:
        count[hit.kind] += 1
    summary = [
        "FSM-LINT",
        reading.rule,
        "level=" + reading.level,
        "examined=" + str(reading.examined),
        reading.unit,
        "hits=" + str(len(reading.hits)),
        "new=" + str(count["new"]),
        "listed=" + str(count["listed"]),
        "stale=" + str(count["stale"]),
    ]
    printed = [" ".join(summary)]
    for hit in reading.hits:
        site = hit.path + ":" + str(hit.line)
        printed.append(" ".join(["FSM-LINT", hit.rule, reading.level, hit.kind, site, *hit.key]))
    return printed


def report(reading):
    """Print the reading's lines and apply its level (R10): at warn nothing a hit says fails; at
    fail a `new` or `stale` hit fails the test. The printed lines are returned."""
    printed = lines(reading)
    for line in printed:
        print(line)
    failing = [hit for hit in reading.hits if hit.kind in ("new", "stale")]
    if reading.level == "fail" and failing:
        raise AssertionError(
            reading.rule + " is at fail and names " + str(len(failing)) + " new or stale hit(s)"
        )
    return printed
