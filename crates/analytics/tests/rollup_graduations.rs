//! The graduations the XP exchange readout divides by (SPEC-075 R4, A16): each stored day's own
//! `daily_rollup.graduations` inside a window, and no day outside it.

// An integration test is test code: its helpers panic on a failed fixture, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_analytics::rollup::graduations;
use deck_streak_kernel::{Db, StudyDay};

/// The study day with epoch day number `day`.
const fn day(day: i64) -> StudyDay {
    StudyDay::from_epoch_day(day)
}

/// Writes a synthetic rollup of `on` holding `graduations` graduated cards.
async fn rollup(db: &Db, on: i64, graduations: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, ?2, 2, 14.0, 0, 0, 40, 36, 64, 60, 2, \
         5, 30, 'live:1736911800000', 77, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
         'synthetic', 1000, 1000)",
    )
    .bind(on)
    .bind(graduations)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    write.commit().await.expect("the commit");
}

#[tokio::test]
async fn the_graduations_of_a_window_are_each_days_own() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    for (on, graduated) in [(19_997_i64, 5_i64), (19_998, 0), (19_999, 7), (20_000, 2)] {
        rollup(&db, on, graduated).await;
    }
    let mut read = db.reader().acquire().await.expect("a reader");

    // The window 19_998 to 19_999 answers those two days, each its own count, and neither edge
    // day outside it.
    let windowed = graduations(&mut read, Some((day(19_998), day(19_999))))
        .await
        .expect("the window's graduations");
    assert_eq!(
        windowed,
        BTreeMap::from([(day(19_998), 0), (day(19_999), 7)])
    );

    // No window answers every stored day.
    let every = graduations(&mut read, None)
        .await
        .expect("every day's graduations");
    println!("examined {} stored day(s)", every.len());
    assert_eq!(
        every,
        BTreeMap::from([
            (day(19_997), 5),
            (day(19_998), 0),
            (day(19_999), 7),
            (day(20_000), 2),
        ])
    );
    drop(read);
    db.close().await;
}
