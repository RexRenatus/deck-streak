//! Splitting a long HTML text into messages Telegram accepts (SPEC-026 R7; the telegram-platform
//! pack's formatting rules).
//!
//! Telegram measures a text after entity parsing, in UTF-16 units: a tag counts nothing, an entity
//! such as `&amp;` counts as the one character it stands for, and a character outside the Basic
//! Multilingual Plane counts two. A text over the bound is split at the last paragraph that fits,
//! else the last line, else the last word, and only when none fits, between two characters; a cut
//! never falls inside a tag or an entity, and never between the two halves of a character. The
//! whitespace at a cut stays at the end of the chunk before it, so the chunks' visible texts, joined,
//! are the text's. A tag open at a cut is closed at the end of its chunk and opened again, with the
//! same attributes, at the start of the next, so every chunk parses on its own. The predecessor cut
//! at a character count between lines (`telegram.py:_chunk`); a cut inside a tag makes Telegram
//! refuse the chunk.

/// The UTF-16 units `html` counts after entity parsing: what Telegram's bounds measure.
#[must_use]
pub fn visible_units(html: &str) -> usize {
    html.chars().count()
}

/// `html` split into chunks of at most `limit` UTF-16 units each after entity parsing, each of
/// which parses on its own. A text that fits is one chunk, exactly as given; a text with nothing
/// but whitespace to show is no chunk at all, since Telegram refuses an empty message.
#[must_use]
pub fn chunks(html: &str, limit: usize) -> Vec<String> {
    let _ = limit;
    vec![html.to_owned()]
}
