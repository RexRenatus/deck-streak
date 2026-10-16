"""One guard holds four Rust house rules at zero over the shipped sources (SPEC-427, ADR-441).

Four rules of the house hold in the shipped Rust sources, and this module refuses the first break
of each, naming the file, the line and the rule:

- `sqlite-one-place` (R4): SQLite is opened only in `crates/kernel/src/db.rs`, the kernel's
  database base (SPEC-020 R17).
- `no-format-into-query` (R5): no `format!` reaches a query, directly, through `.leak()`, or
  through a name bound to one earlier in the same function.
- `no-secret-in-log` (R6, R7): no secret-named name is passed to a `tracing` or `log` macro.
- `randomness-through-getrandom` (R8, R9): randomness is drawn only through `getrandom`.

The population (R2) is every `crates/*/src/**/*.rs` file, sorted, less each file an out-of-line
`#[cfg(test)] mod` declares, its `#[path]` honoured, and less the span of every `#[cfg(test)]`
item. The lexer (R3) blanks comments and the contents of literals and keeps every offset and line,
so a rule reads only code and a refused site's line is the source's own. No site is admitted by
name and no file is left out by name.

Each rule returns one line per refused site, `<rule>: <path>:<line>: <what>`, with what it
examined, and fails when it examined nothing (R10). A1 to A7 plant fixture trees in a temporary
directory at test time; A8 reads the repository's own tree and prints each rule's count through
`_support.examined`. The walk reads every shipped source, so the verdict is read in CI.
"""

import re
import tempfile
import tomllib
import unittest
from pathlib import Path

from _support import REPO, examined

# R2: the shipped sources under each crate; R4: the one file that opens SQLite.
SOURCES = "*/src/**/*.rs"
DATABASE_BASE = "crates/kernel/src/db.rs"

# R3: where the lexer has work (a comment's opening, a raw string's opening, a quote), a nested
# comment's marks, a character literal, and the body of a string that escapes its quotes.
SPECIAL = re.compile(r"""/[/*]|(?<!\w)[bc]?r#*"|["']""")
NESTING = re.compile(r"/\*|\*/")
CHARACTER = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
STRING_END = re.compile(r'(?:[^"\\]|\\.)*', re.S)

# Brackets, and the marks that end an item, in code whose comments and literals are blanked.
BRACKET = re.compile(r"[()\[\]{}]")
ITEM = re.compile(r"[()\[\]{};]")
SEPARATOR = re.compile(r"[()\[\]{},;]")
EQUALS = re.compile(r"[()\[\]{}]|(?<![=!<>])=(?![=>])")

# R2: a `#[cfg(test)]` attribute, an outer attribute after it, an out-of-line module, a path.
TEST_ONLY = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
FURTHER = re.compile(r"\s*(#)\s*\[")
OUT_OF_LINE = re.compile(r"\s*(?:pub(?:\s*\([^()]*\))?\s+)?mod\s+(\w+)\s*;")
PATH_ATTRIBUTE = re.compile(r'#\s*\[\s*path\s*=\s*"')

# A `use` item and the names it binds, an alias in place of what it renames; a function.
USE = re.compile(r"\buse\s+(?:::\s*)?(\w+)\s*::\s*([^;]*);")
BOUND = re.compile(r"\b(\w+)\b(?:\s+as\s+(\w+))?(?!\s*::)")
FUNCTION = re.compile(r"\bfn\s+\w+")


def either(names):
    """A pattern for any one of `names`, as a group that captures nothing."""
    return "(?:" + "|".join(names) + ")"


# R4: the names that open SQLite, the receivers and calls that connect, and the method arm.
OPENERS = (
    "SqliteConnectOptions",
    "SqlitePoolOptions",
    "PoolOptions",
    "rusqlite",
    "libsqlite3_sys",
)
RECEIVERS = (
    "SqlitePool",
    "Pool",
    "SqliteConnection",
    "Connection",
)
CONNECTS = (
    "connect",
    "connect_with",
    "connect_lazy",
    "connect_lazy_with",
)
RECEIVED = r"\s*(?:(?:::\s*)?<[^<>;{}]*>\s*)?>?\s*::\s*"
OPENER = re.compile(r"\b" + either(OPENERS) + r"\b")
CONNECTED = re.compile(r"\b" + either(RECEIVERS) + RECEIVED + either(CONNECTS) + r"\s*\(")
METHOD = re.compile(r"\.\s*(?:connect_with|connect_lazy|connect_lazy_with)\s*\(")
SQLITE_ARMS = (
    (OPENER, "names an opener:"),
    (CONNECTED, "connects:"),
    (METHOD, "calls"),
)

# R5: the query family, a call of one (a turbofish allowed), and what an argument may not hold.
FAMILY = (
    "query",
    "query_as",
    "query_scalar",
    "query_with",
    "query_as_with",
    "query_scalar_with",
    "raw_sql",
)
CALLED = r"\s*(?:::\s*<[^;{}]*?>)?\s*\("
QUERY = re.compile(r"\bsqlx\s*::\s*" + either(FAMILY) + CALLED)
ASSERTED = re.compile(r"\bAssertSqlSafe\s*\(")
FORMAT = re.compile(r"\bformat\s*!")
LEAK = re.compile(r"\.\s*leak\s*\(\s*\)")
LET_FORMAT = re.compile(r"\blet\s+(?:mut\s+)?(\w+)\b[^=;]*=\s*format\s*!")
SET_FORMAT = re.compile(r"(?:^|[;{}])\s*(\w+)\s*=\s*format\s*!", re.M)

