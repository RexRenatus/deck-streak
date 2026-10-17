//! Curriculum's repository over its three tables (SPEC-077 R6, R7, R10, R18; ADR-077): each
//! course's stored progress, the band milestones and the law dues.
//!
//! Every function runs on a connection the caller holds: a write is made inside the fold's own
//! `BEGIN IMMEDIATE` write for the day, so a course's progress, its milestone and the band-up's
//! grant commit together or not at all, and a read takes any connection.

use std::collections::BTreeMap;

use deck_streak_kernel::{CourseCode, KernelError, StudyDay, UtcMillis};
use serde_json::{Value, json};
use sqlx::SqliteConnection;

use crate::law::LawDues;
use crate::progress::CourseProgress;

/// One band of a stored course, as `language_progress.bands` holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredBand {
    /// The band.
    pub band: String,
    /// The counted cards whose unit falls in it.
    pub total: u32,
    /// Those of them that are mature.
    pub mature: u32,
    /// The mean mastery of its cards, in percent.
    pub pct: f64,
    /// Whether the band is achieved.
    pub achieved: bool,
}

/// One course's progress as `language_progress` holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredProgress {
    /// The course's code.
    pub course: String,
    /// The course's name.
    pub name: String,
    /// The course's flag.
    pub flag: String,
    /// The mean mastery of its counted cards, in percent.
    pub mastery_pct: f64,
    /// The current band.
    pub current_band: String,
    /// The mature counted cards.
    pub mature_cards: u32,
    /// The counted cards.
    pub total_cards: u32,
    /// The highest unit holding a mature card, if any.
    pub current_unit: Option<u32>,
    /// Each band's counts, in the bands' order.
    pub bands: Vec<StoredBand>,
    /// When the row was last written.
    pub updated_at: UtcMillis,
}

/// A band milestone to record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewMilestone<'a> {
    /// The course.
    pub course: &'a CourseCode,
    /// The band reached.
    pub band: &'a str,
    /// The study day it is reached on.
    pub study_day: StudyDay,
    /// Whether it is the course's silent baseline, which is written marked and owes nothing.
    pub baseline: bool,
    /// The services clock's instant: the row's creation, and a baseline's mark.
    pub at: UtcMillis,
}

/// What recording a milestone did.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// The row was written: the band is reached for the first time.
    New,
    /// The course already held the band; nothing was written.
    Held,
}

/// One band milestone, as `band_milestones` holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BandMilestone {
    /// The course's code.
    pub course: String,
    /// The band reached.
    pub band: String,
    /// The study day it was reached on.
    pub study_day: StudyDay,
    /// Whether it is the course's silent baseline.
    pub baseline: bool,
    /// When the router answered its celebration, the baseline's write for a baseline, or `None`
    /// while a band-up's celebration is still owed (ADR-303).
    pub celebrated_at: Option<UtcMillis>,
}

/// Each stored course's current band, by course code: what the next recompute compares with.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn stored_bands(
    connection: &mut SqliteConnection,
) -> Result<BTreeMap<String, String>, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT course AS "course!: String", current_band AS "current_band!: String"
               FROM language_progress ORDER BY course"#
    )
    .fetch_all(&mut *connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.course, row.current_band))
        .collect())
}

