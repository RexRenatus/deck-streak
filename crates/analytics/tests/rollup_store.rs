//! Rolling a day up is idempotent, and a settled day keeps what it closed with (SPEC-071 A5, A6;
//! R5, R8, R9, R11, R14, R16): the same reviews write identical rows, a changed day moves only its
//! metrics and `updated_at`, and a card state is never invented for a day no recompute recorded.
//! Every review, card and course is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_analytics::metrics::{daily_metrics, language_metrics};
use deck_streak_analytics::rollup::{
    RolledDay, RollupStore, StoredDay, fingerprint, record_card_state, record_close,
    record_settled, roll_up,
};
use deck_streak_analytics::score::{Baseline, compute_score};
use deck_streak_analytics::snapshot::CardState;
use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{CourseCode, Db, StudyDay, StudyDayRule, UtcMillis};
use tempfile::TempDir;

const DAY: i64 = 20_000;
const DAY_MS: i64 = 86_400_000;
/// A synthetic courses digest.
const DIGEST: &str = "0123456789abcdef";

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// Reviews of cards 1 to `count` on study day `day`, from 05:00 UTC.
fn reviews(day: i64, count: i64) -> Vec<Review> {
    (1..=count)
        .map(|card| Review {
            id: day * DAY_MS + 5 * 3_600_000 + card * 1_000,
            card_id: card,
            ease: 1 + card % 4,
            interval: 20 + card,
            last_interval: 18 + card,
            factor: 2500,
            taken_ms: 7_000 * card,
            kind: card % 4,
        })
        .collect()
}

/// Rolls `day` up over `reviews` at `now`, in one write, and returns whether the day changed.
async fn roll(db: &Db, day: i64, reviews: &[Review], now: i64) -> bool {
    let day = StudyDay::from_epoch_day(day);
    let rule = StudyDayRule::default();
    let card_decks: BTreeMap<i64, i64> = reviews.iter().map(|review| (review.card_id, 7)).collect();
    let course = CourseCode::new("qaa").expect("a code");
    let card_courses: BTreeMap<i64, CourseCode> = reviews
        .iter()
        .filter(|review| review.card_id % 2 == 1)
        .map(|review| (review.card_id, course))
        .collect();
    let metrics = daily_metrics(reviews, rule, day, &card_decks);
    let languages = language_metrics(reviews, rule, &card_courses, &BTreeSet::from([day]));
    let print = fingerprint(reviews, Some(DIGEST));
    let score = compute_score(
        &metrics,
        None,
        1,
        Baseline {
            reviews: 10.0,
            minutes: 5.0,
        },
    );
    let mut write = db.write().await.expect("a write");
    let changed = roll_up(
        &mut write,
        &RolledDay {
            metrics: &metrics,
            languages: &languages,
            fingerprint: &print,
            score: &score,
        },
        UtcMillis::from_epoch_millis(now),
    )
    .await
    .expect("the day rolls up");
    write.commit().await.expect("the write commits");
    changed
}

/// Everything stored of `day`: its rollup and its per-course rows.
async fn stored(db: &Db, day: i64) -> (Vec<StoredDay>, Vec<String>) {
    let store = RollupStore::new(db.clone());
    let day = StudyDay::from_epoch_day(day);
    let rollups = store.days(day, day).await.expect("the rollup reads");
    let languages = store
        .language_days(day)
        .await
        .expect("the per-course rows read")
        .iter()
        .map(|row| format!("{row:?}"))
        .collect();
    (rollups, languages)
}

#[tokio::test]
async fn rerolling_a_day_with_the_same_reviews_writes_identical_rows() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let day_reviews = reviews(DAY, 6);

    assert!(
        roll(&db, DAY, &day_reviews, 1_000).await,
        "a new row is a change"
    );
    let first = stored(&db, DAY).await;
    assert_eq!(first.0.len(), 1, "one row for the day");
    let row = &first.0[0];
    assert_eq!(row.metrics.day.epoch_day(), DAY);
    assert_eq!(row.metrics.reviews, 6, "the day's six reviews");
    assert_eq!(row.fingerprint, fingerprint(&day_reviews, Some(DIGEST)));
    assert_eq!(row.created_at, UtcMillis::from_epoch_millis(1_000));
    assert_eq!(row.updated_at, UtcMillis::from_epoch_millis(1_000));
    assert_eq!(
        first.1.len(),
        1,
        "one row for the day's one course: {:?}",
        first.1
    );

    // The same reviews again, later: nothing changes, not even `updated_at`.
    assert!(
        !roll(&db, DAY, &day_reviews, 9_000).await,
        "the same reviews are no change"
    );
    assert_eq!(
        stored(&db, DAY).await,
        first,
        "a re-roll with the same reviews"
    );
    // Twice more, and the table still holds one row for the day.
    assert!(!roll(&db, DAY, &day_reviews, 10_000).await);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM daily_rollup")
        .fetch_one(db.reader())
        .await
        .expect("the count reads");
    assert_eq!(count, 1, "one row per study day");
}

#[tokio::test]
async fn rerolling_a_settled_day_keeps_its_card_state_and_provenance() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let settled = StudyDay::from_epoch_day(DAY);
    let state = CardState {
        mature_count: 40,
        young_count: 12,
        leech_active: 3,
        backlog: 7,
        due_today: 21,
    };
    assert!(roll(&db, DAY, &reviews(DAY, 4), 1_000).await);
    assert!(roll(&db, DAY + 1, &reviews(DAY + 1, 2), 1_000).await);
    let at = UtcMillis::from_epoch_millis(2_000);
    let mut write = db.write().await.expect("a write");
    record_card_state(&mut write, settled, &state, at)
        .await
        .expect("the card state records");
    record_close(&mut write, settled, 77)
        .await
        .expect("the close records");
    assert!(
        record_settled(&mut write, settled, at)
            .await
            .expect("the settle records"),
        "the settled day has its rollup"
    );
    write.commit().await.expect("the write commits");

    // A review that reaches the copy late re-rolls the settled day.
    let late = reviews(DAY, 5);
    assert!(
        roll(&db, DAY, &late, 5_000).await,
        "a late review is a change"
    );
    let (rows, _) = stored(&db, DAY).await;
    let row = rows.first().expect("the settled day's rollup");
    assert_eq!(row.metrics.reviews, 5, "the late review counts");
    assert_eq!(row.fingerprint, fingerprint(&late, Some(DIGEST)));
    assert_eq!(row.updated_at, UtcMillis::from_epoch_millis(5_000));
    assert_eq!(row.created_at, UtcMillis::from_epoch_millis(1_000));
    // What it closed with stays.
    assert_eq!(row.card_state, Some(state));
    assert_eq!(row.card_state_src.as_deref(), Some("live:2000"));
    assert_eq!(row.score_at_close, Some(77));
    assert_eq!(row.settled_at, Some(at));

    // A day no recompute recorded keeps no card state, however often it is rolled up.
    assert!(roll(&db, DAY + 1, &reviews(DAY + 1, 3), 6_000).await);
    let (rows, _) = stored(&db, DAY + 1).await;
    let row = rows.first().expect("the unrecorded day's rollup");
    assert_eq!(row.metrics.reviews, 3);
    assert_eq!(row.card_state, None, "never recorded, never invented");
    assert_eq!(row.card_state_src, None);
    assert_eq!(row.score_at_close, None);
    assert_eq!(row.settled_at, None);
}