# R6: a macro by a `tracing::` or `log::` path, an `#[instrument]` attribute and its lists, an
# argument that passes no name, a dotted name, a literal as blanked code, an inline capture.
LOGGED = re.compile(r"(?<![\w:.])(tracing|log)\s*::\s*(?:\w+\s*::\s*)*(\w+)\s*!\s*[(\[{]")
INSTRUMENT = re.compile(r"#\s*\[\s*(?:tracing\s*::\s*)?instrument\b")
SKIP = re.compile(r"\bskip(_all)?\b\s*(?:\(([^()]*)\))?")
FIELDS = re.compile(r"\bfields\s*\(")
SKIPPED = re.compile(r"(?:target|parent|name)\s*:(?!:)|(?:tracing\s*::\s*)?Level\s*::")
DOTTED = re.compile(r"[A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*")
LITERAL = re.compile(r'[bc]?r?#*"\s*"#*')
CAPTURE = re.compile(r"\{([A-Za-z_]\w*)(?=[:}])")
NAME = re.compile(r"[A-Za-z_]\w*")
TOKEN = re.compile(r"[A-Za-z_]\w*|\d\w*|::|\S")

# R7: the pairs, the parts and the about-names of ADR-441 D3.
PAIRS = (
    "api key",
    "private key",
    "signing key",
    "init data",
    "session id",
)
PARTS = (
    "token",
    "secret",
    "password",
    "credential",
    "cookie",
    "bearer",
    "authorization",
    "apikey",
    "initdata",
)
ABOUT = (
    "count",
    "len",
    "length",
    "size",
    "total",
    "name",
    "path",
    "file",
    "kind",
    "hash",
    "id",
    "at",
)

# R8, R9: the refused crates, the generators outside getrandom, and the arms that refuse them.
RANDOM_CRATES = (
    "rand",
    "rand_core",
    "rand_chacha",
    "fastrand",
)
GENERATORS = (
    "OsRng",
    "ThreadRng",
    "thread_rng",
    "StdRng",
    "SmallRng",
    "SeedableRng",
)
CRATE_PATH = re.compile(r"(?<!\w)(?<!\w::)(?<!\.)" + either(RANDOM_CRATES) + r"\s*::")
CRATE_USE = re.compile(r"\b(?:use|extern\s+crate)\s+(?:::\s*)?" + either(RANDOM_CRATES) + r"\b")
RING_RAND = re.compile(r"\bring\s*::\s*rand\b")
GENERATOR = re.compile(r"\b" + either(GENERATORS) + r"\b")
PY_RANDOM = re.compile(r"\bPyRandom\s*::\s*new\b")
UUID_RANDOM = re.compile(r"\bUuid\s*::\s*(?:new_v4|new_v7|now_v7)\b")
RANDOM_ARMS = (
    (CRATE_PATH, "a path through a rand-family crate"),
    (CRATE_USE, "imports a rand-family crate"),
    (RING_RAND, "ring::rand"),
    (GENERATOR, "a generator outside getrandom"),
    (PY_RANDOM, "PyRandom::new"),
)


def blank(text):
    """Text with every character but a line break made a space, so offsets and lines are kept."""
    return re.sub(r"[^\n]", " ", text)


def comment_end(text, start):
    """The offset past the block comment that opens at `start`, each comment nested in it closed
    first; the end of the text when it is never closed."""
    depth, at = 1, start + 2
    while depth and (mark := NESTING.search(text, at)):
        depth += 1 if mark.group() == "/*" else -1
        at = mark.end()
    return at if depth == 0 else len(text)


def lexed(text):
    """R3: the text with comments, doc comments and the contents of string, byte-string,
    C-string, raw-string and character literals blanked, and each string literal's contents by
    the offset of its opening quote. Every offset and line break is kept."""
    code, literals, at = [], {}, 0
    while found := SPECIAL.search(text, at):
        start, token = found.start(), found.group()
        if token == "//":
            end = text.find("\n", start)
            opened, closed = start, len(text) if end < 0 else end
            after = closed
        elif token == "/*":
            opened, closed = start, comment_end(text, start)
            after = closed
        elif token == "'":
            quoted = CHARACTER.match(text, start)
            if quoted is None:
                code.append(text[at : start + 1])
                at = start + 1
                continue
            opened, closed, after = start + 1, quoted.end() - 1, quoted.end()
        else:
            hashes, opened = token.count("#"), found.end()
            if token == '"':
                end = STRING_END.match(text, opened).end()
            else:
                end = text.find('"' + "#" * hashes, opened)
            closed = len(text) if end < 0 else end
            literals[opened - 1] = text[opened:closed]
            after = min(len(text), closed + 1 + hashes)
        code.append(text[at:opened] + blank(text[opened:closed]) + text[closed:after])
        at = after
    code.append(text[at:])
    return "".join(code), literals


def closing(code, start):
    """The offset just past the bracket that closes the one at `start`, counting `(`, `[` and `{`
    in blanked code; the end of the code when none closes it."""
    depth = 0
    for mark in BRACKET.finditer(code, start):
        depth += 1 if mark.group() in "([{" else -1
        if depth == 0:
            return mark.end()
    return len(code)


def item_end(code, start):
    """The offset just past the item that starts at `start` (R2): past its closing brace, or past
    its semicolon when one comes first outside brackets; at the bracket that closes the block
    around it otherwise."""
    depth = 0
    for mark in ITEM.finditer(code, start):
        char = mark.group()
        if char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
            if depth < 0:
                return mark.start()
        elif depth == 0 and char == ";":
            return mark.end()
        elif depth == 0 and char == "{":
            return closing(code, mark.start())
        elif depth == 0:
            return mark.start()
    return len(code)


