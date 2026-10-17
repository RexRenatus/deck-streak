//! The landmarks' settings in notifications' own package (SPEC-102 R5, R6; ADR-322): the seed
//! stores the mark and the cursor once and answers whether it read no mark, the cursor reads back
//! as the day stored, and the advance moves it forward only. Every value here is synthetic.

#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};
use deck_streak_notifications::landmark_settings::{advance, offered_through, seed};
use deck_streak_notifications::landmarks::LANDMARK_HIGH_WATER_KEY;

const CURSOR: &str = "landmarks_offered_through";
const NOW: i64 = 1_728_043_200_000;

async fn database() -> (tempfile::TempDir, Db) {
    let scratch = tempfile::tempdir().expect("a scratch");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database");
    (scratch, db)
}

async fn setting(db: &Db, key: &str) -> Option<String> {
    let mut read = db.reader().acquire().await.expect("a reader");
    sqlx::query_scalar::<_, String>("SELECT value FROM notification_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&mut *read)
        .await
        .expect("the setting reads")
}

async fn cursor(db: &Db) -> Option<StudyDay> {
    let mut read = db.reader().acquire().await.expect("a reader");
    offered_through(&mut read).await.expect("the cursor reads")
}

fn now() -> UtcMillis {
    UtcMillis::from_epoch_millis(NOW)
}

#[tokio::test]
async fn the_first_seed_stores_both_and_a_later_one_neither() {
    let (_scratch, db) = database().await;
    assert_eq!(cursor(&db).await, None, "no cursor before the first seed");
    let first = seed(&db, "first mark", StudyDay::from_epoch_day(19_999), now())
        .await
        .expect("the seed writes");
    assert!(first, "the seed that reads no mark is the first run");
    assert_eq!(
        setting(&db, LANDMARK_HIGH_WATER_KEY).await.as_deref(),
        Some("first mark")
    );
    assert_eq!(setting(&db, CURSOR).await.as_deref(), Some("19999"));
    assert_eq!(cursor(&db).await, Some(StudyDay::from_epoch_day(19_999)));

    let later = seed(&db, "later mark", StudyDay::from_epoch_day(20_005), now())
        .await
        .expect("the seed writes");
    assert!(!later, "a seed that reads the mark is not the first run");
    assert_eq!(
        setting(&db, LANDMARK_HIGH_WATER_KEY).await.as_deref(),
        Some("first mark"),
        "the mark is stored once"
    );
    assert_eq!(
        cursor(&db).await,
        Some(StudyDay::from_epoch_day(19_999)),
        "the cursor is seeded once"
    );
}

#[tokio::test]
async fn a_stored_mark_without_a_cursor_seeds_the_cursor_only() {
    let (_scratch, db) = database().await;
    let mut write = db.write().await.expect("a write");
    sqlx::query("INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, 0)")
        .bind(LANDMARK_HIGH_WATER_KEY)
        .bind("imported mark")
        .execute(&mut *write)
        .await
        .expect("the import stores the mark");
    write.commit().await.expect("the write commits");

    let first = seed(&db, "new mark", StudyDay::from_epoch_day(19_999), now())
        .await
        .expect("the seed writes");
    assert!(!first, "an imported mark is not a first run");
    assert_eq!(
        setting(&db, LANDMARK_HIGH_WATER_KEY).await.as_deref(),
        Some("imported mark")
    );
    assert_eq!(cursor(&db).await, Some(StudyDay::from_epoch_day(19_999)));
}

#[tokio::test]
async fn the_cursor_advances_forward_only() {
    let (_scratch, db) = database().await;
    advance(&db, StudyDay::from_epoch_day(20_001), now())
        .await
        .expect("the first advance writes");
    assert_eq!(cursor(&db).await, Some(StudyDay::from_epoch_day(20_001)));
    advance(&db, StudyDay::from_epoch_day(20_003), now())
        .await
        .expect("the advance writes");
    assert_eq!(cursor(&db).await, Some(StudyDay::from_epoch_day(20_003)));
    advance(&db, StudyDay::from_epoch_day(20_002), now())
        .await
        .expect("a slower advance writes nothing");
    assert_eq!(
        setting(&db, CURSOR).await.as_deref(),
        Some("20003"),
        "a slower recompute never moves the cursor back"
    );
}

#[tokio::test]
async fn a_cursor_that_is_not_a_day_is_refused() {
    let (_scratch, db) = database().await;
    let mut write = db.write().await.expect("a write");
    sqlx::query("INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, 0)")
        .bind(CURSOR)
        .bind("not a day")
        .execute(&mut *write)
        .await
        .expect("a malformed cursor is stored");
    write.commit().await.expect("the write commits");
    let mut read = db.reader().acquire().await.expect("a reader");
    let refusal = offered_through(&mut read)
        .await
        .expect_err("a cursor that is not a day is refused");
    assert!(
        matches!(
            &refusal,
            KernelError::Database(sqlx::Error::Protocol(reason)) if reason.contains("not an epoch day")
        ),
        "the refusal names the cursor's value: {refusal:?}"
    );
}
