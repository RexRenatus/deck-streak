//! The rollup repository (SPEC-071 R5, R8 to R11, R14, R16 and R18): `daily_rollup` and
//! `daily_lang_stats`, the day's review fingerprint, and the settle cursor.
//!
//! One row per study day. Rolling a day up writes its metrics, its per-course rows and its
//! fingerprint; a roll-up that finds the same metrics and fingerprint changes nothing, so the
//! row's `updated_at` moves only when the day's reviews did. A day's card state and its provenance
//! are written only by [`record_card_state`], which the recompute calls for the day it evaluates as
//! current or settles as the closing day (R9); no roll-up touches them, nor the score the day closed
//! with, nor the instant it was settled. The last settled day is the cursor (R16).
//!
//! Every write takes the caller's connection, inside the transaction the recompute holds, so one
//! day's settle is one write.

use std::collections::BTreeMap;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::courses::content_digest;
use deck_streak_kernel::{CourseCode, Db, KernelError, StudyDay, UtcMillis};
use serde_json::{Value, json};
use sqlx::SqliteConnection;

use crate::metrics::{DailyMetrics, LanguageDay};
use crate::score::{BASELINE_ROWS_READ, DayVolume, Score, grade_band};
use crate::snapshot::CardState;

/// A recorded card state's provenance starts with this, then the epoch milliseconds it was
/// recorded at.
pub const LIVE_PREFIX: &str = "live:";

/// A day's rollup as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredDay {
    /// The day's metrics.
    pub metrics: DailyMetrics,
    /// The card state, when a recompute recorded one.
    pub card_state: Option<CardState>,
    /// Its provenance, `live:<epoch milliseconds>`, when it was recorded.
    pub card_state_src: Option<String>,
    /// The stored score.
    pub score: Score,
    /// The total the day closed with, once settled.
    pub score_at_close: Option<i64>,
    /// When the day was settled.
    pub settled_at: Option<UtcMillis>,
    /// The day's review fingerprint.
    pub fingerprint: String,
    /// When the row was first written.
    pub created_at: UtcMillis,
    /// When the day was last rolled up with changed reviews.
    pub updated_at: UtcMillis,
}

/// What one roll-up writes for a day.
#[derive(Clone, Copy, Debug)]
pub struct RolledDay<'a> {
    /// The day's metrics.
    pub metrics: &'a DailyMetrics,
    /// The day's per-course rows, which replace the day's earlier ones.
    pub languages: &'a [LanguageDay],
    /// The day's review fingerprint.
    pub fingerprint: &'a str,
    /// The score to store with it.
    pub score: &'a Score,
}

/// The fingerprint of a day's study reviews (R18): a digest of every field of each review, in their
/// order, together with the courses' digest, so a review that arrives late, or a changed courses
/// file, changes it.
#[must_use]
pub fn fingerprint(reviews: &[Review], courses_digest: Option<&str>) -> String {
    let mut bytes = Vec::with_capacity(reviews.len() * 64 + 24);
    for review in reviews {
        for field in [
            review.id,
            review.card_id,
            review.ease,
            review.interval,
            review.last_interval,
            review.factor,
            review.taken_ms,
            review.kind,
        ] {
            bytes.extend_from_slice(&field.to_le_bytes());
        }
    }
    // The courses' digest closes the text, marked so that no courses and an empty digest differ.
    match courses_digest {
        Some(digest) => {
            bytes.push(1);
            bytes.extend_from_slice(digest.as_bytes());
        }
        None => bytes.push(0),
    }
    content_digest(&bytes)
}

/// The provenance of a card state recorded at `at`.
#[must_use]
pub fn provenance(at: UtcMillis) -> String {
    format!("{LIVE_PREFIX}{}", at.epoch_millis())
}