def cfg_test_items(code, literals):
    """R2: each `#[cfg(test)]` item of blanked code, as (its start, the offset past its end, the
    module it declares out of line or None, that module's `#[path]` or None). An item's span runs
    from the attribute through its further attributes to its closing brace or semicolon."""
    found = []
    for match in TEST_ONLY.finditer(code):
        at, given = match.end(), None
        while further := FURTHER.match(code, at):
            named = PATH_ATTRIBUTE.match(code, further.start(1))
            if named:
                given = literals.get(named.end() - 1)
            at = closing(code, further.end() - 1)
        module = OUT_OF_LINE.match(code, at)
        found.append((match.start(), item_end(code, at), module and module.group(1), given))
    return found


def resolved(path):
    """A slash path with each `.` and `..` taken out, as text; a `..` above the root is kept."""
    parts = []
    for part in path.split("/"):
        if part == ".." and parts and parts[-1] != "..":
            parts.pop()
        elif part not in ("", "."):
            parts.append(part)
    return "/".join(parts)


def declared(path, module, given):
    """R2: the files an out-of-line `mod <module>;` in `path` may be, resolved as the compiler
    resolves it: its `#[path]` from the declaring file's directory; else `<module>.rs` or
    `<module>/mod.rs` beside a `mod.rs`, a `lib.rs`, a `main.rs` or a binary's root, and in the
    directory named for any other declaring file."""
    folder, name = path.rsplit("/", 1)
    if given is not None:
        return [resolved(folder + "/" + given)]
    stem = name.removesuffix(".rs")
    if stem not in ("mod", "lib", "main") and not folder.endswith("/src/bin"):
        folder = folder + "/" + stem
    return [folder + "/" + module + ".rs", folder + "/" + module + "/mod.rs"]


def blanked(code, start, end):
    """`code` with the span from `start` to `end` blanked, its line breaks kept."""
    return code[:start] + blank(code[start:end]) + code[end:]


class Source:
    """One population file (R2): its path from the root, its code with comments, the contents of
    literals and its `#[cfg(test)]` items blanked, and its string literals' contents by the offset
    of each opening quote."""

    def __init__(self, path, code, literals):
        self.path, self.code, self.literals = path, code, literals

    def line(self, offset):
        """The source's own line at an offset."""
        return self.code.count("\n", 0, offset) + 1


def population(root):
    """R2: every `crates/*/src/**/*.rs` file under `root`, sorted, less each file an out-of-line
    `#[cfg(test)] mod` declares, each with its `#[cfg(test)]` items blanked."""
    root, read = Path(root), {}
    for file in sorted((root / "crates").glob(SOURCES)):
        if file.is_file():
            read[file.relative_to(root).as_posix()] = lexed(file.read_text(encoding="utf-8"))
    items = {path: cfg_test_items(code, literals) for path, (code, literals) in read.items()}
    dropped = set()
    for path, found in items.items():
        for _start, _end, module, given in found:
            if module is not None:
                dropped.update(declared(path, module, given))
    sources = []
    for path, (code, literals) in read.items():
        if path not in dropped:
            for start, end, _module, _given in items[path]:
                code = blanked(code, start, end)
            sources.append(Source(path, code, literals))
    return sources


def judged(unit, items):
    """R10: what a rule examined, refused when it is nothing, naming `examined 0` and its unit: a
    rule that examined nothing judged nothing, which is never a pass."""
    items = list(items)
    if not items:
        raise AssertionError(f"examined 0 {unit}: the rule examined nothing, so it judged nothing")
    return items


def sqlite_one_place(files):
    """R4: (one line per opener in a population file other than the kernel's database base, the
    files it examined, the openers it found in the base as (path, line))."""
    refused, read, openers = [], [], []
    for source in files:
        found = [
            (source.line(match.start()), what + " " + " ".join(match.group().split()))
            for pattern, what in SQLITE_ARMS
            for match in pattern.finditer(source.code)
        ]
        if source.path == DATABASE_BASE:
            openers += [(source.path, line) for line, _what in found]
            continue
        read.append(source.path)
        refused += [f"sqlite-one-place: {source.path}:{line}: {what}" for line, what in found]
    return refused, judged("shipped source files", read), openers


def imported_from(crates, code):
    """The names a file's `use` items bind from any of `crates`: each imported name, or its alias
    when it is renamed; a module on the way binds none."""
    names = set()
    for use in USE.finditer(code):
        if use.group(1) in crates:
            names.update(alias or name for name, alias in BOUND.findall(use.group(2)))
    return names


def query_sites(code):
    """R5: each query site of a file's code, as (its start, the offset of its opening bracket): a
    call of the family by its `sqlx::` path or by a bare name the file imports from sqlx, and
    each `AssertSqlSafe(...)`."""
    found = [(match.start(), match.end() - 1) for match in QUERY.finditer(code)]
    bare = sorted(imported_from(("sqlx",), code) & set(FAMILY))
    if bare:
        called = re.compile(r"(?<![\w:.])(?<!fn )" + either(bare) + CALLED)
        found += [(match.start(), match.end() - 1) for match in called.finditer(code)]
    for match in ASSERTED.finditer(code):
        found.append((match.start(), match.end() - 1))
    return sorted(found)


def bodies(code):
    """Each function with a body in blanked code, as (the offset of its `fn`, the offset past its
    closing brace)."""
    found = []
    for match in FUNCTION.finditer(code):
        end = item_end(code, match.end())
        if code[end - 1 : end] == "}":
            found.append((match.start(), end))
    return found


