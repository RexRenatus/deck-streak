//! Whether a card was made with AI assistance (SPEC-388 R1, R2; ADR-402 D2): its note carries the
//! tag `ai-assisted`, written in the note's own tags where its package is made. The tag travels in
//! the note's tags on every path the engine already carries them, so the core holds only the one
//! spelling and the one rule both clients share, as it holds the late rule.

/// The tag a card made with AI assistance carries in its note's tags, in the one spelling a package
/// source writes and a reader checks.
pub const MARKER: &str = "ai-assisted";

/// Whether a note's `tags`, the engine's one text of tags separated by blanks, mark it as made with
/// AI assistance: one of them is [`MARKER`] as a whole tag, compared without regard to ASCII case,
/// as the engine compares tags. A longer tag, a child tag or a part of the tag does not mark it, and
/// a note with no tags is unmarked.
#[must_use]
pub fn carries(_tags: &str) -> bool {
    false
}
