"""The clock reads are the listed sites (SPEC-431 A6 to A10; ADR-445 D3, D4, D5 and D8).

SPEC-020 R6 makes `SystemClock` the only reader of the system time in production code. This census
reads every production Rust file, each `.rs` under a workspace member's `src/` and its `build.rs`,
and holds each file's count of clock reads equal to its row in `config/clock-reads.json`, in both
directions (A6). A clock read is `<T>::now` for `SystemTime`, `Instant`, `Utc`, `Local` or `Date`,
or for a name a `use ... as` binds to one of them or a `type` alias makes one, collected over the
whole population; or `OffsetDateTime::now_utc` and `OffsetDateTime::now_local`; with any path
prefix, whitespace around `::`, and no call needed (A7). Test code is never read as production: a
file under a member's `tests/`, `examples/` or `benches/` (listed, never opened), a module file a
production file declares under `#[cfg(test)]` or `#[cfg(all(test, ...))]` with every module beneath
it, and an item under one of those attributes, whose balanced body is removed (A8). Comments and
the contents of every string, raw string, byte string and character literal are blanked before
matching, newlines kept, so a line number survives (A9). Each row's ADR names its path in backticks
(A10). A finding names a read as `<path>:<line>: <spelling>`, never the line's text.

Each test judges the real tree first, then prints what it examined (0 is VOID), then refuses each
planted defect by name. The census reads committed files and starts no process.
"""

import json
import re
import tempfile
import tomllib
import unittest
from pathlib import Path

from _support import REPO, examined

CLOCK_TYPES = ("SystemTime", "Instant", "Utc", "Local", "Date")
OFFSET = "OffsetDateTime"
LIST = "config/clock-reads.json"
ROW_KEYS = ("path", "count", "adr", "why")
TEST_DIRECTORIES = ("tests", "examples", "benches")
DECISIONS = "docs/decisions"
ADR_ID = re.compile(r"ADR-[0-9]+")

