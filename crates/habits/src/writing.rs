//! The writing habit's rules (SPEC-078 R6 to R8; ADR-078): the writing courses, what a study day's
//! confirmations pay, the days every writing course was confirmed on, and the streaks they make.

use std::collections::BTreeSet;

use deck_streak_kernel::{CourseCode, Courses, StudyDay};

/// What a confirmed writing course pays on one study day.
pub const WRITING_XP_PER_DAY: u32 = 75;
/// What `write:all` pays on a day every writing course is confirmed.
pub const WRITING_XP_ALL_THREE_BONUS: u32 = 100;
/// The writing streak the ink week badge needs.
pub const WRITING_STREAK_WEEK: u32 = 7;
/// The writing streak the ink month badge needs.
pub const WRITING_STREAK_MONTH: u32 = 30;
/// The writing streak the ink century badge needs.
pub const WRITING_STREAK_CENTURY: u32 = 100;
/// The source of the writing bonus.
pub const WRITE_ALL_SOURCE: &str = "write:all";

/// A study day's writing XP: each writing course's, then `write:all`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WritingXp {
    /// Each writing course and its XP, in the writing set's order.
    pub courses: Vec<(CourseCode, u32)>,
    /// The bonus.
    pub all: u32,
}

/// The configured courses marked writing, in the file's order.
#[must_use]
pub fn writing_courses(courses: &Courses) -> Vec<CourseCode> {
    courses
        .courses()
        .iter()
        .filter(|course| course.writing)
        .map(|course| course.code)
        .collect()
}

/// The source of `code`'s writing XP.
#[must_use]
pub fn write_source(code: CourseCode) -> String {
    format!("write:{}", code.as_str())
}

/// Every day of `rows` on which each writing course is confirmed. An empty writing set confirms
/// no day (SPEC-078 R7): the predecessor's subset test held over it, and paid the bonus for
/// writing nothing.
#[must_use]
pub fn all_confirmed_days(
    writing: &[CourseCode],
    rows: &[(CourseCode, StudyDay)],
) -> BTreeSet<StudyDay> {
    if writing.is_empty() {
        return BTreeSet::new();
    }
    let confirmed: BTreeSet<(CourseCode, StudyDay)> = rows.iter().copied().collect();
    rows.iter()
        .map(|(_, day)| *day)
        .filter(|day| {
            writing
                .iter()
                .all(|code| confirmed.contains(&(*code, *day)))
        })
        .collect()
}

/// The writing XP `rows` earn on `day`: each confirmed writing course's, and the bonus when every
/// writing course is confirmed.
#[must_use]
pub fn writing_day_xp(
    writing: &[CourseCode],
    day: StudyDay,
    rows: &[(CourseCode, StudyDay)],
) -> WritingXp {
    let courses = writing
        .iter()
        .map(|code| {
            let paid = if rows.contains(&(*code, day)) {
                WRITING_XP_PER_DAY
            } else {
                0
            };
            (*code, paid)
        })
        .collect();
    let all = if all_confirmed_days(writing, rows).contains(&day) {
        WRITING_XP_ALL_THREE_BONUS
    } else {
        0
    };
    WritingXp { courses, all }
}

/// The consecutive days of `days` ending on `day`, or on the day before when `day` is not among
/// them.
#[must_use]
pub fn streak_ending(days: &BTreeSet<StudyDay>, day: StudyDay) -> u32 {
    let previous = StudyDay::from_epoch_day(day.epoch_day() - 1);
    let end = if days.contains(&day) { day } else { previous };
    let mut streak = 0;
    let mut cursor = end.epoch_day();
    while days.contains(&StudyDay::from_epoch_day(cursor)) {
        streak += 1;
        cursor -= 1;
    }
    streak
}

/// The writing streak on `day`: the days every writing course was confirmed on.
#[must_use]
pub fn writing_streak(
    writing: &[CourseCode],
    rows: &[(CourseCode, StudyDay)],
    day: StudyDay,
) -> u32 {
    streak_ending(&all_confirmed_days(writing, rows), day)
}

/// Each writing course's own streak on `day`, in the writing set's order.
#[must_use]
pub fn course_streaks(
    writing: &[CourseCode],
    rows: &[(CourseCode, StudyDay)],
    day: StudyDay,
) -> Vec<(CourseCode, u32)> {
    writing
        .iter()
        .map(|code| {
            let days: BTreeSet<StudyDay> = rows
                .iter()
                .filter(|(row_code, _)| row_code == code)
                .map(|(_, day)| *day)
                .collect();
            (*code, streak_ending(&days, day))
        })
        .collect()
}
