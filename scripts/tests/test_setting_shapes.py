"""Every `Setting` implementation's `SHAPE` is pinned by its literal (SPEC-192).

A refusal names its setting and its shape (`the setting X is malformed: it must be <shape>`). A
test that compares against the constant (`Hour::SHAPE`) reads the code under test back to itself,
so a mutant that rewrites the words passes. The literal is the pin: a test that spells the words,
or a mutation row whose `find` is the constant's line, fails when they change.

The guard enumerates the implementations by walking `crates/*/src` (a git pathspec of that shape
matches nothing), reads each `const SHAPE` literal, and refuses one that is spelled in no test of
its crate (its `tests/`, or the `#[cfg(test)]` module of the implementation's own file) and in no
mutation row that targets the implementation's own file. It reads Rust source as the compiler
does: comments of both forms (`//` and nested `/* */`) do not count, and neither does an
implementation inside one, a `//` inside a string is not a comment, and only a `#[cfg(test)]`
module of the implementation's own file counts, not a line after it. A module is a test module
when its attributes keep it under `--cfg test` and remove it without, read in three-valued logic
in which every option but `test` is unknown (R8). An out-of-line one (`mod tests;`) is read only
from the one file rustc could read for it; an ambiguous or attribute-made choice is refused. A
literal that two implementations of one crate share is pinned only by a row on each
implementation's file.
"""

import functools
import itertools
import json
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

IMPL = re.compile(
    r"^\s*impl\b\s*(?:<[^{};]*>)?\s*(?:\$?[\w:]+::)?Setting\s+for\s+(\$?\w+)", re.MULTILINE
)
DECLARATION = re.compile(
    r"((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+(?:r#)?(\w+)\s*([;{])"
)
ATTRIBUTE = re.compile(r"#\[([^\]]*)\]")
INNER = re.compile(r"\s*#!\[([^\]]*)\]")
WORD = re.compile(r"(?:r#)?(\w+)|::|\S")
SPACE = " \t\n\r\x0b\x0c\x85\u200e\u200f\u2028\u2029\ufeff"
KLEENE = {
    "all": lambda values: False if False in values else None if None in values else True,
    "any": lambda values: True if True in values else None if None in values else False,
    "not": lambda values: None if len(values) != 1 or values[0] is None else not values[0],
}
TOKEN = re.compile(r"//|/\*|(?<![\w])b?r#*\"|\"|'")
RAW = re.compile(r"b?r(#*)\"")
PATH_WORD = re.compile(r"\bpath\b")
CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
SHAPE = re.compile(r'const\s+SHAPE\s*:\s*&\'static\s+str\s*=\s*("(?:[^"\\]|\\.)*")\s*;')
BANDS = REPO / "scripts" / "mutation-rows.d"


def implementations(root):
    """Every `impl Setting for X` under `crates/*/src`: (crate, file, name, quoted literal, span).

    The literal is the first `const SHAPE = "..."` after the impl line and before the next impl,
    and `span` is where that constant sits in the file, so the impl's own line can be excluded.
    """
    found = []
    for path in sorted((root / "crates").glob("*/src/**/*.rs")):
        text, _ = lexed(path.read_text(encoding="utf-8"))
        crate, *inside = path.relative_to(root / "crates").parts
        starts = [(m.start(), m.group(1)) for m in IMPL.finditer(text)]
        for index, (start, name) in enumerate(starts):
            end = starts[index + 1][0] if index + 1 < len(starts) else len(text)
            constant = SHAPE.search(text, start, end)
            found.append(
                (
                    crate,
                    Path(*inside),
                    name,
                    constant.group(1) if constant else None,
                    constant.span(1) if constant else None,
                )
            )
    return found


@functools.cache
def lexed(text):
    """`text` with its comments blanked, and `text` with its string and character literals blanked
    as well; both keep every offset. A `//` or `/*` inside a string is not a comment."""
    bare, skeleton = list(text), list(text)

    def blank(buffers, start, end):
        for buffer in buffers:
            buffer[start:end] = [c if c == "\n" else " " for c in text[start:end]]

    at = 0
    while (token := TOKEN.search(text, at)) is not None:
        start, kind = token.start(), token.group(0)
        if kind == "//":
            end = text.find("\n", start)
            end = len(text) if end == -1 else end
            blank((bare, skeleton), start, end)
        elif kind == "/*":
            depth, end = 1, start + 2
            while end < len(text) and depth:
                step = text[end : end + 2]
                depth += {"/*": 1, "*/": -1}.get(step, 0)
                end += 2 if step in ("/*", "*/") else 1
            blank((bare, skeleton), start, end)
        elif kind == "'":
            char = CHAR.match(text, start)
            end = char.end() if char else start + 1
            if char:
                blank((skeleton,), start, end)
        else:
            raw = RAW.match(text, start)
            if raw:
                close = text.find('"' + raw.group(1), raw.end())
                end = len(text) if close == -1 else close + 1 + len(raw.group(1))
            else:
                end = start + 1
                while end < len(text) and text[end] != '"':
                    end += 2 if text[end] == "\\" else 1
                end = min(end + 1, len(text))
            blank((skeleton,), start, end)
        at = max(end, start + 1)
    return "".join(bare), "".join(skeleton)


