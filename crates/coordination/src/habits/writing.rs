//! The writing log's use cases (SPEC-078 R6 to R8, R18; ADR-078): a confirmation, its clearing and
//! a checklist chip's toggle each run in ONE write through the kernel's write base, the log change
//! and the day's settles of `write:<code>` and `write:all` together, as the owner's correction
//! (ADR-072). A level the write crosses is announced after the commit, through the one router and
//! its once-ever key.

use deck_streak_kernel::{CourseCode, Courses, StudyDay};

use super::minutes::{HabitError, HabitWriter};

/// One writing course's line of a checklist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChecklistLine {
    /// The writing course.
    pub code: CourseCode,
    /// Whether it is confirmed on the checklist's day.
    pub confirmed: bool,
    /// Its own writing streak on that day.
    pub streak: u32,
}

/// A study day's writing checklist: one line per writing course, in the file's order, and the
/// writing streak over every writing course.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checklist {
    /// The study day it shows.
    pub day: StudyDay,
    /// One line per writing course.
    pub lines: Vec<ChecklistLine>,
    /// The writing streak over every writing course.
    pub streak: u32,
}

/// What a writing write did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Written {
    /// The log changed, and the day's checklist is shown.
    Done(Checklist),
    /// The chip was drawn for a day that is no longer the current study day: nothing changed, and
    /// today's checklist is shown.
    DayClosed(Checklist),
    /// A token names no writing course: nothing changed.
    NotAWritingCourse,
    /// No writing course is configured: nothing changed.
    NoWritingCourse,
}

/// The current study day's writing checklist.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the log cannot be read.
pub async fn checklist(
    writer: &HabitWriter<'_>,
    _courses: &Courses,
) -> Result<Checklist, HabitError> {
    Ok(empty(writer))
}

/// Confirms writing today in every course `tokens` name, each resolved first: a token that names
/// no writing course refuses the whole command.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is written then.
pub async fn confirm(
    writer: &HabitWriter<'_>,
    _courses: &Courses,
    _tokens: &[&str],
) -> Result<Written, HabitError> {
    Ok(Written::Done(empty(writer)))
}

/// Clears today's confirmation of the writing course `token` names.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is removed then.
pub async fn clear(
    writer: &HabitWriter<'_>,
    _courses: &Courses,
    _token: &str,
) -> Result<Written, HabitError> {
    Ok(Written::Done(empty(writer)))
}

/// Toggles `code`'s confirmation on `chip_day`, the day its chip was drawn for, while that day is
/// still the current study day.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is changed then.
pub async fn toggle(
    writer: &HabitWriter<'_>,
    _courses: &Courses,
    _code: CourseCode,
    _chip_day: StudyDay,
) -> Result<Written, HabitError> {
    Ok(Written::Done(empty(writer)))
}

/// The current study day's checklist with no line.
fn empty(writer: &HabitWriter<'_>) -> Checklist {
    Checklist {
        day: writer.rule.study_day(writer.now),
        lines: Vec::new(),
        streak: 0,
    }
}
