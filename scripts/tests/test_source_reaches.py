"""Each reach out of its member is a listed row (SPEC-431 A12 to A14, R20 to R25; ADR-445 D11).

A member's Rust source reaches a file by a `path` attribute, set directly or through `cfg_attr`, or
by `include!`, `include_str!` or `include_bytes!`, with any path prefix and any bracket. The census
reads every `.rs` file under each workspace member's directory, and each file outside the members
that a module's `path` attribute or an `include!` brings in, followed until no new file is named
(R20). A site's path resolves as the compiler resolves it: from the directory of the file that
holds it, through the inline `mod` blocks around a `path` attribute, and from the member's
directory for `concat!(env!("CARGO_MANIFEST_DIR"), ...)` (R21). A site whose target sits in its own
member's directory, or in its build output, is counted and printed with no row (R22). Every other
site is a reach, keyed `<from> -> <to> [<code>]`: `from` is the member's name or the brought-in
file's path, and `code` the kind of code that holds the site. Each key has one row in
`config/source-reaches.json` whose `count` equals its sites, and every row has its sites (R23).
Each row cites, in `adr`, one `docs/decisions/ADR-<n>-*.md` file whose text names the reach as
`<from> -> <to>` in backticks (R24). A site whose path the census cannot resolve is refused, and no
row admits it (R25). A finding names a site as `<path>:<line>`, never the line's text.

Each test judges the real tree first, then prints what it examined (0 is VOID), then refuses each
planted defect by name. The census reads committed files and starts no process.
"""

import json
import re
import unittest
from pathlib import PurePosixPath

from _support import REPO, examined
from test_clock_reads import SPACE, brace_end, bracket_end, lex, strip, survey
from test_crate_edges import lexical, listed, manifests, member_name, planted

LIST = "config/source-reaches.json"
ROW_KEYS = ("from", "to", "code", "count", "adr")
NAMED = ("from", "to", "code")
ADR_ID = re.compile(r"ADR-[0-9]+")
DECISIONS = "docs/decisions"
# The kind of code a member's file holds, by the directory under the member that holds it, and the
# files whose inline blocks resolve from their own directory (a crate root or a `mod.rs`).
KINDS = {"tests": "test", "benches": "bench", "examples": "example"}
ROOT_FILES = ("lib.rs", "main.rs", "mod.rs", "build.rs")
MANIFEST_DIR = "CARGO_MANIFEST_DIR"
OUT_DIR = "OUT_DIR"
BUILD_OUTPUT = "the build output"
NEWLINE = "\n"
# A literal's place in a site's shape: lexed source holds this character only inside a literal.
SLOT = "\x00"
# The site forms, read from lexed text, which keeps each literal's quotes and blanks its contents.
ATTRIBUTE = re.compile(r"#\s*(!?)\s*\[")
LITERAL = re.compile(r'(?:\br#*)?"([^"]*)"#*')
WHITESPACE = re.compile(r"\s+")
INCLUDE = re.compile(r"(?<!\w)(include_bytes|include_str|include)\s*!\s*[(\[{]")
PREFIX = r"(?:(?:::)?(?:std|core)::)?"
BUILT = re.compile(rf"{PREFIX}concat!\({PREFIX}env!\({SLOT},?\)(?:,{SLOT})+,?\),?")
VISIBILITY = re.compile(r"pub(?:\s*\([^)]*\))?\s+")
INLINE_MOD = re.compile(r"(?<!\w)mod\s+(\w+)\s*\{")
# Why a site is refused (R25).
NOT_LITERAL = "a value that is not one string literal"
INNER = "an inner path attribute"
UNRESOLVED = "an argument the census cannot resolve"
ESCAPE = "a literal with an escape"
ABSOLUTE = "an absolute path"
ABOVE = "a path above the repository root"
ENVIRONMENT = "an environment-relative path in a file outside the members"