/// Writes `progress` as its course's row, replacing the row the course held, at `at`.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn put_progress(
    connection: &mut SqliteConnection,
    progress: &CourseProgress,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let course = progress.code.as_str();
    let mature = i64::from(progress.mature_cards);
    let total = i64::from(progress.total_cards);
    let unit = progress.current_unit.map(i64::from);
    let bands = Value::Array(
        progress
            .bands
            .iter()
            .map(|band| {
                json!({
                    "band": band.band,
                    "total": band.total,
                    "mature": band.mature,
                    "pct": band.pct,
                    "achieved": band.achieved,
                })
            })
            .collect(),
    )
    .to_string();
    let at = at.epoch_millis();
    // The course's row is replaced whole but for its first write's instant.
    sqlx::query!(
        "INSERT INTO language_progress (course, name, flag, mastery_pct, current_band, \
             mature_cards, total_cards, current_unit, bands, updated_at, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10) \
         ON CONFLICT (course) DO UPDATE SET name = excluded.name, flag = excluded.flag, \
             mastery_pct = excluded.mastery_pct, current_band = excluded.current_band, \
             mature_cards = excluded.mature_cards, total_cards = excluded.total_cards, \
             current_unit = excluded.current_unit, bands = excluded.bands, \
             updated_at = excluded.updated_at",
        course,
        progress.name,
        progress.flag,
        progress.mastery_pct,
        progress.current_band,
        mature,
        total,
        unit,
        bands,
        at
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Every stored course's progress, by course code.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails or a row's bands are not the JSON the store
/// writes.
pub async fn progress(
    connection: &mut SqliteConnection,
) -> Result<Vec<StoredProgress>, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT course AS "course!: String", name, flag, mastery_pct, current_band,
                  mature_cards, total_cards, current_unit, bands, updated_at
               FROM language_progress ORDER BY course"#
    )
    .fetch_all(&mut *connection)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(StoredProgress {
                course: row.course,
                name: row.name,
                flag: row.flag,
                mastery_pct: row.mastery_pct,
                current_band: row.current_band,
                mature_cards: count(row.mature_cards)?,
                total_cards: count(row.total_cards)?,
                current_unit: row.current_unit.map(count).transpose()?,
                bands: stored_bands_of(&row.bands)?,
                updated_at: UtcMillis::from_epoch_millis(row.updated_at),
            })
        })
        .collect()
}

/// A stored count or unit, which the migration's checks keep non-negative and the store writes
/// from a `u32`.
fn count(value: i64) -> Result<u32, KernelError> {
    u32::try_from(value).map_err(|_| undecodable("a stored count is not a u32"))
}

/// The bands `put_progress` writes, read back from their JSON text.
fn stored_bands_of(text: &str) -> Result<Vec<StoredBand>, KernelError> {
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(text) else {
        return Err(undecodable("a course's bands are not a JSON array"));
    };
    items
        .iter()
        .map(|item| {
            let number = |key: &str| {
                item[key]
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| undecodable("a band's count is not a u32"))
            };
            Ok(StoredBand {
                band: item["band"]
                    .as_str()
                    .ok_or_else(|| undecodable("a band has no name"))?
                    .to_owned(),
                total: number("total")?,
                mature: number("mature")?,
                pct: item["pct"]
                    .as_f64()
                    .ok_or_else(|| undecodable("a band has no mastery"))?,
                achieved: item["achieved"]
                    .as_bool()
                    .ok_or_else(|| undecodable("a band has no achieved flag"))?,
            })
        })
        .collect()
}

/// A stored value the store cannot read back; the reason names the rule, never the value.
fn undecodable(reason: &'static str) -> KernelError {
    KernelError::Database(sqlx::Error::Decode(reason.into()))
}

/// Records `milestone` once: the key on the course and the band is the existence check, so a band
/// the course already holds writes nothing.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_milestone(
    connection: &mut SqliteConnection,
    milestone: &NewMilestone<'_>,
) -> Result<Recorded, KernelError> {
    let course = milestone.course.as_str();
    let day = milestone.study_day.epoch_day();
    let baseline = i64::from(milestone.baseline);
    let at = milestone.at.epoch_millis();
    // A baseline owes no celebration, so it is written marked; a band-up's mark stays unset until
    // the router answers (ADR-303). The key on the course and the band is the existence check.
    let marked = milestone.baseline.then_some(at);
    let written = sqlx::query!(
        "INSERT INTO band_milestones (course, band, study_day, baseline, celebrated_at, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT DO NOTHING",
        course,
        milestone.band,
        day,
        baseline,
        marked,
        at
    )
    .execute(&mut *connection)
    .await?
    .rows_affected();
    Ok(if written == 1 {
        Recorded::New
    } else {
        Recorded::Held
    })
}

/// Every band milestone, by course and band.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn milestones(
    connection: &mut SqliteConnection,
) -> Result<Vec<BandMilestone>, KernelError> {
    let rows = sqlx::query_as!(
        MilestoneRow,
        r#"SELECT course AS "course!: String", band, study_day, baseline, celebrated_at
               FROM band_milestones ORDER BY course, band"#
    )
    .fetch_all(&mut *connection)
    .await?;
    Ok(rows.into_iter().map(MilestoneRow::milestone).collect())
}

