//! A note's fields, split and joined on the unit separator U+001F (SPEC-602 R11).
//!
//! `join(split(s)) == s` for every text, and `split(join(fs)) == fs` when no field holds the
//! separator.

/// The separator of a note's fields in `notes.flds`.
pub const FIELD_SEPARATOR: char = '\u{1f}';

/// The fields of `text`: one more than the separators it holds.
#[must_use]
pub fn split(text: &str) -> Vec<&str> {
    text.split(FIELD_SEPARATOR).collect()
}

/// The text of `fields`, the separator between each two.
#[must_use]
pub fn join<S: AsRef<str>>(fields: &[S]) -> String {
    let mut text = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            text.push(FIELD_SEPARATOR);
        }
        text.push_str(field.as_ref());
    }
    text
}
