//! The owner's `/score` (SPEC-071 R21): the current study day's score, its grade, its reviews and
//! its true retention.
//!
//! The numbers are coordination's score reads, the one `GET /api/score` answers from, for the study
//! day the kernel's rule gives the bot's clock, so the bot and the Mini App cannot show two scores
//! for one day. A retention the day does not have, because no review was answered, is said to be
//! absent, never shown as 0. A day no recompute has rolled up yet has no score, and the answer
//! says so and names `/sync`. Like every message of the bot, the text names no date, no time of day
//! and no deadline.

use deck_streak_coordination::score::DayScore;

use crate::commands::Reply;
use crate::transport::escape_html;

/// `/score`'s answer for the current study day: its score, or that it has none yet.
#[must_use]
pub fn score_reply(score: Option<&DayScore>) -> Reply {
    let Some(score) = score else {
        return Reply::text(
            "Today has no score yet: none of today's study has been recomputed. /sync recomputes \
             it now."
                .to_owned(),
        );
    };
    let retention = score.retention.map_or_else(
        || "Retention: absent, since no review was answered today.".to_owned(),
        |retention| format!("Retention: {retention:.1}%"),
    );
    Reply::text(format!(
        "<b>Today's score: {}</b>\nGrade: {} {}\nReviews: {}\n{retention}",
        score.total,
        escape_html(score.grade_label),
        escape_html(score.grade_emoji),
        score.reviews,
    ))
}

/// The answer when the score could not be read.
#[must_use]
pub fn score_failed_reply() -> Reply {
    Reply::text(
        "Your score could not be read, so nothing is shown. Send /score to try again.".to_owned(),
    )
}
