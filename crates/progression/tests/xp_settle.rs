//! A day's derived XP is settled once per source, and a closed day's only rises (SPEC-072 A7 to
//! A11; R6 to R9; ADR-072).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_kernel::{Db, StudyDay, Track, UtcMillis};
use deck_streak_progression::settle::{
    DERIVED_PREFIXES, DERIVED_SOURCES, SettleCause, SettleError, SettleRequest, is_derived, settle,
};

const AT: i64 = 1_700_000_000_000;

async fn database() -> (tempfile::TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

fn request(source: &str, amount: u32, closed: bool) -> SettleRequest<'_> {
    SettleRequest {
        study_day: StudyDay::from_epoch_day(20_000),
        source,
        track: Track::Language,
        amount,
        closed,
    }
}

/// Settles once in its own transaction and answers what the row holds.
async fn settled(db: &Db, request: &SettleRequest<'_>, cause: SettleCause) -> u32 {
    let mut write = db.write().await.expect("a write");
    let held = settle(&mut write, request, cause, UtcMillis::from_epoch_millis(AT))
        .await
        .expect("the settlement runs");
    write.commit().await.expect("commit");
    held
}

async fn rows(db: &Db) -> Vec<(String, i64)> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT source, amount FROM xp_settlement ORDER BY source, track",
    )
    .fetch_all(db.reader())
    .await
    .expect("the rows are read")
}

#[tokio::test]
async fn a_closed_days_settled_xp_is_raised_and_never_lowered_by_a_recompute() {
    let (_directory, db) = database().await;
    assert_eq!(
        settled(&db, &request("reviews", 100, true), SettleCause::Recompute).await,
        100
    );
    assert_eq!(
        settled(&db, &request("reviews", 60, true), SettleCause::Recompute).await,
        100,
        "a recompute over less leaves the closed day's amount"
    );
    assert_eq!(
        settled(&db, &request("reviews", 130, true), SettleCause::Recompute).await,
        130,
        "a recompute over more raises it"
    );
    assert_eq!(rows(&db).await, vec![("reviews".to_owned(), 130)]);
}

#[tokio::test]
async fn the_owners_correction_replaces_a_closed_days_settled_xp() {
    let (_directory, db) = database().await;
    settled(&db, &request("reviews", 100, true), SettleCause::Recompute).await;
    assert_eq!(
        settled(
            &db,
            &request("reviews", 40, true),
            SettleCause::OwnersCorrection
        )
        .await,
        40
    );
    assert_eq!(rows(&db).await, vec![("reviews".to_owned(), 40)]);
    assert_eq!(
        settled(&db, &request("reviews", 10, true), SettleCause::Recompute).await,
        40,
        "the corrected day is closed still: a recompute does not lower it"
    );
}

#[tokio::test]
async fn the_open_days_settled_xp_follows_the_record() {
    let (_directory, db) = database().await;
    settled(&db, &request("reviews", 100, false), SettleCause::Recompute).await;
    assert_eq!(
        settled(&db, &request("reviews", 70, false), SettleCause::Recompute).await,
        70,
        "an open day follows the record down"
    );
    assert_eq!(
        settled(&db, &request("reviews", 90, false), SettleCause::Recompute).await,
        90,
        "and up"
    );
    assert_eq!(rows(&db).await, vec![("reviews".to_owned(), 90)]);
}

#[tokio::test]
async fn settle_keeps_one_row_per_study_day_source_and_track() {
    let (_directory, db) = database().await;
    for _ in 0..3 {
        settled(&db, &request("reviews", 100, true), SettleCause::Recompute).await;
    }
    settled(&db, &request("studied", 50, true), SettleCause::Recompute).await;
    let mut write = db.write().await.expect("a write");
    let other_track = SettleRequest {
        track: Track::Law,
        ..request("reviews", 20, true)
    };
    settle(
        &mut write,
        &other_track,
        SettleCause::Recompute,
        UtcMillis::from_epoch_millis(AT),
    )
    .await
    .expect("the law track settles");
    write.commit().await.expect("commit");
    let held = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM xp_settlement")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(
        held, 3,
        "one row per day, source and track, however often it settles"
    );
}

#[tokio::test]
async fn settle_refuses_a_source_outside_the_derived_registry() {
    let (_directory, db) = database().await;
    let mut write = db.write().await.expect("a write");
    let refusal = settle(
        &mut write,
        &request("reading:read:r1", 10, true),
        SettleCause::Recompute,
        UtcMillis::from_epoch_millis(AT),
    )
    .await
    .expect_err("a grant's source is not a derived one");
    assert!(matches!(refusal, SettleError::NotDerived), "{refusal:?}");
    let message = refusal.to_string();
    assert!(
        message.contains("derived") && !message.contains("reading"),
        "the refusal names the rule and never the source: {message}"
    );
    write.commit().await.expect("commit");
    assert!(rows(&db).await.is_empty(), "nothing was written");
    for source in DERIVED_SOURCES {
        settled(&db, &request(source, 1, true), SettleCause::Recompute).await;
    }
    assert_eq!(
        rows(&db).await.len(),
        DERIVED_SOURCES.len(),
        "every derived source settles"
    );
}

#[tokio::test]
async fn the_habit_sources_are_derived_only_with_a_course_code() {
    let (_directory, db) = database().await;
    assert_eq!(
        DERIVED_PREFIXES,
        ["read:", "readgoal:"],
        "the habit prefixes"
    );
    for source in ["read:qaa", "readgoal:qaa"] {
        assert!(is_derived(source), "{source} is derived");
        assert_eq!(
            settled(
                &db,
                &request(source, 7, false),
                SettleCause::OwnersCorrection
            )
            .await,
            7,
            "{source} settles"
        );
    }
    for source in [
        "read:",
        "read:QAA",
        "readgoal:a b",
        "reading:read:r1",
        "readgoal:",
        "read",
    ] {
        assert!(!is_derived(source), "{source:?} is not derived");
        let mut write = db.write().await.expect("a write");
        let refusal = settle(
            &mut write,
            &request(source, 7, false),
            SettleCause::OwnersCorrection,
            UtcMillis::from_epoch_millis(AT),
        )
        .await;
        assert!(
            matches!(refusal, Err(SettleError::NotDerived)),
            "{source:?} is refused as not derived: {refusal:?}"
        );
        write.commit().await.expect("commit");
    }
    assert_eq!(
        rows(&db).await.len(),
        2,
        "only the two habit sources were written"
    );
}

#[test]
fn the_derived_registry_and_the_tables_are_pinned_whole() {
    assert_eq!(
        DERIVED_SOURCES,
        [
            "reviews",
            "reviews_law",
            "studied",
            "backlog_zero",
            "streak",
            "score90",
            "graduations",
            "consistency",
            "ascendant",
        ]
    );
    assert_eq!(
        deck_streak_progression::settle::XP_SETTLEMENT_TABLE,
        "xp_settlement"
    );
    assert_eq!(deck_streak_progression::data_rights::BUFFS_TABLE, "buffs");
    assert_eq!(deck_streak_progression::buffs::ASCENDANT, "ascendant");
}
