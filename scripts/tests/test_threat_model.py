"""SPEC-375: the app campaign's threat model, and the reader that holds it.

A1 to A3 run `scripts/threat_model.py` over the tree's own root. A4 to A9 each plant a model in a
temporary directory, assert the whole finding line or lines, then that the same plant, corrected,
reads clean with a non-zero examined count. A10 calls `main` in-process, and runs the file as a
script in-process, and reads its exit and its examined line. No test starts a process, and no
fixture tree is committed (ADR-386 D3 and D10).
"""

import contextlib
import dataclasses
import importlib.util
import io
import runpy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "threat_model.py"


def load():
    """The reader as a module, loaded from its file, so a mutant installed there is the one run."""
    spec = importlib.util.spec_from_file_location("threat_model", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


threat_model = load()

CONTROL = "entry here\ncontrol here\npin here\n"

S1 = "| S1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |"
T1 = "| T1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |"
R1 = "| R1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |"
I1 = "| I1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |"
D1 = "| D1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |"
E1 = "| E1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |"

# One surface, one row of each class: line 11 is the surface's row, lines 17 to 22 are S1 to E1.
HOLDING = """---
trace: threat-model
---

# A planted model

## 2. Surfaces

| surface | entry point |
|---|---|
| the planted surface | `src/control.txt:1:entry here` |

## 3. Surface: the planted surface

| id | threat | asset | control | pinned by |
|---|---|---|---|---|
| S1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| T1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| R1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| I1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| D1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| E1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
"""

# Three declared surfaces: the first holds every class, the empty one's section holds no STRIDE
# table, and the second has no R row; the fourth section names no declared surface (line 42).
THREE = """---
trace: threat-model
---

# A planted model of three surfaces

## 2. Surfaces

| surface | entry point |
|---|---|
| the first surface | `src/control.txt:1:entry here` |
| the empty surface | `src/control.txt:1:entry here` |
| the second surface | `src/control.txt:1:entry here` |

## 3. Surface: the first surface

| id | threat | asset | control | pinned by |
|---|---|---|---|---|
| S1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| T1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| R1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| I1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| D1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| E1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |

## 4. Surface: the empty surface

| note | text |
|---|---|
| a note | not a STRIDE table |

## 5. Surface: the second surface

| id | threat | asset | control | pinned by |
|---|---|---|---|---|
| S2 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| T2 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| I2 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| D2 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| E2 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |

## 6. Surface: the unnamed surface

| id | threat | asset | control | pinned by |
|---|---|---|---|---|
| S3 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| T3 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| R3 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| I3 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| D3 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
| E3 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |
"""

EMPTY = "| the empty surface | `src/control.txt:1:entry here` |"
EMPTY_SECTION = "## 4. Surface: the empty surface"
I2 = "| I2 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:3:pin here` |"

# A schematic that declares no trace, its control table under a threat-model heading (line 3).
UNDECLARED = """# Notes beside the model

## Threat model

| threat | control |
|---|---|
| a threat | its answer |
"""

# The same table under a later heading of the same level, which names no threat model.
MOVED_UNDER = """# Notes beside the model

## Threat model

## How the reader is wired into the gate

| threat | control |
|---|---|
| a threat | its answer |
"""


def plant(root, files):
    """Write each planted file under the root."""
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def tree(model):
    """A planted tree: the model and the file its citations name."""
    return {"docs/schematics/model.md": model, "src/control.txt": CONTROL}


def swapped(text, old, new):
    """`text` with its one `old` replaced by `new`; a plant that misnames its row is refused."""
    if text.count(old) != 1:
        raise AssertionError("the plant holds its replaced text " + str(text.count(old)) + " times")
    return text.replace(old, new)


def judged(files):
    """The reader's report over a tree planted from `files`."""
    with tempfile.TemporaryDirectory() as tmp:
        plant(Path(tmp), files)
        return threat_model.judge(Path(tmp))


def commanded(files):
    """`main`'s exit and what it printed, over a tree planted from `files`."""
    with tempfile.TemporaryDirectory() as tmp:
        plant(Path(tmp), files)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = threat_model.main(["--root", tmp])
        return code, out.getvalue()


def scripted(files):
    """The file run as a script, in-process: its exit and what it printed."""
    with tempfile.TemporaryDirectory() as tmp:
        plant(Path(tmp), files)
        out = io.StringIO()
        with mock.patch.object(sys, "argv", [str(SCRIPT), "--root", tmp]):
            with contextlib.redirect_stdout(out):
                try:
                    runpy.run_path(str(SCRIPT), run_name="__main__")
                except SystemExit as stop:
                    return stop.code, out.getvalue()
        return None, out.getvalue()


def holds(case, report):
    """The corrected plant reads clean, and the citations it examined are not none."""
    case.assertEqual(report.findings, [])
    print(f"examined {report.citations} citation(s)")
    case.assertGreater(report.citations, 0, "examined 0 citation(s): nothing was judged")


class TheModelIsDeclared(unittest.TestCase):
    def test_the_campaign_model_is_the_one_declared_model(self):
        """A1"""
        report = threat_model.judge(ROOT)
        self.assertEqual(
            report.declared,
            [
                "docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-"
                "control-cites-a-line-that-holds.md"
            ],
        )
        with self.assertRaises(dataclasses.FrozenInstanceError):
            report.declared = []


class TheModelHolds(unittest.TestCase):
    def test_every_control_cites_a_line_that_holds(self):
        """A2"""
        report = threat_model.judge(ROOT)
        self.assertEqual(
            report.rows,
            ["S1", "T1", "R1", "I1", "I2", "D1", "E1"]
            + ["S2", "T2", "R2", "I3", "I4", "D2", "E2", "E6", "S6"]
            + ["S3", "T3", "R3", "I5", "I6", "D3", "E3"]
            + ["S4", "T4", "R4", "I7", "D4", "E4"]
            + ["S5", "T5", "T6", "R5", "I8", "D5", "E5"],
        )
        self.assertEqual(report.findings, [])
        print(f"examined {report.citations} citation(s)")
        self.assertGreater(report.citations, 0, "examined 0 citation(s): nothing was judged")

    def test_every_surface_has_a_row_for_every_stride_category(self):
        """A3"""
        report = threat_model.judge(ROOT)
        surfaces = [
            "the iPhone and iPad client",
            "the web client",
            "the sync service",
            "the engine and its boundaries",
            "the build and release lanes",
        ]
        self.assertEqual(report.surfaces, surfaces)
        checked = 0
        for name in surfaces:
            self.assertEqual(report.categories[name], frozenset("STRIDE"), name)
            checked += 1
        print(f"examined {checked} of {len(surfaces)} surfaces")
        self.assertEqual(checked, 5, "examined fewer surfaces than the model names")


class ACitationMustHoldInTheTree(unittest.TestCase):
    def test_a_citation_whose_quote_moved_is_refused(self):
        """A4"""
        report = judged(
            tree(
                swapped(
                    HOLDING,
                    S1,
                    "| S1 | threat | asset | `src/control.txt:1:control here` "
                    "| `src/control.txt:3:pin here` |",
                )
            )
        )
        self.assertEqual(
            report.findings,
            [
                "docs/schematics/model.md:17: S1: quote not on a cited line: "
                "src/control.txt:1:control here"
            ],
        )
        ranged = swapped(
            HOLDING,
            S1,
            "| S1 | threat | asset | `src/control.txt:1-2:control here` "
            "| `src/control.txt:3:pin here` |",
        )
        holds(self, judged(tree(ranged)))

    def test_a_citation_naming_no_file_is_refused(self):
        """A5"""
        beside = {
            "target/control.txt": CONTROL,
            ".git/control.txt": CONTROL,
            "node_modules/control.txt": CONTROL,
            "vendor/control.txt": CONTROL,
            "vendor/.git/HEAD": "a repository of its own\n",
        }
        named = swapped(
            HOLDING,
            S1,
            "| S1 | threat | asset | `src/control.txt:2:control here` | "
            "`src/absent.txt:3:pin here`, `src:3:pin here`, `target/control.txt:3:pin here`, "
            "`.git/control.txt:3:pin here`, `node_modules/control.txt:3:pin here`, "
            "`vendor/control.txt:3:pin here` |",
        )
        report = judged({**tree(named), **beside})
        self.assertEqual(
            report.findings,
            [
                "docs/schematics/model.md:17: S1: names no file: src/absent.txt:3:pin here",
                "docs/schematics/model.md:17: S1: names no file: src:3:pin here",
                "docs/schematics/model.md:17: S1: names no file: target/control.txt:3:pin here",
                "docs/schematics/model.md:17: S1: names no file: .git/control.txt:3:pin here",
                "docs/schematics/model.md:17: S1: names no file: "
                "node_modules/control.txt:3:pin here",
                "docs/schematics/model.md:17: S1: names no file: vendor/control.txt:3:pin here",
            ],
        )
        holds(self, judged({**tree(HOLDING), **beside}))

    def test_a_row_without_a_quoted_citation_is_refused(self):
        """A6"""
        bare = swapped(
            HOLDING, T1, "| T1 | threat | asset | plain text | `src/control.txt:3:pin here` |"
        )
        bare = swapped(
            bare, R1, "| R1 | threat | asset | `src/control.txt:2:control here` | plain text |"
        )
        bare = swapped(
            bare, I1, "| I1 | threat | asset | `not-a-citation` | `src/control.txt:3:pin here` |"
        )
        bare = swapped(
            bare,
            D1,
            "| D1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:1` |",
        )
        bare = swapped(
            bare,
            E1,
            "| E1 | threat | asset | `src/control.txt:2:control here` | `src/control.txt:1:` |",
        )
        report = judged(tree(bare))
        self.assertEqual(
            report.findings,
            [
                "docs/schematics/model.md:18: T1: the control cites nothing",
                "docs/schematics/model.md:19: R1: the pin cites nothing",
                "docs/schematics/model.md:20: I1: not a citation: not-a-citation",
                "docs/schematics/model.md:21: D1: no quote: src/control.txt:1",
                "docs/schematics/model.md:22: E1: no quote: src/control.txt:1:",
            ],
        )
        holds(self, judged(tree(HOLDING)))


class EverySurfaceCoversStride(unittest.TestCase):
    def test_a_surface_missing_a_category_is_refused(self):
        """A7"""
        report = judged(tree(THREE))
        self.assertEqual(
            report.findings,
            [
                "docs/schematics/model.md:12: surfaces: surface the empty surface: no table",
                "docs/schematics/model.md:13: surfaces: surface the second surface: no row for R",
                "docs/schematics/model.md:42: sections: section names no declared surface: "
                "the unnamed surface",
            ],
        )
        corrected = swapped(
            THREE, EMPTY, "| the unnamed surface | `src/control.txt:1:entry here` |"
        )
        corrected = swapped(corrected, EMPTY_SECTION, "## 4. Notes on the surfaces")
        corrected = swapped(
            corrected,
            I2,
            "| R2 | threat | asset | `src/control.txt:2:control here` "
            "| `src/control.txt:3:pin here` |\n" + I2,
        )
        holds(self, judged(tree(corrected)))


class EveryRowIdIsStride(unittest.TestCase):
    def test_a_row_id_outside_stride_or_repeated_is_refused(self):
        """A8"""
        doubled = swapped(
            HOLDING,
            E1,
            E1
            + "\n| X1 | threat | asset | `src/control.txt:2:control here` "
            + "| `src/control.txt:3:pin here` |\n"
            + S1,
        )
        report = judged(tree(doubled))
        self.assertEqual(
            report.findings,
            [
                "docs/schematics/model.md:23: X1: not a STRIDE id",
                "docs/schematics/model.md:24: S1: repeats an id",
            ],
        )
        holds(self, judged(tree(HOLDING)))


class AnUndeclaredModelIsRefused(unittest.TestCase):
    def test_an_undeclared_control_table_is_refused(self):
        """A9"""
        report = judged({**tree(HOLDING), "docs/schematics/notes.md": UNDECLARED})
        self.assertEqual(
            report.findings,
            [
                "docs/schematics/notes.md:3: trace: a control table under a threat-model "
                "heading, and the document declares no trace"
            ],
        )
        holds(self, judged({**tree(HOLDING), "docs/schematics/notes.md": MOVED_UNDER}))


class TheCommand(unittest.TestCase):
    def test_the_command_exits_by_its_verdict(self):
        """A10"""
        self.assertEqual(
            commanded(tree(HOLDING)),
            (0, "examined 1 model(s), 1 surface(s), 6 row(s), 13 citation(s)\n"),
        )
        short = swapped(
            HOLDING,
            D1,
            "| S9 | threat | asset | `src/control.txt:2:control here` "
            "| `src/control.txt:3:pin here` |",
        )
        short = swapped(
            short,
            E1,
            "| T9 | threat | asset | `src/control.txt:2:control here` "
            "| `src/control.txt:3:pin here` |",
        )
        self.assertEqual(
            commanded(tree(short)),
            (
                1,
                "docs/schematics/model.md:11: surfaces: surface the planted surface: "
                "no row for DE\n"
                "examined 1 model(s), 1 surface(s), 6 row(s), 13 citation(s)\n",
            ),
        )
        empty = {"docs/schematics/notes.md": "# Notes\n"}
        self.assertEqual(
            commanded(empty),
            (3, "examined 0 model(s), 0 surface(s), 0 row(s), 0 citation(s)\n"),
        )
        self.assertEqual(
            scripted(empty),
            (3, "examined 0 model(s), 0 surface(s), 0 row(s), 0 citation(s)\n"),
        )


if __name__ == "__main__":
    unittest.main()
