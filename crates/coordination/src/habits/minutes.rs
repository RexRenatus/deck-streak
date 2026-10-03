//! The minutes log's use cases (SPEC-078 R2 to R5, R18; ADR-078): an entry and its undo each run
//! in ONE write through the kernel's write base, the log write and its settles of `read:<code>` on
//! the entry's day and `readgoal:<code>` on its week's first study day together, as the owner's
//! correction (ADR-072). A level the entry crosses is announced after the commit, through the one
//! router and its once-ever key.

use deck_streak_habits::minutes::{
    EntryRefusal, entry, goal_bonus, goal_source, most_used, read_source, reading_xp, week_end,
    week_start,
};
use deck_streak_habits::store::{self, LoggedEntry};
use deck_streak_kernel::{
    CourseCode, Courses, Db, KernelError, StudyDay, StudyDayRule, Track, UtcMillis,
};
use deck_streak_notifications::Router;
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::settle::{SettleCause, SettleError, SettleRequest, settle};
use deck_streak_progression::xp::Level;
use sqlx::SqliteConnection;

use crate::level_up::announce_level_up;

/// The cause of every settle a habit write makes: the owner's correction, which replaces the
/// amount whatever it was, so an undo lowers a closed day exactly (ADR-072, ADR-078). Only the
/// fold's recompute steps pass the recompute cause.
const UNDO_CAUSE: SettleCause = SettleCause::OwnersCorrection;

/// What every owner habit write is given.
#[derive(Clone, Copy)]
pub struct HabitWriter<'a> {
    /// The database the write runs on.
    pub db: &'a Db,
    /// The router a crossed level is announced through, when the surface has one.
    pub router: Option<&'a Router>,
    /// The study-day rule.
    pub rule: StudyDayRule,
    /// When the owner wrote.
    pub now: UtcMillis,
}

impl std::fmt::Debug for HabitWriter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HabitWriter")
            .field("router", &self.router.is_some())
            .field("rule", &self.rule)
            .field("now", &self.now)
            .finish_non_exhaustive()
    }
}

/// Which course an entry is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Course<'a> {
    /// The course the owner's token names.
    Token(&'a str),
    /// The course with the most minutes this week, else of all time.
    MostUsed,
}

/// One logged entry, and what its day and week now hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logged {
    /// The entry's id, which its Undo button carries.
    pub entry_id: i64,
    /// The course.
    pub code: CourseCode,
    /// The minutes logged.
    pub minutes: u32,
    /// The course's minutes on the study day.
    pub day_minutes: u32,
    /// The course's reading XP of the study day.
    pub day_xp: u32,
    /// The course's minutes in the study week.
    pub week_minutes: u32,
    /// The course's weekly bonus.
    pub goal_bonus: u32,
}

/// What an undo did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Undone {
    /// The entry was removed, and its day and week settled again.
    Removed {
        /// The entry's course, as stored.
        code: String,
        /// The entry's minutes.
        minutes: u32,
        /// The study day it was logged on.
        day: StudyDay,
    },
    /// There was no entry to remove.
    Nothing,
    /// The button's entry is no longer the newest, so nothing was removed.
    Stale,
}