/// Rolls `rolled` up inside `write` at `now`: the row's metrics, fingerprint and score, and the
/// day's per-course rows replaced. Returns whether the day's metrics or fingerprint changed (or the
/// row is new), which is when `updated_at` moves.
///
/// # Errors
///
/// [`KernelError::Database`] when a write fails.
pub async fn roll_up(
    write: &mut SqliteConnection,
    rolled: &RolledDay<'_>,
    now: UtcMillis,
) -> Result<bool, KernelError> {
    let metrics = rolled.metrics;
    let day = metrics.day.epoch_day();
    let now = now.epoch_millis();
    let score = rolled.score;
    // A row whose metrics and fingerprint are unchanged is left as it is: `updated_at` moves only
    // when the day's reviews did.
    let changed = sqlx::query!(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, score, \
         consistency, retention, workload, volume, mastery, fingerprint, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, \
         ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?25) \
         ON CONFLICT (study_day) DO UPDATE SET reviews = excluded.reviews, \
         learn_count = excluded.learn_count, review_count = excluded.review_count, \
         relearn_count = excluded.relearn_count, filtered_count = excluded.filtered_count, \
         seconds = excluded.seconds, answered = excluded.answered, passed = excluded.passed, \
         true_retention = excluded.true_retention, graduations = excluded.graduations, \
         decks_studied = excluded.decks_studied, avg_answer_seconds = excluded.avg_answer_seconds, \
         young_answered = excluded.young_answered, young_passed = excluded.young_passed, \
         mature_answered = excluded.mature_answered, mature_passed = excluded.mature_passed, \
         fingerprint = excluded.fingerprint, updated_at = excluded.updated_at \
         WHERE daily_rollup.fingerprint IS NOT excluded.fingerprint \
         OR daily_rollup.reviews IS NOT excluded.reviews \
         OR daily_rollup.learn_count IS NOT excluded.learn_count \
         OR daily_rollup.review_count IS NOT excluded.review_count \
         OR daily_rollup.relearn_count IS NOT excluded.relearn_count \
         OR daily_rollup.filtered_count IS NOT excluded.filtered_count \
         OR daily_rollup.seconds IS NOT excluded.seconds \
         OR daily_rollup.answered IS NOT excluded.answered \
         OR daily_rollup.passed IS NOT excluded.passed \
         OR daily_rollup.true_retention IS NOT excluded.true_retention \
         OR daily_rollup.graduations IS NOT excluded.graduations \
         OR daily_rollup.decks_studied IS NOT excluded.decks_studied \
         OR daily_rollup.avg_answer_seconds IS NOT excluded.avg_answer_seconds \
         OR daily_rollup.young_answered IS NOT excluded.young_answered \
         OR daily_rollup.young_passed IS NOT excluded.young_passed \
         OR daily_rollup.mature_answered IS NOT excluded.mature_answered \
         OR daily_rollup.mature_passed IS NOT excluded.mature_passed",
        day,
        metrics.reviews,
        metrics.learn_count,
        metrics.review_count,
        metrics.relearn_count,
        metrics.filtered_count,
        metrics.seconds,
        metrics.answered,
        metrics.passed,
        metrics.true_retention,
        metrics.graduations,
        metrics.decks_studied,
        metrics.avg_answer_seconds,
        metrics.young_answered,
        metrics.young_passed,
        metrics.mature_answered,
        metrics.mature_passed,
        score.total,
        score.consistency,
        score.retention,
        score.workload,
        score.volume,
        score.mastery,
        rolled.fingerprint,
        now,
    )
    .execute(&mut *write)
    .await?
    .rows_affected()
        > 0;
    // The day's per-course rows are replaced in the same write: a course whose reviews went goes,
    // and a row that is unchanged keeps its `created_at`.
    let kept: Vec<&str> = rolled
        .languages
        .iter()
        .map(|row| row.course.as_str())
        .collect();
    let kept = Value::from(kept).to_string();
    sqlx::query!(
        "DELETE FROM daily_lang_stats WHERE study_day = ?1 \
         AND course NOT IN (SELECT value FROM json_each(?2))",
        day,
        kept
    )
    .execute(&mut *write)
    .await?;
    for row in rolled.languages {
        let course = row.course.as_str();
        sqlx::query!(
            "INSERT INTO daily_lang_stats (study_day, course, reviews, seconds, answered, passed, \
             created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT (study_day, course) DO UPDATE SET reviews = excluded.reviews, \
             seconds = excluded.seconds, answered = excluded.answered, passed = excluded.passed \
             WHERE daily_lang_stats.reviews IS NOT excluded.reviews \
             OR daily_lang_stats.seconds IS NOT excluded.seconds \
             OR daily_lang_stats.answered IS NOT excluded.answered \
             OR daily_lang_stats.passed IS NOT excluded.passed",
            day,
            course,
            row.reviews,
            row.seconds,
            row.answered,
            row.passed,
            now,
        )
        .execute(&mut *write)
        .await?;
    }
    record_score(write, metrics.day, score).await?;
    Ok(changed)
}