/// One row of `band_milestones` as the store reads it.
struct MilestoneRow {
    course: String,
    band: String,
    study_day: i64,
    baseline: i64,
    celebrated_at: Option<i64>,
}

impl MilestoneRow {
    /// The milestone the row holds.
    fn milestone(self) -> BandMilestone {
        BandMilestone {
            course: self.course,
            band: self.band,
            study_day: StudyDay::from_epoch_day(self.study_day),
            baseline: self.baseline != 0,
            celebrated_at: self.celebrated_at.map(UtcMillis::from_epoch_millis),
        }
    }
}

/// The band-ups recorded on `day`, never a baseline, by course and band: what the band badge's
/// step awards.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn band_ups_on(
    connection: &mut SqliteConnection,
    day: StudyDay,
) -> Result<Vec<BandMilestone>, KernelError> {
    let day = day.epoch_day();
    let rows = sqlx::query_as!(
        MilestoneRow,
        r#"SELECT course AS "course!: String", band, study_day, baseline, celebrated_at
               FROM band_milestones WHERE study_day = ?1 AND baseline = 0
               ORDER BY course, band"#,
        day
    )
    .fetch_all(&mut *connection)
    .await?;
    Ok(rows.into_iter().map(MilestoneRow::milestone).collect())
}

/// Every band-up whose celebration is still owed, oldest first: what the fold's offers hand to the
/// router (ADR-303).
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn owed_band_ups(
    connection: &mut SqliteConnection,
) -> Result<Vec<BandMilestone>, KernelError> {
    let rows = sqlx::query_as!(
        MilestoneRow,
        r#"SELECT course AS "course!: String", band, study_day, baseline, celebrated_at
               FROM band_milestones WHERE baseline = 0 AND celebrated_at IS NULL
               ORDER BY created_at, study_day, course, band"#
    )
    .fetch_all(&mut *connection)
    .await?;
    Ok(rows.into_iter().map(MilestoneRow::milestone).collect())
}

/// Marks the band-up of `course` to `band` celebrated at `at`, only while it is still owed; answers
/// whether it marked it.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn mark_band_up(
    connection: &mut SqliteConnection,
    course: &str,
    band: &str,
    at: UtcMillis,
) -> Result<bool, KernelError> {
    let at = at.epoch_millis();
    let marked = sqlx::query!(
        "UPDATE band_milestones SET celebrated_at = ?3 \
         WHERE course = ?1 AND band = ?2 AND baseline = 0 AND celebrated_at IS NULL",
        course,
        band,
        at
    )
    .execute(&mut *connection)
    .await?
    .rows_affected();
    Ok(marked == 1)
}

/// Writes `dues` as the law dues, replacing the ones stored, at `at`.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn put_law_dues(
    connection: &mut SqliteConnection,
    dues: &LawDues,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let day = dues.study_day.epoch_day();
    let backlog = i64::from(dues.backlog);
    let due_today = i64::from(dues.due_today);
    let at = at.epoch_millis();
    sqlx::query!(
        "INSERT INTO law_dues (id, study_day, backlog, due_today, updated_at, created_at) \
         VALUES (1, ?1, ?2, ?3, ?4, ?4) \
         ON CONFLICT (id) DO UPDATE SET study_day = excluded.study_day, \
             backlog = excluded.backlog, due_today = excluded.due_today, \
             updated_at = excluded.updated_at",
        day,
        backlog,
        due_today,
        at
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// The law dues the last recompute stored, or `None` before the first, which every surface reads
/// as pending, never 0 (R10).
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn law_dues(connection: &mut SqliteConnection) -> Result<Option<LawDues>, KernelError> {
    let row = sqlx::query!("SELECT study_day, backlog, due_today FROM law_dues WHERE id = 1")
        .fetch_optional(&mut *connection)
        .await?;
    row.map(|row| {
        Ok(LawDues {
            study_day: StudyDay::from_epoch_day(row.study_day),
            backlog: count(row.backlog)?,
            due_today: count(row.due_today)?,
        })
    })
    .transpose()
}