/// Why a habit write did nothing. Each names the rule, never the owner's text.
#[derive(Debug, thiserror::Error)]
pub enum HabitError {
    /// The entry broke one of its rules.
    #[error(transparent)]
    Refused(#[from] EntryRefusal),
    /// A minutes-only entry found no course with minutes logged.
    #[error("a minutes-only entry needs a course with minutes logged")]
    NoCourse,
    /// The database refused the write.
    #[error(transparent)]
    Kernel(#[from] KernelError),
}

impl From<sqlx::Error> for HabitError {
    fn from(error: sqlx::Error) -> Self {
        Self::Kernel(KernelError::Database(error))
    }
}

/// Logs `minutes` of reading for `course`, with `note`, on the current study day: the entry and
/// the settles of its day and its week in one write, then the level it crossed announced.
///
/// # Errors
///
/// [`HabitError`] when the entry is refused, no course has minutes for a minutes-only entry, or
/// the write fails; nothing is written then.
pub async fn log_minutes(
    writer: &HabitWriter<'_>,
    courses: &Courses,
    course: Course<'_>,
    minutes: i64,
    note: &str,
) -> Result<Logged, HabitError> {
    let today = writer.rule.study_day(writer.now);
    let before = level_before(writer).await;
    let mut write = writer.db.write().await?;
    let token = match course {
        Course::Token(token) => token.to_owned(),
        Course::MostUsed => {
            let week =
                store::minutes_by_code(&mut write, week_start(today), week_end(today)).await?;
            let all_time = store::all_time_by_code(&mut write).await?;
            most_used(&week, &all_time).ok_or(HabitError::NoCourse)?
        }
    };
    let entry = entry(courses, &token, minutes, note)?;
    let entry_id = store::insert(&mut write, &entry, today, writer.now).await?;
    let settled = settle_entry(
        &mut write,
        entry.code,
        today,
        week_start(today),
        today,
        writer.now,
    )
    .await?;
    write.commit().await?;
    celebrate(writer, before, today).await;
    Ok(Logged {
        entry_id,
        code: entry.code,
        minutes: entry.minutes,
        day_minutes: settled.day_minutes,
        day_xp: reading_xp(settled.day_minutes),
        week_minutes: settled.week_minutes,
        goal_bonus: goal_bonus(settled.week_minutes),
    })
}

/// Removes the newest entry, whatever its day, and settles its day and its own week again.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is removed then.
pub async fn undo_newest(writer: &HabitWriter<'_>) -> Result<Undone, HabitError> {
    let today = writer.rule.study_day(writer.now);
    let mut write = writer.db.write().await?;
    let Some(newest) = store::newest(&mut write).await? else {
        return Ok(Undone::Nothing);
    };
    let undone = remove(&mut write, newest, today, writer.now).await?;
    write.commit().await?;
    Ok(undone)
}

/// Removes the entry `entry` while it is the newest, as its Undo button asks; an older entry's
/// button removes nothing (ADR-078).
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is removed then.
pub async fn undo_entry(writer: &HabitWriter<'_>, entry: i64) -> Result<Undone, HabitError> {
    let today = writer.rule.study_day(writer.now);
    let mut write = writer.db.write().await?;
    let Some(newest) = store::newest(&mut write).await? else {
        return Ok(Undone::Stale);
    };
    if newest.id != entry {
        return Ok(Undone::Stale);
    }
    let undone = remove(&mut write, newest, today, writer.now).await?;
    write.commit().await?;
    Ok(undone)
}

/// What an entry's settles read: the course's minutes on the day and in the week.
struct Settled {
    day_minutes: u32,
    week_minutes: u32,
}

/// Removes `removed` inside `write`, and settles its day and the week it was logged in, never the
/// current week.
async fn remove(
    write: &mut SqliteConnection,
    removed: LoggedEntry,
    today: StudyDay,
    at: UtcMillis,
) -> Result<Undone, HabitError> {
    store::remove(write, removed.id).await?;
    if let Some(code) = CourseCode::new(&removed.code) {
        settle_entry(write, code, removed.day, week_start(removed.day), today, at).await?;
    }
    Ok(Undone::Removed {
        code: removed.code,
        minutes: removed.minutes,
        day: removed.day,
    })
}

/// Settles `code`'s `read:` on `day` from its minutes there, and its `readgoal:` on `first_day`
/// from its minutes in that study week, each closed when its day is before `today`.
async fn settle_entry(
    write: &mut SqliteConnection,
    code: CourseCode,
    day: StudyDay,
    first_day: StudyDay,
    today: StudyDay,
    at: UtcMillis,
) -> Result<Settled, HabitError> {
    let day_minutes = store::minutes_between(write, code.as_str(), day, day).await?;
    let week_minutes =
        store::minutes_between(write, code.as_str(), first_day, week_end(first_day)).await?;
    let read = read_source(code);
    settle_owner(write, day, &read, reading_xp(day_minutes), day < today, at).await?;
    let goal = goal_source(code);
    let bonus = goal_bonus(week_minutes);
    settle_owner(write, first_day, &goal, bonus, first_day < today, at).await?;
    Ok(Settled {
        day_minutes,
        week_minutes,
    })
}

/// Settles `amount` of `source` on `day`, on the language track, as the owner's correction.
async fn settle_owner(
    write: &mut SqliteConnection,
    day: StudyDay,
    source: &str,
    amount: u32,
    closed: bool,
    at: UtcMillis,
) -> Result<(), HabitError> {
    let request = SettleRequest {
        study_day: day,
        source,
        track: Track::Language,
        amount,
        closed,
    };
    match settle(write, &request, UNDO_CAUSE, at).await {
        Ok(_) => Ok(()),
        Err(SettleError::Database(error)) => Err(error.into()),
        Err(SettleError::NotDerived) => Err(HabitError::Kernel(KernelError::Database(
            sqlx::Error::Protocol(
                "a habit settled a source outside the derived registry".to_owned(),
            ),
        ))),
    }
}

/// The level before a habit write, when there is a router to announce a crossing through. A read
/// that fails is logged, and the write goes on unannounced.
async fn level_before(writer: &HabitWriter<'_>) -> Option<Level> {
    writer.router?;
    match SqliteXpLedger::new(writer.db.clone()).level().await {
        Ok(level) => Some(level),
        Err(error) => {
            tracing::error!(%error, "the level before a habit write could not be read");
            None
        }
    }
}

/// Announces the level a committed habit write crossed from `before`, through the one router and
/// its once-ever key `level:<n>` (SPEC-078 R18). A failure is logged: the entry stands.
async fn celebrate(writer: &HabitWriter<'_>, before: Option<Level>, today: StudyDay) {
    let (Some(router), Some(before)) = (writer.router, before) else {
        return;
    };
    let announced = match SqliteXpLedger::new(writer.db.clone()).level().await {
        Ok(after) => announce_level_up(router, before, after, today)
            .await
            .map(|_| ()),
        Err(error) => Err(error),
    };
    if let Err(error) = announced {
        tracing::error!(%error, "a habit write's level-up was not announced");
    }
}
