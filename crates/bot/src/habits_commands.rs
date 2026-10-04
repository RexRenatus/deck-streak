//! The owner's `/read` and `/undo` (SPEC-078 R2, R4; ADR-078): an entry of reading minutes, by a
//! course and its minutes, by the minutes alone for the most-used course, or by the course picker
//! and then a preset; and the undo of the newest entry, by the command or by a logged reply's Undo
//! button. The owner's `/write` and `/unwrite` (SPEC-078 R6 to R8): a writing confirmation of the
//! current study day, its clearing, and the checklist whose chips toggle the day they were drawn
//! for.
//!
//! The rules and the use cases are coordination's (`coordination::habits`), because the bot cannot
//! name the habits context. Every reply is HTML with the owner's token escaped, and none names a
//! date, a time of day or a deadline.

use deck_streak_coordination::habits::{
    Checklist, Course, EntryRefusal, HabitError, Logged, READING_GOAL_XP_BONUS,
    READING_MAX_ENTRY_MIN, READING_PRESETS, READING_WEEKLY_GOAL_MIN, Undone, Written,
};
use deck_streak_kernel::{CourseCode, Courses, StudyDay};
use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::commands::Reply;
use crate::transport::escape_html;

/// The prefix every habit callback's data starts with.
pub const HABIT_PREFIX: &str = "hb:";

/// The picker's button for one course: `hb:c:<code>`.
const COURSE_PREFIX: &str = "hb:c:";

/// A preset's button: `hb:m:<code>:<minutes>`.
const MINUTES_PREFIX: &str = "hb:m:";

/// A logged reply's Undo button: `hb:u:<entry id>`.
const UNDO_PREFIX: &str = "hb:u:";

/// A writing checklist's chip: `hb:w:<code>:<epoch day>`, the day it was drawn for.
const WRITING_PREFIX: &str = "hb:w:";

/// The presets a row of the preset keyboard holds.
const PRESETS_PER_ROW: usize = 3;

/// What a habit button asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HabitCallback {
    /// The picker's course: show its presets.
    Course(CourseCode),
    /// A preset: log its minutes for its course.
    Minutes {
        /// The course.
        code: CourseCode,
        /// The preset's minutes.
        minutes: u32,
    },
    /// An Undo button: remove its entry while it is the newest.
    Undo(i64),
    /// A writing chip: toggle its course on its day, while that day is the current study day.
    Writing {
        /// The writing course.
        code: CourseCode,
        /// The study day the chip was drawn for.
        day: StudyDay,
    },
}

/// The data of the picker's button for `code`.
#[must_use]
pub fn course_data(code: &CourseCode) -> String {
    format!("{COURSE_PREFIX}{}", code.as_str())
}

/// The data of the preset button of `minutes` for `code`.
#[must_use]
pub fn minutes_data(code: &CourseCode, minutes: u32) -> String {
    format!("{MINUTES_PREFIX}{}:{minutes}", code.as_str())
}

/// The data of the Undo button of entry `entry`.
#[must_use]
pub fn undo_data(entry: i64) -> String {
    format!("{UNDO_PREFIX}{entry}")
}

/// The data of the writing chip of `code`, drawn for `day`.
#[must_use]
pub fn writing_data(code: &CourseCode, day: StudyDay) -> String {
    format!("{WRITING_PREFIX}{}:{}", code.as_str(), day.epoch_day())
}

/// The habit button `data` names, or `None` when it names none.
#[must_use]
pub fn parse_callback(data: &str) -> Option<HabitCallback> {
    if let Some(code) = data.strip_prefix(COURSE_PREFIX) {
        return CourseCode::new(code).map(HabitCallback::Course);
    }
    if let Some(rest) = data.strip_prefix(MINUTES_PREFIX) {
        let (code, minutes) = rest.split_once(':')?;
        let code = CourseCode::new(code)?;
        let minutes = minutes.parse::<u32>().ok()?;
        return Some(HabitCallback::Minutes { code, minutes });
    }
    if let Some(rest) = data.strip_prefix(WRITING_PREFIX) {
        let (code, day) = rest.split_once(':')?;
        let code = CourseCode::new(code)?;
        let day = StudyDay::from_epoch_day(day.parse::<i64>().ok()?);
        return Some(HabitCallback::Writing { code, day });
    }
    let entry = data.strip_prefix(UNDO_PREFIX)?;
    entry.parse::<i64>().ok().map(HabitCallback::Undo)
}

/// What a `/read` message asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadRequest<'a> {
    /// Bare `/read`: the course picker.
    Pick,
    /// A course with no minutes: how `/read` is used.
    Usage,
    /// An entry: its course (none for the most-used one), its minutes as typed, and its note.
    Log {
        /// The course token, or `None` when the minutes came first.
        course: Option<&'a str>,
        /// The minutes, as typed.
        minutes: &'a str,
        /// The note, possibly empty.
        note: &'a str,
    },
}