/// Records `state` as the card state of `day`, recorded at `at` (R8, R9).
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_card_state(
    write: &mut SqliteConnection,
    day: StudyDay,
    state: &CardState,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let day = day.epoch_day();
    let source = provenance(at);
    sqlx::query!(
        "UPDATE daily_rollup SET mature_count = ?2, young_count = ?3, leech_active = ?4, \
         backlog = ?5, due_today = ?6, card_state_src = ?7 WHERE study_day = ?1",
        day,
        state.mature_count,
        state.young_count,
        state.leech_active,
        state.backlog,
        state.due_today,
        source,
    )
    .execute(write)
    .await?;
    Ok(())
}

/// Stores `score` as the day's score and pillars, leaving `updated_at` as it is (R14).
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_score(
    write: &mut SqliteConnection,
    day: StudyDay,
    score: &Score,
) -> Result<(), KernelError> {
    let day = day.epoch_day();
    sqlx::query!(
        "UPDATE daily_rollup SET score = ?2, consistency = ?3, retention = ?4, workload = ?5, \
         volume = ?6, mastery = ?7 WHERE study_day = ?1",
        day,
        score.total,
        score.consistency,
        score.retention,
        score.workload,
        score.volume,
        score.mastery,
    )
    .execute(write)
    .await?;
    Ok(())
}

/// Keeps `total` as the score `day` closed with (R14).
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_close(
    write: &mut SqliteConnection,
    day: StudyDay,
    total: i64,
) -> Result<(), KernelError> {
    let day = day.epoch_day();
    sqlx::query!(
        "UPDATE daily_rollup SET score_at_close = ?2 WHERE study_day = ?1",
        day,
        total
    )
    .execute(write)
    .await?;
    Ok(())
}

/// Records that `day` was settled at `at` (R16): the cursor moves to it. Returns whether the day
/// had a rollup to record it on.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_settled(
    write: &mut SqliteConnection,
    day: StudyDay,
    at: UtcMillis,
) -> Result<bool, KernelError> {
    let day = day.epoch_day();
    let at = at.epoch_millis();
    let recorded = sqlx::query!(
        "UPDATE daily_rollup SET settled_at = ?2 WHERE study_day = ?1",
        day,
        at
    )
    .execute(write)
    .await?
    .rows_affected();
    Ok(recorded > 0)
}

/// The rollup of `day` as `write` sees it.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn stored(
    write: &mut SqliteConnection,
    day: StudyDay,
) -> Result<Option<StoredDay>, KernelError> {
    let day = day.epoch_day();
    let row = sqlx::query_as!(
        RollupRow,
        r#"SELECT study_day AS "study_day!", reviews, learn_count, review_count, relearn_count,
                  filtered_count, seconds, answered, passed, true_retention, graduations,
                  decks_studied, avg_answer_seconds, young_answered, young_passed, mature_answered,
                  mature_passed, mature_count, young_count, leech_active, backlog, due_today,
                  card_state_src, score, consistency, retention, workload, volume, mastery,
                  score_at_close, settled_at, fingerprint, created_at, updated_at
           FROM daily_rollup WHERE study_day = ?1"#,
        day
    )
    .fetch_optional(write)
    .await?;
    Ok(row.map(RollupRow::into_stored))
}

/// Every stored day's fingerprint, as `write` sees them.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn fingerprints(
    write: &mut SqliteConnection,
) -> Result<BTreeMap<StudyDay, String>, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT study_day AS "study_day!", fingerprint FROM daily_rollup ORDER BY study_day"#
    )
    .fetch_all(write)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (StudyDay::from_epoch_day(row.study_day), row.fingerprint))
        .collect())
}

