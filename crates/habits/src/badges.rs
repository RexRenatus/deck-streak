//! The habit badges' conditions (SPEC-078 R9, R17; ADR-078): which habit badges a day's context
//! earns, judged by the fold's habit badge step alone.

use crate::writing::{WRITING_STREAK_CENTURY, WRITING_STREAK_MONTH, WRITING_STREAK_WEEK};

/// The minutes of reading in one study week the marathon reader badge needs.
pub const MARATHON_READER_WEEK_MIN: u32 = 0;

/// What the habit badges are judged on, for one evaluated study day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HabitBadgeContext {
    /// The reading entries logged through the day.
    pub reading_entries: u32,
    /// The writing confirmations through the day.
    pub writing_entries: u32,
    /// The writing streak over every writing course.
    pub writing_all_streak: u32,
    /// The configured courses read in the day's study week.
    pub langs_read_this_week: u32,
    /// Every course's minutes in the day's study week.
    pub week_total_min: u32,
    /// Whether every configured course met the weekly goal.
    pub all_langs_goal_met: bool,
    /// How many courses are configured.
    pub courses: u32,
}

/// The habit badges `context` earns, in the catalog's order.
#[must_use]
pub fn earned(context: &HabitBadgeContext) -> Vec<&'static str> {
    let mut keys = Vec::new();
    if context.reading_entries > 0 {
        keys.push("first_page");
    }
    if context.writing_entries > 0 {
        keys.push("quill_initiate");
    }
    if context.writing_all_streak >= WRITING_STREAK_WEEK {
        keys.push("ink_week");
    }
    if context.writing_all_streak >= WRITING_STREAK_MONTH {
        keys.push("ink_month");
    }
    if context.writing_all_streak >= WRITING_STREAK_CENTURY {
        keys.push("ink_century");
    }
    if context.all_langs_goal_met {
        keys.push("bookworm_week");
    }
    if context.langs_read_this_week >= context.courses {
        keys.push("polyglot_reader");
    }
    if context.week_total_min >= MARATHON_READER_WEEK_MIN {
        keys.push("marathon_reader");
    }
    keys
}
