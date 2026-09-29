//! The streaks' fold step (SPEC-076 A5, A6, A10, A14; R17 to R20): both tracks' streaks, the freeze
//! ledger and the governor's anchor, written by phase 3 for every study day the fold visits. Every
//! review, card and instant is synthetic and set by hand.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::streaks::StreaksStep;
use deck_streak_coordination::recompute::{Fold, Phase};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{Db, StudyDay, StudyDayRule, Track, UtcMillis};
use sqlx::Row;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
const D0: i64 = 20_000;
const CREATED: i64 = D0 - 1_000;

const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

const fn review(instant: i64, card: i64) -> Review {
    Review {
        id: instant,
        card_id: card,
        ease: 3,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

const fn card(id: i64, track: Track) -> Card {
    Card {
        id,
        note_id: id,
        deck_id: 1,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due: 1_090,
        interval: 30,
        factor: 2500,
        reps: 3,
        lapses: 0,
        track,
        course: None,
        tier: None,
    }
}

fn collection(reviews: Vec<Review>) -> CollectionData {
    CollectionData {
        reviews,
        cards: vec![card(1, Track::Language), card(2, Track::Law)],
        created_at: UtcMillis::from_epoch_millis(at(CREATED, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

fn fold() -> Fold {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    let (step, _due) = StreaksStep::new();
    fold.register(Phase::StreaksAndGovernor, Box::new(step))
        .expect("the streaks step is phase 3's");
    fold
}

async fn recompute(fold: &Fold, db: &Db, data: &CollectionData, now: i64, synced_in: i64) {
    fold.run(
        db,
        &deck_streak_coordination::recompute::FoldInput {
            data,
            rule: StudyDayRule::default(),
            now: UtcMillis::from_epoch_millis(now),
            synced_in: Some(StudyDay::from_epoch_day(synced_in)),
            courses_digest: Some("0123456789abcdef"),
        },
    )
    .await
    .expect("the fold runs");
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// `(current, longest, freezes)` of `track`'s streak row, when it has one.
async fn streak(db: &Db, track: &str) -> Option<(i64, i64, i64)> {
    let mut write = db.write().await.expect("a write");
    sqlx::query("SELECT current_days, longest_days, freezes FROM streak_state WHERE track = ?")
        .bind(track)
        .fetch_optional(&mut *write)
        .await
        .expect("the streak row reads")
        .map(|row| (row.get(0), row.get(1), row.get(2)))
}

async fn events(db: &Db) -> Vec<(i64, i64, String)> {
    let mut write = db.write().await.expect("a write");
    sqlx::query("SELECT study_day, delta, reason FROM freeze_events ORDER BY study_day, reason")
        .fetch_all(&mut *write)
        .await
        .expect("the freeze events read")
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect()
}

/// One language review on each study day of `first..=last`.
fn language_run(first: i64, last: i64) -> Vec<Review> {
    (first..=last).map(|d| review(at(d, 9), 1)).collect()
}

#[tokio::test]
async fn a_settled_day_writes_its_freeze_events_once() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let data = collection(language_run(D0 - 7, D0 - 1));
    let fold = fold();
    // The seventh study day is closed by D0's morning sync, then settled again by the next.
    recompute(&fold, &db, &data, at(D0, 10), D0).await;
    let first = events(&db).await;
    recompute(&fold, &db, &data, at(D0, 15), D0).await;
    recompute(&fold, &db, &data, at(D0 + 1, 10), D0 + 1).await;
    assert_eq!(
        events(&db).await,
        first,
        "a later recompute writes no event again"
    );
    assert_eq!(
        first,
        vec![(D0 - 1, 1, "streak_earn".to_owned())],
        "the seventh day earns one freeze, once"
    );
    assert_eq!(streak(&db, "language").await, Some((7, 7, 2)));
    println!("examined {} freeze events", first.len());
}

#[tokio::test]
async fn a_streak_studied_daily_survives_one_sync_a_day() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let data = collection(language_run(D0 - 9, D0));
    let fold = fold();
    for d in D0 - 9..=D0 {
        recompute(&fold, &db, &data, at(d, 12), d).await;
    }
    assert_eq!(
        streak(&db, "language").await,
        Some((10, 10, 2)),
        "ten study days, one sync each, leave one unbroken streak"
    );
    let broken = events(&db)
        .await
        .into_iter()
        .filter(|(_, _, reason)| reason == "streak_break" || reason == "consumed")
        .count();
    assert_eq!(
        broken, 0,
        "a daily streak neither breaks nor spends a freeze"
    );
}

#[tokio::test]
async fn the_law_streak_decays_without_law_study() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let mut reviews = language_run(D0 - 6, D0);
    reviews.extend((D0 - 5..=D0 - 3).map(|d| review(at(d, 10), 2)));
    let data = collection(reviews);
    let fold = fold();
    recompute(&fold, &db, &data, at(D0 - 3, 12), D0 - 3).await;
    assert_eq!(
        streak(&db, "law").await.map(|s| s.0),
        Some(3),
        "three law days make a law streak"
    );
    // Days pass with language study only: the law streak is asked every day, so it decays.
    recompute(&fold, &db, &data, at(D0, 12), D0).await;
    assert_eq!(
        streak(&db, "law").await,
        Some((0, 3, 1)),
        "the law streak fell to nothing; its longest and its start freeze stand"
    );
    assert_eq!(
        streak(&db, "language").await.map(|s| s.0),
        Some(7),
        "the language streak is untouched"
    );
}

#[tokio::test]
async fn one_episode_keeps_its_anchor_across_recomputes() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    // Five study days far before today, then silence longer than the walk's cap.
    let data = collection(language_run(D0 - 130, D0 - 126));
    let fold = fold();
    let anchor = |db: &Db| {
        let db = db.clone();
        async move {
            let mut write = db.write().await.expect("a write");
            sqlx::query("SELECT lapse_since FROM governor_state WHERE id = 1")
                .fetch_optional(&mut *write)
                .await
                .expect("the governor row reads")
                .and_then(|row| row.get::<Option<i64>, _>(0))
        }
    };
    recompute(&fold, &db, &data, at(D0, 12), D0).await;
    assert_eq!(
        anchor(&db).await,
        Some(D0 - 120),
        "the walk's horizon anchors it"
    );
    recompute(&fold, &db, &data, at(D0 + 1, 12), D0 + 1).await;
    assert_eq!(
        anchor(&db).await,
        Some(D0 - 120),
        "the next day's recompute keeps the episode's anchor"
    );
}
