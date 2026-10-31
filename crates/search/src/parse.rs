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
use crate::query::{Compare, Operator, Prop, Query, State, Template, Term};

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

/// The engine's keys the grammar does not implement, each refused by its own name.
const UNSUPPORTED: [(&str, &str); 15] = [
    ("flag", "flag:"),
    ("resched", "resched:"),
    ("edited", "edited:"),
    ("introduced", "introduced:"),
    ("did", "did:"),
    ("mid", "mid:"),
    ("nid", "nid:"),
    ("cid", "cid:"),
    ("re", "re:"),
    ("nc", "nc:"),
    ("sc", "sc:"),
    ("w", "w:"),
    ("dupe", "dupe:"),
    ("has-cd", "has-cd:"),
    ("preset", "preset:"),
];

/// The engine's `prop:` names the grammar does not implement.
const UNSUPPORTED_PROPS: [(&str, &str); 9] = [
    ("ease", "prop:ease"),
    ("pos", "prop:pos"),
    ("rated", "prop:rated"),
    ("resched", "prop:resched"),
    ("cdn:", "prop:cdn"),
    ("cds:", "prop:cds"),
    ("s", "prop:s"),
    ("d", "prop:d"),
    ("r", "prop:r"),
];

/// One item of a group's list, before `and` and `or` are folded.
enum Item {
    Query(Query),
    And,
    Or,
}

/// Parses `search` under `profile` within `limits`.
pub(crate) fn parse(search: &str, profile: &Profile, limits: Limits) -> Result<Query, Refusal> {
    if search.len() > limits.length {
        return Err(Refusal::TooLong);
    }
    let chars: Vec<char> = search
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let start = chars.iter().position(|c| *c != ' ').unwrap_or(chars.len());
    let end = chars
        .iter()
        .rposition(|c| *c != ' ')
        .map_or(start, |i| i + 1);
    if start >= end {
        return Err(Refusal::Blank);
    }
    let mut parser = Parser {
        chars: &chars[start..end],
        at: 0,
        profile,
        limits,
        depth: 0,
    };
    let items = parser.group_inner()?;
    if parser.at < parser.chars.len() {
        // The one character no node consumes is a `)` with no group open.
        return Err(Refusal::UnopenedGroup);
    }
    fold(items, limits)
}