def formatted(code, functions, at):
    """R5: the names a `let` binds, or an assignment sets, to a `format!` before `at` in the
    innermost function around it; none outside a function."""
    around = [start for start, end in functions if start <= at < end]
    if not around:
        return set()
    before = code[max(around) : at]
    names = {match.group(1) for match in LET_FORMAT.finditer(before)}
    names |= {match.group(1) for match in SET_FORMAT.finditer(before)}
    return names


def no_format_into_query(files):
    """R5: (one line per query site whose argument holds a `format!`, a `.leak()` or a name bound
    to a `format!` earlier in the same function, the sites it examined)."""
    refused, read = [], []
    for source in files:
        code = source.code
        found = query_sites(code)
        functions = bodies(code) if found else []
        for start, opened in found:
            argument = code[opened + 1 : closing(code, opened) - 1]
            site = source.path + ":" + str(source.line(start))
            read.append(site)
            reasons = []
            if FORMAT.search(argument):
                reasons.append("a format!")
            if LEAK.search(argument):
                reasons.append("a .leak()")
            for name in sorted(formatted(code, functions, start)):
                if re.search(r"(?<![\w.])" + name + r"\b", argument):
                    reasons.append(name + ", bound to a format!")
            if reasons:
                called = " ".join(code[start:opened].split())
                refused.append(
                    f"no-format-into-query: {site}: {called} takes " + "; ".join(reasons)
                )
    return refused, judged("query sites", read)


def split(code, start, end):
    """The top-level arguments in the code from `start` to `end`, as (start, end) offsets: split
    at each comma, and at a semicolon, which ends `log`'s key-values."""
    found, depth, at = [], 0, start
    for mark in SEPARATOR.finditer(code, start, end):
        char = mark.group()
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif depth == 0 and char in ",;":
            found.append((at, mark.start()))
            at = mark.end()
    found.append((at, end))
    return [(left, right) for left, right in found if code[left:right].strip()]


def field(text):
    """A `key = value` argument's key and value, split at its first top-level `=` that is no
    comparison; (None, None) for any other argument."""
    depth = 0
    for mark in EQUALS.finditer(text):
        char = mark.group()
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif depth == 0:
            return text[: mark.start()].strip(), text[mark.end() :]
    return None, None


def chains(code):
    """R6: each path chain of an expression's code, its call parentheses dropped: `a.b(c).d` is
    `a.b.d`, and `c` is a chain of its own."""
    found, levels = [], [[[], False]]
    for token in TOKEN.findall(code):
        chain, joined = levels[-1]
        if NAME.fullmatch(token):
            if chain and not joined:
                found.append(".".join(chain))
                chain = []
            levels[-1] = [chain + [token], False]
        elif token in (".", "::"):
            levels[-1][1] = bool(chain)
        elif token in ("(", "["):
            levels.append([[], False])
        elif token in (")", "]") and len(levels) > 1:
            inner, _joined = levels.pop()
            if inner:
                found.append(".".join(inner))
        else:
            if chain:
                found.append(".".join(chain))
            levels[-1] = [[], False]
    found += [".".join(chain) for chain, _joined in levels if chain]
    return found


def captured(text):
    """R6: each inline `{name}` capture of a format string's text; `{{` and `}}` capture nothing."""
    return CAPTURE.findall(text.replace("{{", "").replace("}}", ""))


def passed(source, start, end):
    """R6: the names a call passes, from its arguments in the code from `start` to `end`: each
    field key (a name, a dotted name, or a literal's letters and digits), each path chain in a
    value, the shorthand, `%` and `?` forms, each positional format argument and each inline
    `{name}` capture. A `target:`, `parent:` or `name:` argument and a `Level::` argument pass
    none."""
    code, names, message = source.code, [], False
    for at, until in split(code, start, end):
        text = code[at:until]
        stripped = text.strip()
        if SKIPPED.match(stripped):
            continue
        key, value = field(text)
        if key is not None:
            if DOTTED.fullmatch(key):
                names.append(re.sub(r"\s+", "", key))
            elif LITERAL.fullmatch(key):
                quoted = source.literals.get(at + text.index('"'), "")
                names.append("_".join(re.findall(r"[A-Za-z0-9]+", quoted)))
            names += chains(value)
        elif message:
            names += chains(text)
        elif LITERAL.fullmatch(stripped):
            message = True
            names += captured(source.literals.get(at + text.index('"'), ""))
        else:
            bare = stripped
            if bare[:1] in ("%", "?"):
                bare = bare[1:].strip()
            names += [re.sub(r"\s+", "", bare)] if DOTTED.fullmatch(bare) else chains(bare)
    return names


def parameters(code, opened):
    """The names a function's parameter list binds, from its opening bracket: each parameter's last
    name before its type (`mut token: &str` binds `token`, `&self` binds `self`)."""
    names = []
    for start, end in split(code, opened + 1, closing(code, opened) - 1):
        bound = NAME.findall(code[start:end].split(":", 1)[0])
        if bound:
            names.append(bound[-1])
    return names


def instrumented(source, match):
    """R6: the names an `#[instrument]` attribute passes: its function's parameters unless
    `skip_all`, less those `skip(..)` names, plus its `fields(..)`."""
    code = source.code
    closed = closing(code, code.index("[", match.start()))
    attribute = code[match.end() : closed - 1]
    function = FUNCTION.search(code, closed)
    opened = code.find("(", function.end()) if function else -1
    names = parameters(code, opened) if opened >= 0 else []
    skip = SKIP.search(attribute)
    if skip and skip.group(1):
        names = []
    elif skip and skip.group(2):
        names = [name for name in names if name not in NAME.findall(skip.group(2))]
    fields = FIELDS.search(attribute)
    if fields:
        opened = match.end() + fields.end() - 1
        names += passed(source, opened + 1, closing(code, opened) - 1)
    return names


