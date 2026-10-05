//! The core's table, judged over every pair a transport can send (SPEC-345 A1, A2).
//!
//! The expected sets are literals written from the SPEC's measurement (M1, M4, M8), never read
//! from the core, so a row the table loses, gains, renumbers or marks for the wrong transport fails
//! here by its pair.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeSet;

use deck_streak_engine_core::table::{
    Decision, EXEMPT, ExemptWrite, ORDINARY, TargetKind, Transport, decide,
};

/// The pairs the native adapter's allow-list holds (SPEC-345 M1).
const NATIVE: [(u32, u32); 7] = [(1, 3), (3, 0), (3, 8), (7, 13), (13, 3), (13, 4), (27, 6)];

/// The web engine's study calls (SPEC-345 M4).
const WEB: [(u32, u32); 16] = [
    (3, 0),
    (3, 1),
    (3, 8),
    (13, 3),
    (13, 4),
    (23, 8),
    (25, 0),
    (25, 2),
    // The review's eight (SPEC-350 R1, M10).
    (7, 4),
    (7, 22),
    (27, 6),
    (27, 9),
    (13, 24),
    (3, 7),
    (13, 14),
    (5, 4),
];

/// The six exempt writes, the never-list's entries 2, 3, 6, 7 and 8 (SPEC-345 M8).
const HELD: [(u32, u32); 6] = [(5, 2), (11, 5), (13, 17), (13, 19), (23, 15), (25, 7)];

/// The highest service and method index the census sends: past every index the engine numbers.
const LAST: u32 = 64;

#[test]
fn every_pair_is_admitted_held_or_refused_by_its_transport() {
    for (transport, ordinary) in [(Transport::Native, &NATIVE[..]), (Transport::Web, &WEB[..])] {
        let pairs: Vec<(u32, u32)> = (0..=LAST)
            .flat_map(|service| (0..=LAST).map(move |method| (service, method)))
            .collect();
        let pairs = support::examined(&format!("pair(s) on {transport:?}"), pairs);
        let mut admitted = BTreeSet::new();
        let mut held = BTreeSet::new();
        let mut refused = 0_usize;
        for &(service, method) in &pairs {
            match decide(transport, service, method) {
                Decision::Admit => {
                    admitted.insert((service, method));
                }
                Decision::NeedsGesture => {
                    held.insert((service, method));
                }
                Decision::NotAllowed => refused += 1,
            }
        }
        assert_eq!(
            admitted,
            ordinary.iter().copied().collect::<BTreeSet<_>>(),
            "the pairs {transport:?} admits"
        );
        assert_eq!(
            held,
            HELD.into_iter().collect::<BTreeSet<_>>(),
            "the pairs {transport:?} holds for a gesture"
        );
        assert_eq!(
            (pairs.len(), refused),
            (4225, 4225 - ordinary.len() - HELD.len()),
            "the pairs {transport:?} refuses as not allowed"
        );
    }
}

#[test]
fn each_exempt_write_names_its_engine_call_and_its_target_kind() {
    let table: Vec<_> = EXEMPT
        .iter()
        .map(|row| (row.write, row.service, row.method, row.name, row.kind))
        .collect();
    assert_eq!(
        table,
        vec![
            (
                ExemptWrite::Forget,
                13,
                17,
                "SchedulerService.ScheduleCardsAsNew",
                TargetKind::Card,
            ),
            (
                ExemptWrite::SetDueDate,
                13,
                19,
                "SchedulerService.SetDueDate",
                TargetKind::Card,
            ),
            (
                ExemptWrite::DeletePreset,
                11,
                5,
                "DeckConfigService.RemoveDeckConfig",
                TargetKind::Preset,
            ),
            (
                ExemptWrite::ChangeNoteType,
                23,
                15,
                "NotetypesService.ChangeNotetype",
                TargetKind::Note,
            ),
            (
                ExemptWrite::DeleteCard,
                5,
                2,
                "CardsService.RemoveCards",
                TargetKind::Card,
            ),
            (
                ExemptWrite::DeleteNote,
                25,
                7,
                "NotesService.RemoveNotes",
                TargetKind::Note,
            ),
        ]
    );
}

/// The review's eight pairs (SPEC-350 R1, M10), each with the engine's name for it and what the
/// native transport decides for it, written from the engine's protos at the pin, never read from
/// the core: seven new rows the web alone may call, and the card render both may.
const REVIEW: [(u32, u32, &str, Decision); 8] = [
    (7, 4, "DecksService.DeckTree", Decision::NotAllowed),
    (7, 22, "DecksService.SetCurrentDeck", Decision::NotAllowed),
    (
        27,
        6,
        "CardRenderingService.RenderExistingCard",
        Decision::Admit,
    ),
    (
        27,
        9,
        "CardRenderingService.StripAvTags",
        Decision::NotAllowed,
    ),
    (
        13,
        24,
        "SchedulerService.DescribeNextStates",
        Decision::NotAllowed,
    ),
    (
        3,
        7,
        "CollectionService.GetUndoStatus",
        Decision::NotAllowed,
    ),
    (
        13,
        14,
        "SchedulerService.BuryOrSuspendCards",
        Decision::NotAllowed,
    ),
    (5, 4, "CardsService.SetFlag", Decision::NotAllowed),
];

#[test]
fn the_review_pairs_are_ordinary_on_the_web() {
    for (service, method, name, native) in support::examined("review pair(s)", REVIEW.to_vec()) {
        assert_eq!(
            decide(Transport::Web, service, method),
            Decision::Admit,
            "{name} ({service}, {method}) on the web"
        );
        let rows: Vec<&str> = ORDINARY
            .iter()
            .filter(|row| (row.service, row.method) == (service, method))
            .map(|row| row.name)
            .collect();
        assert_eq!(
            rows,
            vec![name],
            "the core's one row for ({service}, {method})"
        );
        assert_eq!(
            decide(Transport::Native, service, method),
            native,
            "{name} ({service}, {method}) on the native transport"
        );
    }
}