/// The settle cursor as `write` sees it: the last settled day, or `None` before the first settle.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn settle_cursor(write: &mut SqliteConnection) -> Result<Option<StudyDay>, KernelError> {
    let cursor = sqlx::query_scalar!(
        r#"SELECT max(study_day) AS "cursor: i64" FROM daily_rollup WHERE settled_at IS NOT NULL"#
    )
    .fetch_one(write)
    .await?;
    Ok(cursor.map(StudyDay::from_epoch_day))
}

/// The volumes of the [`RECENT_ROLLUPS`] most recent rows on or before `through`, most recent
/// first: what the baseline of `through` reads.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn recent_volumes(
    write: &mut SqliteConnection,
    through: StudyDay,
) -> Result<Vec<DayVolume>, KernelError> {
    let through = through.epoch_day();
    let limit = i64::try_from(BASELINE_ROWS_READ).unwrap_or(i64::MAX);
    let rows = sqlx::query!(
        r#"SELECT study_day AS "study_day!", reviews, seconds FROM daily_rollup
           WHERE study_day <= ?1 ORDER BY study_day DESC LIMIT ?2"#,
        through,
        limit
    )
    .fetch_all(write)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| DayVolume {
            day: StudyDay::from_epoch_day(row.study_day),
            reviews: row.reviews,
            seconds: row.seconds,
        })
        .collect())
}

/// The rollup repository's reads over the service's database, for the surfaces.
#[derive(Clone, Debug)]
pub struct RollupStore {
    db: Db,
}

impl RollupStore {
    /// The repository over `db`, whose migrations created both tables.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// The rollups of the days from `first` to `last`, inclusive, oldest first.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn days(
        &self,
        first: StudyDay,
        last: StudyDay,
    ) -> Result<Vec<StoredDay>, KernelError> {
        let (first, last) = (first.epoch_day(), last.epoch_day());
        let rows = sqlx::query_as!(
            RollupRow,
            r#"SELECT study_day AS "study_day!", reviews, learn_count, review_count, relearn_count,
                  filtered_count, seconds, answered, passed, true_retention, graduations,
                  decks_studied, avg_answer_seconds, young_answered, young_passed, mature_answered,
                  mature_passed, mature_count, young_count, leech_active, backlog, due_today,
                  card_state_src, score, consistency, retention, workload, volume, mastery,
                  score_at_close, settled_at, fingerprint, created_at, updated_at
               FROM daily_rollup WHERE study_day BETWEEN ?1 AND ?2 ORDER BY study_day"#,
            first,
            last
        )
        .fetch_all(self.db.reader())
        .await?;
        Ok(rows.into_iter().map(RollupRow::into_stored).collect())
    }

    /// The per-course rows of `day`, by course code.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn language_days(&self, day: StudyDay) -> Result<Vec<LanguageDay>, KernelError> {
        let day = day.epoch_day();
        let rows = sqlx::query!(
            "SELECT course, reviews, seconds, answered, passed FROM daily_lang_stats \
             WHERE study_day = ?1 ORDER BY course",
            day
        )
        .fetch_all(self.db.reader())
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                Some(LanguageDay {
                    day: StudyDay::from_epoch_day(day),
                    course: CourseCode::new(&row.course)?,
                    reviews: row.reviews,
                    seconds: row.seconds,
                    answered: row.answered,
                    passed: row.passed,
                })
            })
            .collect())
    }
}