def logged(source):
    """R6: each matched call of a file, as (its start, the names it passes): a macro by a
    `tracing::` or `log::` path, a bare macro the file imports from either crate, and each
    `#[instrument]` attribute."""
    code = source.code
    calls = list(LOGGED.finditer(code))
    bare = sorted(imported_from(("tracing", "log"), code))
    if bare:
        calls += list(re.compile(r"(?<![\w:.])" + either(bare) + r"\s*!\s*[(\[{]").finditer(code))
    found = []
    for match in calls:
        found.append(
            (
                match.start(),
                passed(source, match.end(), closing(code, match.end() - 1) - 1),
            )
        )
    for match in INSTRUMENT.finditer(code):
        found.append((match.start(), instrumented(source, match)))
    return found


def words(segment):
    """R7: a segment's words, split at `_` and at each lower-to-upper case change, lower-cased; a
    plural is read as its singular when the singular is a word of the lists."""
    known = set(PARTS) | set(ABOUT) | {word for pair in PAIRS for word in pair.split()}
    found = []
    for word in re.sub(r"([a-z])([A-Z])", r"\1_\2", segment).lower().split("_"):
        if word not in known and word.endswith("s") and word[:-1] in known:
            word = word[:-1]
        elif word not in known and word.endswith("es") and word[:-2] in known:
            word = word[:-2]
        if word:
            found.append(word)
    return found


def secret_named(name):
    """R7: whether a passed name is a secret's: a pair in any of its segments; otherwise, unless
    its last segment ends in an about-name, a part in any segment."""
    segments = [words(segment) for segment in re.split(r"\.|::", name)]
    paired = any(" ".join(pair) in PAIRS for each in segments for pair in zip(each, each[1:]))
    about = bool(segments[-1]) and segments[-1][-1] in ABOUT
    parted = any(word in PARTS for each in segments for word in each)
    return paired or (not about and parted)


def no_secret_in_log(files):
    """R6, R7: (one line per matched call that passes a secret-named name, the calls it
    examined)."""
    refused, read = [], []
    for source in files:
        for start, names in logged(source):
            site = source.path + ":" + str(source.line(start))
            read.append(site)
            secret = sorted({name for name in names if secret_named(name)})
            if secret:
                refused.append(f"no-secret-in-log: {site}: passes " + ", ".join(secret))
    return refused, judged("log macro calls", read)


def lock_draws_rand(root):
    """R9: whether the `uuid` package of the `Cargo.lock` at `root` depends on a rand-family
    package; a tree with no lock, or no `uuid` in it, draws nothing through rand."""
    lock = Path(root) / "Cargo.lock"
    if not lock.is_file():
        return False
    packages = tomllib.loads(lock.read_text(encoding="utf-8")).get("package", [])
    return any(
        dependency.split()[0] in RANDOM_CRATES
        for package in packages
        if package.get("name") == "uuid"
        for dependency in package.get("dependencies", [])
    )


def manifest_dependencies(manifest):
    """R8: each entry of a member manifest's `dependencies`, `build-dependencies` and
    `target.*.dependencies` tables, as (its key, its `package`, or its key when it renames
    nothing)."""
    tables = [manifest.get("dependencies", {}), manifest.get("build-dependencies", {})]
    tables += [target.get("dependencies", {}) for target in manifest.get("target", {}).values()]
    return [
        (key, value.get("package", key) if isinstance(value, dict) else key)
        for table in tables
        for key, value in table.items()
    ]


def key_line(text, key):
    """The line of a manifest that states a dependency's key, as a key or in a table header."""
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if re.match(r"\W?" + re.escape(key) + r"\W?\s*=", stripped):
            return number
        if stripped.endswith("." + key + "]"):
            return number
    return 1


def randomness_through_getrandom(files, root):
    """R8, R9: (one line per draw outside getrandom in a population file or a member manifest, the
    files and manifests it examined)."""
    refused, read = [], []
    drawn = lock_draws_rand(root)
    for source in files:
        read.append(source.path)
        found = [
            (match.start(), what)
            for pattern, what in RANDOM_ARMS
            for match in pattern.finditer(source.code)
        ]
        if drawn:
            found += [
                (match.start(), "uuid draws through rand")
                for match in UUID_RANDOM.finditer(source.code)
            ]
        for at, what in found:
            refused.append(f"randomness-through-getrandom: {source.path}:{source.line(at)}: {what}")
    for manifest in sorted((Path(root) / "crates").glob("*/Cargo.toml")):
        path = manifest.relative_to(root).as_posix()
        text = manifest.read_text(encoding="utf-8")
        read.append(path)
        for key, package in manifest_dependencies(tomllib.loads(text)):
            if key in RANDOM_CRATES or package in RANDOM_CRATES:
                line = key_line(text, key)
                refused.append(f"randomness-through-getrandom: {path}:{line}: depends on {package}")
    return refused, judged("shipped source files and manifests", read)


