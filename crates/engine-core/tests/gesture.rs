//! SPEC-345 A12 (R7; ADR-356 D5): an owner's gesture takes one target of its write's kind, and
//! its one constructor refuses a target of another kind, naming the write and the target.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::table::{EXEMPT, ExemptWrite};

/// One target of each kind, each with its own id.
const TARGETS: [Target; 3] = [Target::Card(11), Target::Note(12), Target::Preset(13)];

/// The one target of each write that a gesture takes, by the kinds the owner-taps ruling names:
/// written here, not read from the exempt table, so the test does not compare the code with
/// itself.
const TAKEN: [(ExemptWrite, Target); 6] = [
    (ExemptWrite::Forget, Target::Card(11)),
    (ExemptWrite::SetDueDate, Target::Card(11)),
    (ExemptWrite::DeletePreset, Target::Preset(13)),
    (ExemptWrite::ChangeNoteType, Target::Note(12)),
    (ExemptWrite::DeleteCard, Target::Card(11)),
    (ExemptWrite::DeleteNote, Target::Note(12)),
];

#[test]
fn a_gesture_takes_one_target_of_its_writes_kind() {
    let mut taken = Vec::new();
    let mut refused = Vec::new();
    for write in EXEMPT.map(|row| row.write) {
        for target in TARGETS {
            match OwnerGesture::from_tap(write, target) {
                Ok(_) => taken.push((write, target)),
                Err(refusal) => refused.push(refusal),
            }
        }
    }
    let wrong_kind: Vec<GestureRefusal> = EXEMPT
        .map(|row| row.write)
        .into_iter()
        .flat_map(|write| {
            TARGETS
                .into_iter()
                .filter(move |target| !TAKEN.contains(&(write, *target)))
                .map(move |target| GestureRefusal::WrongKind { write, target })
        })
        .collect();
    assert_eq!(
        (taken, refused),
        (TAKEN.to_vec(), wrong_kind),
        "each write takes its own kind of target and refuses the other two by name"
    );
    let taps = support::examined(
        "tap(s), each exempt write by each kind of target",
        EXEMPT
            .iter()
            .flat_map(|row| TARGETS.map(|target| (row.write, target)))
            .collect(),
    );
    assert_eq!(taps.len(), 18, "six writes by three kinds of target");
    assert_eq!(
        GestureRefusal::WrongKind {
            write: ExemptWrite::Forget,
            target: Target::Note(12),
        }
        .to_string(),
        "the Forget write does not take Note(12)",
        "a refused tap names its write and its target"
    );
}