/// What the `/read` message `text` asks for. A first token of digits alone is the minutes, for the
/// most-used course.
#[must_use]
pub fn parse_read(text: &str) -> ReadRequest<'_> {
    let Some((_command, rest)) = text.trim().split_once(char::is_whitespace) else {
        return ReadRequest::Pick;
    };
    let (first, rest) = split_token(rest);
    if first.is_empty() {
        return ReadRequest::Pick;
    }
    if first.bytes().all(|byte| byte.is_ascii_digit()) {
        return ReadRequest::Log {
            course: None,
            minutes: first,
            note: rest,
        };
    }
    let (minutes, note) = split_token(rest);
    if minutes.is_empty() {
        return ReadRequest::Usage;
    }
    ReadRequest::Log {
        course: Some(first),
        minutes,
        note,
    }
}

/// The first whitespace-separated token of `text`, and the trimmed rest.
fn split_token(text: &str) -> (&str, &str) {
    let text = text.trim();
    text.split_once(char::is_whitespace)
        .map_or((text, ""), |(token, rest)| (token, rest.trim()))
}

/// The name a reply shows for `code`: its course's, else the code itself.
#[must_use]
pub fn course_name(courses: &Courses, code: &str) -> String {
    courses
        .courses()
        .iter()
        .find(|course| course.code.as_str() == code)
        .map_or_else(|| code.to_owned(), |course| course.name.clone())
}

/// The answer to an entry logged as `logged`, for the course named `name`, with its Undo button.
#[must_use]
pub fn logged_reply(logged: &Logged, name: &str) -> Reply {
    let week = if logged.goal_bonus > 0 {
        format!(
            "This week: {} minutes, so the weekly goal is reached: +{} XP.",
            logged.week_minutes, logged.goal_bonus
        )
    } else {
        format!(
            "This week: {} of {READING_WEEKLY_GOAL_MIN} minutes toward the +{READING_GOAL_XP_BONUS} \
             XP goal.",
            logged.week_minutes
        )
    };
    let undo = InlineKeyboardButton::builder()
        .text("Undo")
        .callback_data(undo_data(logged.entry_id))
        .build();
    Reply {
        text: format!(
            "Logged {} minutes of reading for <b>{}</b>.\nToday: {} minutes, {} XP.\n{week}",
            logged.minutes,
            escape_html(name),
            logged.day_minutes,
            logged.day_xp,
        ),
        keyboard: Some(
            InlineKeyboardMarkup::builder()
                .inline_keyboard(vec![vec![undo]])
                .build(),
        ),
    }
}

/// The course picker: one button for each of `courses`.
#[must_use]
pub fn pick_course_reply(courses: &Courses) -> Reply {
    let rows = courses
        .courses()
        .iter()
        .map(|course| {
            vec![
                InlineKeyboardButton::builder()
                    .text(course.name.clone())
                    .callback_data(course_data(&course.code))
                    .build(),
            ]
        })
        .collect();
    Reply {
        text: "Which course did you read for?".to_owned(),
        keyboard: Some(
            InlineKeyboardMarkup::builder()
                .inline_keyboard(rows)
                .build(),
        ),
    }
}

/// The minute presets for `code`, the course named `name`.
#[must_use]
pub fn presets_reply(code: &CourseCode, name: &str) -> Reply {
    let rows = READING_PRESETS
        .chunks(PRESETS_PER_ROW)
        .map(|row| {
            row.iter()
                .map(|minutes| {
                    InlineKeyboardButton::builder()
                        .text(format!("{minutes} min"))
                        .callback_data(minutes_data(code, *minutes))
                        .build()
                })
                .collect()
        })
        .collect();
    Reply {
        text: format!(
            "How many minutes did you read for <b>{}</b>?",
            escape_html(name)
        ),
        keyboard: Some(
            InlineKeyboardMarkup::builder()
                .inline_keyboard(rows)
                .build(),
        ),
    }
}

/// How `/read` is used, when a course came with no minutes.
#[must_use]
pub fn usage_reply() -> Reply {
    Reply::text(
        "Send <code>/read &lt;course&gt; &lt;minutes&gt; [note]</code> to log your reading, \
         <code>/read &lt;minutes&gt;</code> for your most-used course, or /read alone to pick a \
         course and the minutes."
            .to_owned(),
    )
}

/// The answer when the minutes are not a whole number within the bounds.
#[must_use]
pub fn refused_minutes_reply() -> Reply {
    Reply::text(format!(
        "Minutes are a whole number from 1 to {READING_MAX_ENTRY_MIN}, so nothing was logged."
    ))
}

