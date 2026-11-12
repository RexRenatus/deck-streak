//! The owner's `/badges` and `/records` (SPEC-073 R18).
//!
//! `/badges` lists the [`BADGES_LISTED`] most recently awarded badges, newest first, each its emoji
//! and name, or the predecessor's line when none is earned (`bot.py:CommandBot._render`). The
//! predecessor printed its twenty oldest first; newest first is this delivery's choice. `/records`
//! lists each record with its value, the day that set it and the value it beat, then the line
//! naming the record today is closest to (`bot.py:CommandBot._render_records`). The day is the one
//! date these messages name, as an ISO date (`2025-01-09`): R18 asks for it, and it is a fact of
//! the past, never a deadline.
//!
//! Both read coordination's views, the ones `GET /api/badges` and `GET /api/records` answer from.

use deck_streak_coordination::progression::badges_view::EarnedLine;
use deck_streak_coordination::progression::records_view::RecordsView;

use crate::commands::Reply;
use crate::transport::escape_html;

/// The badges `/badges` lists at most.
pub const BADGES_LISTED: usize = 20;

/// `/badges`' answer for `earned`, most recently awarded first.
#[must_use]
pub fn badges_reply(earned: &[EarnedLine]) -> Reply {
    if earned.is_empty() {
        return Reply::text(
            "No badges yet \u{2014} study to earn your first! \u{1f45f}".to_owned(),
        );
    }
    let lines: Vec<String> = earned
        .iter()
        .take(BADGES_LISTED)
        .map(|line| format!("{} {}", escape_html(&line.emoji), escape_html(&line.name)))
        .collect();
    Reply::text(format!("\u{1f3c5} <b>Badges</b>\n{}", lines.join("\n")))
}

/// `/badges`' answer when the badges could not be read.
#[must_use]
pub fn badges_failed_reply() -> Reply {
    Reply::text(
        "Your badges could not be read, so nothing is shown. Send /badges to try again.".to_owned(),
    )
}

/// `/records`' answer for `view`.
#[must_use]
pub fn records_reply(view: &RecordsView) -> Reply {
    if view.lines.is_empty() {
        return Reply::text(
            "\u{1f4c8} No records yet \u{2014} they mint themselves as you study.".to_owned(),
        );
    }
    let mut lines = vec!["\u{1f3c5} <b>Personal records</b>".to_owned()];
    lines.extend(view.lines.iter().map(|line| {
        format!(
            "\u{2022} {}: <b>{}</b> ({}, was {})",
            escape_html(line.label),
            line.value,
            line.study_day,
            line.previous
        )
    }));
    if let Some((kind, gap)) = view.chase
        && let Some(chased) = view.lines.iter().find(|line| line.kind == kind)
    {
        lines.push(format!(
            "\u{1f3c3} Chase it: {gap} from \u{201c}{}\u{201d} today.",
            escape_html(chased.label)
        ));
    }
    Reply::text(lines.join("\n"))
}

/// `/records`' answer when the records could not be read.
#[must_use]
pub fn records_failed_reply() -> Reply {
    Reply::text(
        "Your records could not be read, so nothing is shown. Send /records to try again."
            .to_owned(),
    )
}
