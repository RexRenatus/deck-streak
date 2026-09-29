//! What a stored reading is (SPEC-046 R9): its id, its word count and its minutes.

/// A reading's id: 32 hex digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingId(String);

impl ReadingId {
    /// The id of the reading of `topic` first generated on `first_day`, for `digest`.
    #[must_use]
    pub fn of(topic: &str, first_day: i64, digest: &str) -> Self {
        let _ = (topic, first_day, digest);
        Self(String::new())
    }

    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The words of a reading's prose.
#[must_use]
pub fn word_count(text: &str) -> u32 {
    let _ = text;
    0
}

/// The minutes a reading takes.
#[must_use]
pub fn minutes(text: &str, lang: Option<&str>) -> u32 {
    let _ = (text, lang);
    0
}