def planted(root, files):
    """Write each fixture file under `root`, its directories made on the way."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


def sites(lines):
    """The `<path>:<line>` each refused line names."""
    return {line.split(": ", 2)[1] for line in lines}


# A1: the database base, whose openers are its own, and a crate that opens SQLite every way R4
# names; a `SqlitePool` used as a type, and a call that connects nothing, are not refused.
DATABASE_PLANT = """\
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
pub async fn open(url: &str) {
    let options = SqliteConnectOptions::new();
    let pool = SqlitePoolOptions::new().connect_with(options).await;
    let other = sqlx::SqlitePool::connect(url).await;
}
"""
SQLITE_PLANT = """\
pub fn opens(url: &str, options: Options, pool: SqlitePool) {
    let a = SqliteConnectOptions::from_str(url);
    let b = SqlitePoolOptions::new();
    let c = PoolOptions::<Sqlite>::new();
    let d = rusqlite::Connection::open(url);
    let e = libsqlite3_sys::sqlite3_open(url);
    let f = SqlitePool::connect(url);
    let g = sqlx::Pool::<Sqlite>::connect_with(options);
    let h = SqliteConnection::connect_lazy(url);
    let i = <sqlx::SqliteConnection as sqlx::Connection>::connect_lazy_with(options);
    let j = options.connect_lazy_with();
    let k: SqlitePool = shared();
    let m = SqlitePool::clone(&pool);
}
"""

# A2: each family function and an imported bare name with a direct `format!`, `AssertSqlSafe`, a
# `.leak()`, a `let`-bound and an assigned `format!`; then a name bound in another function, a
# literal and a literal-only macro, none refused.
QUERY_PLANT = """\
use sqlx::{query_scalar, Row};
fn direct(t: &str, args: Args, built: String) {
    sqlx::query(&format!("SELECT {t}"));
    sqlx::query_as::<_, Row>(&format!("SELECT {t}"));
    sqlx::query_scalar(&format!("SELECT {t}"));
    sqlx::query_with(&format!("SELECT {t}"), args);
    sqlx::query_as_with(&format!("SELECT {t}"), args);
    sqlx::query_scalar_with(&format!("SELECT {t}"), args);
    sqlx::raw_sql(&format!("SELECT {t}"));
    query_scalar(&format!("SELECT {t}"));
    let asserted = AssertSqlSafe(format!("SELECT {t}"));
    sqlx::query(built.leak());
}
fn bound(t: &str) {
    let sql: String = format!("SELECT {t}");
    sqlx::query(&sql);
    let mut set = String::new();
    set = format!("SELECT {t}");
    sqlx::query(&set);
}
fn elsewhere() {
    sqlx::query(&sql);
    sqlx::query("SELECT 1");
    sqlx::query!("SELECT {}", format!("x"));
}
"""

# A3: each macro form and each passed-name form, carrying each part and each pair, refused; each
# about-name, a `target:` and a `Level::` argument, and the skipped parameters, not refused.
LOG_PLANT = """\
use tracing::{info, Level};
use log::warn as logged;
fn logs(cfg: &Config, session: &Session, tokens: &[u8], name: &str) {
    tracing::info!(bot_token = 1, "starting");
    tracing::warn!("webhook.secret" = 1);
    tracing::error!(value = cfg.sync_password.expose());
    tracing::debug!(%passkey_credential);
    tracing::trace!(?session_cookie);
    tracing::info!(bearer);
    tracing::info!("refreshing {}", session.authorization);
    tracing::info!("the {apikey} is read");
    log::info!(user = name; "sent {initdata}");
    info!(api_key = 1);
    logged!(private_key = 1);
    tracing::info_span!("sign", signing_key = 1);
    tracing::event!(Level::INFO, init_data = 1);
    tracing::info!(session_id = 1);
    tracing::info!(target: AUTH_TOKEN_TARGET, "fine");
    tracing::info!(token_count = 1, count = tokens.len(), "counted");
    tracing::info!(secret_length = 1, password_size = 1, token_total = 1);
    tracing::info!(secret_name = name, credential_path = 1, cookie_file = 1);
    tracing::info!(token_kind = 1, password_hash = 1, credential_id = 1);
    tracing::info!(token_expires_at = 1);
    tracing::event!(tracing::Level::INFO, count = 1);
}
#[instrument]
fn instrumented(secret: &str) {}
#[tracing::instrument(skip(token), fields(user = %name))]
fn skipped(token: &str, name: &str) {}
#[instrument(skip_all)]
fn all_skipped(password: &str) {}
"""

# A4: each refused path, identifier, import and `PyRandom::new`, and a uuid constructor under a
# lock whose uuid depends on rand; the getrandom draws and `RandomState` are not refused.
RANDOM_PLANT = """\
use getrandom::fill;
extern crate rand;
use fastrand;
fn draws(source: &mut Source, bytes: &mut [u8]) {
    let a = rand::random::<u64>();
    let b = rand_core::RngCore::next_u64(source);
    let c = rand_chacha::ChaCha20Rng::from_entropy();
    let d = fastrand::u64(..);
    let e = ring::rand::SystemRandom::new();
    let f = OsRng;
    let g: ThreadRng = made();
    let h = thread_rng();
    let i = StdRng::from_entropy();
    let j = SmallRng::from_entropy();
    let k = <Seeded as SeedableRng>::seed_from_u64(1);
    let m = PyRandom::new(42);
    let n = Uuid::new_v4();
    let o = Uuid::now_v7();
    getrandom::fill(bytes);
    let p = getrandom::u64();
    let q = getrandom::u32();
    getrandom::fill_uninit(bytes);
    let r = RandomState::new();
}
"""
RANDOM_MANIFEST = """\
[package]
name = "draw"

[dependencies]
getrandom = "0.3"
rand = "0.8"

[build-dependencies]
seeded = { package = "fastrand", version = "2" }

