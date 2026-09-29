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
module of the implementation's own file counts, not a line after it. An out-of-line module
(`#[cfg(test)] mod tests;`) is that file's own test module, read only from the one file rustc
could read for it; an ambiguous or attribute-made choice is refused. A literal that two implementations of one crate share is pinned only by a row on
each implementation's file.
"""

import functools
import json
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

IMPL = re.compile(
    r"^\s*impl\b\s*(?:<[^{};]*>)?\s*(?:\$?[\w:]+::)?Setting\s+for\s+(\$?\w+)", re.MULTILINE
)
TEST_MODULE = re.compile(
    r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{"
)
TOKEN = re.compile(r"//|/\*|(?<![\w])b?r#*\"|\"|'")
RAW = re.compile(r"b?r(#*)\"")
OUT_OF_LINE = re.compile(r"((?:#\[[^\]]*\]\s*)+)(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
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


def cfg_test_spans(skeleton):
    """The spans of the `#[cfg(test)] mod name { ... }` blocks, braces matched outside comments and
    literals."""
    spans = []
    for module in TEST_MODULE.finditer(skeleton):
        depth, end = 1, module.end()
        while end < len(skeleton) and depth:
            depth += {"{": 1, "}": -1}.get(skeleton[end], 0)
            end += 1
        spans.append((module.start(), end))
    return spans


def out_of_line(own):
    """The file of each `#[cfg(test)] mod name;` that `own` declares, read only when rustc's choice
    is not in doubt (R8): of every file rustc could read for it (`name.rs` or `name/mod.rs`, in
    `own`'s module directory or beside `own`, since a crate root, a `src/bin` file, a `mod.rs` and
    a file an attribute loaded all read their modules beside themselves), exactly one exists, and
    no attribute of the declaration carries `path` in any spelling (`#[path]`, a raw string,
    `cfg_attr` under any predicate). Any other declaration is not read, so a shape only it spells
    is refused. A declaration inside an inline module or a block is not followed."""
    bare, skeleton = lexed(own.read_text(encoding="utf-8"))
    files = []
    for module in OUT_OF_LINE.finditer(skeleton):
        if skeleton.count("{", 0, module.start()) != skeleton.count("}", 0, module.start()):
            continue
        if "#[cfg(test)]" not in module.group(1):
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
        ("a/depth.rs", (), "a/depth/tests.rs"),
        ("a/mod.rs", (), "a/tests.rs"),
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

    def test_every_module_file_choice_is_read_from_rustcs_file_or_refused(self):
        """R8's population: each declaring-file kind, each attribute that can choose the file, and
        a stale spelling at each other place rustc could look. The guard reads rustc's file or
        nothing; it never pins a shape from a file rustc does not read."""
        members = 0
        for own, extra, default in self.KINDS:
            stem, folder = Path(own).with_suffix(""), Path(own).parent
            places = [p / leaf for p in (stem, folder) for leaf in ("tests.rs", "tests/mod.rs")]
            for attribute in self.ATTRIBUTES:
                chosen = (folder / "t.rs").as_posix() if attribute else default
                stale = [p.as_posix() for p in places if p.as_posix() != chosen]
                for misplaced in (None, *stale):
                    for spelled in (True, False):
                        members += 1
                        text = self.SPELLING if spelled else "let shape = 1;\n"
                        files = [*extra, (chosen, text)]
                        if misplaced:
                            files.append((misplaced, self.SPELLING))
                        declaration = f"#[cfg(test)]\n{attribute}mod tests;\n"
                        root = self.declared(declaration, *files, own=own)
                        src = root / "crates" / "demo" / "src"
                        case = f"{own} {attribute!r} stale={misplaced} spelled={spelled}"
                        self.assertIn(out_of_line(src / own), ([src / chosen], []), case)
                        if not spelled:
                            self.assertEqual(len(unpinned(root)), 1, case)
        print(f"R8 population: {members} members")


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