def member_files(root):
    """`{each .rs file under a member's directory: (its member's name, that directory)}` (R20)."""
    found = {}
    for directory, manifest in manifests(root).items():
        for path in sorted((root / directory).rglob("*.rs")):
            found[path.relative_to(root).as_posix()] = (member_name(manifest), directory)
    return found


def owns(directory, target):
    """Whether `target` sits in `directory`."""
    return target == directory or target.startswith(f"{directory}/")


def code_kind(relative, directory):
    """The kind of code a member's file holds, by where it sits in its member (R22)."""
    parts = PurePosixPath(relative).relative_to(directory).parts
    if parts == ("build.rs",):
        return "build"
    if len(parts) > 1 and parts[0] == "src":
        return "production"
    return KINDS.get(parts[0], "other") if len(parts) > 1 else "other"


def is_mod_rs(relative, directory):
    """Whether a file is a crate root or a `mod.rs`, whose inline blocks resolve from its own
    directory rather than from its stem's (R21)."""
    path = PurePosixPath(relative)
    if path.name in ROOT_FILES:
        return True
    if directory is None:
        return False
    parts = path.relative_to(directory).parts
    return (len(parts) == 2 and parts[0] in KINDS) or (
        len(parts) == 3 and parts[:2] == ("src", "bin")
    )


def closer(text, start):
    """The index just past the bracket that closes the `(`, `[` or `{` at `start`, counting all
    three kinds."""
    depth = 0
    for index in range(start, len(text)):
        if text[index] in "([{":
            depth += 1
        elif text[index] in ")]}":
            depth -= 1
            if not depth:
                return index + 1
    return len(text)


def top_parts(text):
    """`text` split at each comma outside brackets, each part with its offset in `text`."""
    parts, depth, begin = [], 0, 0
    for index, char in enumerate(text):
        depth += (char in "([{") - (char in ")]}")
        if char == "," and not depth:
            parts.append((text[begin:index], begin))
            begin = index + 1
    return [*parts, (text[begin:], begin)]


def shape(lexed, text, start, end):
    """The lexed span `start:end` with whitespace dropped and each literal as one SLOT, and each
    literal's value read back from `text` at its span, with whether it is raw."""
    pieces, values, at = [], [], start
    for literal in LITERAL.finditer(lexed, start, end):
        pieces += [lexed[at : literal.start()], SLOT]
        values.append((text[literal.start(1) : literal.end(1)], literal.group().startswith("r")))
        at = literal.end()
    return WHITESPACE.sub("", "".join([*pieces, lexed[at:end]])), values


def checked(value, raw):
    """`(value, None)`, or `(None, refusal)` for an escape or an absolute path (R25)."""
    if "\\" in value and not raw:
        return None, ESCAPE
    if value.startswith("/"):
        return None, ABSOLUTE
    return value, None


def located(base, value):
    """`(base/value with each .. taken, None)`, or `(None, refusal)` when it climbs above the
    repository root (R21, R25)."""
    target = lexical(f"{base}/{value}")
    return (None, ABOVE) if target is None else (target, None)


def attribute_paths(body, offset, spelled):
    """Each `path` an attribute's body sets, directly or through `cfg_attr` at any depth: `(what
    follows its "path=", the index of the literal there, whether cfg_attr sets it)`. `offset` is
    where `body` starts in `spelled`, the attribute's shape."""
    if body.startswith("path="):
        return [(body[len("path=") :], spelled[: offset + len("path=")].count(SLOT), False)]
    opening = "cfg_attr("
    if not (body.startswith(opening) and body.endswith(")")):
        return []
    found = []
    for part, at in top_parts(body[len(opening) : -1])[1:]:
        nested = attribute_paths(part, offset + len(opening) + at, spelled)
        found += [(rest, index, True) for rest, index, _through in nested]
    return found


