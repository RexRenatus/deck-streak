//! The habits' owner use cases (SPEC-078 R2 to R5, R18; ADR-078): a minutes log entry and its undo,
//! each one write with its settles, which the bot calls and, later, the Mini App's routes.
//!
//! The bot cannot name the habits context (docs/CONTEXT-MAP.md), so the rules and constants its
//! replies render are re-exported here, from their one home.

pub mod minutes;

pub use deck_streak_habits::minutes::{
    EntryRefusal, READING_GOAL_XP_BONUS, READING_MAX_ENTRY_MIN, READING_PRESETS,
    READING_WEEKLY_GOAL_MIN, resolve_course,
};
pub use minutes::{
    Course, HabitError, HabitWriter, Logged, Undone, log_minutes, undo_entry, undo_newest,
};
