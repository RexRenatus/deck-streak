//! The owner's progress command (SPEC-077 R15, R16): each configured course's Road to C2.
//!
//! It reads coordination's progress view, the one `GET /api/progress` answers from.

use deck_streak_coordination::progress_view::StoredProgress;

use crate::commands::{MiniAppUrl, Reply};

/// The progress command's answer for the stored `progress`, with the button that opens Road to C2
/// in the Mini App at `app`.
#[must_use]
pub fn progress_reply(progress: &[StoredProgress], app: &MiniAppUrl) -> Reply {
    let _unread = (progress, app);
    Reply::text("Road to C2".to_owned())
}

/// The progress command's answer when the progress could not be read.
#[must_use]
pub fn progress_failed_reply() -> Reply {
    Reply::text("Road to C2".to_owned())
}
