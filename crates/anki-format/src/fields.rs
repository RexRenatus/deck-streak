//! A note's fields, split and joined on the unit separator U+001F (SPEC-602 R11).
//!
//! Red-first stand-in: split and join answer a fixed wrong value.

/// The separator of a note's fields in `notes.flds`.
pub const FIELD_SEPARATOR: char = '\u{1f}';

/// The fields of `text`.
#[must_use]
pub fn split(text: &str) -> Vec<&str> {
    let _ = text;
    Vec::new()
}

/// The text of `fields`.
#[must_use]
pub fn join<S: AsRef<str>>(fields: &[S]) -> String {
    let _ = fields;
    String::new()
}
