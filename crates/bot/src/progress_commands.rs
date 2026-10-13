//! The owner's progress command (SPEC-077 R15, R16): each configured course's Road to C2.
//!
//! It reads coordination's progress view, the one `GET /api/progress` answers from: one line per
//! course, ordered by name, naming its current band, its mastery and its current unit. The reply
//! carries one button, which opens Road to C2 in the Mini App; when no course is stored yet it says
//! the courses appear after the next sync, and the button stays, since it is the reply's one way
//! into the Mini App, whose page says the same. When courses are listed, the reply closes with one
//! italic line saying what the mastery figure measures (SPEC-408).

use deck_streak_coordination::progress_view::StoredProgress;
use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup, WebAppInfo};

use crate::commands::{MiniAppUrl, Reply};
use crate::transport::escape_html;

/// What the mastery figure measures, as the reply's closing line. It repeats the English message
/// file's `progress_mastery_about` for a test to compare, because the bot carries no message files.
const MASTERY_ABOUT: &str = "Mastery is an estimate from your reviews: the average, over the cards it counts, of how likely you are to recall each card now, with cards not yet firmly learned counted for less and new or suspended cards counted as 0.";

/// The path segment of the Mini App's Road to C2 page.
const PROGRESS_PAGE: &str = "progress";

/// The progress command's answer for the stored `progress`, with the button that opens Road to C2
/// in the Mini App at `app`.
#[must_use]
pub fn progress_reply(progress: &[StoredProgress], app: &MiniAppUrl) -> Reply {
    let text = if progress.is_empty() {
        "Road to C2 has no course yet: it appears after your next sync.".to_owned()
    } else {
        let mut lines = vec!["<b>Road to C2</b>".to_owned()];
        lines.extend(progress.iter().map(course_line));
        lines.push(format!("<i>{}</i>", escape_html(MASTERY_ABOUT)));
        lines.join("\n")
    };
    let open = InlineKeyboardButton::builder()
        .text("Open Road to C2")
        .web_app(
            WebAppInfo::builder()
                .url(page_url(app, PROGRESS_PAGE))
                .build(),
        )
        .build();
    Reply {
        text,
        keyboard: Some(
            InlineKeyboardMarkup::builder()
                .inline_keyboard(vec![vec![open]])
                .build(),
        ),
    }
}

/// The progress command's answer when the progress could not be read.
#[must_use]
pub fn progress_failed_reply() -> Reply {
    Reply::text(
        "Your Road to C2 could not be read, so nothing is shown. Send /progress to try again."
            .to_owned(),
    )
}

/// One course's line: its flag and name, its current band, its mastery to the whole percent, and
/// its current unit, or `no unit yet` before a card of it is mature. Every stored text is escaped
/// where it enters the markup.
fn course_line(stored: &StoredProgress) -> String {
    let unit = stored
        .current_unit
        .map_or_else(|| "no unit yet".to_owned(), |unit| format!("unit {unit}"));
    format!(
        "{} <b>{}</b>: {}, {}% mastery, {unit}",
        escape_html(&stored.flag),
        escape_html(&stored.name),
        escape_html(&stored.current_band),
        stored.mastery_pct.round()
    )
}

/// `app` with the path segment `page` appended to its path, its query and fragment kept.
fn page_url(app: &MiniAppUrl, page: &str) -> String {
    let url = app.as_str();
    let (path, rest) = url.split_at(url.find(['?', '#']).unwrap_or(url.len()));
    format!("{}/{page}{rest}", path.trim_end_matches('/'))
}
