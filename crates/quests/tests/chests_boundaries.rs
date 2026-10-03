//! The chest rules' boundaries, held by the public API alone (SPEC-081 section 10, T15).
//!
//! Each test sits exactly on an edge a one-token change would move: the stored choice's three
//! names, the Legendary odds boundary and a chest that pays nothing.

// An integration test is test code: its helpers panic on a missing row.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_quests::chest_store::{self, ChestState, Choice, NewChest, Origin};
use deck_streak_quests::chests::{
    BASE_ODDS_LEGENDARY, ChestError, Opened, Rarity, open_chest_on, roll_rarity,
};

/// The instant the seed write records.
const AT: UtcMillis = UtcMillis::from_epoch_millis(1_700_020_800_000);

#[test]
fn a_stored_choice_reads_back_as_the_choice_it_names() {
    assert!(
        matches!(Choice::from_stored(""), Ok(None)),
        "'' is no choice"
    );
    assert!(
        matches!(Choice::from_stored("token"), Ok(Some(Choice::Token))),
        "token reads as the token choice"
    );
    assert!(
        matches!(Choice::from_stored("freeze"), Ok(Some(Choice::Freeze))),
        "freeze reads as the freeze choice"
    );
    assert!(
        matches!(
            Choice::from_stored("bonus"),
            Err(ChestError::Unreadable {
                column: "chests.choice",
                ..
            })
        ),
        "any other name is unreadable"
    );
}

#[test]
fn a_draw_exactly_on_the_legendary_boundary_is_not_legendary() {
    let u = BASE_ODDS_LEGENDARY / 100.0;
    assert!(
        (u.clamp(0.0, 1.0) * 100.0 - BASE_ODDS_LEGENDARY).abs() == 0.0,
        "the draw sits exactly on the boundary"
    );
    assert_ne!(
        roll_rarity(u, 0, 0, 0.0),
        Rarity::Legendary,
        "the boundary itself is the next band"
    );
    assert_eq!(
        roll_rarity(0.0, 0, 0, 0.0),
        Rarity::Legendary,
        "a draw below the boundary is Legendary"
    );
}

#[tokio::test]
async fn a_chest_that_pays_nothing_answers_no_payout() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    let chest = NewChest {
        study_day: StudyDay::from_epoch_day(19_000),
        origin: Origin::Session,
        session_start: 1,
        rarity: Rarity::Common,
        payout_xp: 0,
        state: ChestState::Vaulted,
    };
    let id = chest_store::insert_chest(&mut write, &chest, AT)
        .await
        .expect("the chest stored")
        .expect("a key of its own");
    let opened = open_chest_on(&mut write, id).await.expect("the open");
    write.commit().await.expect("the write committed");
    match opened {
        Opened::Revealed { payout, .. } => {
            assert_eq!(payout, None, "a chest paying 0 answers no payout");
        }
        other => panic!("a Common chest reveals, not {other:?}"),
    }
}
