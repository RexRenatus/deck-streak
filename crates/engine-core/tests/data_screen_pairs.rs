//! No new engine pair (SPEC-516 R10, B10): the data screens' back up now, list and export reach
//! the engine through the core's fixed statement and the pool or folder, never through a pair, so
//! the per-transport table's rows stay the base's, by pair and name. The census reads every row of
//! the ordinary, exempt and answered tables and holds each set equal to a literal written from the
//! table at this delivery's base, never read from the core.

#![allow(
    clippy::print_stdout,
    reason = "an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeSet;

use deck_streak_engine_core::table::{ANSWERED, EXEMPT, ORDINARY};

/// The ordinary rows at the base, by pair and the engine's name.
const ORDINARY_ROWS: [(u32, u32, &str); 18] = [
    (1, 3, "BackendSyncService.SyncLogin"),
    (1, 5, "BackendSyncService.SyncCollection"),
    (3, 0, "BackendCollectionService.OpenCollection"),
    (3, 1, "BackendCollectionService.CloseCollection"),
    (3, 7, "CollectionService.GetUndoStatus"),
    (5, 4, "CardsService.SetFlag"),
    (7, 4, "DecksService.DeckTree"),
    (7, 13, "DecksService.GetDeckNames"),
    (7, 22, "DecksService.SetCurrentDeck"),
    (13, 3, "SchedulerService.GetQueuedCards"),
    (13, 14, "SchedulerService.BuryOrSuspendCards"),
    (13, 24, "SchedulerService.DescribeNextStates"),
    (23, 8, "NotetypesService.GetNotetypeNames"),
    (25, 0, "NotesService.NewNote"),
    (25, 2, "NotesService.AddNotes"),
    (27, 6, "CardRenderingService.RenderExistingCard"),
    (27, 9, "CardRenderingService.StripAvTags"),
    (27, 14, "CardRenderingService.HtmlToTextLine"),
];

/// The exempt rows at the base, by pair and the engine's name.
const EXEMPT_ROWS: [(u32, u32, &str); 8] = [
    (13, 17, "SchedulerService.ScheduleCardsAsNew"),
    (13, 19, "SchedulerService.SetDueDate"),
    (11, 5, "DeckConfigService.RemoveDeckConfig"),
    (23, 15, "NotetypesService.ChangeNotetype"),
    (5, 2, "CardsService.RemoveCards"),
    (25, 7, "NotesService.RemoveNotes"),
    (1, 6, "BackendSyncService.FullUploadOrDownload"),
    (3, 8, "CollectionService.Undo"),
];

/// The one answered row at the base.
const ANSWERED_ROWS: [(u32, u32, &str); 1] = [(13, 4, "SchedulerService.AnswerCard")];

#[test]
fn the_data_screens_add_no_engine_pair() {
    let ordinary: Vec<(u32, u32, &str)> = ORDINARY
        .iter()
        .map(|row| (row.service, row.method, row.name))
        .collect();
    let exempt: Vec<(u32, u32, &str)> = EXEMPT
        .iter()
        .map(|row| (row.service, row.method, row.name))
        .collect();
    let answered: Vec<(u32, u32, &str)> = ANSWERED
        .iter()
        .map(|row| (row.service, row.method, row.name))
        .collect();
    let rows: Vec<(u32, u32, &str)> = ordinary
        .iter()
        .chain(&exempt)
        .chain(&answered)
        .copied()
        .collect();
    let rows = support::examined("row(s) of the per-transport table", rows);
    assert_eq!(rows.len(), 27, "18 ordinary, 8 exempt and 1 answered");
    for (table, found, base) in [
        ("ordinary", &ordinary, &ORDINARY_ROWS[..]),
        ("exempt", &exempt, &EXEMPT_ROWS[..]),
        ("answered", &answered, &ANSWERED_ROWS[..]),
    ] {
        assert_eq!(
            found.iter().copied().collect::<BTreeSet<_>>(),
            base.iter().copied().collect::<BTreeSet<_>>(),
            "the {table} rows are the base's, by pair and name"
        );
        assert_eq!(found.len(), base.len(), "the {table} table holds no row twice");
    }
    let pairs: BTreeSet<(u32, u32)> = rows.iter().map(|(service, method, _)| (*service, *method)).collect();
    assert_eq!(pairs.len(), rows.len(), "no pair is in two rows");
}