[target.'cfg(unix)'.dependencies]
rand_core = { workspace = true }
"""
RAND_LOCK = """\
version = 4

[[package]]
name = "uuid"
version = "1.0.0"
dependencies = [
 "getrandom",
 "rand 0.8.5",
]
"""
# A4: the same constructors under a lock whose uuid depends on getrandom alone, admitted.
UUID_PLANT = """\
fn handle(stamp: Timestamp) {
    let a = Uuid::new_v4();
    let b = Uuid::new_v7(stamp);
}
"""
UUID_MANIFEST = """\
[package]
name = "handle"

[dependencies]
uuid = "1"
"""
GETRANDOM_LOCK = """\
version = 4

[[package]]
name = "uuid"
version = "1.0.0"
dependencies = [
 "getrandom 0.4.3",
 "js-sys",
]
"""

# A5: a crate whose shipped sources hold an inline test module and an inline test function, an
# out-of-line test module beside its file and one by `#[path]`, a nested source, an integration
# test, an example and a build script, each of the last three outside the glob.
POPULATION_PLANT = {
    "crates/app/src/lib.rs": """\
mod deep;
mod other;
mod skip_write;
pub fn shipped() {
    sqlx::query("SELECT 1");
    tracing::info!(count = 1);
}
#[cfg(test)]
mod tests {
    fn opens() {
        let pool = sqlx::SqlitePool::connect(url);
        tracing::info!(bot_token = 1);
        let draw = rand::random::<u64>();
        sqlx::query(&format!("x"));
    }
}
#[cfg(test)]
#[allow(dead_code)]
fn helper() {
    let options = SqliteConnectOptions::new();
}
""",
    "crates/app/src/deep/mod.rs": "pub fn deep() {}\n",
    "crates/app/src/other.rs": """\
