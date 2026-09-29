"""Every `Setting` implementation's `SHAPE` is pinned by its literal (SPEC-192).

A refusal names its setting and its shape (`the setting X is malformed: it must be <shape>`). A
test that compares against the constant (`Hour::SHAPE`) reads the code under test back to itself,
so a mutant that rewrites the words passes. The literal is the pin: a test that spells the words,
or a mutation row whose `find` is the constant's line, fails when they change.

The guard enumerates the implementations by walking `crates/*/src` (a git pathspec of that shape
matches nothing), reads each `const SHAPE` literal, and refuses one that is spelled in no test of
its crate and in no mutation row that targets the implementation's own file.
"""

import json
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

IMPL = re.compile(r"^\s*impl\s+(?:[\w:]+::)?Setting\s+for\s+(\w+)", re.MULTILINE)
SHAPE = re.compile(r'const\s+SHAPE\s*:\s*&\'static\s+str\s*=\s*("(?:[^"\\]|\\.)*")\s*;')
BANDS = REPO / "scripts" / "mutation-rows.d"


def implementations(root):
    """Every `impl Setting for X` under `crates/*/src`: (crate, file, name, quoted literal, span).

    The literal is the first `const SHAPE = "..."` after the impl line and before the next impl,
    and `span` is where that constant sits in the file, so the impl's own line can be excluded.
    """
    found = []
    for path in sorted((root / "crates").glob("*/src/**/*.rs")):
        text = path.read_text(encoding="utf-8")
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


def spelled_elsewhere(root, crate, file, literal, own_span):
    """True when `literal` is spelled, quoted, anywhere in the crate's tests or sources but at the
    implementation's own constant."""
    base = root / "crates" / crate
    for path in sorted(list((base / "tests").rglob("*.rs")) + list((base / "src").rglob("*.rs"))):
        text = path.read_text(encoding="utf-8")
        at = text.find(literal)
        while at != -1:
            if not (path == base / file and (at, at + len(literal)) == tuple(own_span)):
                return True
            at = text.find(literal, at + 1)
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
    """The implementations whose literal no test spells and no row of their file finds."""
    return [
        f"{crate}::{name} ({file.as_posix()}) {literal}"
        for crate, file, name, literal, span in implementations(root)
        if literal is None
        or not (
            spelled_elsewhere(root, crate, file, literal, span)
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

    def test_a_shape_a_test_of_the_crate_spells_is_pinned(self):
        root = self.tree('const X: &str = "a whole depth";')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])

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
        root = self.src(self.tree(), '/// Reads "a whole depth".\npub fn depth() {}\n')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), ['demo::Depth (src/depth.rs) "a whole depth"'])

    def test_a_shape_two_implementations_of_one_crate_share_needs_a_row_of_each_file(self):
        root = self.src(
            self.tree('const X: &str = "a whole depth";'),
            'impl Setting for Deep {\n    const SHAPE: &\'static str = "a whole depth";\n}\n',
        )
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(len(unpinned(root)), 2)

    def test_a_shape_only_a_comment_spells_is_refused(self):
        root = self.tree('// "a whole depth"\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)

if __name__ == "__main__":
    unittest.main()
