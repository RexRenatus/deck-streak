"""FSM-UNCOVERED (SPEC-433 R9 and R11; ADR-447 D4): a state enum is covered only when a formal
entry anchors the enum itself.

A12 reads this repository's own tree and prints the rule's lines into the job's log. A13 reads a
fixture tree written under a temporary directory, and prints nothing."""

import re
import tempfile
import unittest
from pathlib import Path

import _fsm_lints
from _support import REPO, examined

#: The register's seeded `listed` entries (SPEC-433 section 4), frozen here: the eleven product
#: state enums other than `Phase`. R2 admits an entry with no `ruling` only when SEED holds it.
SEED = (
    ("crates/curriculum/src/progress.rs", "BandStep"),
    ("crates/daemon/src/lifecycle.rs", "NotifyState"),
    ("crates/daemon/src/wiring.rs", "Step"),
    ("crates/ingest/src/gate.rs", "AnchorState"),
    ("crates/ingest/src/skip.rs", "SkipState"),
    ("crates/ingest/src/sync_runs.rs", "RunStatus"),
    ("crates/quests/src/chest_store.rs", "ChestState"),
    ("crates/readings/src/state.rs", "TopicState"),
    ("crates/insights/src/registry.rs", "State"),
    ("crates/identity/src/passkeys.rs", "State"),
    ("crates/bot/src/poll.rs", "Step"),
)
SUMMARY = (
    r"^FSM-LINT FSM-UNCOVERED level=(warn|fail) examined=[1-9][0-9]* enums"
    r" hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$"
)
#: An enum declared by name, read as text: the reading's enums must each be one of these.
DECLARED = re.compile(r"\benum\s+([A-Za-z_][A-Za-z0-9_]*)")
PLANT = "crates/plant/src/lib.rs"

#: A13: an enum its anchor covers, a listed one now covered, an unlisted one whose file is covered
#: at another anchor and whose name is anchored in another file, a listed one still uncovered,
#: and a listed one the tree no longer declares.
COVER_TREE = {
    PLANT: """\
pub enum CoveredState {
    On,
    Off,
}

pub enum StaleState {
    On,
    Off,
}

pub enum NewState {
    On,
    Off,
}

pub enum ListedState {
    On,
    Off,
}

pub fn new_state_reader(state: NewState) -> bool {
    matches!(state, NewState::On)
}
""",
    "crates/other/src/lib.rs": """\
pub fn other() -> u8 {
    0
}
""",
    "formal/tla/Plant/Plant.tla": """\
---- MODULE Plant ----
\\* @phx covers crates/plant/src/lib.rs anchor=CoveredState
\\* @phx covers crates/plant/src/lib.rs anchor=StaleState
\\* @phx covers crates/plant/src/lib.rs anchor=new_state_reader
====
""",
    "formal/lean/Formal/Plant.lean": """\
-- @phx covers crates/other/src/lib.rs anchor=NewState
""",
}
COVER_RULE = {
    "level": "warn",
    "listed": [
        {"path": PLANT, "enum": "StaleState"},
        {"path": PLANT, "enum": "ListedState"},
        {"path": PLANT, "enum": "GoneState"},
    ],
}
COVER_SEED = ((PLANT, "StaleState"), (PLANT, "ListedState"), (PLANT, "GoneState"))
COVER_HITS = {
    ("stale", PLANT, 6, ("enum=StaleState",)),
    ("new", PLANT, 11, ("enum=NewState",)),
    ("listed", PLANT, 16, ("enum=ListedState",)),
    ("stale", PLANT, 0, ("enum=GoneState",)),
}


def plant(root, files):
    """Write a fixture tree: each repository-relative path with its text."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


class UncoveredLint(unittest.TestCase):
    def test_the_tree_reading_reads_every_formal_module_and_state_enum(self):
        body = _fsm_lints.read_register(REPO)["rules"]["FSM-UNCOVERED"]
        reading = _fsm_lints.uncovered(REPO, body, SEED)
        self.assertGreater(len(reading.covers), 0)
        formal = examined(
            "formal modules",
            sorted(
                p
                for p in (REPO / "formal").rglob("*")
                if p.is_file() and p.suffix in (".tla", ".lean")
            ),
        )
        self.assertEqual(reading.files, len(formal))
        self.assertGreater(len(reading.enums), 0)
        self.assertEqual(reading.examined, len(reading.enums))
        crates = REPO / "crates"
        declared = set()
        for path in crates.rglob("*.rs"):
            if path.relative_to(crates).parts[1:2] == ("src",):
                text = path.read_text(encoding="utf-8")
                where = path.relative_to(REPO).as_posix()
                declared |= {(where, name) for name in DECLARED.findall(text)}
        self.assertLessEqual(set(reading.enums), declared)
        printed = _fsm_lints.report(reading)
        self.assertRegex(printed[0], SUMMARY)
        self.assertEqual(len(printed), 1 + len(reading.hits))

    def test_only_an_anchor_on_the_enum_covers_it(self):
        with tempfile.TemporaryDirectory() as scratch:
            plant(scratch, COVER_TREE)
            reading = _fsm_lints.uncovered(Path(scratch), COVER_RULE, COVER_SEED)
        found = {(hit.kind, hit.path, hit.line, hit.key) for hit in reading.hits}
        self.assertEqual(found, COVER_HITS)
        self.assertEqual(reading.examined, 4)
        self.assertEqual(len(reading.covers), 4)


if __name__ == "__main__":
    unittest.main()
