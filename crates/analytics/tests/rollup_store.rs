//! Rolling a day up is idempotent, and a settled day keeps what it closed with (SPEC-071 A5, A6;
//! R5, R8, R9, R11, R14, R16): the same reviews write identical rows, a changed day moves only its
//! metrics and `updated_at`, and a card state is never invented for a day no recompute recorded.
//! The recent totals read the stored rows on or before a day, most recent first (SPEC-073 R5, R10).
//! Every review, card and course is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_analytics::metrics::{daily_metrics, language_metrics};
use deck_streak_analytics::rollup::{
    self, RolledDay, RollupStore, StoredDay, fingerprint, fingerprints, recent_totals,
    recent_volumes, record_card_state, record_close, record_score, record_settled, roll_up,
    settle_cursor,
};
use deck_streak_analytics::score::Score;
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

/// SPEC-071 §10: each read of the store answers what its writes recorded, so the fold's reads are
/// proved in this package too, whose tests are the only ones cargo-mutants runs on it.
#[tokio::test]
async fn the_stores_reads_answer_what_its_writes_recorded() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let day_reviews = reviews(DAY, 3);
    assert!(roll(&db, DAY, &day_reviews, 1_000).await, "a new row");
    let day = StudyDay::from_epoch_day(DAY);
    let missing = StudyDay::from_epoch_day(DAY + 1);
    let mut write = db.write().await.expect("a write");

    let rolled = rollup::stored(&mut write, day)
        .await
        .expect("the day reads")
        .expect("the rolled day is stored");
    assert_eq!(rolled.metrics.reviews, 3, "the day's three reviews");
    let absent = rollup::stored(&mut write, missing)
        .await
        .expect("the day reads");
    assert!(
        absent.is_none(),
        "a day never rolled up is none: {absent:?}"
    );
    assert_eq!(
        fingerprints(&mut write)
            .await
            .expect("the fingerprints read"),
        BTreeMap::from([(day, fingerprint(&day_reviews, Some(DIGEST)))])
    );
    let volumes: Vec<(StudyDay, i64, u64)> = recent_volumes(&mut write, day)
        .await
        .expect("the volumes read")
        .iter()
        .map(|volume| (volume.day, volume.reviews, volume.seconds.to_bits()))
        .collect();
    assert_eq!(volumes, [(day, 3, rolled.metrics.seconds.to_bits())]);

    // A new score replaces the day's.
    let rescored = Score {
        total: rolled.score.total + 1,
        consistency: rolled.score.consistency + 1.0,
        mastery: rolled.score.mastery + 1.0,
        ..rolled.score
    };
    record_score(&mut write, day, &rescored)
        .await
        .expect("the score records");
    let read = rollup::stored(&mut write, day)
        .await
        .expect("the day reads")
        .expect("the day is stored")
        .score;
    assert_eq!(
        (
            read.total,
            read.consistency.to_bits(),
            read.mastery.to_bits()
        ),
        (
            rescored.total,
            rescored.consistency.to_bits(),
            rescored.mastery.to_bits()
        )
    );

    // Settling a day with no rollup records nothing; settling the day moves the cursor to it.
    let at = UtcMillis::from_epoch_millis(2_000);
    assert_eq!(
        settle_cursor(&mut write).await.expect("the cursor reads"),
        None
    );
    assert!(
        !record_settled(&mut write, missing, at)
            .await
            .expect("the settle runs"),
        "no row to settle"
    );
    assert!(
        record_settled(&mut write, day, at)
            .await
            .expect("the settle records"),
        "the day's row settles"
    );
    assert_eq!(
        settle_cursor(&mut write).await.expect("the cursor reads"),
        Some(day)
    );
}

/// The recent totals on or before `through`, at most `limit` of them, as `(epoch day, score,
/// reviews, the bits of the seconds)`.
async fn recent(db: &Db, through: i64, limit: i64) -> Vec<(i64, i64, i64, u64)> {
    let mut write = db.write().await.expect("a write");
    recent_totals(&mut write, StudyDay::from_epoch_day(through), limit)
        .await
        .expect("the recent totals read")
        .iter()
        .map(|row| {
            (
                row.day.epoch_day(),
                row.score,
                row.reviews,
                row.seconds.to_bits(),
            )
        })
        .collect()
}

/// SPEC-073 R5, R10: the recent totals are the `limit` stored rows on or before the day, most
/// recent first, each with its stored score, its study reviews and their seconds.
#[tokio::test]
async fn the_recent_totals_read_the_latest_rows_on_or_before_the_day() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    // Four days with their own review counts, each scored over its computed score.
    for (number, count, total) in [
        (DAY - 2, 2, 41),
        (DAY - 1, 4, 57),
        (DAY, 5, 63),
        (DAY + 1, 3, 72),
    ] {
        assert!(
            roll(&db, number, &reviews(number, count), 1_000).await,
            "a new row"
        );
        let day = StudyDay::from_epoch_day(number);
        let mut write = db.write().await.expect("a write");
        let score = rollup::stored(&mut write, day)
            .await
            .expect("the day reads")
            .expect("the rolled day is stored")
            .score;
        record_score(&mut write, day, &Score { total, ..score })
            .await
            .expect("the score records");
        write.commit().await.expect("the score commits");
    }
    // Card n answers in 7n seconds, under the cap: 2 cards 21 s, 4 cards 70 s, 5 cards 105 s,
    // 3 cards 42 s.
    assert_eq!(
        recent(&db, DAY, 2).await,
        [
            (DAY, 63, 5, 105.0_f64.to_bits()),
            (DAY - 1, 57, 4, 70.0_f64.to_bits()),
        ],
        "the two most recent on or before the day, the later day excluded"
    );
    assert_eq!(
        recent(&db, DAY + 1, 10).await,
        [
            (DAY + 1, 72, 3, 42.0_f64.to_bits()),
            (DAY, 63, 5, 105.0_f64.to_bits()),
            (DAY - 1, 57, 4, 70.0_f64.to_bits()),
            (DAY - 2, 41, 2, 21.0_f64.to_bits()),
        ],
        "every row, most recent first"
    );
    assert_eq!(
        recent(&db, DAY - 3, 10).await,
        [],
        "no row on or before a day before the first"
    );
}

/// SPEC-071 R18: a fingerprint moves with every review and with the courses' digest.
#[test]
fn a_fingerprint_changes_with_the_reviews_and_the_courses() {
    let two = reviews(DAY, 2);
    let prints = [
        fingerprint(&two, Some(DIGEST)),
        fingerprint(&reviews(DAY, 3), Some(DIGEST)),
        fingerprint(&two, None),
        fingerprint(&two, Some("")),
    ];
    let distinct: BTreeSet<&String> = prints.iter().collect();
    assert_eq!(distinct.len(), prints.len(), "{prints:?}");
    assert_eq!(
        fingerprint(&reviews(DAY, 2), Some(DIGEST)),
        prints[0],
        "the same reviews print the same"
    );
}
