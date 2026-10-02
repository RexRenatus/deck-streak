//! A record beaten on the seed's own study day, through the production fold (SPEC-073 R11, R12;
//! ADR-303): the daemon's `recompute_fold`, every step registered, with the awards' offers over a
//! recording router. The first recompute seeds the bests and offers no record; a later recompute of
//! the same study day beats two of them, and each beat is offered once under its day's key; a third
//! recompute of that day beats one again and offers nothing new. Every review, card and instant is
//! synthetic.

#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::{AwardOffers, Celebrate, Celebration, FoldInput, Offers};
use deck_streak_daemon::wiring::recompute_fold;
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{Db, PortFuture, StudyDay, StudyDayRule, Track, UtcMillis};
use sqlx::Row;
use tempfile::TempDir;

/// Milliseconds in a day.
const DAY_MS: i64 = 86_400_000;
/// Milliseconds in an hour.
const HOUR_MS: i64 = 3_600_000;
/// The study day of every review: the owner's first, and today.
const TODAY: i64 = 20_000;

/// `hour` o'clock UTC on `day`, after the default rule's 04:00 rollover.
const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

/// `count` good answers of card 1 on `TODAY`, a minute apart from 09:00.
fn answers(count: i64) -> CollectionData {
    let reviews = (0..count)
        .map(|n| Review {
            id: at(TODAY, 9) + n * 60_000,
            card_id: 1,
            ease: 3,
            interval: 25,
            last_interval: 15,
            factor: 2500,
            taken_ms: 9_000,
            kind: 1,
        })
        .collect();
    CollectionData {
        reviews,
        cards: vec![Card {
            id: 1,
            note_id: 1,
            deck_id: 1,
            original_deck_id: 0,
            queue: 2,
            kind: 2,
            due: 1_090,
            interval: 30,
            factor: 2500,
            reps: 3,
            lapses: 0,
            track: Track::Language,
            course: None,
            tier: None,
        }],
        created_at: UtcMillis::from_epoch_millis(at(TODAY - 1_000, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// A router that answers every celebration and keeps each key it is handed.
#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl Celebrate for Recording {
    fn celebrate<'a>(&'a self, celebration: &'a Celebration) -> PortFuture<'a, ()> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(celebration.key.clone());
            Ok(())
        })
    }
}

/// One recompute of `data` at `now`, after a successful sync that started today, with the awards'
/// offers: the record keys it offered, in order.
async fn recompute(db: &Db, data: &CollectionData, now: i64) -> Vec<String> {
    let router = Arc::new(Recording::default());
    let offers = AwardOffers::new(router.clone());
    recompute_fold(AnalyticsSettings::default())
        .expect("the fold registers")
        .run(
            db,
            &FoldInput {
                data,
                rule: StudyDayRule::default(),
                now: UtcMillis::from_epoch_millis(now),
                synced_in: Some(StudyDay::from_epoch_day(TODAY)),
                courses_digest: Some("0123456789abcdef"),
                base_reviews: 0,
                offers: Some(&offers as &dyn Offers),
            },
        )
        .await
        .expect("the fold runs");
    let keys = router
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    keys.into_iter()
        .filter(|key| key.starts_with("pr:"))
        .collect()
}

/// Each stored record as `(kind, value, study day, previous, marked)`, in the kinds' text order.
async fn records(db: &Db) -> Vec<(String, i64, i64, i64, bool)> {
    let mut read = db.reader().acquire().await.expect("a reader");
    sqlx::query(
        "SELECT kind, value, study_day, previous, celebrated_at IS NOT NULL AS marked \
         FROM records ORDER BY kind",
    )
    .fetch_all(&mut *read)
    .await
    .expect("the records read")
    .iter()
    .map(|row| {
        (
            row.get("kind"),
            row.get("value"),
            row.get("study_day"),
            row.get("previous"),
            row.get("marked"),
        )
    })
    .collect()
}

#[tokio::test]
async fn a_record_beaten_on_the_seeds_own_day_is_offered_through_the_production_fold() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let seeded = recompute(&db, &answers(3), at(TODAY, 12)).await;
    let beaten = recompute(&db, &answers(4), at(TODAY, 18)).await;
    assert_eq!(
        beaten,
        ["pr:best_score:20000", "pr:most_reviews:20000"],
        "each beat of the seed's own day is offered under its day's key"
    );
    assert!(seeded.is_empty(), "the seed offers no record: {seeded:?}");
    assert_eq!(
        records(&db).await,
        [
            ("best_score".to_owned(), 76, TODAY, 75, true),
            ("most_reviews".to_owned(), 4, TODAY, 3, true),
        ],
        "each beat holds the seeded value it beat, marked at the router's answer"
    );
    let again = recompute(&db, &answers(5), at(TODAY, 20)).await;
    assert!(
        again.is_empty(),
        "a later beat of the same day, after the celebration, offers nothing new: {again:?}"
    );
}
