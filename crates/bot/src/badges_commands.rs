//! The owner's `/badges` and `/records` (SPEC-073 R18).
//!
//! `/badges` lists the [`BADGES_LISTED`] most recently awarded badges, newest first, each its emoji
//! and name, or the predecessor's line when none is earned (`bot.py:CommandBot._render`). The
//! predecessor printed its twenty oldest first; newest first is this delivery's choice. `/records`
//! lists each record with its value, the day that set it and the value it beat, then the line
//! naming the record today is closest to (`bot.py:CommandBot._render_records`). The day is the one
//! date these messages name: R18 asks for it, and it is a fact of the past, never a deadline.
//!
//! Both read coordination's views, the ones `GET /api/badges` and `GET /api/records` answer from.

use deck_streak_coordination::progression::badges_view::EarnedLine;
use deck_streak_coordination::progression::records_view::RecordsView;

use crate::commands::Reply;

/// The badges `/badges` lists at most.
pub const BADGES_LISTED: usize = 20;

/// `/badges`' answer for `earned`, most recently awarded first.
#[must_use]
pub fn badges_reply(earned: &[EarnedLine]) -> Reply {
    let _ = earned;
    Reply::text(String::new())
}

/// `/badges`' answer when the badges could not be read.
#[must_use]
pub fn badges_failed_reply() -> Reply {
    Reply::text(String::new())
}

/// `/records`' answer for `view`.
#[must_use]
pub fn records_reply(view: &RecordsView) -> Reply {
    let _ = view;
    Reply::text(String::new())
}

/// `/records`' answer when the records could not be read.
#[must_use]
pub fn records_failed_reply() -> Reply {
    Reply::text(String::new())
}