struct Parser<'a> {
    chars: &'a [char],
    at: usize,
    profile: &'a Profile,
    limits: Limits,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.at += 1;
        }
    }

    /// Zero or more nodes, with an `and` between two that have no operator between them.
    fn group_inner(&mut self) -> Result<Vec<Item>, Refusal> {
        let mut items = Vec::new();
        while let Some(item) = self.node()? {
            if items.len() % 2 == 0 {
                if matches!(item, Item::And | Item::Or) {
                    return Err(Refusal::MisplacedOperator);
                }
            } else if !matches!(item, Item::And | Item::Or) {
                items.push(Item::And);
            }
            items.push(item);
        }
        if matches!(items.last(), Some(Item::And | Item::Or)) {
            return Err(Refusal::MisplacedOperator);
        }
        self.skip_spaces();
        Ok(items)
    }

    /// One node: a negated group or text, a group, or text; none at the end or before a `)`.
    fn node(&mut self) -> Result<Option<Item>, Refusal> {
        self.skip_spaces();
        match self.peek() {
            None | Some(')') => Ok(None),
            Some('-') => {
                let dash = self.at;
                self.at += 1;
                match self.peek() {
                    // A dash with nothing to negate is text, as the engine reads it.
                    None | Some(' ' | ')') => {
                        self.at = dash;
                        self.text().map(Some)
                    }
                    Some('(') => Ok(Some(Item::Query(Query::negate(self.group()?)))),
                    Some(_) => match self.text()? {
                        Item::Query(query) => Ok(Some(Item::Query(Query::negate(query)))),
                        Item::And | Item::Or => Err(Refusal::MisplacedOperator),
                    },
                }
            }
            Some('(') => Ok(Some(Item::Query(self.group()?))),
            Some(_) => self.text().map(Some),
        }
    }

    /// `(` nodes `)`: one or more nodes, within the depth limit.
    fn group(&mut self) -> Result<Query, Refusal> {
        self.at += 1;
        self.depth += 1;
        if self.depth > self.limits.depth {
            return Err(Refusal::TooDeep);
        }
        let items = self.group_inner()?;
        if self.peek() != Some(')') {
            return Err(Refusal::UnclosedGroup);
        }
        self.at += 1;
        self.depth -= 1;
        if items.is_empty() {
            return Err(Refusal::EmptyGroup);
        }
        fold(items, self.limits)
    }

    /// A quoted term, a key with a quoted value, or an unquoted term.
    fn text(&mut self) -> Result<Item, Refusal> {
        if self.peek() == Some('"') {
            let inner = self.quoted()?;
            return Ok(Item::Query(Query::Term(self.term(&inner)?)));
        }
        if let Some((key, value)) = self.partially_quoted()? {
            if key.is_empty() {
                return Err(Refusal::MissingKey);
            }
            return Ok(Item::Query(Query::Term(self.keyed(&key, &value)?)));
        }
        let raw = self.unquoted()?;
        if raw.eq_ignore_ascii_case("and") {
            Ok(Item::And)
        } else if raw.eq_ignore_ascii_case("or") {
            Ok(Item::Or)
        } else {
            Ok(Item::Query(Query::Term(self.term(&raw)?)))
        }
    }

    /// The text between two unescaped quotes, its escapes kept.
    fn quoted(&mut self) -> Result<String, Refusal> {
        self.at += 1;
        let mut inner = String::new();
        loop {
            match self.peek() {
                None => return Err(Refusal::UnclosedQuote),
                Some('"') => {
                    self.at += 1;
                    break;
                }
                Some('\\') => {
                    let Some(next) = self.chars.get(self.at + 1).copied() else {
                        return Err(Refusal::UnclosedQuote);
                    };
                    inner.push('\\');
                    inner.push(next);
                    self.at += 2;
                }
                Some(c) => {
                    inner.push(c);
                    self.at += 1;
                }
            }
        }
        if inner.is_empty() {
            return Err(Refusal::EmptyQuote);
        }
        Ok(inner)
    }

    /// `key:"value"`, or nothing (and the position restored) when the text is not of that shape.
    fn partially_quoted(&mut self) -> Result<Option<(String, String)>, Refusal> {
        let start = self.at;
        let mut key = String::new();
        loop {
            match self.peek() {
                Some('\\') => match self.chars.get(self.at + 1).copied() {
                    Some(next) if next != ' ' => {
                        key.push('\\');
                        key.push(next);
                        self.at += 2;
                    }
                    _ => break,
                },
                Some(c) if !matches!(c, '"' | '(' | ')' | ':' | ' ') => {
                    key.push(c);
                    self.at += 1;
                }
                _ => break,
            }
        }
        if self.peek() == Some(':') && self.chars.get(self.at + 1) == Some(&'"') {
            self.at += 1;
            let value = self.quoted()?;
            return Ok(Some((key, value)));
        }
        self.at = start;
        Ok(None)
    }

    /// Text up to white space or an unescaped `"`, `(` or `)`, its escapes kept.
    fn unquoted(&mut self) -> Result<String, Refusal> {
        let mut raw = String::new();
        loop {
            match self.peek() {
                Some('\\') => match self.chars.get(self.at + 1).copied() {
                    Some(next) if next != ' ' => {
                        raw.push('\\');
                        raw.push(next);
                        self.at += 2;
                    }
                    _ => return Err(Refusal::BadEscape),
                },
                Some(c) if !matches!(c, '"' | '(' | ')' | ' ') => {
                    raw.push(c);
                    self.at += 1;
                }
                _ => break,
            }
        }
        if raw.is_empty() {
            // Reached only after a dash: what follows it is a quote's or a group's.
            return Err(Refusal::MisplacedOperator);
        }
        Ok(raw)
    }

    /// One term's text: unqualified text, or `key:value` split at its first unescaped colon.
    fn term(&self, raw: &str) -> Result<Term, Refusal> {
        let chars: Vec<char> = raw.chars().collect();
        let mut at = 0;
        let mut key = String::new();
        while at < chars.len() {
            match chars[at] {
                '\\' if at + 1 < chars.len() => {
                    key.push('\\');
                    key.push(chars[at + 1]);
                    at += 2;
                }
                ':' => break,
                c => {
                    key.push(c);
                    at += 1;
                }
            }
        }
        if key.is_empty() {
            return Err(Refusal::MissingKey);
        }
        if at >= chars.len() {
            let term = Term::Text(unescape(&key)?);
            return self.admitted(term);
        }
        let value: String = chars[at + 1..].iter().collect();
        self.keyed(&key, &value)
    }

    /// The term `key:value`, held to the grammar's keys and to the profile.
    fn keyed(&self, key: &str, value: &str) -> Result<Term, Refusal> {
        let lower = key.to_ascii_lowercase();
        if let Some((_, name)) = UNSUPPORTED.iter().find(|(k, _)| *k == lower) {
            return Err(Refusal::Unsupported(name));
        }
        let term = match lower.as_str() {
            "deck" => deck(value)?,
            "note" => Term::Note(unescape(value)?),
            "tag" => tag(value)?,
            "card" => match value.parse::<u16>() {
                Ok(n) => Term::Card(Template::Ordinal(n.max(1) - 1)),
                Err(_) => Term::Card(Template::Name(unescape(value)?)),
            },
            "prop" => prop(value)?,
            "added" => Term::Added(
                value
                    .parse::<u32>()
                    .map_err(|_| Refusal::BadValue(Operator::Added))?
                    .max(1),
            ),
            "rated" => rated(value)?,
            "is" => Term::Is(state(value)?),
            _ => {
                if !self.profile.admits(Operator::Field) {
                    return Err(Refusal::UnknownKey);
                }
                if value.starts_with("re:") || value.starts_with("nc:") {
                    return Err(Refusal::Unsupported("a field's re: or nc: mode"));
                }
                Term::Field {
                    field: unescape(key)?,
                    text: unescape(value)?,
                }
            }
        };
        self.admitted(term)
    }

    fn admitted(&self, term: Term) -> Result<Term, Refusal> {
        let operator = term.operator();
        if self.profile.admits(operator) {
            Ok(term)
        } else if operator == Operator::Field {
            Err(Refusal::UnknownKey)
        } else {
            Err(Refusal::NotInProfile(operator))
        }
    }
}