def attribute_sites(lexed, text, start, close):
    """Each `path` one attribute sets, `(start, form, value, refusal)` (R20, R25)."""
    spelled, values = shape(lexed, text, start, close)
    inner = spelled.startswith("#!")
    opening = spelled.find("[") + 1
    found = []
    for rest, index, through in attribute_paths(spelled[opening:-1], opening, spelled):
        form = f"#{'!' if inner else ''}[{'cfg_attr(path)' if through else 'path'}]"
        if inner:
            found.append((start, form, None, INNER))
        elif rest != SLOT:
            found.append((start, form, None, NOT_LITERAL))
        else:
            found.append((start, form, *checked(*values[index])))
    return found


def include_target(path, directory, spelled, values):
    """An include's `(target, refusal)`: one literal from the file's directory, or
    `concat!(env!("CARGO_MANIFEST_DIR"), <literals>)` from the member's directory; and
    `concat!(env!("OUT_DIR"), ...)` names the member's build output (R21, R25)."""
    if spelled in (SLOT, f"{SLOT},"):
        value, refusal = checked(*values[0])
        return (None, refusal) if refusal else located(path.parent.as_posix(), value)
    if not BUILT.fullmatch(spelled) or values[0][0] not in (MANIFEST_DIR, OUT_DIR):
        return None, UNRESOLVED
    if directory is None:
        return None, ENVIRONMENT
    if values[0][0] == OUT_DIR:
        return BUILD_OUTPUT, None
    if any("\\" in value and not raw for value, raw in values[1:]):
        return None, ESCAPE
    target = lexical(directory + "".join(value for value, _raw in values[1:]))
    return (None, ABOVE) if target is None else (target, None)


def file_sites(relative, text, lexed, directory):
    """Each site in one file, `(start, form, target, refusal, brings)`: a `path` attribute's target
    through the inline blocks around it, and an include's from the file's directory (R20, R21).
    `brings` is whether the target is Rust source the compiler reads, a module's file or an
    `include!`'s; `directory` is the file's member directory, or None outside the members."""
    path = PurePosixPath(relative)
    blocks, found, claimed, at = [], [], set(), 0
    while match := ATTRIBUTE.search(lexed, at):
        run, at = [], match.start()
        while attribute := ATTRIBUTE.match(lexed, at):
            close = bracket_end(lexed, attribute.end() - 1)
            run += attribute_sites(lexed, text, attribute.start(), close)
            at = SPACE.match(lexed, close).end()
        visibility = VISIBILITY.match(lexed, at)
        block = INLINE_MOD.match(lexed, visibility.end() if visibility else at)
        if block:
            claimed.add(block.start())
            named = [value for _start, _form, value, refusal in run if refusal is None]
            opening = block.end() - 1
            name = named[0] if named else block.group(1)
            blocks.append((opening, brace_end(lexed, opening), name))
        found += [(*site, block is not None) for site in run]
    blocks += [
        (block.end() - 1, brace_end(lexed, block.end() - 1), block.group(1))
        for block in INLINE_MOD.finditer(lexed)
        if block.start() not in claimed
    ]
    sites = []
    for start, form, value, refusal, on_block in found:
        target = None
        if refusal is None:
            chain = [name for opening, end, name in sorted(blocks) if opening < start < end]
            base = path.parent
            if chain or on_block:
                stem = [] if is_mod_rs(relative, directory) else [path.stem]
                base = base.joinpath(*stem, *chain)
            target, refusal = located(base.as_posix(), value)
        sites.append((start, form, target, refusal, not on_block))
    for match in INCLUDE.finditer(lexed):
        end = closer(lexed, match.end() - 1) - 1
        spelled, values = shape(lexed, text, match.end(), end)
        target, refusal = include_target(path, directory, spelled, values)
        brings = match.group(1) == "include"
        sites.append((match.start(), f"{match.group(1)}!", target, refusal, brings))
    return sorted(sites, key=lambda site: site[0])