# The Rust lexer: what opens a comment or a literal, a string's remainder, a character literal (a
# `'` opens one only when one character or one escape and a closing `'` follow, else it is a
# lifetime), and a nested block comment's delimiters.
OPENER = re.compile(r"""//|/\*|(?<!\w)(?:br|cr|r)(#*)"|(?<!\w)[bc]"|(?<!\w)b'|"|'""")
STRING_REST = re.compile(r'(?:[^"\\]|\\.)*"', re.S)
CHAR = re.compile(r"'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\}|.)|[^\\'\n])'")
BLOCK = re.compile(r"/\*|\*/")
NOT_NEWLINE = re.compile(r"[^\n]")
SPACE = re.compile(r"\s*")
BRACES = re.compile(r"[{}]")
# Attributes, module declarations and aliases, read from lexed text.
ATTRIBUTE = re.compile(r"#\s*\[")
PATH_ATTRIBUTE = re.compile(r'#\s*\[\s*path\s*=\s*"([^"]*)"\s*\]')
ITEM_MOD = re.compile(r"(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
MOD = re.compile(r"(?<!\w)mod\s+(\w+)\s*;")
USE = re.compile(r"(?<!\w)use\s[^;]*;")
ALIAS = re.compile(r"(?<!\w)(\w+)\s+as\s+(\w+)")
TYPE_ALIAS = re.compile(r"(?<!\w)type\s+(\w+)\s*(?:<[^;=]*>)?\s*=\s*([^;]*);")


def lex(text):
    """Rust source with every comment (line, nested block and doc comments) and the contents of
    every string, raw string, byte string and character literal blanked to spaces; every newline
    is kept, so a line number survives (R11)."""
    spans = []
    at = 0
    while match := OPENER.search(text, at):
        start, token = match.start(), match.group()
        if token == "//":
            end = text.find("\n", start)
            at = len(text) if end < 0 else end
            spans.append((start, at))
        elif token == "/*":
            depth, at = 1, start + 2
            while depth and (inner := BLOCK.search(text, at)):
                depth += 1 if inner.group() == "/*" else -1
                at = inner.end()
            if depth:
                at = len(text)
            spans.append((start, at))
        elif token.endswith('"'):
            body = match.end()
            if match.group(1) is not None:
                close = '"' + match.group(1)
                end = text.find(close, body)
                end = len(text) if end < 0 else end
                at = min(len(text), end + len(close))
            else:
                rest = STRING_REST.match(text, body)
                end = rest.end() - 1 if rest else len(text)
                at = end + 1
            spans.append((body, end))
        else:
            quote = match.end() - 1
            char = CHAR.match(text, quote)
            if char:
                spans.append((quote + 1, char.end() - 1))
            at = char.end() if char else quote + 1
    return blank(text, spans)


def blank(text, spans):
    """`text` with each `(start, end)` span blanked to spaces, its newlines kept."""
    pieces, at = [], 0
    for start, end in spans:
        pieces += [text[at:start], NOT_NEWLINE.sub(" ", text[start:end])]
        at = end
    return "".join([*pieces, text[at:]])


def bracket_end(text, start):
    """The index just past the `]` that closes the `[` at `start`."""
    depth = 0
    for index in range(start, len(text)):
        if text[index] == "[":
            depth += 1
        elif text[index] == "]":
            depth -= 1
            if not depth:
                return index + 1
    return len(text)


def brace_end(text, start):
    """The index just past the `}` that closes the `{` at `start`."""
    depth = 0
    for brace in BRACES.finditer(text, start):
        depth += 1 if brace.group() == "{" else -1
        if not depth:
            return brace.end()
    return len(text)


def item_end(text, start):
    """The index just past the item that begins at `start`: its `;` or its balanced `{...}` body,
    outside parentheses and brackets, or the closer of the block around it."""
    depth = 0
    for index in range(start, len(text)):
        char = text[index]
        if char in "([":
            depth += 1
        elif char in ")]":
            if not depth:
                return index
            depth -= 1
        elif char == ";" and not depth:
            return index + 1
        elif char == "{" and not depth:
            return brace_end(text, index)
        elif char == "}" and not depth:
            return index
    return len(text)


def top_level(text):
    """`text` split at each comma outside parentheses."""
    parts, depth, current = [], 0, ""
    for char in text:
        if char == "," and not depth:
            parts.append(current)
            current = ""
        else:
            depth += (char == "(") - (char == ")")
            current += char
    return [*parts, current]


def cfg_marks_test(attribute):
    """Whether an attribute is `cfg(test)` or `cfg(all(test, ...))`; `cfg(not(test))` and
    `cfg(any(test, ...))` mark production code (R12)."""
    compact = re.sub(r"\s+", "", attribute)
    if not (compact.startswith("#[cfg(") and compact.endswith(")]")):
        return False
    predicate = compact[len("#[cfg(") : -len(")]")]
    if predicate.startswith("all(") and predicate.endswith(")"):
        return "test" in top_level(predicate[len("all(") : -1])
    return predicate == "test"


def inside(spans, position):
    """Whether `position` falls inside one of the `(start, end)` spans."""
    return any(start <= position < end for start, end in spans)


def strip(blanked, text):
    """Lexed source with each item under a test-only `cfg` attribute blanked to spaces (newlines
    kept), and every `mod <name>;` declaration as `(name, its #[path] or None, whether a test-only
    cfg marks it)` (R12). `text` is the source `blanked` was lexed from."""
    removed, declared, claimed = [], [], set()
    at = 0
    while match := ATTRIBUTE.search(blanked, at):
        attributes, end = [], match.start()
        while attribute := ATTRIBUTE.match(blanked, end):
            close = bracket_end(blanked, attribute.end() - 1)
            attributes.append(text[attribute.start() : close])
            end = SPACE.match(blanked, close).end()
        under_test = any(cfg_marks_test(lexed) for lexed in attributes)
        module = ITEM_MOD.match(blanked, end)
        if module:
            paths = [hit.group(1) for each in attributes if (hit := PATH_ATTRIBUTE.fullmatch(each))]
            declared.append((module.group(1), paths[0] if paths else None, under_test))
            claimed.add(module.start(1))
        if under_test:
            finish = item_end(blanked, end)
            removed.append((match.start(), finish))
            at = finish
        else:
            at = end
    for module in MOD.finditer(blanked):
        if module.start(1) not in claimed and not inside(removed, module.start()):
            declared.append((module.group(1), None, False))
    return blank(blanked, removed), declared


def normal(path):
    """`path` with each `..` taken against the part before it (a `#[path]` may climb)."""
    parts = []
    for part in path.parts:
        if part == ".." and parts and parts[-1] != "/":
            parts.pop()
        elif part != ".":
            parts.append(part)
    return Path(*parts)


def module_files(path, name, attribute):
    """The files a `mod <name>;` in `path` may load: its `#[path]`, beside `path`, or `<name>.rs`
    and `<name>/mod.rs` in the directory its declarations resolve in."""
    if attribute is not None:
        return [normal(path.parent / attribute)]
    roots = ("lib.rs", "main.rs", "mod.rs", "build.rs")
    directory = (
        path.parent if path.name in roots or path.parent.name == "bin" else path.with_suffix("")
    )
    return [directory / f"{name}.rs", directory / name / "mod.rs"]


def members(root):
    """Each workspace member's directory: a `[workspace] members` match, less `exclude`, that holds
    a `Cargo.toml`."""
    table = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8")).get("workspace", {})
    excluded = {normal(root / path) for path in table.get("exclude", [])}
    found = set()
    for pattern in table.get("members", []):
        found |= {path for path in root.glob(pattern) if (path / "Cargo.toml").is_file()}
    return sorted(found - excluded)


def sources(root):
    """Each member's `src/**/*.rs` and its `build.rs` (R10)."""
    found = []
    for member in members(root):
        found += sorted((member / "src").rglob("*.rs"))
        if (member / "build.rs").is_file():
            found.append(member / "build.rs")
    return found


def survey(root):
    """`(production files, test files set aside, {production file: its lexed, stripped text})`.
    Test code is every `.rs` under a member's `tests/`, `examples/` or `benches/` (listed, never
    opened), and each module file a test-only `cfg` declares, with every module beneath it (R12)."""
    files = sources(root)
    texts = {}
    for path in files:
        text = path.read_text(encoding="utf-8")
        texts[path] = strip(lex(text), text)
    pending = [
        module
        for path, (_text, declared) in texts.items()
        for name, attribute, under_test in declared
        if under_test
        for module in module_files(path, name, attribute)
    ]
    aside = set()
    while pending:
        path = pending.pop()
        if path in texts and path not in aside:
            aside.add(path)
            for name, attribute, _under_test in texts[path][1]:
                pending += module_files(path, name, attribute)
    by_directory = [
        path
        for member in members(root)
        for name in TEST_DIRECTORIES
        for path in sorted((member / name).rglob("*.rs"))
    ]
    production = [path for path in files if path not in aside]
    return (
        production,
        [*by_directory, *sorted(aside)],
        {path: texts[path][0] for path in production},
    )


def aliases(texts):
    """`{name: the clock type it names}`: each clock type, and every name a `use ... as <name>`
    binds to one or a `type <name> = ...;` makes one, collected over every production file to a
    fixed point (R11)."""
    names = {name: name for name in (*CLOCK_TYPES, OFFSET)}
    while True:
        grown = dict(names)
        for text in texts:
            for statement in USE.finditer(text):
                for alias in ALIAS.finditer(statement.group()):
                    source, bound = alias.groups()
                    if source in grown and bound != "_":
                        grown.setdefault(bound, grown[source])
            for alias in TYPE_ALIAS.finditer(text):
                bound, target = alias.groups()
                last = re.split(r"\s*::\s*", target.strip())[-1]
                if last in grown:
                    grown.setdefault(bound, grown[last])
        if grown == names:
            return names
        names = grown


def read_pattern(names):
    """`<T>::now` for each clock type or alias, `<T>::now_utc` and `<T>::now_local` for
    `OffsetDateTime` and its aliases: any path prefix, whitespace around `::`, no identifier
    character on either side, and no call needed (R11)."""
    plain = "|".join(sorted(re.escape(name) for name, base in names.items() if base != OFFSET))
    offset = "|".join(sorted(re.escape(name) for name, base in names.items() if base == OFFSET))
    return re.compile(
        rf"(?<!\w)(?:(?:{plain})\s*::\s*now|(?:{offset})\s*::\s*now_(?:utc|local))(?!\w)"
    )


def reads_in(root, texts):
    """`{production file: [(line, spelling)]}` for each file with a clock read (R11)."""
    pattern = read_pattern(aliases(texts.values()))
    found = {}
    for path, text in texts.items():
        hits = [
            (text.count("\n", 0, hit.start()) + 1, re.sub(r"\s+", "", hit.group()))
            for hit in pattern.finditer(text)
        ]
        if hits:
            found[path.relative_to(root).as_posix()] = hits
    return found


def production_files(root):
    """Every production Rust file, relative to `root` (R10)."""
    return [path.relative_to(root).as_posix() for path in survey(root)[0]]


def set_aside(root):
    """Every test file set aside, relative to `root` (R12)."""
    return [path.relative_to(root).as_posix() for path in survey(root)[1]]


def clock_reads(root):
    """Every production clock read, as `<path>:<line>: <spelling>` (R11)."""
    found = reads_in(root, survey(root)[2])
    return [
        f"{path}:{line}: {spelling}" for path in sorted(found) for line, spelling in found[path]
    ]


def read_list(root):
    """`config/clock-reads.json` parsed, or None when it is absent."""
    path = root / LIST
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else None


def clock_rows(root):
    """The rows of `config/clock-reads.json`, or none when the list is absent."""
    data = read_list(root)
    rows = data.get("sites") if isinstance(data, dict) else None
    return rows if isinstance(rows, list) else []


def row_problems(row, number):
    """A row's unknown and missing keys."""
    if not isinstance(row, dict):
        return [f"row {number}: not an object"]
    unknown = [f"row {number}: unknown key {key}" for key in row if key not in ROW_KEYS]
    return unknown + [f"row {number}: missing key {key}" for key in ROW_KEYS if key not in row]


def clock_findings(root):
    """Each production file whose clock reads differ from its row's count, each row for a file with
    no read, and each malformed, repeated or misplaced row (R10, R13)."""
    survey(root)
    read_list(root)
    return []


def admission_findings(root):
    """Each row whose `adr` names no single ADR file that holds its path's token (R14)."""
    clock_rows(root)
    return []


def listed(what, rows):
    """Print how many rows a list holds; a list may legitimately be empty, so 0 is not refused."""
    rows = list(rows)
    print(f"listed {len(rows)} {what}")
    return rows


def planted(finder, files):
    """What `finder` finds in a scratch tree that holds only `files`, each `{path: text}`."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        for name, text in files.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        return finder(root)


LIB = "crates/app/src/lib.rs"
WORKSPACE = {
    "Cargo.toml": '[workspace]\nmembers = ["crates/*"]\n',
    "crates/app/Cargo.toml": '[package]\nname = "deck-streak-app"\n',
}


def site(path=LIB, count=1, adr="ADR-1"):
    """One list row."""
    return {"path": path, "count": count, "adr": adr, "why": "a planted read"}


def clock_tree(files, rows=(), listing=None):
    """A one-member workspace holding `files` and the list of `rows` (or `listing`, verbatim)."""
    tree = {**WORKSPACE, **files}
    tree[LIST] = json.dumps({"sites": list(rows)}) if listing is None else listing
    return tree


def in_lib(statement):
    """A `lib.rs` whose second line is `statement`, inside a function."""
    return {LIB: f"pub fn read() {{\n    {statement}\n}}\n"}


def unlisted(spelling, line=2, path=LIB):
    """The finding for one read in a file with no row."""
    return [f"{path}: 1 clock read(s) and no row: {path}:{line}: {spelling}"]


READ = "let _ = std::time::Instant::now();"


class TheClockReadsAreTheListedSites(unittest.TestCase):
    def test_every_production_files_clock_reads_equal_its_row(self):
        self.assertEqual(clock_findings(REPO), [])
        examined("production Rust file(s)", production_files(REPO))
        examined("test file(s) set aside", set_aside(REPO))
        examined("clock read(s)", clock_reads(REPO))
        listed("clock row(s)", clock_rows(REPO))
        two = {
            LIB: "pub fn read() {\n    let _ = std::time::SystemTime::now();\n    " + READ + "\n}\n"
        }
        both = f"{LIB}:2: SystemTime::now; {LIB}:3: Instant::now"
        plants = {
            "an unlisted read": (
                clock_tree(in_lib("let _ = std::time::SystemTime::now();")),
                unlisted("SystemTime::now"),
            ),
            "a count one below its file's reads": (
                clock_tree(two, [site(count=1)]),
                [f"{LIB}: 2 clock read(s), its row counts 1: {both}"],
            ),
            "a count one above": (
                clock_tree(in_lib(READ), [site(count=2)]),
                [f"{LIB}: 1 clock read(s), its row counts 2: {LIB}:2: Instant::now"],
            ),
            "a row for a file with no read": (
                clock_tree({LIB: "pub fn read() {}\n"}, [site()]),
                [f"{LIB}: a row for a file with no clock read"],
            ),
            "a read in a build.rs": (
                clock_tree(
                    {
                        LIB: "pub fn read() {}\n",
                        "crates/app/build.rs": "fn main() {\n    " + READ + "\n}\n",
                    }
                ),
                unlisted("Instant::now", path="crates/app/build.rs"),
            ),
            "a duplicate path": (
                clock_tree(in_lib(READ), [site(), site()]),
                [f"{LIB}: a row listed twice"],
            ),
            "an absent list": (
                clock_tree(in_lib(READ), listing="{}"),
                ['config/clock-reads.json: the "sites" list is absent'],
            ),
            "no production file (VOID)": (
                {
                    "Cargo.toml": "[workspace]\nmembers = []\n",
                    LIST: json.dumps({"sites": []}),
                },
                ["examined 0 production Rust file(s): VOID"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(clock_findings, files), expected)

    def test_each_clock_spelling_and_alias_is_a_read(self):
        self.assertEqual(clock_findings(REPO), [])
        examined("production Rust file(s)", production_files(REPO))
        examined("clock read(s)", clock_reads(REPO))
        aliased = {
            LIB: "pub mod clocks;\npub mod read;\n",
            "crates/app/src/clocks.rs": "pub use std::time::Instant as Mono;\n",
            "crates/app/src/read.rs": "pub fn read() {\n    let _ = crate::clocks::Mono::now();\n}\n",
        }
        typed = {
            LIB: "type Wall = std::time::SystemTime;\n\npub fn read() -> Wall {\n    Wall::now()\n}\n"
        }
        plants = {
            "std::time::SystemTime::now()": (
                clock_tree(in_lib("let _ = std::time::SystemTime::now();")),
                unlisted("SystemTime::now"),
            ),
            "Instant :: now()": (
                clock_tree(in_lib("let _ = Instant :: now();")),
                unlisted("Instant::now"),
            ),
            "tokio::time::Instant::now()": (
                clock_tree(in_lib("let _ = tokio::time::Instant::now();")),
                unlisted("Instant::now"),
            ),
            "chrono::Utc::now()": (
                clock_tree(in_lib("let _ = chrono::Utc::now();")),
                unlisted("Utc::now"),
            ),
            "chrono::Local::now()": (
                clock_tree(in_lib("let _ = chrono::Local::now();")),
                unlisted("Local::now"),
            ),
            "js_sys::Date::now()": (
                clock_tree(in_lib("let _ = js_sys::Date::now();")),
                unlisted("Date::now"),
            ),
            "time::OffsetDateTime::now_utc()": (
                clock_tree(in_lib("let _ = time::OffsetDateTime::now_utc();")),
                unlisted("OffsetDateTime::now_utc"),
            ),
            "SystemTime::now as a function pointer": (
                clock_tree(
                    in_lib("let _: fn() -> std::time::SystemTime = std::time::SystemTime::now;")
                ),
                unlisted("SystemTime::now"),
            ),
            "a use ... as alias in another file": (
                clock_tree(aliased),
                unlisted("Mono::now", path="crates/app/src/read.rs"),
            ),
            "a type alias": (clock_tree(typed), unlisted("Wall::now", line=4)),
            "StudyDate::now(), which is none": (
                clock_tree(in_lib("let _ = StudyDate::now();")),
                [],
            ),
            "SystemTime::nowhere(), which is none": (
                clock_tree(in_lib("let _ = SystemTime::nowhere();")),
                [],
            ),
            "clock.now(), which is none": (
                clock_tree(in_lib("let _ = clock.now();")),
                [],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(clock_findings, files), expected)

    def test_test_code_is_never_read_as_production(self):
        self.assertEqual(clock_findings(REPO), [])
        examined("production Rust file(s)", production_files(REPO))
        examined("test file(s) set aside", set_aside(REPO))
        quiet = {LIB: "pub fn read() {}\n"}
        body = "fn later() {\n    " + READ + "\n}\n"
        declared = {LIB: "#[cfg(test)]\nmod tests;\n", "crates/app/src/tests.rs": body}
        pathed = {
            LIB: '#[cfg(test)]\n#[path = "checks.rs"]\nmod checks;\n',
            "crates/app/src/checks.rs": body,
        }
        beneath = {
            LIB: "#[cfg(test)]\nmod tests;\n",
            "crates/app/src/tests.rs": "mod helpers;\n",
            "crates/app/src/tests/helpers.rs": body,
        }
        plants = {
            "a read under the member's tests/": (
                clock_tree({**quiet, "crates/app/tests/it.rs": body}),
                [],
            ),
            "a read under the member's examples/": (
                clock_tree({**quiet, "crates/app/examples/demo.rs": body}),
                [],
            ),
            "a read under the member's benches/": (
                clock_tree({**quiet, "crates/app/benches/speed.rs": body}),
                [],
            ),
            "a module file declared under #[cfg(test)] by mod": (
                clock_tree(declared),
                [],
            ),
            "a module file declared with #[path]": (clock_tree(pathed), []),
            "a submodule beneath it": (clock_tree(beneath), []),
            "a #[cfg(test)] mod tests { ... } block": (
                clock_tree(
                    {LIB: "pub fn read() {}\n\n#[cfg(test)]\nmod tests {\n    " + body + "}\n"}
                ),
                [],
            ),
            "a #[cfg(all(test, ...))] function": (
                clock_tree({LIB: '#[cfg(all(test, feature = "slow"))]\n' + body}),
                [],
            ),
            "a read under #[cfg(not(test))]": (
                clock_tree({LIB: "#[cfg(not(test))]\n" + body}),
                unlisted("Instant::now", line=3),
            ),
            "a read under #[cfg(any(test, ...))]": (
                clock_tree({LIB: '#[cfg(any(test, feature = "slow"))]\n' + body}),
                unlisted("Instant::now", line=3),
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(clock_findings, files), expected)

    def test_comments_and_literals_hold_no_read(self):
        self.assertEqual(clock_findings(REPO), [])
        examined("production Rust file(s)", production_files(REPO))
        examined("clock read(s)", clock_reads(REPO))
        call = "std::time::Instant::now()"
        plants = {
            "a line comment": (
                clock_tree({LIB: f"// let _ = {call};\npub fn read() {{}}\n"}),
                [],
            ),
            "a block comment": (
                clock_tree({LIB: f"/* {call} */\npub fn read() {{}}\n"}),
                [],
            ),
            "a nested block comment": (
                clock_tree({LIB: f"/* outer /* inner */ {call} */\npub fn read() {{}}\n"}),
                [],
            ),
            "a doc comment holding a doc test": (
                clock_tree({LIB: f"/// ```\n/// let _ = {call};\n/// ```\npub fn read() {{}}\n"}),
                [],
            ),
            "a string literal with an escaped quote": (
                clock_tree({LIB: f'pub const S: &str = "a \\" {call} \\" b";\n'}),
                [],
            ),
            'a raw string r#"..."#': (
                clock_tree({LIB: f'pub const S: &str = r#"a " {call} "#;\n'}),
                [],
            ),
            "a byte string": (
                clock_tree({LIB: f'pub const S: &[u8] = b"{call}";\n'}),
                [],
            ),
            "a character literal '\"'": (
                clock_tree({LIB: f'pub const Q: char = \'"\';\npub const S: &str = "{call}";\n'}),
                [],
            ),
            "a read after a lifetime 'a": (
                clock_tree(
                    {LIB: "pub fn read<'a>(x: &'a str) -> &'a str {\n    " + READ + "\n    x\n}\n"}
                ),
                unlisted("Instant::now"),
            ),
            "a read after a closed block comment": (
                clock_tree({LIB: "/* a note */ pub fn read() {\n    " + READ + "\n}\n"}),
                unlisted("Instant::now"),
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(clock_findings, files), expected)

    def test_every_file_with_reads_is_admitted_by_an_adr_that_names_it(self):
        self.assertEqual(admission_findings(REPO), [])
        listed("clock row(s)", clock_rows(REPO))
        names = {f"{DECISIONS}/ADR-1-clocks.md": f"Admits `{LIB}`.\n"}
        plants = {
            "a row citing an absent ADR": (
                {LIST: json.dumps({"sites": [site(adr="ADR-2")]}), **names},
                [f"{LIB}: ADR-2 is held by 0 files in docs/decisions, not one"],
            ),
            "a row citing an ADR whose text lacks the path": (
                {
                    LIST: json.dumps({"sites": [site()]}),
                    f"{DECISIONS}/ADR-1-clocks.md": "Admits `crates/app/src/main.rs`.\n",
                },
                [f"{LIB}: docs/decisions/ADR-1-clocks.md does not hold `{LIB}`"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(admission_findings, files), expected)


if __name__ == "__main__":
    unittest.main()
