//! The minutes log's rules (SPEC-078 R2 to R4; ADR-078): resolving a course token, an entry's
//! bounds, the reading XP of a day, the study week's first day and the weekly bonus.

use deck_streak_kernel::{CourseCode, Courses, StudyDay};

/// The XP one minute of reading earns.
pub const READING_XP_PER_MIN: u32 = 0;
/// The most reading XP one course earns on one study day.
pub const READING_XP_DAILY_CAP_PER_LANG: u32 = 0;
/// The most minutes one entry may log.
pub const READING_MAX_ENTRY_MIN: u32 = 0;
/// The longest note an entry keeps, in characters.
pub const NOTE_MAX_CHARS: usize = 0;
/// The minutes the picker offers.
pub const READING_PRESETS: [u32; 6] = [0; 6];
/// The minutes of one course's study week that earn the weekly bonus.
pub const READING_WEEKLY_GOAL_MIN: u32 = 0;
/// The weekly bonus.
pub const READING_GOAL_XP_BONUS: u32 = 0;

/// Why an entry was refused. Each names the rule, never the value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntryRefusal {
    /// The token names none of the owner's courses.
    #[error("an entry's course is one of the owner's courses")]
    UnknownCourse,
    /// The minutes are outside the bounds.
    #[error("an entry's minutes are a whole number within the bounds")]
    Minutes,
}

/// One entry, checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The course.
    pub code: CourseCode,
    /// The minutes.
    pub minutes: u32,
    /// The note, cut to its bound.
    pub note: String,
}

/// The reading XP of `minutes` of one course on one study day.
#[must_use]
pub const fn reading_xp(minutes: u32) -> u32 {
    let _ = minutes;
    0
}

/// The weekly bonus of `minutes` of one course in one study week.
#[must_use]
pub const fn goal_bonus(minutes: u32) -> u32 {
    let _ = minutes;
    0
}

/// The first study day of `day`'s study week.
#[must_use]
pub const fn week_start(day: StudyDay) -> StudyDay {
    day
}

/// The last study day of `day`'s study week.
#[must_use]
pub const fn week_end(day: StudyDay) -> StudyDay {
    day
}

/// Whether `day` is its study week's first day.
#[must_use]
pub const fn is_week_start(day: StudyDay) -> bool {
    let _ = day;
    false
}

/// The course `token` names among `courses`: a code first, then an alias.
#[must_use]
pub fn resolve_course(courses: &Courses, token: &str) -> Option<CourseCode> {
    let _ = (courses, token);
    None
}

/// `note` cut to [`NOTE_MAX_CHARS`] characters.
#[must_use]
pub fn cut_note(note: &str) -> String {
    note.to_owned()
}

/// The entry of `minutes` for the course `token` names, with `note`.
///
/// # Errors
///
/// [`EntryRefusal::UnknownCourse`] when the token names no course, then
/// [`EntryRefusal::Minutes`] when the minutes are out of bounds.
pub fn entry(courses: &Courses, token: &str, minutes: i64, note: &str) -> Result<Entry, EntryRefusal> {
    let _ = (courses, token, minutes, note);
    Err(EntryRefusal::UnknownCourse)
}

/// The course with the most minutes this week, else of all time, the first of a tie.
#[must_use]
pub fn most_used(week: &[(String, u32)], all_time: &[(String, u32)]) -> Option<String> {
    let _ = (week, all_time);
    None
}

/// The source of a course's reading XP.
#[must_use]
pub fn read_source(code: CourseCode) -> String {
    let _ = code;
    String::new()
}

/// The source of a course's weekly bonus.
#[must_use]
pub fn goal_source(code: CourseCode) -> String {
    let _ = code;
    String::new()
}
