//! The skip day's search, wrapped by structure (SPEC-514 R8): the configured search is parsed
//! under [`Profile::SKIP`] first, so it is one expression of that profile, and the holds are
//! joined to it as the operands of one top-level `and`. A configured text that closes the group
//! it is wrapped in (`) or (`) is refused at the parse, so it never reaches the wrap.
//!
//! The text the engine runs is the predecessor's wrap of the configured text followed by the
//! holds, byte for byte; because the configured text parsed as one expression, that text parses
//! back to [`SkipQuery::query`].

use crate::parse::Refusal;
use crate::profile::Profile;
use crate::query::{Compare, Prop, Query, State, Term};

/// The wrap's review-only terms, as the predecessor wrote them.
pub const REVIEW_ONLY: &str = "-is:new -is:learn";
/// What holds a skip to the study day's due review cards and out of filtered decks.
pub const HOLDS: &str = "prop:due=0 -is:suspended -is:buried -deck:filtered";

/// A configured skip search, parsed and wrapped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkipQuery {
    configured: Query,
    text: String,
}

impl SkipQuery {
    /// Parses `configured` under [`Profile::SKIP`] and wraps it.
    ///
    /// # Errors
    ///
    /// The [`Refusal`] the parse gives: a text that is not one expression of the skip profile is
    /// never wrapped.
    pub fn new(configured: &str) -> Result<Self, Refusal> {
        let parsed = Profile::SKIP.parse(configured)?;
        Ok(Self {
            configured: parsed,
            text: format!("({configured}) {REVIEW_ONLY} {HOLDS}"),
        })
    }

    /// The configured search, parsed.
    #[must_use]
    pub const fn configured(&self) -> &Query {
        &self.configured
    }

    /// The wrapped query: the configured search and each hold, the operands of one `and`.
    #[must_use]
    pub fn query(&self) -> Query {
        let mut operands = vec![self.configured.clone()];
        operands.extend(holds());
        Query::And(operands)
    }

    /// The wrapped text the engine runs.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// The holds, in their written order: review cards only, due today, neither suspended nor
/// buried, and in no filtered deck.
#[must_use]
pub fn holds() -> Vec<Query> {
    vec![
        Query::negate(Query::Term(Term::Is(State::New))),
        Query::negate(Query::Term(Term::Is(State::Learn))),
        Query::Term(Term::Prop(Prop::Due, Compare::Eq, 0)),
        Query::negate(Query::Term(Term::Is(State::Suspended))),
        Query::negate(Query::Term(Term::Is(State::Buried))),
        Query::negate(Query::Term(Term::Deck("filtered".to_owned()))),
    ]
}
