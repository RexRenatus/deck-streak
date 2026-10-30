//! The one grammar that finds every media name a note's text writes (SPEC-513 R4; ADR-531 D4).
//! STUB: every function keeps its shape and answers wrongly, so the crate's tests are seen red
//! first.

use std::borrow::Cow;

/// The tags whose `src` or `data` attribute names a file, in the order the grammar tries them.
pub const TAGS: [&str; 5] = ["img", "audio", "video", "object", "source"];

/// The attributes that name a file, in the order the grammar tries them.
pub const ATTRIBUTES: [&str; 2] = ["src", "data"];

/// The opening of a sound tag, matched in lower case only.
pub const SOUND_OPEN: &str = "[sound:";

/// Every media name `text` writes. STUB: none.
#[must_use]
pub fn scan(_text: &str) -> Vec<String> {
    Vec::new()
}

/// Whether a written name is remote. STUB: never.
#[must_use]
pub fn is_remote(_written: &str) -> bool {
    false
}

/// `written` with its HTML entities decoded. STUB: as written.
#[must_use]
pub fn decode_entities(written: &str) -> Cow<'_, str> {
    Cow::Borrowed(written)
}

/// A sound tag's name at `open`. STUB: none.
#[allow(dead_code, reason = "the stub keeps the covered item's shape")]
fn sound_at(_text: &str, _open: usize) -> Option<(&str, usize)> {
    None
}
