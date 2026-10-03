//! The minutes log's rules (SPEC-078 R2 to R4; ADR-078): resolving a course token, an entry's
//! bounds, the reading XP of a day, the study week's first day and the weekly bonus.

use deck_streak_kernel::{CourseCode, Courses, StudyDay};

/// The XP one minute of reading earns.
pub const READING_XP_PER_MIN: u32 = 2;
/// The most reading XP one course earns on one study day.
pub const READING_XP_DAILY_CAP_PER_LANG: u32 = 240;
/// The most minutes one entry may log.
pub const READING_MAX_ENTRY_MIN: u32 = 600;
/// The longest note an entry keeps, in characters.
pub const NOTE_MAX_CHARS: usize = 200;
/// The minutes the picker offers.
pub const READING_PRESETS: [u32; 6] = [10, 15, 20, 30, 45, 60];
/// The minutes of one course's study week that earn the weekly bonus.
pub const READING_WEEKLY_GOAL_MIN: u32 = 210;
/// The weekly bonus.
pub const READING_GOAL_XP_BONUS: u32 = 150;

/// The days of a study week.
const WEEK_DAYS: i64 = 7;

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
pub fn reading_xp(minutes: u32) -> u32 {
    minutes
        .saturating_mul(READING_XP_PER_MIN)
        .min(READING_XP_DAILY_CAP_PER_LANG)
}

/// The weekly bonus of `minutes` of one course in one study week.
#[must_use]
pub const fn goal_bonus(minutes: u32) -> u32 {
    if minutes >= READING_WEEKLY_GOAL_MIN {
        READING_GOAL_XP_BONUS
    } else {
        0
    }
}

/// The first study day of `day`'s study week: its Monday, since epoch day 0 was a Thursday.
#[must_use]
pub const fn week_start(day: StudyDay) -> StudyDay {
    let number = day.epoch_day();
    StudyDay::from_epoch_day(number - (number + 3).rem_euclid(WEEK_DAYS))
}

/// The last study day of `day`'s study week.
#[must_use]
pub const fn week_end(day: StudyDay) -> StudyDay {
    StudyDay::from_epoch_day(week_start(day).epoch_day() + WEEK_DAYS - 1)
}

/// Whether `day` is its study week's first day.
#[must_use]
pub const fn is_week_start(day: StudyDay) -> bool {
    week_start(day).epoch_day() == day.epoch_day()
}

/// The course `token` names among `courses`: a code first, then an alias.
#[must_use]
pub fn resolve_course(courses: &Courses, token: &str) -> Option<CourseCode> {
    let token = token.trim().to_lowercase();
    if let Some(course) = courses
        .courses()
        .iter()
        .find(|course| course.code.as_str() == token)
    {
        return Some(course.code);
    }
    let mut characters = token.chars();
    let (Some(alias), None) = (characters.next(), characters.next()) else {
        return None;
    };
    courses
        .courses()
        .iter()
        .find(|course| course.alias.to_ascii_lowercase() == alias)
        .map(|course| course.code)
}

/// `note` cut to [`NOTE_MAX_CHARS`] characters.
#[must_use]
pub fn cut_note(note: &str) -> String {
    note.chars().take(NOTE_MAX_CHARS).collect()
}

/// The entry of `minutes` for the course `token` names, with `note`.
///
/// # Errors
///
/// [`EntryRefusal::UnknownCourse`] when the token names no course, then
/// [`EntryRefusal::Minutes`] when the minutes are out of bounds.
pub fn entry(
    courses: &Courses,
    token: &str,
    minutes: i64,
    note: &str,
) -> Result<Entry, EntryRefusal> {
    let code = resolve_course(courses, token).ok_or(EntryRefusal::UnknownCourse)?;
    let minutes = u32::try_from(minutes)
        .ok()
        .filter(|minutes| (1..=READING_MAX_ENTRY_MIN).contains(minutes))
        .ok_or(EntryRefusal::Minutes)?;
    Ok(Entry {
        code,
        minutes,
        note: cut_note(note),
    })
}

/// The course with the most minutes this week, else of all time, the first of a tie: `week` in
/// code order and `all_time` in the order each course was first logged.
#[must_use]
pub fn most_used(week: &[(String, u32)], all_time: &[(String, u32)]) -> Option<String> {
    first_max(week).or_else(|| first_max(all_time))
}

/// The first code holding the most minutes, when any holds a minute.
fn first_max(minutes: &[(String, u32)]) -> Option<String> {
    let mut best: Option<&(String, u32)> = None;
    for candidate in minutes {
        if candidate.1 > best.map_or(0, |held| held.1) {
            best = Some(candidate);
        }
    }
    best.map(|(code, _)| code.clone())
}

/// The source of a course's reading XP.
#[must_use]
pub fn read_source(code: CourseCode) -> String {
    format!("read:{}", code.as_str())
}

/// The source of a course's weekly bonus.
#[must_use]
pub fn goal_source(code: CourseCode) -> String {
    format!("readgoal:{}", code.as_str())
}
