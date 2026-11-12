//! The score reads answer the rollups the recompute stored (SPEC-071 R10, R20, R22; §10): a day's
//! score, whose retention is present only on a day with an answered review, and the rollups of a
//! range the reads accept. The API's and the bot's tests read them through their surfaces; these
//! prove them in coordination itself, whose own tests are the ones cargo-mutants runs when it
//! mutates this package. Every review is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use deck_streak_analytics::metrics::daily_metrics;
use deck_streak_analytics::rollup::{RolledDay, fingerprint, roll_up};
use deck_streak_analytics::score::{Baseline, compute_score};
use deck_streak_coordination::score::{MAX_RANGE_DAYS, RangeError, day_rollups, day_score};
use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Db, StudyDay, StudyDayRule, UtcMillis};
use tempfile::TempDir;

const DAY: i64 = 20_000;
const DAY_MS: i64 = 86_400_000;
/// Anki's review answers; a learning step answers no review.
const REVIEW: i64 = 1;
const LEARNING: i64 = 0;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// `count` answers of `kind`, of cards 1 to `count`, on study day `number` from 05:00 UTC.
fn answers(number: i64, count: i64, kind: i64) -> Vec<Review> {
    (1..=count)
        .map(|card| Review {
            id: number * DAY_MS + 5 * 3_600_000 + card * 1_000,
            card_id: card,
            ease: 3,
            interval: 20,
            last_interval: 18,
            factor: 2500,
            taken_ms: 6_000,
            kind,
        })
        .collect()
}

/// Rolls study day `number` up over `reviews`, as the recompute's analytics step writes it, and
/// returns how many review answers its metrics count.
async fn roll(db: &Db, number: i64, reviews: &[Review]) -> i64 {
    let metrics = daily_metrics(
        reviews,
        StudyDayRule::default(),
        day(number),
        &BTreeMap::new(),
    );
    let score = compute_score(
        &metrics,
        None,
        1,
        Baseline {
            reviews: 10.0,
            minutes: 5.0,
        },
    );
    let print = fingerprint(reviews, None);
    let mut write = db.write().await.expect("a write");
    roll_up(
        &mut write,
        &RolledDay {
            metrics: &metrics,
            languages: &[],
            fingerprint: &print,
            score: &score,
        },
        UtcMillis::from_epoch_millis(1_000),
    )
    .await
    .expect("the day rolls up");
    write.commit().await.expect("the write commits");
    metrics.answered
}

#[tokio::test]
async fn a_days_score_is_its_rollups_and_has_a_retention_only_when_a_review_was_answered() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let answered = roll(&db, DAY, &answers(DAY, 4, REVIEW)).await;
    assert!(answered > 0, "the day answers reviews");
    let learning = roll(&db, DAY + 1, &answers(DAY + 1, 4, LEARNING)).await;
    assert_eq!(learning, 0, "learning steps answer no review");

    let score = day_score(&db, day(DAY))
        .await
        .expect("the score reads")
        .expect("the rolled day has a score");
    assert_eq!((score.day, score.reviews), (day(DAY), 4));
    assert!(
        score.retention.is_some() && score.pillars.retention.is_some(),
        "{score:?}"
    );
    let unanswered = day_score(&db, day(DAY + 1))
        .await
        .expect("the score reads")
        .expect("the rolled day has a score");
    assert_eq!(unanswered.reviews, 4);
    assert_eq!(
        (unanswered.retention, unanswered.pillars.retention),
        (None, None),
        "a day with no answered review has no retention to show"
    );
    let never = day_score(&db, day(DAY + 2)).await.expect("the score reads");
    assert_eq!(never, None, "a day never rolled up has no score");
}

#[tokio::test]
async fn a_range_is_read_whole_up_to_the_window_and_a_backwards_or_longer_one_is_refused() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    roll(&db, DAY, &answers(DAY, 2, REVIEW)).await;
    roll(&db, DAY + 1, &answers(DAY + 1, 3, REVIEW)).await;

    let read = day_rollups(&db, day(DAY), day(DAY + 1))
        .await
        .expect("a forward range reads");
    let read: Vec<(StudyDay, i64)> = read
        .iter()
        .map(|rollup| (rollup.metrics.day, rollup.metrics.reviews))
        .collect();
    assert_eq!(read, [(day(DAY), 2), (day(DAY + 1), 3)]);
    let one = day_rollups(&db, day(DAY), day(DAY))
        .await
        .expect("a range of one day reads");
    assert_eq!(one.len(), 1);
    let longest = day_rollups(&db, day(DAY), day(DAY + MAX_RANGE_DAYS - 1))
        .await
        .expect("a range of the window's length reads");
    assert_eq!(longest.len(), 2);

    let longer = day_rollups(&db, day(DAY), day(DAY + MAX_RANGE_DAYS)).await;
    assert!(matches!(longer, Err(RangeError::TooLong)), "{longer:?}");
    let backwards = day_rollups(&db, day(DAY + 1), day(DAY)).await;
    assert!(
        matches!(backwards, Err(RangeError::Backwards)),
        "{backwards:?}"
    );
}
