"""The four state-machine lints' shared behaviour (SPEC-433 R1, R2, R10 and R11; ADR-447 D5 and D7):
VOID at every level, the ratchet's admission, the levels, the line printer and the register.

Every test reads a fixture tree written under a temporary directory, or a reading built here, and
prints nothing into the job's log; A18 also reads the committed register."""

import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import _fsm_lints
from _support import REPO

RULES = ["FSM-CHECK-LIST", "FSM-UNCOVERED", "FSM-UNREACHABLE", "FSM-WILDCARD"]
PLANT = "crates/plant/src/lib.rs"

#: A14: a tree holding a file for each walk and nothing any rule examines.
IDLE_TREE = {
    PLANT: "pub fn idle() -> u8 {\n    1\n}\n",
    "migrations/001_notes.sql": "CREATE TABLE notes (\n    id INTEGER PRIMARY KEY,\n    body TEXT\n);\n",
    "formal/tla/Idle/Idle.tla": "---- MODULE Idle ----\n====\n",
}
#: A14: a tree with one domain, so only the STRICT probe can make it VOID.
STRICT_TREE = {
    "migrations/001_t.sql": "CREATE TABLE t (\n    state TEXT CHECK (state IN ('a', 'b'))\n);\n",
}
#: A table option this SQLite does not know, refused as an older SQLite refuses `STRICT`.
UNKNOWN_OPTION = "CREATE TABLE fsm_strict_probe (probe TEXT) STRICTER"

#: A15: one seeded site, and the same site's sibling outside the seed.
SEEDED = {"path": PLANT, "enum": "PlantState", "fn": "class"}
UNSEEDED = {"path": PLANT, "enum": "PlantState", "fn": "reason"}
ADMIT_SEED = ((PLANT, "PlantState", "class"),)
ADMIT_TREE = {
    "docs/rulings/listing-ruling.md": "A ruling that admits one listed site.\n",
    "README.md": "Not a ruling.\n",
}

#: A17: a catch-all arm whose line holds text no hit line may carry.
SITE_TEXT = "SITE_TEXT_ONLY_THE_SOURCE_HOLDS"
SITE_TREE = {
    PLANT: """\
pub enum PlantState {
    Up,
    Down,
}

pub fn class(state: PlantState) -> u8 {
    match state {
        PlantState::Up => 1,
        _ => SITE_TEXT_ONLY_THE_SOURCE_HOLDS,
    }
}
""",
}
SITE_LINES = [
    "FSM-LINT FSM-WILDCARD level=warn examined=1 matches hits=1 new=1 listed=0 stale=0",
    "FSM-LINT FSM-WILDCARD warn new crates/plant/src/lib.rs:9 enum=PlantState fn=class",
]

#: A18: a register with the four rules and nothing listed.
BASE_REGISTER = {
    "rules": {
        "FSM-CHECK-LIST": {"level": "warn", "pairs": [], "unpaired": []},
        "FSM-WILDCARD": {"level": "warn", "listed": []},
        "FSM-UNREACHABLE": {"level": "warn", "listed": []},
        "FSM-UNCOVERED": {"level": "warn", "listed": []},
    }
}


