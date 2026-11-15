"""FSM-WILDCARD (SPEC-433 R6, R7 and R11; ADR-447 D2): no catch-all arm in a match on a state enum,
in product source.

A7 reads this repository's own tree and prints the rule's lines into the job's log. A8 and A9 each
read a fixture tree written under a temporary directory, and print nothing."""

import tempfile
import unittest
from pathlib import Path

import _fsm_lints
from _support import REPO, examined

#: The register's seeded `listed` entries (SPEC-433 section 4), frozen here: R2 admits an entry
#: with no `ruling` only when SEED holds it.
SEED = (
    ("crates/readings/src/state.rs", "TopicState", "class"),
    ("crates/readings/src/state.rs", "TopicState", "reason"),
)
SUMMARY = (
    r"^FSM-LINT FSM-WILDCARD level=(warn|fail) examined=[1-9][0-9]* matches"
    r" hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$"
)
PLANT = "crates/plant/src/lib.rs"

#: A8: one fn per catch-all form and per way a match is on a state enum, over a state enum of each
#: suffix, each fn with one catch-all arm.
CATCH_ALL_TREE = {
    PLANT: """\
use std::fmt;

pub enum DoorState {
    Open,
    Shut,
}

pub enum JobStatus {
    Queued,
    Done,
}

pub enum MoonPhase {
    New,
    Full,
}

pub enum BuildStage {
    Fetch,
    Link,
}

pub enum WalkStep {
    Left,
    Right,
}

pub enum UnitLifecycle {
    Born,
    Gone,
}

pub enum ViewMode {
    Grid,
    List,
}

pub fn bare(door: DoorState) -> u8 {
    match door {
        DoorState::Open => 1,
        _ => 0,
    }
}

pub fn guarded(job: JobStatus, tries: u8) -> u8 {
    match job {
        JobStatus::Queued => 1,
        _ if tries > 0 => 2,
        JobStatus::Done => 0,
    }
}

pub fn bound(moon: MoonPhase) -> u8 {
    match moon {
        MoonPhase::New => 1,
        other => other as u8,
    }
}

pub fn by_ref(stage: &BuildStage) -> u8 {
    match stage {
        BuildStage::Fetch => 1,
        ref later => 0,
    }
}

pub fn by_mut(step: WalkStep) -> u8 {
    match step {
        WalkStep::Left => 1,
        mut right => 0,
    }
}

pub fn at_wildcard(unit: UnitLifecycle) -> u8 {
    match unit {
        UnitLifecycle::Born => 1,
        gone @ _ => 0,
    }
}

pub fn wrapped(mode: Option<ViewMode>) -> u8 {
    match mode {
        Some(ViewMode::Grid) => 1,
        _ => 0,
    }
}

impl DoorState {
    pub fn code(self) -> u8 {
        match self {
            Self::Open => 1,
            _ => 0,
        }
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            _ => write!(f, "other"),
        }
    }
}
""",
}
CATCH_ALL_HITS = {
    ("new", PLANT, ("enum=DoorState", "fn=bare")),
    ("new", PLANT, ("enum=JobStatus", "fn=guarded")),
    ("new", PLANT, ("enum=MoonPhase", "fn=bound")),
    ("new", PLANT, ("enum=BuildStage", "fn=by_ref")),
    ("new", PLANT, ("enum=WalkStep", "fn=by_mut")),
    ("new", PLANT, ("enum=UnitLifecycle", "fn=at_wildcard")),
    ("new", PLANT, ("enum=ViewMode", "fn=wrapped")),
    ("new", PLANT, ("enum=DoorState", "fn=code")),
    ("new", PLANT, ("enum=JobStatus", "fn=fmt")),
}

#: A9: one planted catch-all, beside a `_` in a tuple position, a comment, a string, `matches!`,
#: `if let`, `let ... else`, a non-state enum and an item under `#[cfg(test)]`.
PLANTED_TREE = {
    PLANT: """\
pub enum DoorState {
    Open,
    Shut,
}

pub enum Colour {
    Red,
    Blue,
}

pub fn planted(door: DoorState) -> u8 {
    match door {
        DoorState::Open => 1,
        _ => 0,
    }
}

pub fn tupled(door: DoorState, open: bool) -> u8 {
    match (door, open) {
        (DoorState::Open, _) => 1,
        (DoorState::Shut, _) => 0,
    }
}

// pub fn commented(door: DoorState) -> u8 { match door { DoorState::Open => 1, _ => 0 } }

pub fn quoted() -> &'static str {
    "pub fn quoted_inner(door: DoorState) -> u8 { match door { DoorState::Open => 1, _ => 0 } }"
}

pub fn asked(door: DoorState) -> bool {
    matches!(door, DoorState::Open)
}

pub fn lets(door: DoorState) -> u8 {
    if let DoorState::Open = door {
        return 1;
    }
    let DoorState::Shut = door else {
        return 2;
    };
    0
}

pub fn colours(colour: Colour) -> u8 {
    match colour {
        Colour::Red => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::DoorState;

    fn tested(door: DoorState) -> u8 {
        match door {
            DoorState::Open => 1,
            _ => 0,
        }
    }
}
""",
}
PLANTED_HITS = {("new", PLANT, ("enum=DoorState", "fn=planted"))}


def plant(root, files):
    """Write a fixture tree: each repository-relative path with its text."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


def read(tree):
    """FSM-WILDCARD's reading of a fixture tree, nothing listed."""
    with tempfile.TemporaryDirectory() as scratch:
        plant(scratch, tree)
        return _fsm_lints.wildcard(Path(scratch), {"level": "warn", "listed": []}, ())


class WildcardLint(unittest.TestCase):
    def test_the_tree_reading_reads_every_product_file_and_its_matches(self):
        body = _fsm_lints.read_register(REPO)["rules"]["FSM-WILDCARD"]
        reading = _fsm_lints.wildcard(REPO, body, SEED)
        self.assertGreater(len(reading.matches), 0)
        crates = REPO / "crates"
        listing = examined(
            "product files",
            sorted(
                p
                for p in crates.rglob("*.rs")
                if p.is_file() and p.relative_to(crates).parts[1:2] == ("src",)
            ),
        )
        self.assertEqual(reading.files, len(listing))
        self.assertEqual(reading.examined, len(reading.matches))
        printed = _fsm_lints.report(reading)
        self.assertRegex(printed[0], SUMMARY)
        self.assertEqual(len(printed), 1 + len(reading.hits))

    def test_each_catch_all_arm_over_a_state_enum_is_a_hit(self):
        reading = read(CATCH_ALL_TREE)
        self.assertEqual({(hit.kind, hit.path, hit.key) for hit in reading.hits}, CATCH_ALL_HITS)
        self.assertEqual(reading.examined, 9)

    def test_only_the_planted_catch_all_is_a_hit(self):
        reading = read(PLANTED_TREE)
        self.assertEqual({(hit.kind, hit.path, hit.key) for hit in reading.hits}, PLANTED_HITS)
        self.assertEqual(reading.examined, 2)


if __name__ == "__main__":
    unittest.main()
