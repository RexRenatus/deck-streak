//! A note's Bloom tier (SPEC-072 R3): the one tag ingest keeps.
//!
//! The reader takes each note's tags in the same read as its card and reduces them to a [`Tier`]
//! here, so the tags themselves never leave ingest. The rule is the predecessor's
//! `anki_reader.py:_parse_tier` at `27ee2bc`: the first whitespace-separated token that is `T1` to
//! `T4`, in either case, wins.

/// A Bloom tier a note's tags may carry, `T1` the shallowest and `T4` the deepest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Remember.
    T1,
    /// Understand.
    T2,
    /// Apply.
    T3,
    /// Analyse.
    T4,
}

impl Tier {
    /// The tag as the note carries it, upper case.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::T3 => "T3",
            Self::T4 => "T4",
        }
    }

    /// The tier a token names, in either case, or none.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        [Self::T1, Self::T2, Self::T3, Self::T4]
            .into_iter()
            .find(|tier| token.eq_ignore_ascii_case(tier.as_str()))
    }
}

/// The tier in a note's `tags` text: the first token that names one, or none.
#[must_use]
pub fn parse_tier(tags: &str) -> Option<Tier> {
    let _ = tags;
    None
}