def plant(root, files):
    """Write a fixture tree: each repository-relative path with its text."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


def register_with(change):
    """BASE_REGISTER with one change made to a copy of it."""
    register = copy.deepcopy(BASE_REGISTER)
    change(register["rules"])
    return register


def unknown_rule(rules):
    rules["FSM-EXTRA"] = {"level": "warn", "listed": []}


def unknown_level(rules):
    rules["FSM-WILDCARD"]["level"] = "error"


def unknown_key(rules):
    rules["FSM-UNREACHABLE"]["listing"] = []


def unknown_field(rules):
    rules["FSM-UNCOVERED"]["listed"] = [{"path": PLANT, "enum": "PlantState", "line": "3"}]


def hit_reading(level, kind):
    """A reading of one hit of `kind`, at `level`, built here and not by a rule."""
    hit = _fsm_lints.Hit("FSM-UNCOVERED", kind, PLANT, 3, ("enum=PlantState",))
    return _fsm_lints.Reading("FSM-UNCOVERED", level, "enums", 1, examined=1, hits=[hit])


class SharedBehaviour(unittest.TestCase):
    def test_every_rule_is_void_with_nothing_to_examine(self):
        rules = (
            (
                "FSM-CHECK-LIST",
                _fsm_lints.check_list,
                {"level": "warn", "pairs": [], "unpaired": []},
            ),
            ("FSM-WILDCARD", _fsm_lints.wildcard, {"level": "warn", "listed": []}),
            ("FSM-UNREACHABLE", _fsm_lints.unreachable, {"level": "warn", "listed": []}),
            ("FSM-UNCOVERED", _fsm_lints.uncovered, {"level": "warn", "listed": []}),
        )
        with tempfile.TemporaryDirectory() as scratch:
            plant(scratch, IDLE_TREE)
            for name, rule, body in rules:
                with self.subTest(rule=name), self.assertRaises(_fsm_lints.Void):
                    rule(Path(scratch), body, ())
        body = {"level": "warn", "pairs": [], "unpaired": []}
        with tempfile.TemporaryDirectory() as scratch:
            plant(scratch, STRICT_TREE)
            with mock.patch.object(_fsm_lints, "STRICT_PROBE", UNKNOWN_OPTION):
                with self.assertRaises(_fsm_lints.Void):
                    _fsm_lints.check_list(Path(scratch), body, ())
            reading = _fsm_lints.check_list(Path(scratch), body, ())
        self.assertEqual(reading.columns, {("t", "state"): ("text", ("a", "b"))})

    def test_an_entry_outside_the_seed_needs_a_ruling_that_exists(self):
        refused = (
            ("no ruling", UNSEEDED),
            ("a ruling no file holds", dict(UNSEEDED, ruling="docs/rulings/absent.md")),
            ("a ruling outside docs/rulings", dict(UNSEEDED, ruling="README.md")),
        )
        with tempfile.TemporaryDirectory() as scratch:
            plant(scratch, ADMIT_TREE)
            for case, entry in refused:
                with self.subTest(case=case), self.assertRaises(_fsm_lints.Refused):
                    _fsm_lints.admit(Path(scratch), "FSM-WILDCARD", [SEEDED, entry], ADMIT_SEED)
            ruled = dict(UNSEEDED, ruling="docs/rulings/listing-ruling.md")
            admitted = _fsm_lints.admit(Path(scratch), "FSM-WILDCARD", [SEEDED, ruled], ADMIT_SEED)
        self.assertEqual(
            admitted, {(PLANT, "PlantState", "class"), (PLANT, "PlantState", "reason")}
        )

    def test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them(self):
        for kind, counts in (
            ("new", "new=1 listed=0 stale=0"),
            ("stale", "new=0 listed=0 stale=1"),
        ):
            with self.subTest(kind=kind):
                with self.assertRaises(AssertionError), contextlib.redirect_stdout(io.StringIO()):
                    _fsm_lints.report(hit_reading("fail", kind))
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    printed = _fsm_lints.report(hit_reading("warn", kind))
                expected = [
                    "FSM-LINT FSM-UNCOVERED level=warn examined=1 enums hits=1 " + counts,
                    "FSM-LINT FSM-UNCOVERED warn "
                    + kind
                    + " crates/plant/src/lib.rs:3 enum=PlantState",
                ]
                self.assertEqual(printed, expected)
                self.assertEqual(out.getvalue().splitlines(), expected)
        with contextlib.redirect_stdout(io.StringIO()):
            printed = _fsm_lints.report(hit_reading("fail", "listed"))
        self.assertEqual(len(printed), 2)

    def test_a_hit_line_names_its_site_and_never_its_source(self):
        with tempfile.TemporaryDirectory() as scratch:
            plant(scratch, SITE_TREE)
            reading = _fsm_lints.wildcard(Path(scratch), {"level": "warn", "listed": []}, ())
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            printed = _fsm_lints.report(reading)
        self.assertEqual(printed, SITE_LINES)
        self.assertEqual(out.getvalue().splitlines(), SITE_LINES)
        self.assertNotIn(SITE_TEXT, out.getvalue())

    def test_the_register_names_four_rules_at_warn_and_refuses_the_unknown(self):
        changes = (unknown_rule, unknown_level, unknown_key, unknown_field)
        with tempfile.TemporaryDirectory() as scratch:
            path = Path(scratch) / "config" / "fsm-lints.json"
            path.parent.mkdir(parents=True)
            for change in changes:
                path.write_text(json.dumps(register_with(change)), encoding="utf-8")
                with self.subTest(change=change.__name__), self.assertRaises(_fsm_lints.Refused):
                    _fsm_lints.read_register(Path(scratch))
            path.write_text(json.dumps(BASE_REGISTER), encoding="utf-8")
            self.assertEqual(_fsm_lints.read_register(Path(scratch)), BASE_REGISTER)
        register = _fsm_lints.read_register(REPO)
        self.assertEqual(sorted(register["rules"]), RULES)
        levels = {name: body["level"] for name, body in register["rules"].items()}
        self.assertEqual(levels, dict.fromkeys(RULES, "warn"))


if __name__ == "__main__":
    unittest.main()