/// Folds a group's items into a query: `and` binds before `or`.
fn fold(items: Vec<Item>, limits: Limits) -> Result<Query, Refusal> {
    let mut disjuncts: Vec<Query> = Vec::new();
    let mut conjuncts: Vec<Query> = Vec::new();
    let mut terms = 0_usize;
    for item in items {
        match item {
            Item::Query(query) => {
                terms += 1;
                conjuncts.push(query);
            }
            Item::And => {}
            Item::Or => disjuncts.push(join(std::mem::take(&mut conjuncts), Query::And)),
        }
    }
    if terms > limits.arity {
        return Err(Refusal::TooWide);
    }
    disjuncts.push(join(conjuncts, Query::And));
    Ok(join(disjuncts, Query::Or))
}

/// One query of `operands`, joined by `with` when there are two or more.
fn join(mut operands: Vec<Query>, with: fn(Vec<Query>) -> Query) -> Query {
    if operands.len() == 1 {
        operands.remove(0)
    } else {
        with(operands)
    }
}

/// Removes the parser's escapes (`\"`, `\:`, `\(`, `\)`, `\-`) and keeps the wildcard escapes
/// (`\\`, `\*`, `\_`); a backslash before any other character, or at the end, is refused.
fn unescape(text: &str) -> Result<String, Refusal> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some(next @ ('"' | ':' | '(' | ')' | '-')) => out.push(next),
            Some(next @ ('\\' | '*' | '_')) => {
                out.push('\\');
                out.push(next);
            }
            _ => return Err(Refusal::BadEscape),
        }
    }
    Ok(out)
}

fn deck(value: &str) -> Result<Term, Refusal> {
    let name = unescape(value)?;
    if name == "current" {
        return Err(Refusal::Unsupported("deck:current"));
    }
    Ok(Term::Deck(name))
}

fn tag(value: &str) -> Result<Term, Refusal> {
    if value.starts_with("re:") {
        return Err(Refusal::Unsupported("tag:re:"));
    }
    if value.starts_with("nc:") {
        return Err(Refusal::Unsupported("tag:nc:"));
    }
    Ok(Term::Tag(unescape(value)?))
}

fn state(value: &str) -> Result<State, Refusal> {
    Ok(match value {
        "new" => State::New,
        "learn" => State::Learn,
        "review" => State::Review,
        "due" => State::Due,
        "buried" => State::Buried,
        "suspended" => State::Suspended,
        _ => return Err(Refusal::BadValue(Operator::Is)),
    })
}

fn prop(value: &str) -> Result<Term, Refusal> {
    let known = [
        ("lapses", Prop::Lapses),
        ("reps", Prop::Reps),
        ("due", Prop::Due),
        ("ivl", Prop::Interval),
    ];
    let Some((name, kind)) = known.iter().find(|(name, _)| value.starts_with(name)) else {
        if let Some((_, named)) = UNSUPPORTED_PROPS
            .iter()
            .find(|(name, _)| value.starts_with(name))
        {
            return Err(Refusal::Unsupported(named));
        }
        return Err(Refusal::BadValue(Operator::Prop));
    };
    let tail = &value[name.len()..];
    let operators = [
        ("<=", Compare::Le),
        (">=", Compare::Ge),
        ("!=", Compare::Ne),
        ("=", Compare::Eq),
        ("<", Compare::Lt),
        (">", Compare::Gt),
    ];
    let Some((text, compare)) = operators.iter().find(|(text, _)| tail.starts_with(text)) else {
        return Err(Refusal::BadValue(Operator::Prop));
    };
    let number = &tail[text.len()..];
    let parsed = if *kind == Prop::Due {
        number.parse::<i32>().map(i64::from).ok()
    } else {
        number.parse::<u32>().map(i64::from).ok()
    };
    let n = parsed.ok_or(Refusal::BadValue(Operator::Prop))?;
    Ok(Term::Prop(*kind, *compare, n))
}

fn rated(value: &str) -> Result<Term, Refusal> {
    let (days, button) = match value.split_once(':') {
        Some((days, button)) => (days, Some(button)),
        None => (value, None),
    };
    let days = days
        .parse::<u32>()
        .map_err(|_| Refusal::BadValue(Operator::Rated))?
        .max(1);
    let ease = match button {
        None => None,
        Some(button) => match button.parse::<u8>() {
            Ok(b @ 1..=4) => Some(b),
            _ => return Err(Refusal::BadValue(Operator::Rated)),
        },
    };
    Ok(Term::Rated { days, ease })
}