def predicate(words, at, test):
    """The configuration predicate at `words[at]` and the index after it. `test` is the one option
    known here; any other option or key-value is None (unknown), and `all`, `any` and `not` combine
    values in three-valued logic, so a True or False holds whatever else is configured."""
    word = words[at]
    if word in KLEENE and words[at + 1] == "(":
        values, at = [], at + 2
        while words[at] != ")":
            value, at = predicate(words, at, test)
            values.append(value)
            if words[at] not in (",", ")"):
                raise ValueError(words[at])
            at += words[at] == ","
        return KLEENE[word](values), at + 1
    if not word.isidentifier():
        raise ValueError(word)
    if words[at + 1] == "=":
        return None, at + 2
    return (test if word == "test" else None), at + 1


def applied(words):
    """The attributes a `cfg_attr` applies, split at its top-level commas."""
    attributes, depth = [[]], 0
    for word in words:
        depth += (word in ("(", "[", "{")) - (word in (")", "]", "}"))
        if word == "," and depth == 0:
            attributes.append([])
        else:
            attributes[-1].append(word)
    return [attribute for attribute in attributes if attribute]


def condition(words, test):
    """Whether one attribute keeps its item: a `cfg` when its predicate holds, a `cfg_attr` when its
    predicate fails or every attribute it applies keeps the item, and any other attribute always."""
    if words[0] not in ("cfg", "cfg_attr"):
        return True
    if words[1] != "(" or words[-1] != ")":
        raise ValueError(words)
    value, at = predicate(words, 2, test)
    if words[0] == "cfg":
        if at != len(words) - 1:
            raise ValueError(words)
        return value
    if words[at] != ",":
        raise ValueError(words)
    kept = [condition(attribute, test) for attribute in applied(words[at + 1 : -1])]
    return KLEENE["any"]([KLEENE["not"]([value]), KLEENE["all"](kept)])


def test_only(attributes):
    """True when `attributes` keep a module under `--cfg test` and remove it without, whatever
    else is configured (R8). A malformed attribute, or a run that could not be read whole (None),
    decides nothing, so the module is not read."""
    if attributes is None:
        return False
    words = [[word.group(1) or word.group(0) for word in WORD.finditer(a)] for a in attributes]
    try:
        held = [KLEENE["all"]([condition(each, test) for each in words]) for test in (True, False)]
    except (IndexError, ValueError):
        return False
    return held == [True, False]


def attributes(skeleton, declaration, body, at):
    """The attributes that decide whether `declaration` is compiled: its outer run, and the inner
    attributes that open its body (`body` from `at`). None when either run may not be whole: the
    outer one when something other than an item's end comes before it, the inner one when an
    attribute is left unread."""
    if skeleton[: declaration.start()].rstrip(SPACE)[-1:] not in ("", ";", "}"):
        return None
    inner = []
    while (attribute := INNER.match(body, at)) is not None:
        inner.append(attribute.group(1))
        at = attribute.end()
    if body[at:].lstrip(SPACE).startswith(("#!", "]")):
        return None
    return ATTRIBUTE.findall(declaration.group(1)) + inner


def cfg_test_spans(skeleton):
    """The spans of the file's top-level inline test modules (`mod name { ... }` that only
    `--cfg test` compiles), braces matched outside comments and literals. One declared inside
    another module is not read (#433)."""
    spans = []
    for block in DECLARATION.finditer(skeleton):
        if block.group(3) != "{":
            continue
        if skeleton[: block.start()].count("{") != skeleton[: block.start()].count("}"):
            continue
        if not test_only(attributes(skeleton, block, skeleton, block.end())):
            continue
        depth, end = 1, block.end()
        while end < len(skeleton) and depth:
            depth += {"{": 1, "}": -1}.get(skeleton[end], 0)
            end += 1
        spans.append((block.start(), end))
    return spans