/// The answer when `token` names none of the owner's courses.
#[must_use]
pub fn unknown_course_reply(token: &str) -> Reply {
    Reply::text(format!(
        "<code>{}</code> is not one of your courses, so nothing was logged. Send /read alone to \
         pick one.",
        escape_html(token)
    ))
}

/// The answer when no course is configured.
#[must_use]
pub fn no_courses_reply() -> Reply {
    Reply::text("No courses are configured, so there is no course to log reading for.".to_owned())
}

/// The answer when the entry could not be written.
#[must_use]
pub fn read_failed_reply() -> Reply {
    Reply::text(
        "The minutes could not be logged, so nothing changed. Send /read to try again.".to_owned(),
    )
}

/// The answer once an entry of `minutes` for the course named `name` was removed.
#[must_use]
pub fn undo_done_reply(name: &str, minutes: u32) -> Reply {
    Reply::text(format!(
        "Removed {minutes} minutes of reading for <b>{}</b>. Its day's XP and its week's bonus are \
         settled again.",
        escape_html(name)
    ))
}

/// The answer when there is no entry to remove.
#[must_use]
pub fn undo_nothing_reply() -> Reply {
    Reply::text("There is no reading entry to undo.".to_owned())
}

/// The answer to an Undo button whose entry is no longer the newest.
#[must_use]
pub fn undo_stale_reply() -> Reply {
    Reply::text(
        "That entry is no longer the newest, so nothing was removed. Send /undo to remove the \
         newest entry."
            .to_owned(),
    )
}

/// The answer when the undo could not be written.
#[must_use]
pub fn undo_failed_reply() -> Reply {
    Reply::text(
        "The entry could not be removed, so nothing changed. Send /undo to try again.".to_owned(),
    )
}

/// The answer to an entry for `course` that ended as `logged`, over `courses`.
#[must_use]
pub fn read_outcome_reply(
    courses: &Courses,
    course: Course<'_>,
    logged: Result<Logged, HabitError>,
) -> Reply {
    match logged {
        Ok(logged) => logged_reply(&logged, &course_name(courses, logged.code.as_str())),
        Err(HabitError::NoCourse) => pick_course_reply(courses),
        Err(HabitError::Refused(EntryRefusal::Minutes)) => refused_minutes_reply(),
        Err(HabitError::Refused(EntryRefusal::UnknownCourse)) => match course {
            Course::Token(token) => unknown_course_reply(token),
            Course::MostUsed => pick_course_reply(courses),
        },
        Err(HabitError::Kernel(error)) => {
            tracing::error!(%error, "the owner's reading was not logged");
            read_failed_reply()
        }
    }
}

/// The answer to an undo that ended as `undone`, over `courses`.
#[must_use]
pub fn undo_outcome_reply(courses: &Courses, undone: Result<Undone, HabitError>) -> Reply {
    match undone {
        Ok(Undone::Removed { code, minutes, .. }) => {
            undo_done_reply(&course_name(courses, &code), minutes)
        }
        Ok(Undone::Nothing) => undo_nothing_reply(),
        Ok(Undone::Stale) => undo_stale_reply(),
        Err(error) => {
            tracing::error!(%error, "the owner's undo was not written");
            undo_failed_reply()
        }
    }
}

/// The writing checklist of `checklist` over `courses`, with one toggle chip per writing course.
#[must_use]
pub fn checklist_reply(_courses: &Courses, _checklist: &Checklist) -> Reply {
    Reply::text(String::new())
}

/// The answer once today's writing was confirmed, with the checklist.
#[must_use]
pub fn write_confirmed_reply(_courses: &Courses, _checklist: &Checklist) -> Reply {
    Reply::text(String::new())
}

/// The answer once today's writing confirmation was cleared, with the checklist.
#[must_use]
pub fn write_cleared_reply(_courses: &Courses, _checklist: &Checklist) -> Reply {
    Reply::text(String::new())
}

/// The answer to a chip drawn for a day that has closed, with today's checklist.
#[must_use]
pub fn write_day_closed_reply(_courses: &Courses, _checklist: &Checklist) -> Reply {
    Reply::text(String::new())
}

/// The answer when a token names no writing course.
#[must_use]
pub fn not_a_writing_course_reply() -> Reply {
    Reply::text(String::new())
}

/// The answer when no writing course is configured.
#[must_use]
pub fn no_writing_course_reply() -> Reply {
    Reply::text(String::new())
}

/// The answer when the writing could not be written.
#[must_use]
pub fn write_failed_reply() -> Reply {
    Reply::text(String::new())
}

/// The answer to a writing write that ended as `written`, over `courses`, where `done` words a
/// write that changed the log.
#[must_use]
pub fn written_reply(
    _courses: &Courses,
    _written: Result<Written, HabitError>,
    _done: fn(&Courses, &Checklist) -> Reply,
) -> Reply {
    Reply::text(String::new())
}