/// A `daily_rollup` row as SQL returns it.
#[derive(Clone, Debug)]
pub(crate) struct RollupRow {
    pub(crate) study_day: i64,
    pub(crate) reviews: i64,
    pub(crate) learn_count: i64,
    pub(crate) review_count: i64,
    pub(crate) relearn_count: i64,
    pub(crate) filtered_count: i64,
    pub(crate) seconds: f64,
    pub(crate) answered: i64,
    pub(crate) passed: i64,
    pub(crate) true_retention: f64,
    pub(crate) graduations: i64,
    pub(crate) decks_studied: i64,
    pub(crate) avg_answer_seconds: f64,
    pub(crate) young_answered: i64,
    pub(crate) young_passed: i64,
    pub(crate) mature_answered: i64,
    pub(crate) mature_passed: i64,
    pub(crate) mature_count: Option<i64>,
    pub(crate) young_count: Option<i64>,
    pub(crate) leech_active: Option<i64>,
    pub(crate) backlog: Option<i64>,
    pub(crate) due_today: Option<i64>,
    pub(crate) card_state_src: Option<String>,
    pub(crate) score: i64,
    pub(crate) consistency: f64,
    pub(crate) retention: f64,
    pub(crate) workload: f64,
    pub(crate) volume: f64,
    pub(crate) mastery: f64,
    pub(crate) score_at_close: Option<i64>,
    pub(crate) settled_at: Option<i64>,
    pub(crate) fingerprint: String,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

impl RollupRow {
    /// The row as the domain reads it: a card state only when one was recorded whole.
    fn into_stored(self) -> StoredDay {
        let card_state = match (
            &self.card_state_src,
            self.mature_count,
            self.young_count,
            self.leech_active,
            self.backlog,
            self.due_today,
        ) {
            (
                Some(_),
                Some(mature_count),
                Some(young_count),
                Some(leech_active),
                Some(backlog),
                Some(due_today),
            ) => Some(CardState {
                mature_count,
                young_count,
                leech_active,
                backlog,
                due_today,
            }),
            _ => None,
        };
        let (grade_label, grade_emoji) = grade_band(self.score);
        StoredDay {
            metrics: DailyMetrics {
                day: StudyDay::from_epoch_day(self.study_day),
                reviews: self.reviews,
                learn_count: self.learn_count,
                review_count: self.review_count,
                relearn_count: self.relearn_count,
                filtered_count: self.filtered_count,
                seconds: self.seconds,
                answered: self.answered,
                passed: self.passed,
                true_retention: self.true_retention,
                graduations: self.graduations,
                decks_studied: self.decks_studied,
                avg_answer_seconds: self.avg_answer_seconds,
                young_answered: self.young_answered,
                young_passed: self.young_passed,
                mature_answered: self.mature_answered,
                mature_passed: self.mature_passed,
            },
            card_state,
            card_state_src: self.card_state_src,
            score: Score {
                consistency: self.consistency,
                retention: self.retention,
                workload: self.workload,
                volume: self.volume,
                mastery: self.mastery,
                total: self.score,
                grade_label,
                grade_emoji,
            },
            score_at_close: self.score_at_close,
            settled_at: self.settled_at.map(UtcMillis::from_epoch_millis),
            fingerprint: self.fingerprint,
            created_at: UtcMillis::from_epoch_millis(self.created_at),
            updated_at: UtcMillis::from_epoch_millis(self.updated_at),
        }
    }

    /// The row as an export carries it: every column, by name.
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "study_day": self.study_day,
            "reviews": self.reviews,
            "learn_count": self.learn_count,
            "review_count": self.review_count,
            "relearn_count": self.relearn_count,
            "filtered_count": self.filtered_count,
            "seconds": self.seconds,
            "answered": self.answered,
            "passed": self.passed,
            "true_retention": self.true_retention,
            "graduations": self.graduations,
            "decks_studied": self.decks_studied,
            "avg_answer_seconds": self.avg_answer_seconds,
            "young_answered": self.young_answered,
            "young_passed": self.young_passed,
            "mature_answered": self.mature_answered,
            "mature_passed": self.mature_passed,
            "mature_count": self.mature_count,
            "young_count": self.young_count,
            "leech_active": self.leech_active,
            "backlog": self.backlog,
            "due_today": self.due_today,
            "card_state_src": self.card_state_src,
            "score": self.score,
            "consistency": self.consistency,
            "retention": self.retention,
            "workload": self.workload,
            "volume": self.volume,
            "mastery": self.mastery,
            "score_at_close": self.score_at_close,
            "settled_at": self.settled_at,
            "fingerprint": self.fingerprint,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        })
    }
}
