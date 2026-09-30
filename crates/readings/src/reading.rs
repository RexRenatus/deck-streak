//! What a stored reading is (SPEC-046 R9): its id, its word count and its minutes.

use std::fmt::Write as _;

use sha2::{Digest, Sha256};
use unicode_segmentation::UnicodeSegmentation;

/// How many hex digits a reading id keeps.
pub const ID_HEX_DIGITS: usize = 32;
/// The words a learner reads in a minute.
pub const WORDS_PER_MINUTE: u32 = 200;
/// Chinese characters to a word, in tenths.
const ZH_CHARS_PER_WORD_TENTHS: u32 = 15;
/// Japanese characters to a word, in tenths.
const JA_CHARS_PER_WORD_TENTHS: u32 = 20;

/// A reading's id: 32 hex digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingId(String);

impl ReadingId {
    /// The id of the reading of `topic` first generated on `first_day`, for `digest`.
    #[must_use]
    pub fn of(topic: &str, first_day: i64, digest: &str) -> Self {
        let hash = Sha256::digest(format!("{topic}\n{first_day}\n{digest}").as_bytes());
        let mut hex = String::with_capacity(ID_HEX_DIGITS);
        for byte in hash.iter().take(ID_HEX_DIGITS / 2) {
            let _ = write!(hex, "{byte:02x}");
        }
        Self(hex)
    }

    /// The id a stored text is, or `None` unless it is 32 lowercase hex digits.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        (text.len() == ID_HEX_DIGITS
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .then(|| Self(text.to_owned()))
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
    u32::try_from(text.unicode_words().count()).unwrap_or(u32::MAX)
}

/// The minutes a reading takes.
#[must_use]
pub fn minutes(text: &str, lang: Option<&str>) -> u32 {
    let tenths = match lang {
        Some("zh") => Some(ZH_CHARS_PER_WORD_TENTHS),
        Some("ja") => Some(JA_CHARS_PER_WORD_TENTHS),
        _ => None,
    };
    match tenths {
        Some(tenths) => {
            let chars = u64::try_from(text.chars().filter(|c| !c.is_whitespace()).count())
                .unwrap_or(u64::MAX);
            // chars / (tenths / 10) words at WORDS_PER_MINUTE, rounded up, in integers.
            let per_minute = u64::from(tenths) * u64::from(WORDS_PER_MINUTE);
            u32::try_from((chars * 10).div_ceil(per_minute)).unwrap_or(u32::MAX)
        }
        None => word_count(text).div_ceil(WORDS_PER_MINUTE),
    }
}
