//! The closed grammar's parser (SPEC-514 R2 to R4): the engine's search syntax, read into a
//! [`Query`] under a [`Profile`] and within [`Limits`], every refusal named.
//!
//! The syntax is the pinned engine's (`rslib/src/search/parser.rs`): terms joined by white space
//! are `and`ed, `or` and `and` are words in any case, `-` negates the term or group it prefixes,
//! `(` and `)` group, a quoted text is one term, and a key may quote its value (`deck:"a b"`).
//! What differs is the vocabulary: every key is one of the grammar's own, or a field name only
//! where the profile admits field searches, and every other key is refused by name. Nothing is
//! read past a refusal, and no refusal carries the search's text.

use std::fmt;

use crate::profile::Profile;
use crate::query::{Operator, Query};

/// The bounds every parse is held to (SPEC-514 R4): a search past one is refused, never
/// truncated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes a search may hold.
    pub length: usize,
    /// The most groups that may nest inside one another.
    pub depth: usize,
    /// The most terms one group, or the search's top level, may join.
    pub arity: usize,
}

impl Limits {
    /// The limits a configured search is parsed within.
    pub const DEFAULT: Self = Self {
        length: 1024,
        depth: 8,
        arity: 32,
    };
}

/// Why a search is refused. No variant carries the search's text, so a refusal can be logged or
/// shown wherever the search itself may not be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The search is empty or white space.
    Blank,
    /// A `)` closes a group that was never opened.
    UnopenedGroup,
    /// A `(` is never closed.
    UnclosedGroup,
    /// A group holds nothing.
    EmptyGroup,
    /// A `"` is never closed.
    UnclosedQuote,
    /// A quote holds nothing.
    EmptyQuote,
    /// An `and` or an `or` has no term on one side.
    MisplacedOperator,
    /// A colon has no key before it.
    MissingKey,
    /// A backslash escapes a character the grammar does not escape, or ends the search.
    BadEscape,
    /// A key the profile does not know: neither one of the grammar's keys nor, where the profile
    /// admits field searches, a field name. The key is part of the search, so it is not named.
    UnknownKey,
    /// An engine operator the grammar does not implement, named from the grammar's own list.
    Unsupported(&'static str),
    /// A grammar operator the profile does not admit.
    NotInProfile(Operator),
    /// An operator given a value its closed list does not hold.
    BadValue(Operator),
    /// The search is longer than [`Limits::length`].
    TooLong,
    /// Groups nest deeper than [`Limits::depth`].
    TooDeep,
    /// A group joins more terms than [`Limits::arity`].
    TooWide,
}

impl Refusal {
    /// Whether the refusal is of the search's shape: it does not parse as one expression,
    /// whatever its vocabulary.
    #[must_use]
    pub const fn is_structural(self) -> bool {
        matches!(
            self,
            Self::Blank
                | Self::UnopenedGroup
                | Self::UnclosedGroup
                | Self::EmptyGroup
                | Self::UnclosedQuote
                | Self::EmptyQuote
                | Self::MisplacedOperator
                | Self::MissingKey
                | Self::BadEscape
        )
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blank => f.write_str("the search is blank"),
            Self::UnopenedGroup => f.write_str("the search closes a group it never opened"),
            Self::UnclosedGroup => f.write_str("the search leaves a group open"),
            Self::EmptyGroup => f.write_str("the search holds an empty group"),
            Self::UnclosedQuote => f.write_str("the search leaves a quote open"),
            Self::EmptyQuote => f.write_str("the search holds an empty quote"),
            Self::MisplacedOperator => {
                f.write_str("the search has an and or an or with no term on one side")
            }
            Self::MissingKey => f.write_str("the search has a colon with no key before it"),
            Self::BadEscape => {
                f.write_str("the search has a backslash before a character it cannot escape")
            }
            Self::UnknownKey => f.write_str("the search names a key the profile does not know"),
            Self::Unsupported(name) => {
                write!(
                    f,
                    "the search uses {name}, which the grammar does not implement"
                )
            }
            Self::NotInProfile(operator) => {
                write!(
                    f,
                    "the search uses {operator}, which the profile does not admit"
                )
            }
            Self::BadValue(operator) => {
                write!(
                    f,
                    "the search gives {operator} a value its list does not hold"
                )
            }
            Self::TooLong => f.write_str("the search is longer than its length limit"),
            Self::TooDeep => f.write_str("the search nests groups deeper than its depth limit"),
            Self::TooWide => f.write_str("a group of the search joins more terms than its limit"),
        }
    }
}

impl std::error::Error for Refusal {}

/// The stub's parse: every search is refused as blank.
pub(crate) fn parse(search: &str, profile: &Profile, limits: Limits) -> Result<Query, Refusal> {
    let _ = (search, profile, limits, Operator::Text);
    Err(Refusal::Blank)
}