def census(root):
    """`(brought-in files, own sites, {reach key: its sites}, refusals)` over every member's Rust
    files and each file outside the members a site brings in (R20 to R22, R25)."""
    files = member_files(root)
    directories = sorted({directory for _name, directory in files.values()})
    aside = {path.relative_to(root).as_posix() for path in survey(root)[1]}
    pending = [(relative, *files[relative]) for relative in files]
    seen, brought, own, reaches, refusals = set(files), [], [], {}, []
    while pending:
        relative, name, directory = pending.pop(0)
        text = (root / relative).read_text(encoding="utf-8")
        lexed = lex(text)
        tested = strip(lexed, text)[0]
        kind = "outside" if directory is None else code_kind(relative, directory)
        for start, form, target, refusal, brings in file_sites(relative, text, lexed, directory):
            place = f"{relative}:{lexed.count(NEWLINE, 0, start) + 1}"
            if refusal is not None:
                refusals.append(f"{place}: {form}, {refusal}, is refused")
                continue
            if target == BUILD_OUTPUT or (directory is not None and owns(directory, target)):
                own.append(place)
                continue
            code = kind
            if kind == "production" and (relative in aside or tested[start] != lexed[start]):
                code = "test"
            source = relative if directory is None else name
            reaches.setdefault((source, target, code), []).append(place)
            outside = not any(owns(member, target) for member in directories)
            if brings and outside and target not in seen and (root / target).is_file():
                seen.add(target)
                brought.append(target)
                pending.append((target, None, None))
    return brought, own, reaches, refusals


def read_list(root):
    """`config/source-reaches.json` parsed, or None when it is absent."""
    path = root / LIST
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else None


def reach_rows(root):
    """The rows of `config/source-reaches.json`, or none when the list is absent."""
    data = read_list(root)
    rows = data.get("reaches") if isinstance(data, dict) else None
    return rows if isinstance(rows, list) else []


def label(key):
    """A reach or a row's key as a finding names it: `<from> -> <to> [<code>]`."""
    source, to, code = key
    return f"{source} -> {to} [{code}]"


def whole(count):
    """Whether a row's count is a whole number above 0."""
    return isinstance(count, int) and not isinstance(count, bool) and count > 0


def row_problems(row, number):
    """A row's unknown and missing keys, and each key holding the wrong kind of value (R23)."""
    if not isinstance(row, dict):
        return [f"row {number}: not an object"]
    found = [f"row {number}: unknown key {key}" for key in row if key not in ROW_KEYS]
    found += [f"row {number}: missing key {key}" for key in ROW_KEYS if key not in row]
    found += [
        f"row {number}: {key} {row[key]!r} is not a string"
        for key in NAMED
        if key in row and not isinstance(row[key], str)
    ]
    if "count" in row and not whole(row["count"]):
        found.append(f"row {number}: count {row['count']!r} is not a whole number above 0")
    return found


def usable(row):
    """Whether a row holds every key, each of its kind, so it has a reach's key to compare."""
    return (
        isinstance(row, dict)
        and all(key in row for key in ROW_KEYS)
        and all(isinstance(row[key], str) for key in NAMED)
        and whole(row["count"])
    )


def row_key(row):
    """A usable row's `(from, to, code)`."""
    return (row["from"], row["to"], row["code"])


def reach_findings(root):
    """Every way the members' sites and `config/source-reaches.json` disagree (R20 to R23, R25)."""
    return []


def admission_findings(root):
    """Each row whose `adr` names no single ADR file that holds its reach's token (R24)."""
    return []


WORKSPACE = '[workspace]\nmembers = ["crates/*"]\n'
TEST = "crates/bot/tests/a.rs"
LIB = "crates/bot/src/lib.rs"
TOOL = "tools/t/t.rs"
DATA = "crates/kernel/d.txt"
REACH = '#[path = "../../kernel/src/x.rs"]\nmod x;\n'
BRINGS = '#[path = "../../../tools/t/t.rs"]\nmod t;\n'
KEY = "bot -> crates/kernel/src/x.rs [test]"


