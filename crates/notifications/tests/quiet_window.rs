//! The one quiet check (SPEC-411 A18; R12): `in_quiet_window_on` reads the owner's quiet hours
//! before the policy's, at the offset it is given. Every instant and setting is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, UtcMillis, UtcOffset};
use deck_streak_notifications::Policy;
use deck_streak_notifications::router::in_quiet_window_on;

/// One hour, in milliseconds.
const HOUR_MS: i64 = 3_600_000;
/// Midnight UTC opening the epoch's second day.
const DAY_TWO: i64 = 86_400_000;

/// `hour` o'clock and `minute` UTC on the epoch's second day.
const fn utc(hour: i64, minute: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(DAY_TWO + hour * HOUR_MS + minute * 60_000)
}

/// Whether each of `instants`, read at its offset, is in the quiet window, as `db` reads it.
async fn quiet(db: &Db, policy: &Policy, instants: &[(UtcOffset, UtcMillis)]) -> Vec<bool> {
    let mut write = db.write().await.expect("a write");
    let mut answers = Vec::new();
    for (offset, now) in instants {
        answers.push(
            in_quiet_window_on(&mut write, policy, *offset, *now)
                .await
                .expect("the quiet window is read"),
        );
    }
    answers
}

#[tokio::test]
async fn the_quiet_window_reads_the_owners_hours_before_the_policys() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let policy = Policy::compiled().expect("the compiled policy parses");
    let two_hours_west = UtcOffset::from_minutes(-120).expect("an offset in bounds");
    // Noon and 23:30 UTC, and 13:30 UTC read two hours west, which is 11:30 there.
    let instants = [
        (UtcOffset::UTC, utc(12, 0)),
        (UtcOffset::UTC, utc(23, 30)),
        (two_hours_west, utc(13, 30)),
    ];
    let before = quiet(&db, &policy, &instants).await;

    // The owner's quiet hours, 10:00 to 13:00.
    let mut write = db.write().await.expect("a write");
    for (key, value) in [("quiet_start_min", "600"), ("quiet_end_min", "780")] {
        sqlx::query(
            "INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, 0) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&mut *write)
        .await
        .expect("a setting is written");
    }
    write.commit().await.expect("the settings committed");
    let after = quiet(&db, &policy, &instants).await;

    assert_eq!(
        (before, after),
        (vec![false, true, false], vec![true, false, true]),
        "the policy's 23:00 to 07:30 answers until the owner sets hours, and the owner's after"
    );
}
