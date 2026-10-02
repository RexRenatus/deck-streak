//! The award port (SPEC-073 R1, R3, R4): one row of `badges_earned` per key and tier, written in
//! one `BEGIN IMMEDIATE` write and never updated except for its celebration mark, and never
//! deleted but by the data-rights erase.
//!
//! A catalog badge is written with its mark unset, so the fold's offers raise its celebration until
//! the router answers (ADR-303); a band badge is written marked, because the band-up celebrates it.
//! A key that names neither is refused before any write, by an error that names the rule.

use deck_streak_kernel::{Courses, KernelError, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use super::catalog::is_catalog_key;

/// The bands a band badge may name, in order.
pub const BANDS: [&str; 6] = ["A1", "A2", "B1", "B2", "C1", "C2"];

/// What an award did.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Award {
    /// The row was written.
    Awarded,
    /// The key and tier were already held; nothing was written.
    AlreadyAwarded,
}

/// Why an award was refused.
#[derive(Debug, thiserror::Error)]
pub enum AwardError {
    /// The key names no catalog badge and no band badge of a configured course.
    #[error(
        "a badge key names a catalog badge, or band_<code>_<band> for a configured course and a \
         band from A1 to C2"
    )]
    UnknownKey,
    /// The write failed.
    #[error(transparent)]
    Database(#[from] KernelError),
}

/// Which rule a key is accepted by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// A catalog badge.
    Catalog,
    /// A band badge, which the band-up celebrates.
    Band,
}

/// A badge to award.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewBadge<'a> {
    /// The key: a catalog key or a band key.
    pub key: &'a str,
    /// The tier.
    pub tier: u32,
    /// The name a screen shows.
    pub name: &'a str,
    /// The emoji a screen shows.
    pub emoji: &'a str,
    /// The study day it is earned on.
    pub study_day: StudyDay,
    /// The services clock's instant: the row's creation, and a band badge's mark.
    pub at: UtcMillis,
}

/// One earned badge, as `badges_earned` holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EarnedBadge {
    /// The key.
    pub key: String,
    /// The tier.
    pub tier: u32,
    /// The name.
    pub name: String,
    /// The emoji.
    pub emoji: String,
    /// The study day it was earned on.
    pub study_day: StudyDay,
    /// When the router answered its celebration, or `None` while it is still owed.
    pub celebrated_at: Option<UtcMillis>,
}

/// The rule that accepts `key` under `courses`, or `None` when none does (R3).
#[must_use]
pub fn key_kind(courses: &Courses, key: &str) -> Option<KeyKind> {
    if is_catalog_key(key) {
        return Some(KeyKind::Catalog);
    }
    // A band key is `band_<code>_<band>`: the band is the last segment, the code what precedes it.
    let (code, band) = key.strip_prefix("band_")?.rsplit_once('_')?;
    let configured = courses
        .courses()
        .iter()
        .any(|course| course.code.as_str() == code);
    (configured && BANDS.contains(&band)).then_some(KeyKind::Band)
}

/// Awards `badge` inside `write`, the caller's `BEGIN IMMEDIATE` transaction (R3, R4).
///
/// # Errors
///
/// [`AwardError::UnknownKey`] before any write when no rule accepts the key, and
/// [`AwardError::Database`] when the write fails.
pub async fn award(
    write: &mut SqliteConnection,
    courses: &Courses,
    badge: &NewBadge<'_>,
) -> Result<Award, AwardError> {
    let kind = key_kind(courses, badge.key).ok_or(AwardError::UnknownKey)?;
    // A band badge is celebrated by the band-up, so it is written marked; a catalog badge is
    // written owed, and the fold's offers raise it until the router answers (ADR-303).
    let marked = match kind {
        KeyKind::Catalog => None,
        KeyKind::Band => Some(badge.at.epoch_millis()),
    };
    let tier = i64::from(badge.tier);
    let day = badge.study_day.epoch_day();
    let at = badge.at.epoch_millis();
    // The primary key on the key and tier is the existence check: a conflict writes nothing.
    let written = sqlx::query!(
        "INSERT INTO badges_earned \
             (badge_key, tier, name, emoji, study_day, celebrated_at, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT DO NOTHING",
        badge.key,
        tier,
        badge.name,
        badge.emoji,
        day,
        marked,
        at
    )
    .execute(&mut *write)
    .await
    .map_err(KernelError::from)?
    .rows_affected();
    Ok(if written == 1 {
        Award::Awarded
    } else {
        Award::AlreadyAwarded
    })
}

/// Every badge whose celebration is still owed, oldest first.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn unmarked(connection: &mut SqliteConnection) -> Result<Vec<EarnedBadge>, KernelError> {
    let rows = sqlx::query!(
        "SELECT badge_key, tier, name, emoji, study_day, celebrated_at FROM badges_earned \
         WHERE celebrated_at IS NULL ORDER BY created_at, study_day, badge_key, tier"
    )
    .fetch_all(&mut *connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| EarnedBadge {
            key: row.badge_key,
            // The column's check keeps a tier non-negative, and every tier is written from a u32.
            tier: u32::try_from(row.tier).unwrap_or(u32::MAX),
            name: row.name,
            emoji: row.emoji,
            study_day: StudyDay::from_epoch_day(row.study_day),
            celebrated_at: row.celebrated_at.map(UtcMillis::from_epoch_millis),
        })
        .collect())
}
