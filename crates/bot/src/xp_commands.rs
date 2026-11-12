//! The owner's `/level` (SPEC-072 R25): the title's emoji, the level and the title; the total and
//! the XP into the level over the XP the level spans; and the consistency line when the
//! multiplier is above 1.0.
//!
//! The numbers are coordination's level view, the one `GET /api/level` answers from, for the study
//! day the kernel's rule gives the bot's clock. Like every message of the bot, the text names no
//! date, no time of day and no deadline.

use std::fmt::Write as _;

use deck_streak_coordination::progression::level_view::LevelView;

use crate::commands::Reply;
use crate::transport::escape_html;

/// `/level`'s answer for `view`.
#[must_use]
pub fn level_reply(view: &LevelView) -> Reply {
    let info = &view.info;
    let mut text = format!(
        "<b>{} Level {}: {}</b>\nXP: {} ({}/{} to the next level)",
        escape_html(info.emoji),
        info.level.get(),
        escape_html(info.title),
        info.total_xp,
        info.xp_into_level,
        info.xp_for_next,
    );
    if view.multiplier > 1.0 {
        let _ = write!(
            text,
            "\nConsistency: x{:.2} ({}-day run)",
            view.multiplier, view.run
        );
    }
    Reply::text(text)
}

/// The answer when the level could not be read.
#[must_use]
pub fn level_failed_reply() -> Reply {
    Reply::text(
        "Your level could not be read, so nothing is shown. Send /level to try again.".to_owned(),
    )
}