def member(name):
    """A member manifest whose package is `deck-streak-<name>`."""
    return f'[package]\nname = "deck-streak-{name}"\n'


def reach_tree(files, rows=(), listing=None):
    """A workspace of the members `bot` and `kernel`, the Rust `files` `{path: text}`, and the list
    of `rows` (or `listing`, verbatim)."""
    tree = {
        "Cargo.toml": WORKSPACE,
        "crates/bot/Cargo.toml": member("bot"),
        "crates/kernel/Cargo.toml": member("kernel"),
        "crates/kernel/src/lib.rs": "",
        **files,
    }
    tree[LIST] = json.dumps({"reaches": list(rows)}) if listing is None else listing
    return tree


def reach(to="crates/kernel/src/x.rs", count=1, adr="ADR-1"):
    """One list row, from `bot` in test code."""
    return {"from": "bot", "to": to, "code": "test", "count": count, "adr": adr}


def unlisted(key, *places):
    """The finding a reach with no row reads as."""
    return f"{key}: {len(places)} site(s) and no row in {LIST}: {', '.join(places)}"


def refused(place, form, refusal):
    """The finding a refused site reads as."""
    return f"{place}: {form}, {refusal}, is refused"


def included(argument, path=TEST):
    """A file whose one line reads `include_str!(<argument>)`."""
    return {path: f"const D: &str = include_str!({argument});\n"}


def admitting(rows, decisions):
    """A tree holding only the list of `rows` and the ADR files `{name: text}`."""
    files = {LIST: json.dumps({"reaches": list(rows)})}
    for name, text in decisions.items():
        files[f"{DECISIONS}/{name}"] = text
    return files


