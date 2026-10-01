//! Streaks' data-rights port (SPEC-076 A19; R20, R27; CHARTER 13): the streak rows, the freeze
//! ledger, the habit strength and the relight's due list are exported whole and erased, the
//! governor's row is reset in place, and an erase returns both tracks to their start state. Every
//! row here is synthetic.

#![allow(clippy::expect_used)]

use deck_streak_kernel::{DataRights, Db, Disposition, ExportedTable};
use deck_streak_streaks::data_rights::StreaksDataRights;
use deck_streak_streaks::streak::StreakState;
use serde_json::Value;

fn rows<'a>(exported: &'a [ExportedTable], table: &str) -> &'a [Value] {
    exported
        .iter()
        .find(|t| t.table == table)
        .map_or(&[][..], |t| t.rows.as_slice())
}

#[tokio::test]
async fn an_erase_returns_both_tracks_to_their_start_state() {
    let declaration = StreaksDataRights.declaration().expect("a declaration");
    assert_eq!(declaration.context(), "streaks");
    let tables: Vec<&str> = declaration.tables().iter().map(|t| t.table).collect();
    assert_eq!(
        tables,
        [
            "streak_state",
            "freeze_events",
            "habit_strength",
            "relight_due",
            "governor_state"
        ]
    );
    assert!(matches!(
        declaration.disposition("governor_state"),
        Some(Disposition::ResetInPlace { .. })
    ));

    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    for (track, current) in [("language", 9), ("law", 4)] {
        sqlx::query("INSERT INTO streak_state VALUES (?, ?, ?, 0, 19999, 1, 1000)")
            .bind(track)
            .bind(current)
            .bind(current)
            .execute(&mut *write)
            .await
            .expect("a streak row");
    }
    sqlx::query("INSERT INTO freeze_events (study_day, delta, reason, created_at) VALUES (19999, -1, 'consumed', 1000)")
        .execute(&mut *write)
        .await
        .expect("an event");
    sqlx::query("INSERT INTO habit_strength VALUES (19999, 0.5, 1000)")
        .execute(&mut *write)
        .await
        .expect("a strength row");
    sqlx::query("INSERT INTO relight_due VALUES (19999, 1000)")
        .execute(&mut *write)
        .await
        .expect("a due row");
    sqlx::query("UPDATE governor_state SET lapse_since = 19990, standby = 1, notified_day = 19995")
        .execute(&mut *write)
        .await
        .expect("the governor row");

    let exported = StreaksDataRights
        .export(&mut write)
        .await
        .expect("the export runs");
    assert_eq!(rows(&exported, "streak_state").len(), 2);
    assert_eq!(rows(&exported, "freeze_events").len(), 1);
    assert_eq!(rows(&exported, "habit_strength").len(), 1);
    assert_eq!(rows(&exported, "relight_due").len(), 1);
    assert_eq!(rows(&exported, "governor_state").len(), 1);

    StreaksDataRights
        .erase(&mut write)
        .await
        .expect("the erase runs");
    let after = StreaksDataRights
        .export(&mut write)
        .await
        .expect("the export runs");
    for table in [
        "streak_state",
        "freeze_events",
        "habit_strength",
        "relight_due",
    ] {
        assert!(
            rows(&after, table).is_empty(),
            "{table} is empty after an erase"
        );
    }
    let governor = rows(&after, "governor_state");
    assert_eq!(governor.len(), 1, "the governor's one row is kept");
    assert_eq!(governor[0]["lapse_since"], Value::Null);
    assert_eq!(governor[0]["standby"], 0);
    assert_eq!(governor[0]["notified_day"], Value::Null);
    // Both tracks read as the start state again: one freeze, no streak.
    assert_eq!(StreakState::start().freezes, 1);
    assert_eq!(StreakState::start().current, 0);
}