def out_of_line(own):
    """The file of each out-of-line test module (`mod name;`) that `own` declares, read only when
    rustc's choice is not in doubt (R8): of every file rustc could read for it (`name.rs` or
    `name/mod.rs`, in `own`'s module directory or beside `own`, since a crate root, a `src/bin`
    file, a `mod.rs` and a file an attribute loaded all read their modules beside themselves),
    exactly one exists, no attribute of the declaration carries `path` in any spelling (`#[path]`,
    a raw string, `cfg_attr` under any predicate), and the declaration's attributes with that
    file's inner ones make it a test module. Any other declaration is not read, so a shape only it
    spells is refused. A declaration inside an inline module or a block is not followed (#433)."""
    bare, skeleton = lexed(own.read_text(encoding="utf-8"))
    files = []
    for module in DECLARATION.finditer(skeleton):
        if skeleton.count("{", 0, module.start()) != skeleton.count("}", 0, module.start()):
            continue
        if module.group(3) != ";":
            continue
        if PATH_WORD.search(module.group(1)):
            continue
        name = module.group(2)
        found = [
            path
            for folder in (own.with_suffix(""), own.parent)
            for path in (folder / f"{name}.rs", folder / name / "mod.rs")
            if path.is_file()
        ]
        if len(found) == 1:
            _, file = lexed(found[0].read_text(encoding="utf-8"))
            if test_only(attributes(skeleton, module, file, 0)):
                files += found
    return files


def spelled_elsewhere(root, crate, file, literal, own_span):
    """True when `literal` is spelled, quoted, in the crate's tests or inside a `#[cfg(test)]`
    module of the implementation's own source file, outside a comment, and never at any `SHAPE`
    constant."""
    base = root / "crates" / crate
    constants = {
        (path, span)
        for holder, path, _, _, span in implementations(root)
        if holder == crate and span
    }
    candidates = sorted((base / "tests").rglob("*.rs")) + [base / file] + out_of_line(base / file)
    for path in candidates:
        bare, skeleton = lexed(path.read_text(encoding="utf-8"))
        spans = cfg_test_spans(skeleton) if path == base / file else [(0, len(bare))]
        at = bare.find(literal)
        while at != -1:
            inside = any(start <= at and at + len(literal) <= end for start, end in spans)
            if inside and (path.relative_to(base), (at, at + len(literal))) not in constants:
                return True
            at = bare.find(literal, at + 1)
    return False


def rows_pinning(root, crate, file, literal):
    """Ids of the mutation rows that target this implementation's file and whose `find` holds the
    literal."""
    ids = []
    for band in sorted((root / "scripts" / "mutation-rows.d").glob("*.json")):
        table = json.loads(band.read_text(encoding="utf-8")).get("tables", {})
        for row in table.get("MUTATIONS", []):
            if row[1] == crate and row[2] == file.as_posix() and literal in row[3]:
                ids.append(row[0])
    return ids


def unpinned(root):
    """The implementations whose literal no test spells and no row of their file finds. A literal
    that two implementations of one crate share is pinned only by a row of each one's file."""
    found = implementations(root)
    holders = {}
    for crate, _, _, literal, _ in found:
        holders[(crate, literal)] = holders.get((crate, literal), 0) + 1
    return [
        f"{crate}::{name} ({file.as_posix()}) {literal}"
        for crate, file, name, literal, span in found
        if literal is None
        or not (
            (holders[(crate, literal)] == 1 and spelled_elsewhere(root, crate, file, literal, span))
            or rows_pinning(root, crate, file, literal)
        )
    ]


class EverySettingShapeIsPinnedByItsLiteral(unittest.TestCase):
    def test_every_setting_impl_has_a_shape_literal_that_a_test_or_a_row_pins(self):
        impls = examined("Setting impl(s)", implementations(REPO))
        self.assertGreater(len(impls), 0)
        self.assertEqual(unpinned(REPO), [], f"{len(impls)} impl(s) examined")


IMPL_TEXT = 'impl Setting for Depth {\n    const SHAPE: &\'static str = "a whole depth";\n}\n'


