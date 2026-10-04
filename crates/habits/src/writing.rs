//! The writing habit's rules (SPEC-078 R6 to R8; ADR-078): the writing courses, what a study day's
//! confirmations pay, the days every writing course was confirmed on, and the streaks they make.

use std::collections::BTreeSet;

use deck_streak_kernel::{CourseCode, Courses, StudyDay};

/// What a confirmed writing course pays on one study day.
pub const WRITING_XP_PER_DAY: u32 = 0;
/// What `write:all` pays on a day every writing course is confirmed.
pub const WRITING_XP_ALL_THREE_BONUS: u32 = 0;
/// The writing streak the ink week badge needs.
pub const WRITING_STREAK_WEEK: u32 = 0;
/// The writing streak the ink month badge needs.
pub const WRITING_STREAK_MONTH: u32 = 0;
/// The writing streak the ink century badge needs.
pub const WRITING_STREAK_CENTURY: u32 = 0;
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
pub fn writing_courses(_courses: &Courses) -> Vec<CourseCode> {
    Vec::new()
}

/// The source of `code`'s writing XP.
#[must_use]
pub fn write_source(code: CourseCode) -> String {
    format!("write:{}", code.as_str())
}

/// Every day of `rows` on which each writing course is confirmed.
#[must_use]
pub fn all_confirmed_days(
    writing: &[CourseCode],
    rows: &[(CourseCode, StudyDay)],
) -> BTreeSet<StudyDay> {
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

/// The writing XP `rows` earn on `day`.
#[must_use]
pub fn writing_day_xp(
    writing: &[CourseCode],
    day: StudyDay,
    rows: &[(CourseCode, StudyDay)],
) -> WritingXp {
    let courses = writing.iter().map(|code| (*code, 0)).collect();
    let all = if writing.iter().all(|code| rows.contains(&(*code, day))) {
        WRITING_XP_ALL_THREE_BONUS
    } else {
        0
    };
    WritingXp { courses, all }
}

/// The consecutive days of `days` ending on `day` or the day before.
#[must_use]
pub fn streak_ending(_days: &BTreeSet<StudyDay>, _day: StudyDay) -> u32 {
    0
}

/// The writing streak on `day`.
#[must_use]
pub fn writing_streak(
    writing: &[CourseCode],
    rows: &[(CourseCode, StudyDay)],
    day: StudyDay,
) -> u32 {
    streak_ending(&all_confirmed_days(writing, rows), day)
}

/// Each writing course's own streak on `day`.
#[must_use]
pub fn course_streaks(
    writing: &[CourseCode],
    _rows: &[(CourseCode, StudyDay)],
    _day: StudyDay,
) -> Vec<(CourseCode, u32)> {
    writing.iter().map(|code| (*code, 0)).collect()
}
