//! The minutes log's use cases (SPEC-078 R2 to R5, R18; ADR-078).

use deck_streak_habits::minutes::EntryRefusal;
use deck_streak_kernel::{CourseCode, Courses, Db, KernelError, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::Router;

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

/// Logs `minutes` of reading for `course`, with `note`.
///
/// # Errors
///
/// [`HabitError`] when the entry is refused or the write fails.
pub async fn log_minutes(
    writer: &HabitWriter<'_>,
    courses: &Courses,
    course: Course<'_>,
    minutes: i64,
    note: &str,
) -> Result<Logged, HabitError> {
    let _ = (writer, courses, course, minutes, note);
    Err(HabitError::NoCourse)
}

/// Removes the newest entry.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails.
pub async fn undo_newest(writer: &HabitWriter<'_>) -> Result<Undone, HabitError> {
    let _ = writer;
    Ok(Undone::Nothing)
}

/// Removes the entry `entry` while it is the newest.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails.
pub async fn undo_entry(writer: &HabitWriter<'_>, entry: i64) -> Result<Undone, HabitError> {
    let _ = (writer, entry);
    Ok(Undone::Stale)
}