class TheGuardJudgesAPlantedTree(unittest.TestCase):
    """The guard's own arms, on a tree built at run time: one crate, one implementation."""

    def tree(self, test_text=None, rows=()):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        (root / "crates" / "demo" / "src").mkdir(parents=True)
        (root / "crates" / "demo" / "tests").mkdir()
        (root / "scripts" / "mutation-rows.d").mkdir(parents=True)
        (root / "crates" / "demo" / "src" / "depth.rs").write_text(IMPL_TEXT, encoding="utf-8")
        if test_text is not None:
            (root / "crates" / "demo" / "tests" / "depth.rs").write_text(
                test_text, encoding="utf-8"
            )
        band = {"tables": {"MUTATIONS": list(rows)}}
        (root / "scripts" / "mutation-rows.d" / "S00000-S00099.json").write_text(
            json.dumps(band), encoding="utf-8"
        )
        return root

    def test_a_shape_only_its_own_constant_spells_is_refused_by_crate_impl_and_literal(self):
        found = unpinned(self.tree())
        self.assertEqual(found, ['demo::Depth (src/depth.rs) "a whole depth"'])
        double = self.tree()
        (double / "crates" / "demo" / "src" / "depth.rs").write_text(
            "#[cfg(test)]\nmod tests {\n" + IMPL_TEXT + "}\n", encoding="utf-8"
        )
        self.assertEqual(unpinned(double), ['demo::Depth (src/depth.rs) "a whole depth"'])
        twin = self.tree()
        module = "#[cfg(test)]\nmod t {"
        pad = IMPL_TEXT.index('"a whole depth"') - len(module) - len("const Y: &str = ")
        module += " " * pad + 'const Y: &str = "a whole depth";\n}\n'
        (twin / "crates" / "twin" / "src").mkdir(parents=True)
        (twin / "crates" / "twin" / "src" / "depth.rs").write_text(
            module + IMPL_TEXT.replace("Depth", "Twin"), encoding="utf-8"
        )
        self.assertEqual(unpinned(twin), ['demo::Depth (src/depth.rs) "a whole depth"'])

    def test_a_shape_a_test_of_the_crate_spells_is_pinned(self):
        root = self.tree('const X: &str = "a whole depth";')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        deeper = self.tree()
        (deeper / "crates" / "demo" / "tests" / "sub").mkdir()
        (deeper / "crates" / "demo" / "tests" / "sub" / "depth.rs").write_text(
            'const X: &str = "a whole depth";', encoding="utf-8"
        )
        self.assertEqual(unpinned(deeper), [])
        quoted = self.tree('const X: &str = "a \\"q\\" shape";')
        (quoted / "crates" / "demo" / "src" / "depth.rs").write_text(
            IMPL_TEXT.replace('"a whole depth"', '"a \\"q\\" shape"'), encoding="utf-8"
        )
        self.assertEqual(unpinned(quoted), [])

    def test_a_shape_the_test_only_reads_back_from_the_constant_is_refused(self):
        self.assertEqual(len(unpinned(self.tree("let shape = Depth::SHAPE;"))), 1)

    def test_a_shape_a_row_of_the_implementations_own_file_finds_is_pinned(self):
        row = ["S00001-DEPTH", "demo", "src/depth.rs", 'str = "a whole depth";', "", "t::k", "d"]
        root = self.tree(rows=[row])
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])

    def test_a_row_of_another_file_does_not_pin_it(self):
        row = ["S00001-DEPTH", "demo", "src/other.rs", 'str = "a whole depth";', "", "t::k", "d"]
        self.assertEqual(len(unpinned(self.tree(rows=[row]))), 1)
        row = ["S00001-DEPTH", "demo", "src/depth.rs", 'str = "a whole width";', "", "t::k", "d"]
        self.assertEqual(len(unpinned(self.tree(rows=[row]))), 1)
        row = ["S00001-DEPTH", "else", "src/depth.rs", 'str = "a whole depth";', "", "t::k", "d"]
        self.assertEqual(len(unpinned(self.tree(rows=[row]))), 1)

    def src(self, root, text):
        (root / "crates" / "demo" / "src" / "more.rs").write_text(text, encoding="utf-8")
        return root

    def test_a_generic_implementation_is_examined(self):
        root = self.src(
            self.tree('const X: &str = "a whole depth";'),
            'impl<T> Setting for Wide<T> {\n    const SHAPE: &\'static str = "a whole width";\n}\n',
        )
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(unpinned(root), ['demo::Wide (src/more.rs) "a whole width"'])

    def test_a_shape_only_a_production_line_of_the_crate_spells_is_refused(self):
        code = 'pub fn depth() -> &\'static str {\n    "a whole depth"\n}\n'
        root = self.src(self.tree(), '/// Reads "a whole depth".\n' + code)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), ['demo::Depth (src/depth.rs) "a whole depth"'])

    def test_a_shape_two_implementations_of_one_crate_share_needs_a_row_of_each_file(self):
        root = self.src(
            self.tree('const X: &str = "a whole depth";'),
            'impl Setting for Deep {\n    const SHAPE: &\'static str = "a whole depth";\n}\n',
        )
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(len(unpinned(root)), 2)
        named = self.src(
            self.tree('const X: &str = "a whole depth";\nconst Y: &str = "a whole width";'),
            "impl Setting for Named {\n    const SHAPE: &'static str = WIDTH;\n}\n"
            'impl Setting for Wide {\n    const SHAPE: &\'static str = "a whole width";\n}\n',
        )
        try:
            found = unpinned(named)
        except TypeError as error:
            self.fail(f"a shape with no literal must be refused, not crash the guard: {error}")
        self.assertEqual(found, ["demo::Named (src/more.rs) None"])

    def test_a_shape_only_a_comment_spells_is_refused(self):
        root = self.tree('// "a whole depth"\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)
        quote = self.tree('let q = \'"\'; // "a whole depth"\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(quote)), 1)
        escaped = self.tree('let q = \'\\"\'; // "a whole depth"\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(escaped)), 1)


class TheGuardReadsRustSource(unittest.TestCase):
    """Macro impls, comments, strings and test modules as the compiler reads them (A12)."""

    tree = TheGuardJudgesAPlantedTree.tree
    src = TheGuardJudgesAPlantedTree.src

    def own(self, root, text):
        (root / "crates" / "demo" / "src" / "depth.rs").write_text(text, encoding="utf-8")
        return root

    def macro(self, trait):
        body = 'impl TRAIT for $t {\n    const SHAPE: &\'static str = "a whole width";\n}\n'
        return self.src(self.tree('const X: &str = "a whole depth";'), body.replace("TRAIT", trait))

    def test_a_macro_implementation_is_examined(self):
        root = self.macro("Setting")
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(unpinned(root), ['demo::$t (src/more.rs) "a whole width"'])

    def test_a_macro_implementation_through_dollar_crate_is_examined(self):
        root = self.macro("$crate::settings::Setting")
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(unpinned(root), ['demo::$t (src/more.rs) "a whole width"'])

    def test_a_shape_the_own_files_test_module_spells_is_pinned(self):
        module = '#[cfg(test)]\nmod tests {\n    const X: &str = "a whole depth";\n}\n'
        root = self.own(self.tree(), IMPL_TEXT + module)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        for header in (
            "mod tests {\n    fn t() {}\n",
            "#[allow(unused)]\nmod tests {\n",
            "pub mod tests {\n",
        ):
            module = "#[cfg(test)]\n" + header + '    const X: &str = "a whole depth";\n}\n'
            self.assertEqual(unpinned(self.own(self.tree(), IMPL_TEXT + module)), [], header)

    def test_a_shape_a_test_spells_after_a_url_on_its_line_is_pinned(self):
        root = self.tree('let (url, shape) = ("http://host/", "a whole depth");')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        raw = self.tree('let (r, shape) = (r#"http://host/"#, "a whole depth");')
        self.assertEqual(unpinned(raw), [])
        escaped = self.tree('let (e, shape) = ("a \\" //", "a whole depth");')
        self.assertEqual(unpinned(escaped), [])
        hashed = self.tree('let (h, shape) = (r#"a" // "#, "a whole depth");')
        self.assertEqual(unpinned(hashed), [])
        byte = self.tree('let (b, shape) = (br##"a"# // " // "##, "a whole depth");')
        self.assertEqual(unpinned(byte), [])

    def test_a_shape_only_a_block_comment_spells_is_refused(self):
        root = self.tree('/* "a whole depth" */\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)
        nested = self.tree('/* a /* b */ "a whole depth" */\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(nested)), 1)
        slashed = self.tree('/*/ "a whole depth" */\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(slashed)), 1)

    def test_a_production_line_after_the_own_files_test_module_is_refused(self):
        module = '#[cfg(test)]\nmod tests {}\n\npub const X: &str = "a whole depth";\n'
        root = self.own(self.tree(), IMPL_TEXT + module)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)
        braces = "#[cfg(test)]\nmod tests {\n    /* { */\n    // {\n"
        braces += "    const C: char = '{';\n    const S: &str = \"{\";\n}\n"
        root = self.own(self.tree(), IMPL_TEXT + braces + 'pub const X: &str = "a whole depth";\n')
        self.assertEqual(len(unpinned(root)), 1)

    def test_a_test_attribute_on_a_use_opens_no_test_module(self):
        text = (
            "#[cfg(test)]\nuse std::fmt;\n" + IMPL_TEXT + 'pub const X: &str = "a whole depth";\n'
        )
        root = self.own(self.tree(), text)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)


class TheGuardReadsOutOfLineTestModules(unittest.TestCase):
    """A `#[cfg(test)] mod name;` of the implementation's own file, read where the compiler looks
    for it (A13)."""

    tree = TheGuardJudgesAPlantedTree.tree
    SPELLING = 'const X: &str = "a whole depth";\n'

    def declared(self, declaration, *files, own="depth.rs", text=IMPL_TEXT):
        """A tree whose `src/<own>` declares a module, and the module files written at `files`."""
        root = self.tree()
        src = root / "crates" / "demo" / "src"
        if own != "depth.rs":
            (src / "depth.rs").unlink()
        (src / own).parent.mkdir(parents=True, exist_ok=True)
        (src / own).write_text(text + declaration, encoding="utf-8")
        for name, body in files:
            (src / name).parent.mkdir(parents=True, exist_ok=True)
            (src / name).write_text(body, encoding="utf-8")
        return root

    def test_a_module_file_beside_the_own_file_is_its_test_module(self):
        root = self.declared("#[cfg(test)]\nmod tests;\n", ("depth/tests.rs", self.SPELLING))
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        public = "#[cfg(test)]\npub(crate) mod tests;\n"
        self.assertEqual(unpinned(self.declared(public, ("depth/tests.rs", self.SPELLING))), [])

    def test_a_module_directory_with_a_mod_file_is_its_test_module(self):
        root = self.declared("#[cfg(test)]\nmod tests;\n", ("depth/tests/mod.rs", self.SPELLING))
        own = root / "crates" / "demo" / "src" / "depth.rs"
        self.assertEqual(out_of_line(own), [own.with_suffix("") / "tests" / "mod.rs"])
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])

    def test_a_module_whose_file_an_attribute_chooses_is_not_read(self):
        for declaration in (
            '#[cfg(test)]\n#[path = "words/shape.rs"]\nmod tests;\n',
            '#[path = "words/shape.rs"]\n#[cfg(test)]\nmod tests;\n',
            '#[cfg(test)]\n#[path="words/shape.rs"]\nmod tests;\n',
        ):
            root = self.declared(declaration, ("words/shape.rs", self.SPELLING))
            self.assertEqual(len(implementations(root)), 1)
            self.assertEqual(len(unpinned(root)), 1, declaration)

    def test_the_module_of_a_lib_file_is_read_from_the_crates_src_directory(self):
        root = self.declared(
            "#[cfg(test)]\nmod tests;\n", ("tests.rs", self.SPELLING), own="lib.rs"
        )
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        for own, name in (("main.rs", "tests.rs"), ("a/mod.rs", "a/tests.rs")):
            root = self.declared("#[cfg(test)]\nmod tests;\n", (name, self.SPELLING), own=own)
            self.assertEqual(len(implementations(root)), 1, own)
            self.assertEqual(unpinned(root), [], own)

    def test_a_file_that_is_no_declared_test_module_is_not_read_as_one(self):
        elsewhere = ("depth/tests.rs", self.SPELLING)
        for declaration in (
            "",
            "mod tests;\n",
            "#[cfg(test)]\nmod other;\n",
            "mod inner {\n    #[cfg(test)]\n    mod tests;\n}\n",
            "#[allow(dead_code)]\nmod tests;\n",
            'const D: &str = "#[cfg(test)] mod tests;";\n',
        ):
            root = self.declared(declaration, elsewhere)
            self.assertEqual(len(implementations(root)), 1)
            self.assertEqual(len(unpinned(root)), 1, declaration)
        root = self.declared("#[cfg(test)]\nmod tests;\n", ("depth/tests.rs", "let shape = 1;\n"))
        self.assertEqual(len(unpinned(root)), 1)
        other = '#[path = "words.rs"]\nmod words;\n#[cfg(test)]\nmod tests;\n'
        self.assertEqual(len(unpinned(self.declared(other, ("words.rs", self.SPELLING)))), 1)
        path = '#[cfg(test)]\n#[path = "words/shape.rs"]\nmod tests;\n'
        root = self.declared(path, ("words/shape.rs", "let shape = 1;\n"), elsewhere)
        self.assertEqual(len(unpinned(root)), 1)

    KINDS = (
        ("lib.rs", (), "tests.rs"),
        ("main.rs", (), "tests.rs"),
        ("bin/tool.rs", (), "bin/tests.rs"),
        ("a/depth.rs", (("lib.rs", "mod a;\n"), ("a/mod.rs", "mod depth;\n")), "a/depth/tests.rs"),
        ("a/mod.rs", (("lib.rs", "mod a;\n"),), "a/tests.rs"),
        ("x/depth.rs", (("lib.rs", '#[path = "x/depth.rs"]\nmod depth;\n'),), "x/tests.rs"),
    )
    ATTRIBUTES = (
        "",
        '#[path = "t.rs"]\n',
        '#[path="t.rs"]\n',
        '#[ path = "t.rs" ]\n',
        '#[path = r"t.rs"]\n',
        '#[cfg_attr(test, path = "t.rs")]\n',
        '#[cfg_attr(all(test, unix), path = "t.rs")]\n',
    )
    LEAVES = ("test", "x", 'feature = "slow"')
    UNKNOWN = re.compile(r"\bx\b|\bfeature\b")
    CARRIERS = (
        'const _: &str = "{}";',
        'const _: &[u8] = b"{}";',
        "const _: char = '{}';",
        'const _: &str = r#"{}"#;',
        "/* {} */",
        "// {}",
    )

    def predicates(self):
        """Every predicate to depth two over the leaves, built from the guard's own operators, each
        at every arity it evaluates (so `not` takes exactly one argument)."""
        arity = {op: [n for n in range(3) if KLEENE[op]([True] * n) is not None] for op in KLEENE}

        def calls(op, parts, n):
            return [f"{op}({', '.join(args)})" for args in itertools.product(parts, repeat=n)]

        one = [*self.LEAVES] + [
            p for op in KLEENE for n in arity[op] for p in calls(op, self.LEAVES, n)
        ]
        two = [f"{op}({p})" for op in KLEENE if 1 in arity[op] for p in one]
        two += [
            f"{op}({p}, {q})" for op in KLEENE if 2 in arity[op] for p in one for q in self.LEAVES
        ]
        return one, one + two

    def members(self):
        """R8's population, generated: (case, files, root, own, decided). Every module source rustc
        could compile holds a probe naming its place, and the declaring file ends with one that
        every build compiles, so rustc names each source it compiles. A decided member names no
        option but `test` and leaves the guard one candidate file, so the guard must read exactly
        what rustc compiles only under test; any other member it may refuse.
        """
        found = []

        def add(case, files, own, decided, root=None):
            files = {**files, own: files[own] + probe("own")}
            files = {
                path: text.replace("PROBE ", f"PROBE {len(found)} ") for path, text in files.items()
            }
            found.append((case, files, root or own, own, decided))

        def probe(place):
            return f'compile_error!("PROBE {place}");\n'

        gate = f"#[cfg({self.LEAVES[0]})]\n"
        one, every = self.predicates()
        negated = [*self.LEAVES, *(f"not({leaf})" for leaf in self.LEAVES)]
        attributes = [f"cfg({p})" for p in every]
        attributes += [f"cfg_attr({p}, cfg({q}))" for p in negated for q in negated]
        attributes += [f"cfg_attr({p}, allow(dead_code))" for p in self.LEAVES]
        runs = []
        for a in attributes:
            known = not self.UNKNOWN.search(a)
            runs += [(f"#[{a}]\n{gate}", "", known), (f"{gate}#[{a}]\n", "", known)]
            runs += [(gate, f"#![{a}]\n", known), (f"#[{a}]\n", "", known)]
        bracket = 'doc = concat!["a"]'
        for p in one:
            runs += [(f"#[cfg({p})]\n#[{bracket}]\n{gate}", "", False)]
            runs += [(gate, f"#![{bracket}]\n#![cfg({p})]\n", False)]
        runs += [(gate + vis, "", True) for vis in ("pub ", "pub(crate) ")]
        for outer, inner, known in runs:
            for name in ("tests", "r#tests") if outer.endswith(" ") else ("tests",):
                declaration = f"{outer}mod {name}"
                for place, decides in (("tests.rs", known), ("lib/tests.rs", False)):
                    files = {"lib.rs": declaration + ";\n", place: inner + probe(place)}
                    add(f"lib.rs {declaration}; {inner!r} at {place}", files, "lib.rs", decides)
                files = {"lib.rs": f"{declaration} {{\n{inner}{probe('inline')}}}\n"}
                files["tests.rs"] = probe("tests.rs")
                add(f"lib.rs {declaration} {{ {inner!r} }}", files, "lib.rs", known)
        for wrapper in ("", "#[cfg(any())]\n"):
            for carrier in ("", *(c.format(b) for c in self.CARRIERS for b in "{}")):
                opened = f"{wrapper}mod inner {{\n{carrier}\n{gate}mod tests"
                files = {"lib.rs": opened + ";\n}\n", "inner/tests.rs": probe("inner/tests.rs")}
                add(f"{opened}; }}", {**files, "tests.rs": probe("tests.rs")}, "lib.rs", False)
                inline = f"{opened} {{\n{probe('inline')}}}\n}}\n"
                add(f"{opened} {{ }} }}", {"lib.rs": inline}, "lib.rs", False)
        for own, extra, default in self.KINDS:
            stem, folder = Path(own).with_suffix(""), Path(own).parent
            places = [p / leaf for p in (stem, folder) for leaf in ("tests.rs", "tests/mod.rs")]
            root = next((path for path, _ in extra if path == "lib.rs"), own)
            for attribute in self.ATTRIBUTES:
                chosen = (folder / "t.rs").as_posix() if attribute else default
                stale = [p.as_posix() for p in places if p.as_posix() != chosen]
                for misplaced in (None, *stale):
                    files = {**dict(extra), own: f"{gate}{attribute}mod tests;\n"}
                    files.update({place: probe(place) for place in (chosen, misplaced) if place})
                    decided = not attribute and misplaced is None
                    add(f"{own} {attribute!r} stale={misplaced}", files, own, decided, root)
        return found

    def compiled(self, src, count):
        """What rustc compiles of each member under every setting of `test`, `x` and a feature:
        (compiled in every run with `--cfg test`, compiled in any run without, members in error)."""
        rustc = shutil.which("rustc")
        if rustc is None:
            self.fail("rustc is not on PATH, so R8's oracle cannot run: this test never skips")
        switches = (("--cfg", "test"), ("--cfg", "x"), ("--cfg", 'feature="slow"'))
        runs = []
        for chosen in itertools.product((False, True), repeat=len(switches)):
            flags = [flag for on, pair in zip(chosen, switches) if on for flag in pair]
            log = (src.parent / f"{len(runs)}.log").open("w", encoding="utf-8")
            command = [rustc, "--edition", "2024", "--crate-type", "lib", "-A", "warnings"]
            command += ["--error-format=json", "--emit=dep-info", "-o", f"{log.name}.d", *flags]
            process = subprocess.Popen([*command, str(src / "lib.rs")], cwd=REPO, stderr=log)
            runs.append((chosen[0], process, log))
        under, without, broken = [None] * count, [set() for _ in range(count)], set()
        for test, process, log in runs:
            process.wait()
            log.close()
            places = [set() for _ in range(count)]
            for line in Path(log.name).read_text(encoding="utf-8").splitlines():
                message = json.loads(line) if line.startswith("{") else {}
                if message.get("level") != "error":
                    continue
                words = message["message"].split(" ", 2)
                if words[0] == "PROBE":
                    places[int(words[1])].add(words[2])
                    continue
                owners = {re.search(r"/m(\d+)/", s["file_name"]) for s in message["spans"]}
                if None in owners or not owners and not words[0].startswith("aborting"):
                    self.fail(f"rustc failed outside the population: {message['message']}")
                broken |= {int(owner.group(1)) for owner in owners}
            for index, compiled in enumerate(places):
                if test:
                    under[index] = compiled if under[index] is None else under[index] & compiled
                else:
                    without[index] |= compiled
        return under, without, broken

    def test_every_module_file_choice_is_read_from_rustcs_file_or_refused(self):
        """R8's class, generated from the guard's grammar and judged by rustc: the guard reads only
        a source that rustc compiles under `--cfg test` and never without it, whatever else is
        configured, and a decided member exactly that source; otherwise it refuses."""
        members = self.members()
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        src = Path(directory.name) / "src"
        roots = []
        for index, (_, files, root, _, _) in enumerate(members):
            for path, text in files.items():
                (src / f"m{index}" / path).parent.mkdir(parents=True, exist_ok=True)
                (src / f"m{index}" / path).write_text(text, encoding="utf-8")
            roots.append(f'#[path = "m{index}/{root}"]\nmod m{index};\n')
        (src / "lib.rs").write_text("".join(roots), encoding="utf-8")
        under, without, broken = self.compiled(src, len(members))
        wrong, judged = [], []
        for index, (case, _, _, own, decided) in enumerate(members):
            if decided and index in broken:
                wrong.append(f"{case}: a decided member does not compile")
            if index in broken:
                continue
            judged.append(case)
            base = src / f"m{index}"
            text = (base / own).read_text(encoding="utf-8")
            reads = {path.relative_to(base).as_posix() for path in out_of_line(base / own)}
            spans = cfg_test_spans(lexed(text)[1])
            probes = re.finditer(r'PROBE \d+ (\S+)"', text)
            reads |= {p.group(1) for p in probes for s, e in spans if s <= p.start() < e}
            only = under[index] - without[index]
            if not reads <= only or (decided and reads != only):
                wrong.append(f"{case}: reads {sorted(reads)}, only under test {sorted(only)}")
        examined("R8 member(s) judged against rustc", judged)
        self.assertGreater(len(judged), 0)
        self.assertEqual(wrong[:1], [], f"{len(wrong)} of {len(members)} members")


class TheGuardIgnoresAnImplementationInAComment(unittest.TestCase):
    """An `impl Setting for` inside a comment is no implementation (A14)."""

    tree = TheGuardJudgesAPlantedTree.tree
    src = TheGuardJudgesAPlantedTree.src

    def test_a_block_comment_holding_an_impl_is_not_examined(self):
        ghost = 'impl Setting for Ghost {\n    const SHAPE: &\'static str = "a ghost shape";\n}\n'
        pinned = self.tree('const X: &str = "a whole depth";')
        for comment in ("/*\n" + ghost + "*/\n", "/* a /* nested */\n" + ghost + "*/\n"):
            root = self.src(pinned, comment)
            self.assertEqual(len(implementations(root)), 1)
            self.assertEqual(unpinned(root), [])
        line = self.src(pinned, "// " + ghost.replace("\n", "\n// "))
        self.assertEqual(len(implementations(line)), 1)
        live = self.src(pinned, ghost)
        self.assertEqual(unpinned(live), ['demo::Ghost (src/more.rs) "a ghost shape"'])
        after = self.src(pinned, "const A: u8 = 1; /* a\n b */ " + ghost)
        self.assertEqual(unpinned(after), ['demo::Ghost (src/more.rs) "a ghost shape"'])


if __name__ == "__main__":
    unittest.main()