pub fn other() {}
#[cfg(test)]
#[path = "other_tests.rs"]
mod tests;
""",
    "crates/app/src/other_tests.rs": "fn opens() {\n    let o = SqliteConnectOptions::new();\n}\n",
    "crates/app/src/skip_write.rs": "pub fn write() {}\n#[cfg(test)]\nmod tests;\n",
    "crates/app/src/skip_write/tests.rs": "fn logs() {\n    tracing::info!(bot_token = 1);\n}\n",
    "crates/app/tests/integration.rs": "fn opens() {\n    let o = SqliteConnectOptions::new();\n}\n",
    "crates/app/examples/demo.rs": "fn main() {\n    let draw = rand::random::<u64>();\n}\n",
    "crates/app/build.rs": "fn main() {\n    let o = SqliteConnectOptions::new();\n}\n",
}

# A6: a violation of each rule in a line comment, a nested block comment, a string and a raw
# string, none refused; and a format string's inline capture, refused.
LEXED_PLANT = """\
pub fn lexed() {
    // let pool = SqlitePool::connect(url);
    // sqlx::query(&format!(sql));
    // tracing::info!(bot_token = 1);
    // let draw = rand::random::<u64>();
    /* outer /* inner */ let pool = SqlitePool::connect(url); */
    /* outer /* inner */ sqlx::query(&format!(sql)); */
    /* outer /* inner */ tracing::info!(bot_token = 1); */
    /* outer /* inner */ let draw = rand::random::<u64>(); */
    let quoted = "let pool = SqlitePool::connect(url);";
    let quoted = "sqlx::query(&format!(sql));";
    let quoted = "tracing::info!(bot_token = 1);";
    let quoted = "let draw = rand::random::<u64>();";
    let raw = r#"a" let pool = SqlitePool::connect(url); "b"#;
    let raw = r#"a" sqlx::query(&format!(sql)); "b"#;
    let raw = r#"a" tracing::info!(bot_token = 1); "b"#;
    let raw = r#"a" let draw = rand::random::<u64>(); "b"#;
    sqlx::query("SELECT 1");
    tracing::info!("refreshing {bot_token}");
}
"""


class RustHouseRules(unittest.TestCase):
    def test_sqlite_is_opened_only_in_the_kernels_database_base(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(
                scratch,
                {
                    "crates/kernel/src/db.rs": DATABASE_PLANT,
                    "crates/store/src/lib.rs": SQLITE_PLANT,
                },
            )
            refused, read, _openers = sqlite_one_place(population(scratch))
        self.assertEqual(
            sites(refused),
            {
                "crates/store/src/lib.rs:2",
                "crates/store/src/lib.rs:3",
                "crates/store/src/lib.rs:4",
                "crates/store/src/lib.rs:5",
                "crates/store/src/lib.rs:6",
                "crates/store/src/lib.rs:7",
                "crates/store/src/lib.rs:8",
                "crates/store/src/lib.rs:9",
                "crates/store/src/lib.rs:10",
                "crates/store/src/lib.rs:11",
            },
        )
        examined("fixture files outside the database base", read)

    def test_no_format_reaches_a_query(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, {"crates/query/src/lib.rs": QUERY_PLANT})
            refused, read = no_format_into_query(population(scratch))
        self.assertEqual(
            sites(refused),
            {
                "crates/query/src/lib.rs:3",
                "crates/query/src/lib.rs:4",
                "crates/query/src/lib.rs:5",
                "crates/query/src/lib.rs:6",
                "crates/query/src/lib.rs:7",
                "crates/query/src/lib.rs:8",
                "crates/query/src/lib.rs:9",
                "crates/query/src/lib.rs:10",
                "crates/query/src/lib.rs:11",
                "crates/query/src/lib.rs:12",
                "crates/query/src/lib.rs:16",
                "crates/query/src/lib.rs:19",
            },
        )
        examined("planted calls of the family and AssertSqlSafe sites", read)

    def test_no_secret_named_name_reaches_a_log_macro(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, {"crates/log/src/lib.rs": LOG_PLANT})
            refused, read = no_secret_in_log(population(scratch))
        self.assertEqual(
            sites(refused),
            {
                "crates/log/src/lib.rs:4",
                "crates/log/src/lib.rs:5",
                "crates/log/src/lib.rs:6",
                "crates/log/src/lib.rs:7",
                "crates/log/src/lib.rs:8",
                "crates/log/src/lib.rs:9",
                "crates/log/src/lib.rs:10",
                "crates/log/src/lib.rs:11",
                "crates/log/src/lib.rs:12",
                "crates/log/src/lib.rs:13",
                "crates/log/src/lib.rs:14",
                "crates/log/src/lib.rs:15",
                "crates/log/src/lib.rs:16",
                "crates/log/src/lib.rs:17",
                "crates/log/src/lib.rs:26",
            },
        )
        examined("planted log calls and instrument attributes", read)

    def test_randomness_is_drawn_only_through_getrandom(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(
                scratch,
                {
                    "crates/draw/src/lib.rs": RANDOM_PLANT,
                    "crates/draw/Cargo.toml": RANDOM_MANIFEST,
                    "Cargo.lock": RAND_LOCK,
                },
            )
            drawn, read = randomness_through_getrandom(population(scratch), scratch)
        with tempfile.TemporaryDirectory() as scratch:
            planted(
                scratch,
                {
                    "crates/id/src/lib.rs": UUID_PLANT,
                    "crates/id/Cargo.toml": UUID_MANIFEST,
                    "Cargo.lock": GETRANDOM_LOCK,
                },
            )
            admitted, read_too = randomness_through_getrandom(population(scratch), scratch)
        self.assertEqual(
            sites(drawn),
            {
                "crates/draw/src/lib.rs:2",
                "crates/draw/src/lib.rs:3",
                "crates/draw/src/lib.rs:5",
                "crates/draw/src/lib.rs:6",
                "crates/draw/src/lib.rs:7",
                "crates/draw/src/lib.rs:8",
                "crates/draw/src/lib.rs:9",
                "crates/draw/src/lib.rs:10",
                "crates/draw/src/lib.rs:11",
                "crates/draw/src/lib.rs:12",
                "crates/draw/src/lib.rs:13",
                "crates/draw/src/lib.rs:14",
                "crates/draw/src/lib.rs:15",
                "crates/draw/src/lib.rs:16",
                "crates/draw/src/lib.rs:17",
                "crates/draw/src/lib.rs:18",
                "crates/draw/Cargo.toml:6",
                "crates/draw/Cargo.toml:9",
                "crates/draw/Cargo.toml:12",
            },
        )
        self.assertEqual(admitted, [])
        examined("planted files and manifests", read + read_too)

    def test_the_population_is_the_shipped_sources_without_test_code(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, POPULATION_PLANT)
            files = population(scratch)
            refused = (
                sqlite_one_place(files)[0]
                + no_format_into_query(files)[0]
                + no_secret_in_log(files)[0]
                + randomness_through_getrandom(files, scratch)[0]
            )
        self.assertEqual(
            {source.path for source in files},
            {
                "crates/app/src/deep/mod.rs",
                "crates/app/src/lib.rs",
                "crates/app/src/other.rs",
                "crates/app/src/skip_write.rs",
            },
        )
        self.assertEqual(refused, [])
        examined("fixture files in the population", files)

    def test_comments_and_literals_are_not_code_but_a_format_capture_is_read(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, {"crates/lexed/src/lib.rs": LEXED_PLANT})
            files = population(scratch)
            logged_lines = no_secret_in_log(files)[0]
            others = (
                sqlite_one_place(files)[0]
                + no_format_into_query(files)[0]
                + randomness_through_getrandom(files, scratch)[0]
            )
        self.assertEqual(sites(logged_lines), {"crates/lexed/src/lib.rs:19"})
        self.assertEqual(others, [])
        examined("fixture files lexed", files)

    def test_each_rule_refuses_an_empty_population(self):
        with tempfile.TemporaryDirectory() as scratch:
            files = population(scratch)
            cases = (
                ("shipped source files", sqlite_one_place, (files,)),
                ("query sites", no_format_into_query, (files,)),
                ("log macro calls", no_secret_in_log, (files,)),
                (
                    "shipped source files and manifests",
                    randomness_through_getrandom,
                    (files, scratch),
                ),
            )
            for unit, rule, given in cases:
                with self.subTest(unit=unit):
                    with self.assertRaisesRegex(AssertionError, "examined 0 " + unit):
                        rule(*given)
        examined("rules over an empty population", cases)

    def test_the_repository_holds_every_rule_at_zero(self):
        files = population(REPO)
        opened, shipped, openers = sqlite_one_place(files)
        self.assertTrue(openers, "sqlite-one-place found no opener in crates/kernel/src/db.rs")
        queried, query_sites_read = no_format_into_query(files)
        logged_lines, calls = no_secret_in_log(files)
        drawn, read = randomness_through_getrandom(files, REPO)
        refused = opened + queried + logged_lines + drawn
        for line in refused:
            print(line)
        self.assertEqual(refused, [])
        examined("shipped source files", shipped)
        examined("query sites", query_sites_read)
        examined("log macro calls", calls)
        examined("shipped source files and manifests", read)


if __name__ == "__main__":
    unittest.main()
