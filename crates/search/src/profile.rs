//! The profiles (SPEC-514 R3): which of the closed grammar's operators a search may use where it
//! is read. A profile is a constant list; nothing adds to it at run time.

use crate::parse::{self, Limits, Refusal};
use crate::query::{Operator, Query};

/// A named subset of the grammar's operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    name: &'static str,
    operators: &'static [Operator],
}

impl Profile {
    /// The skip day's configured search: the operators that pick a study day's due review cards.
    /// It reads no field and no history, so a key outside its list is unknown to it.
    pub const SKIP: Self = Self {
        name: "skip",
        operators: &[
            Operator::Text,
            Operator::Deck,
            Operator::Tag,
            Operator::Is,
            Operator::Prop,
            Operator::Card,
            Operator::Note,
        ],
    };

    /// Every operator of the grammar, the one the evaluator is held to the engine's goldens by.
    pub const OPERATOR: Self = Self {
        name: "operator",
        operators: &Operator::ALL,
    };

    /// Every profile.
    pub const ALL: [Self; 2] = [Self::SKIP, Self::OPERATOR];

    /// The profile's name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The operators the profile admits.
    #[must_use]
    pub const fn operators(&self) -> &'static [Operator] {
        self.operators
    }

    /// Whether the profile admits `operator`.
    #[must_use]
    pub fn admits(&self, operator: Operator) -> bool {
        self.operators.contains(&operator)
    }

    /// Parses `search` under this profile within [`Limits::DEFAULT`].
    ///
    /// # Errors
    ///
    /// The [`Refusal`] that names why the search is not one expression of this profile.
    pub fn parse(&self, search: &str) -> Result<Query, Refusal> {
        parse::parse(search, self, Limits::DEFAULT)
    }

    /// Parses `search` under this profile within `limits`.
    ///
    /// # Errors
    ///
    /// The [`Refusal`] that names why the search is not one expression of this profile.
    pub fn parse_within(&self, search: &str, limits: Limits) -> Result<Query, Refusal> {
        parse::parse(search, self, limits)
    }
}