class EachReachOutOfItsMemberIsAListedRow(unittest.TestCase):
    def test_every_reach_out_of_its_member_has_one_row_and_every_row_its_sites(self):
        self.assertEqual(reach_findings(REPO), [])
        examined("member Rust file(s)", member_files(REPO))
        brought, own, reaches, _refusals = census(REPO)
        listed("brought-in file(s)", brought)
        listed("own site(s)", own)
        listed("reach site(s)", [place for places in reaches.values() for place in places])
        listed("reach key(s)", reaches)
        listed("reach row(s)", reach_rows(REPO))
        two = REACH + '#[path = "../../kernel/src/x.rs"]\nmod y;\n'
        beside = REACH + '#[path = "../../kernel/src/y.rs"]\nmod y;\n'
        other = reach(to="crates/kernel/src/y.rs")
        unadmitted = {key: value for key, value in reach().items() if key != "adr"}
        plants = {
            "an unlisted reach": (reach_tree({TEST: REACH}), [unlisted(KEY, f"{TEST}:1")]),
            "a row whose count differs": (
                reach_tree({TEST: two}, rows=(reach(),)),
                [f"{KEY}: 2 site(s), its row counts 1: {TEST}:1, {TEST}:3"],
            ),
            "a row with no reach": (
                reach_tree({TEST: REACH}, rows=(reach(), other)),
                ["bot -> crates/kernel/src/y.rs [test]: a row with no reach"],
            ),
            "a duplicated row": (
                reach_tree({TEST: REACH}, rows=(reach(), reach())),
                [f"{KEY}: a row listed twice"],
            ),
            "a row out of order": (
                reach_tree({TEST: beside}, rows=(other, reach())),
                [f"{KEY}: a row out of order, sorted by from, to and code"],
            ),
            "an unknown key": (
                reach_tree({TEST: REACH}, rows=({**reach(), "note": "x"},)),
                ["row 1: unknown key note"],
            ),
            "a missing key": (
                reach_tree({TEST: REACH}, rows=(unadmitted,)),
                ["row 1: missing key adr", unlisted(KEY, f"{TEST}:1")],
            ),
            "a count that is not a whole number": (
                reach_tree({TEST: REACH}, rows=(reach(count="1"),)),
                ["row 1: count '1' is not a whole number above 0", unlisted(KEY, f"{TEST}:1")],
            ),
            "an absent list": (
                reach_tree({TEST: REACH}, listing="{}"),
                [f'{LIST}: the "reaches" list is absent'],
            ),
            "a workspace with no member Rust file (VOID)": (
                {"Cargo.toml": WORKSPACE, "crates/bot/Cargo.toml": member("bot")},
                ["examined 0 member Rust file(s): VOID"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(reach_findings, files), expected)

    def test_each_form_kind_and_path_resolves_and_an_unresolvable_path_is_refused(self):
        self.assertEqual(reach_findings(REPO), [])
        listed("refused site(s)", census(REPO)[3])
        bench, example = "crates/bot/benches/b.rs", "crates/bot/examples/e.rs"
        build, fuzz = "crates/bot/build.rs", "crates/bot/fuzz/f.rs"
        module, aside = "crates/bot/src/m.rs", "crates/bot/src/t.rs"
        x_rs = "bot -> crates/kernel/src/x.rs [production]"
        moved = "bot -> crates/kernel/x.rs [production]"
        read = 'const D: &str = include_str!("../../kernel/d.txt");\n'
        plants = {
            "an outer path attribute in test code": ({TEST: REACH}, [unlisted(KEY, f"{TEST}:1")]),
            "a cfg_attr path in production code": (
                {LIB: '#[cfg_attr(unix, path = "../../kernel/src/x.rs")]\nmod x;\n'},
                [unlisted(x_rs, f"{LIB}:1")],
            ),
            "include! in production code": (
                {LIB: 'include!("../../kernel/src/x.rs");\n'},
                [unlisted(x_rs, f"{LIB}:1")],
            ),
            "include_str! with a path prefix in a bench": (
                {bench: 'const D: &str = std::include_str!("../../kernel/d.txt");\n'},
                [unlisted(f"bot -> {DATA} [bench]", f"{bench}:1")],
            ),
            "include_bytes! in brackets in an example": (
                {example: 'const D: &[u8] = include_bytes!["../../kernel/d.txt"];\n'},
                [unlisted(f"bot -> {DATA} [example]", f"{example}:1")],
            ),
            "include_str! in braces in a build script": (
                {build: 'fn main() {\n    let _ = include_str!{"../kernel/d.txt"};\n}\n'},
                [unlisted(f"bot -> {DATA} [build]", f"{build}:2")],
            ),
            "a raw string elsewhere in the member": (
                {fuzz: 'const D: &str = include_str!(r#"../../kernel/d.txt"#);\n'},
                [unlisted(f"bot -> {DATA} [other]", f"{fuzz}:1")],
            ),
            "a cfg(test) item in production code": (
                {LIB: f"#[cfg(test)]\nmod tests {{\n    {read}}}\n"},
                [unlisted(f"bot -> {DATA} [test]", f"{LIB}:3")],
            ),
            "a module file declared under cfg(test)": (
                {LIB: "#[cfg(test)]\nmod t;\n", aside: read},
                [unlisted(f"bot -> {DATA} [test]", f"{aside}:1")],
            ),
            "a path attribute in an inline block of a crate root": (
                {LIB: 'mod a {\n    #[path = "../../../kernel/x.rs"]\n    mod b;\n}\n'},
                [unlisted(moved, f"{LIB}:2")],
            ),
            "a path attribute in an inline block of a module file": (
                {module: 'mod a {\n    #[path = "../../../../kernel/x.rs"]\n    mod b;\n}\n'},
                [unlisted(moved, f"{module}:2")],
            ),
            "an inline block's own path attribute": (
                {LIB: '#[path = "../../kernel"]\nmod k {\n    mod x;\n}\n'},
                [unlisted("bot -> crates/kernel [production]", f"{LIB}:1")],
            ),
            "a manifest-relative concat!": (
                included('concat!(env!("CARGO_MANIFEST_DIR"), "/../kernel/d.txt")'),
                [unlisted(f"bot -> {DATA} [test]", f"{TEST}:1")],
            ),
            "a file outside the members and the reach it brings in": (
                {TEST: BRINGS, **included('"../../crates/kernel/d.txt"', TOOL)},
                [
                    unlisted(f"bot -> {TOOL} [test]", f"{TEST}:1"),
                    unlisted(f"{TOOL} -> {DATA} [outside]", f"{TOOL}:1"),
                ],
            ),
            "a site inside its own member": (
                {TEST: '#[path = "support/s.rs"]\nmod s;\n', LIB: 'include!("inner.rs");\n'},
                [],
            ),
            "a climb back into its own member": (included('"../../bot/tests/d.txt"'), []),
            "the build output": (
                {LIB: 'include!(concat!(env!("OUT_DIR"), "/generated.rs"));\n'},
                [],
            ),
            "comments and literals": (
                {
                    LIB: '// include_str!("../../kernel/d.txt")\n'
                    '/* #[path = "../../kernel/x.rs"] */\n'
                    'const S: &str = "include!(\\"../../kernel/x.rs\\")";\n'
                },
                [],
            ),
            "a path attribute whose value is not a literal": (
                {TEST: '#[path = concat!("x", ".rs")]\nmod x;\n'},
                [refused(f"{TEST}:1", "#[path]", NOT_LITERAL)],
            ),
            "an inner path attribute": (
                {TEST: 'mod a {\n    #![path = "x"]\n}\n'},
                [refused(f"{TEST}:2", "#![path]", INNER)],
            ),
            "an argument the census cannot resolve": (
                included('env!("DATA")'),
                [refused(f"{TEST}:1", "include_str!", UNRESOLVED)],
            ),
            "a literal with an escape": (
                included('"d\\x2etxt"'),
                [refused(f"{TEST}:1", "include_str!", ESCAPE)],
            ),
            "an absolute path": (
                included('"/data/d.txt"'),
                [refused(f"{TEST}:1", "include_str!", ABSOLUTE)],
            ),
            "a path above the repository root": (
                included('"../../../../d.txt"'),
                [refused(f"{TEST}:1", "include_str!", ABOVE)],
            ),
            "an environment-relative path outside the members": (
                {TEST: BRINGS, TOOL: 'include!(concat!(env!("CARGO_MANIFEST_DIR"), "/x.rs"));\n'},
                [
                    refused(f"{TOOL}:1", "include!", ENVIRONMENT),
                    unlisted(f"bot -> {TOOL} [test]", f"{TEST}:1"),
                ],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(reach_findings, reach_tree(files)), expected)

    def test_every_reach_row_is_admitted_by_an_adr_that_names_it(self):
        self.assertEqual(admission_findings(REPO), [])
        listed("reach row(s)", reach_rows(REPO))
        token = "`bot -> crates/kernel/src/x.rs`"
        names = {"ADR-1-reaches.md": f"Admits {token}.\n"}
        plants = {
            "a row citing an absent ADR": (
                admitting([reach(adr="ADR-2")], names),
                [f"{KEY}: ADR-2 is held by 0 files in docs/decisions, not one"],
            ),
            "a row citing an ADR whose text lacks the token": (
                admitting([reach()], {"ADR-1-reaches.md": "Admits `bot -> crates/kernel`.\n"}),
                [f"{KEY}: docs/decisions/ADR-1-reaches.md does not hold {token}"],
            ),
            "a malformed adr": (
                admitting([reach(adr="ADR 1")], names),
                [f"{KEY}: adr 'ADR 1' is not ADR-<n>"],
            ),
            "an id two files share": (
                admitting([reach()], {**names, "ADR-1-again.md": f"Admits {token}.\n"}),
                [f"{KEY}: ADR-1 is held by 2 files in docs/decisions, not one"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(admission_findings, files), expected)


if __name__ == "__main__":
    unittest.main()
