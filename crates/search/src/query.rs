//! The typed query (SPEC-514 R1): what a search means once it has parsed, as data a caller can
//! check, wrap and evaluate, never as text that is glued to more text.
//!
//! A search is a disjunction of conjunctions: the engine writes its node list into SQL, where
//! `and` binds before `or`, so `a b OR c` is `(a and b) or c`. The parser builds that shape, and a
//! group keeps its own node: `(a b) c` is an [`Query::And`] whose first operand is an `And`.

use std::fmt;

/// One parsed search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    /// One term.
    Term(Term),
    /// The query's complement.
    Not(Box<Query>),
    /// Every operand holds; never empty.
    And(Vec<Query>),
    /// Some operand holds; never empty.
    Or(Vec<Query>),
}

/// One term of the closed grammar (SPEC-514 R2). Each value keeps the wildcard escapes `\*`,
/// `\_` and `\\` the matcher reads; the parser's own escapes (`\"`, `\:`, `\(`, `\)`, `\-`) are
/// already removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    /// Text with no key: a substring of the note's fields or its sort field.
    Text(String),
    /// `field:text`: the text in one named field. The name is any key the grammar does not
    /// reserve, so only a profile that admits [`Operator::Field`] reads it.
    Field {
        /// The field's name, a glob over the note type's field names.
        field: String,
        /// The field's whole text, a glob.
        text: String,
    },
    /// `deck:name`: the card's deck or its original deck, or a child of either; `filtered` and
    /// `*` are the engine's two special names.
    Deck(String),
    /// `tag:name`: a tag, or a parent of a tag, a glob.
    Tag(String),
    /// `is:state`.
    Is(State),
    /// `prop:due`, `prop:ivl`, `prop:reps` or `prop:lapses` against a whole number.
    Prop(Prop, Compare, i64),
    /// `card:n` or `card:name`.
    Card(Template),
    /// `note:name`: the note type's name, a glob.
    Note(String),
    /// `added:n`: the card was added in the last `n` study days.
    Added(u32),
    /// `rated:n` or `rated:n:b`: the card was answered in the last `n` study days, with any button
    /// or with button `b`.
    Rated {
        /// The days counted back, at least 1.
        days: u32,
        /// The answer button, 1 to 4, or any.
        ease: Option<u8>,
    },
}

/// The states `is:` names (SPEC-514 R2): the closed list the grammar admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    /// `is:new`: a new card.
    New,
    /// `is:learn`: a learning or relearning card.
    Learn,
    /// `is:review`: a review or relearning card.
    Review,
    /// `is:due`: a review card due today, or a learning card due within the learn-ahead limit.
    Due,
    /// `is:buried`: buried by hand or as a sibling.
    Buried,
    /// `is:suspended`.
    Suspended,
}

/// The properties `prop:` names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Prop {
    /// `prop:due`: days from today.
    Due,
    /// `prop:ivl`: the interval in days.
    Interval,
    /// `prop:reps`: the review count.
    Reps,
    /// `prop:lapses`: the lapse count.
    Lapses,
}

/// A comparison of `prop:`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Compare {
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `=`.
    Eq,
    /// `!=`.
    Ne,
    /// `>=`.
    Ge,
    /// `>`.
    Gt,
}

impl Compare {
    /// Whether `left` stands in this relation to `right`.
    #[must_use]
    pub fn holds(self, left: i64, right: i64) -> bool {
        match self {
            Self::Lt => left < right,
            Self::Le => left <= right,
            Self::Eq => left == right,
            Self::Ne => left != right,
            Self::Ge => left >= right,
            Self::Gt => left > right,
        }
    }

    /// The comparison's text.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Ge => ">=",
            Self::Gt => ">",
        }
    }
}

/// What `card:` names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Template {
    /// The template's ordinal from 0: `card:1` is ordinal 0.
    Ordinal(u16),
    /// The template's name, a glob.
    Name(String),
}

/// The operators of the closed grammar: one per kind of [`Term`]. A profile admits a subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Operator {
    /// Text with no key.
    Text,
    /// A field search.
    Field,
    /// `deck:`.
    Deck,
    /// `tag:`.
    Tag,
    /// `is:`.
    Is,
    /// `prop:`.
    Prop,
    /// `card:`.
    Card,
    /// `note:`.
    Note,
    /// `added:`.
    Added,
    /// `rated:`.
    Rated,
}

impl Operator {
    /// Every operator, in declaration order.
    pub const ALL: [Self; 10] = [
        Self::Text,
        Self::Field,
        Self::Deck,
        Self::Tag,
        Self::Is,
        Self::Prop,
        Self::Card,
        Self::Note,
        Self::Added,
        Self::Rated,
    ];

    /// The operator's name: its key, or what it is when it has none.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Field => "field",
            Self::Deck => "deck",
            Self::Tag => "tag",
            Self::Is => "is",
            Self::Prop => "prop",
            Self::Card => "card",
            Self::Note => "note",
            Self::Added => "added",
            Self::Rated => "rated",
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Term {
    /// The operator this term is written with.
    #[must_use]
    pub const fn operator(&self) -> Operator {
        match self {
            Self::Text(_) => Operator::Text,
            Self::Field { .. } => Operator::Field,
            Self::Deck(_) => Operator::Deck,
            Self::Tag(_) => Operator::Tag,
            Self::Is(_) => Operator::Is,
            Self::Prop(..) => Operator::Prop,
            Self::Card(_) => Operator::Card,
            Self::Note(_) => Operator::Note,
            Self::Added(_) => Operator::Added,
            Self::Rated { .. } => Operator::Rated,
        }
    }
}

impl Query {
    /// The query of one term.
    #[must_use]
    pub const fn term(term: Term) -> Self {
        Self::Term(term)
    }

    /// The complement of `query`.
    #[must_use]
    pub fn negate(query: Self) -> Self {
        Self::Not(Box::new(query))
    }

    /// Every term of the query, depth first, left to right.
    #[must_use]
    pub fn terms(&self) -> Vec<&Term> {
        let mut found = Vec::new();
        let mut pending = vec![self];
        while let Some(query) = pending.pop() {
            match query {
                Self::Term(term) => found.push(term),
                Self::Not(inner) => pending.push(inner),
                Self::And(operands) | Self::Or(operands) => pending.extend(operands.iter().rev()),
            }
        }
        found
    }
}
