//! The owner's `/streak` (SPEC-076 R22): both tracks, the law track first when it has activity,
//! with the language track's heat and freezes.
//!
//! The numbers are coordination's streak view, the one `GET /api/streak` answers from. Like every
//! message of the bot, the text names no date, no time of day and no deadline.

use deck_streak_coordination::streak_views::StreakView;

use crate::commands::Reply;
use crate::transport::escape_html;

/// `/streak`'s answer for `view`.
#[must_use]
pub fn streak_reply(view: &StreakView) -> Reply {
    let heat = if view.language_heat.is_empty() {
        String::new()
    } else {
        format!(" {}", escape_html(view.language_heat))
    };
    let freezes = view.language.freezes;
    let noun = if freezes == 1 { "freeze" } else { "freezes" };
    let language = format!(
        "Language: {}{heat} (best {}, {freezes} {noun})",
        view.language.current, view.language.longest
    );
    let law = format!("Law: {} (best {})", view.law.current, view.law.longest);
    let lines = if view.law.current > 0 {
        [law, language]
    } else {
        [language, law]
    };
    Reply::text(format!("<b>Streaks</b>\n{}\n{}", lines[0], lines[1]))
}

/// The answer when the streaks could not be read.
#[must_use]
pub fn streak_failed_reply() -> Reply {
    Reply::text(
        "Your streaks could not be read, so nothing is shown. Send /streak to try again."
            .to_owned(),
    )
}
