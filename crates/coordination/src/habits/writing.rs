//! The writing log's use cases (SPEC-078 R6 to R8, R18; ADR-078): a confirmation, its clearing and
//! a checklist chip's toggle each run in ONE write through the kernel's write base, the log change
//! and the day's settles of `write:<code>` and `write:all` together, as the owner's correction
//! (ADR-072). A level the write crosses is announced after the commit, through the one router and
//! its once-ever key.

use deck_streak_habits::minutes::resolve_course;
use deck_streak_habits::store;
use deck_streak_habits::writing::{
    WRITE_ALL_SOURCE, course_streaks, write_source, writing_courses, writing_day_xp, writing_streak,
};
use deck_streak_kernel::{CourseCode, Courses, StudyDay};
use sqlx::SqliteConnection;

use super::minutes::{HabitError, HabitWriter, celebrate, level_before, settle_owner};

/// The first study day the log is read from: every confirmation through a day counts toward its
/// streaks.
const FIRST_DAY: StudyDay = StudyDay::from_epoch_day(i64::MIN);

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

/// The change one writing write makes to the current study day's log.
enum Change {
    /// Confirm each course.
    Confirm(Vec<CourseCode>),
    /// Clear the course.
    Clear(CourseCode),
    /// Clear the course when it is confirmed, else confirm it.
    Toggle(CourseCode),
}

/// The current study day's writing checklist.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the log cannot be read.
pub async fn checklist(
    writer: &HabitWriter<'_>,
    courses: &Courses,
) -> Result<Checklist, HabitError> {
    let today = writer.rule.study_day(writer.now);
    let mut read = writer.db.reader().acquire().await?;
    let rows = confirmed_through(&mut read, today).await?;
    Ok(day_checklist(&writing_courses(courses), &rows, today))
}

/// Confirms writing today in every course `tokens` name, each resolved first: a token that names
/// no writing course refuses the whole command.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is written then.
pub async fn confirm(
    writer: &HabitWriter<'_>,
    courses: &Courses,
    tokens: &[&str],
) -> Result<Written, HabitError> {
    let writing = writing_courses(courses);
    if writing.is_empty() {
        return Ok(Written::NoWritingCourse);
    }
    let mut codes = Vec::new();
    for token in tokens {
        match resolve_course(courses, token).filter(|code| writing.contains(code)) {
            Some(code) => codes.push(code),
            None => return Ok(Written::NotAWritingCourse),
        }
    }
    let today = writer.rule.study_day(writer.now);
    let checklist = write_day(writer, &writing, today, Change::Confirm(codes)).await?;
    Ok(Written::Done(checklist))
}

/// Clears today's confirmation of the writing course `token` names.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is removed then.
pub async fn clear(
    writer: &HabitWriter<'_>,
    courses: &Courses,
    token: &str,
) -> Result<Written, HabitError> {
    let writing = writing_courses(courses);
    if writing.is_empty() {
        return Ok(Written::NoWritingCourse);
    }
    let Some(code) = resolve_course(courses, token).filter(|code| writing.contains(code)) else {
        return Ok(Written::NotAWritingCourse);
    };
    let today = writer.rule.study_day(writer.now);
    let checklist = write_day(writer, &writing, today, Change::Clear(code)).await?;
    Ok(Written::Done(checklist))
}

/// Toggles `code`'s confirmation on `chip_day`, the day its chip was drawn for, while that day is
/// still the current study day.
///
/// # Errors
///
/// [`HabitError::Kernel`] when the write fails; nothing is changed then.
pub async fn toggle(
    writer: &HabitWriter<'_>,
    courses: &Courses,
    code: CourseCode,
    chip_day: StudyDay,
) -> Result<Written, HabitError> {
    let writing = writing_courses(courses);
    if writing.is_empty() {
        return Ok(Written::NoWritingCourse);
    }
    if !writing.contains(&code) {
        return Ok(Written::NotAWritingCourse);
    }
    let today = writer.rule.study_day(writer.now);
    if chip_day != today {
        return Ok(Written::DayClosed(checklist(writer, courses).await?));
    }
    let checklist = write_day(writer, &writing, today, Change::Toggle(code)).await?;
    Ok(Written::Done(checklist))
}

/// Makes `change` to `today`'s log in one write, with the settles of the day's `write:<code>` of
/// every writing course and its `write:all`, then announces the level the write crossed.
async fn write_day(
    writer: &HabitWriter<'_>,
    writing: &[CourseCode],
    today: StudyDay,
    change: Change,
) -> Result<Checklist, HabitError> {
    let before = level_before(writer).await;
    let mut write = writer.db.write().await?;
    match change {
        Change::Confirm(codes) => {
            for code in codes {
                store::confirm_writing(&mut write, code.as_str(), today, writer.now).await?;
            }
        }
        Change::Clear(code) => {
            store::clear_writing(&mut write, code.as_str(), today).await?;
        }
        Change::Toggle(code) => {
            if !store::clear_writing(&mut write, code.as_str(), today).await? {
                store::confirm_writing(&mut write, code.as_str(), today, writer.now).await?;
            }
        }
    }
    let rows = confirmed_through(&mut write, today).await?;
    let xp = writing_day_xp(writing, today, &rows);
    for (code, amount) in &xp.courses {
        settle_owner(
            &mut write,
            today,
            &write_source(*code),
            *amount,
            false,
            writer.now,
        )
        .await?;
    }
    settle_owner(
        &mut write,
        today,
        WRITE_ALL_SOURCE,
        xp.all,
        false,
        writer.now,
    )
    .await?;
    write.commit().await?;
    celebrate(writer, before, today).await;
    Ok(day_checklist(writing, &rows, today))
}

/// Every confirmation through `last`, as its course and day; a stored code that is no longer a
/// valid one is left out.
async fn confirmed_through(
    connection: &mut SqliteConnection,
    last: StudyDay,
) -> Result<Vec<(CourseCode, StudyDay)>, HabitError> {
    Ok(store::confirmations_between(connection, FIRST_DAY, last)
        .await?
        .into_iter()
        .filter_map(|(code, day)| CourseCode::new(&code).map(|code| (code, day)))
        .collect())
}

/// The checklist of `day` over the writing courses `writing` and the confirmations `rows`.
fn day_checklist(
    writing: &[CourseCode],
    rows: &[(CourseCode, StudyDay)],
    day: StudyDay,
) -> Checklist {
    let lines = course_streaks(writing, rows, day)
        .into_iter()
        .map(|(code, streak)| ChecklistLine {
            code,
            confirmed: rows.contains(&(code, day)),
            streak,
        })
        .collect();
    Checklist {
        day,
        lines,
        streak: writing_streak(writing, rows, day),
    }
}
