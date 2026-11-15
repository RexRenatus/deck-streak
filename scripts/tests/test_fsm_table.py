"""SPEC-434: FSM-TABLE, a transition table for every product state enum (ADR-448 D1).

A product state enum is an enum declared in a `.rs` file under `crates/*/src`, outside every item
under `#[cfg(test)]`, whose name ends in `State`, `Status`, `Phase`, `Stage`, `Step`, `Lifecycle`
or `Mode`. One with no table in `config/fsm-tables.json` is a hit. A1 reads the tree and prints
its summary line, which CI's log carries and no test pins, so a sibling's new enum only prints at
`warn`. A2 to A4 read fixture trees under a temporary directory, each with its hits, its states or
its VOID stated as a literal.
"""

import unittest

import _fsm_tables
from _support import REPO, examined
from test_fsm_table_lints import LIB, SUMMARY, echoed, planted, register_text, table, tree_register

# One enum of each suffix, after the forms a reader must not read: a raw string with two hashes
# holding a quote and a hash, a nested block comment, a line comment, a string holding an escaped
# quote, and a lifetime beside a character literal that holds a quote. An enum with no suffix and
# one under `#[cfg(test)]` follow them.
SUFFIXED = """\
//! A planted crate: one enum of each suffix, after the forms a reader must not read.
pub const RAW: &str = r##"pub enum RawState { "# Shut }"##;
/* pub enum BlockState { Open } /* nested */ pub enum NestedState { Open } */
// pub enum LineState { Open }
pub const TEXT: &str = "pub enum StringState { \\" Open }";
pub fn quote<'a>(text: &'a str) -> char { if text.is_empty() { '"' } else { '{' } }
pub enum DoorState { Open, Shut }
pub enum JobStatus { Queued, Done }
pub enum MoonPhase { New, Full }
pub enum RocketStage { First, Second }
pub enum WizardStep { Start, Finish }
pub enum LeaseLifecycle { Held, Freed }
pub enum DisplayMode { Light, Dark }
pub enum Colour { Red, Blue }
#[cfg(test)]
mod tests {
    pub enum TestOnlyState { Open, Shut }
}
"""
INTEGRATION = "pub enum IntegrationState { Open }\n"
SUFFIX_HITS = [
    (LIB, 7, "DoorState"),
    (LIB, 8, "JobStatus"),
    (LIB, 9, "MoonPhase"),
    (LIB, 10, "RocketStage"),
    (LIB, 11, "WizardStep"),
    (LIB, 12, "LeaseLifecycle"),
    (LIB, 13, "DisplayMode"),
]
SUFFIX_STATES = {
    "DoorState": ["Open", "Shut"],
    "JobStatus": ["Queued", "Done"],
    "MoonPhase": ["New", "Full"],
    "RocketStage": ["First", "Second"],
    "WizardStep": ["Start", "Finish"],
    "LeaseLifecycle": ["Held", "Freed"],
    "DisplayMode": ["Light", "Dark"],
}
# Every variant form: unit, tuple, struct, with a discriminant, with an outer attribute, under
# `#[cfg(test)]`, and a struct variant that carries a discriminant.
FORMS = """\
pub enum FormState {
    /// A unit variant, with a doc comment.
    Unit,
    Tuple(u8, &'static str),
    Struct { left: u8, right: (u16, u16) },
    Numbered = 4,
    #[allow(dead_code)]
    Attributed,
    #[cfg(test)]
    TestOnly,
    Last { inner: Vec<u8> } = 9,
}
"""
FORM_STATES = [
    ("Unit", 3),
    ("Tuple", 4),
    ("Struct", 5),
    ("Numbered", 6),
    ("Attributed", 8),
    ("Last", 11),
]
GATE = "pub enum GateState { Open, Shut }\n"
ABSENT = [
    (
        table("GateState", ["Open"], ["Shut"], file="crates/demo/src/absent.rs"),
        r"names crates/demo/src/absent\.rs, a product file that is absent",
    ),
    (table("LatchState", ["Open"], ["Shut"]), r"declares no enum LatchState"),
    (table("GateState", ["Ajar"], ["Shut"]), r"names Ajar, which it lacks"),
    (table("GateState", ["Open"], ["Ajar"]), r"names Ajar, which it lacks"),
    (
        table("GateState", ["Open"], ["Shut"], [("Open", "Ajar", "push")]),
        r"names Ajar, which it lacks",
    ),
]


class EveryProductStateEnumHasATable(unittest.TestCase):
    def test_the_tree_reading_reads_every_product_file_and_every_table(self):
        """A1: the tree's reading reads every product file a listing made here counts, every
        table the register names and at least one product state enum, and prints its summary
        line."""
        crates = REPO / "crates"
        listing = examined(
            "product files a listing made here counts",
            [
                path
                for path in crates.rglob("*.rs")
                if path.relative_to(crates).parts[1:2] == ("src",) and path.is_file()
            ],
        )
        tables = examined("tables the register names", tree_register()["tables"])
        reading = _fsm_tables.read(REPO, "FSM-TABLE")
        lines = echoed(reading)
        self.assertEqual(
            reading.files,
            len(listing),
            f"the walk read {reading.files} product files and the listing counts {len(listing)}",
        )
        self.assertEqual(reading.tables, len(tables))
        self.assertGreater(len(reading.examined), 0)
        self.assertRegex("\n".join(lines), SUMMARY.format(rule="FSM-TABLE", unit="state-enums"))

    def test_only_a_product_state_enum_with_no_table_is_a_hit(self):
        """A2: an enum of each of the seven suffixes with no table is a `new` hit and a table for
        it clears it; an enum under `#[cfg(test)]`, in a comment, in a string, under
        `crates/*/tests` and one with no suffix are not read."""
        files = {
            LIB: SUFFIXED,
            "crates/demo/tests/flow.rs": INTEGRATION,
            "config/fsm-tables.json": register_text([]),
        }
        with planted(files) as root:
            reading = _fsm_tables.read(root, "FSM-TABLE")
        self.assertEqual(sorted(reading.new), SUFFIX_HITS)
        self.assertEqual(sorted(reading.examined), SUFFIX_HITS)
        tables = [table(name, states, states) for name, states in SUFFIX_STATES.items()]
        files["config/fsm-tables.json"] = register_text(tables)
        with planted(files) as root:
            reading = _fsm_tables.read(root, "FSM-TABLE")
        self.assertEqual(reading.new, [])
        self.assertEqual(sorted(reading.examined), SUFFIX_HITS)

    def test_every_variant_form_is_read_and_a_test_variant_is_not(self):
        """A3: a unit, tuple, struct, discriminant-carrying and attributed variant are each a
        state, at its own line, and a `#[cfg(test)]` variant and a struct variant's fields are
        not."""
        with planted({LIB: FORMS}) as root:
            self.assertEqual(_fsm_tables.table_states(root, LIB, "FormState"), (1, FORM_STATES))

    def test_a_table_naming_what_is_absent_is_void(self):
        """A4: a table naming an absent file, an enum its file lacks, or a state its enum lacks in
        `initial`, `terminal` or a row is VOID."""
        for entry, void in ABSENT:
            files = {LIB: GATE, "config/fsm-tables.json": register_text([entry])}
            with planted(files) as root:
                with self.assertRaisesRegex(_fsm_tables.Void, void):
                    _fsm_tables.read(root, "FSM-TABLE")


if __name__ == "__main__":
    unittest.main()
