//! What a reading earns (SPEC-047 R6, ADR-047): the amounts, the once-scoped sources and the track.
//! The amounts are this context's constants, not keys of the economy pack.

use deck_streak_kernel::Track;

use crate::reading::ReadingId;
use crate::topic::TopicKey;

/// XP for the owner's tap.
pub const READ_XP: u32 = 0;
/// XP for a studied reading.
pub const STUDIED_XP: u32 = 0;
/// The most a reading earns.
pub const MAX_READING_XP: u32 = 0;

/// The grant source of the read tap.
#[must_use]
pub fn read_source(id: &ReadingId) -> String {
    let _ = id;
    String::new()
}

/// The grant source of the studied verdict.
#[must_use]
pub fn studied_source(id: &ReadingId) -> String {
    let _ = id;
    String::new()
}

/// The track a topic's XP is earned on.
#[must_use]
pub fn track_of(topic: &TopicKey) -> Track {
    let _ = topic;
    Track::Language
}
