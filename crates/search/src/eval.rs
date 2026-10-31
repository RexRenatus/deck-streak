//! The evaluator (SPEC-514 R5): the cards a [`Query`] selects in a [`Collection`] on an
//! [`EngineDay`], in the engine's order, computed from the rows with no SQL and no I/O.
//!
//! Each term is the pinned engine's (`rslib/src/search/sqlwriter.rs`) read as a predicate over one
//! card and its note: the text searches are its `LIKE`s, the name searches its `(?i)` globs, the
//! states its queue and type numbers, and the day searches its cut-offs. The order is the one the
//! engine's unordered search returns: a search whose top level holds a deck name lists the cards
//! of the matching decks by deck, queue and due (the engine's `ix_cards_sched`), then the cards
//! borrowed from them by a filtered deck; every other search lists cards by id.

use crate::collection::{Card, Collection, Note, Notetype};
use crate::day::EngineDay;
use crate::pattern::{Glob, like, strip_html, to_like};
use crate::query::{Prop, Query, State, Template, Term};

/// Seconds in a day.
const DAY: i64 = 86_400;
/// The engine's default learn-ahead limit, in seconds: a learning card due within it is due now.
pub const LEARN_AHEAD_SECS: i64 = 1_200;
/// The separator of a deck name's levels as the engine stores it.
const NATIVE_SEPARATOR: &str = "\u{1f}";
/// The separator of a note's fields as the engine stores them.
const FIELD_SEPARATOR: &str = "\u{1f}";

/// The indices of the cards `query` selects, in the engine's order.
#[must_use]
pub fn eval(query: &Query, collection: &Collection, day: EngineDay) -> Vec<usize> {
    let scope = Scope { collection, day };
    order(query, collection)
        .into_iter()
        .filter(|&index| scope.holds(query, index))
        .collect()
}

/// Whether `query` selects the card at `index`.
#[must_use]
pub fn selects(query: &Query, collection: &Collection, day: EngineDay, index: usize) -> bool {
    index < collection.cards.len() && Scope { collection, day }.holds(query, index)
}

/// The engine's order of every card for `query`.
fn order(query: &Query, collection: &Collection) -> Vec<usize> {
    let mut all: Vec<usize> = (0..collection.cards.len()).collect();
    let cards = &collection.cards;
    all.sort_by_key(|&index| cards[index].id);
    let Some(name) = indexed_deck(query) else {
        return all;
    };
    let wanted = DeckName::new(name);
    let named = |id: i64| {
        collection
            .decks
            .iter()
            .any(|d| d.id == id && wanted.matches(&d.name))
    };
    let (mut own, rest): (Vec<usize>, Vec<usize>) = all
        .into_iter()
        .partition(|&index| named(cards[index].deck_id));
    own.sort_by_key(|&index| {
        let card = &cards[index];
        (card.deck_id, card.queue, card.due, card.id)
    });
    let (mut borrowed, others): (Vec<usize>, Vec<usize>) = rest.into_iter().partition(|&index| {
        let card = &cards[index];
        card.original_deck_id != 0 && named(card.original_deck_id)
    });
    borrowed.sort_by_key(|&index| (cards[index].original_deck_id, cards[index].id));
    own.extend(borrowed);
    own.extend(others);
    own
}

/// The deck name the top level of `query` holds, the one the engine's index serves.
fn indexed_deck(query: &Query) -> Option<&str> {
    fn is_name(query: &Query) -> Option<&str> {
        match query {
            Query::Term(Term::Deck(name)) if name != "*" && name != "filtered" => {
                Some(name.as_str())
            }
            _ => None,
        }
    }
    match query {
        Query::And(operands) => operands.iter().find_map(is_name),
        other => is_name(other),
    }
}

/// A deck search's name, read as the engine reads it: its levels, each trimmed, joined by the
/// stored separator, a glob over the stored name and its children.
struct DeckName(Glob);

impl DeckName {
    fn new(name: &str) -> Self {
        let native: Vec<&str> = name.split("::").map(str::trim).collect();
        Self(Glob::new(&native.join(NATIVE_SEPARATOR)))
    }

    fn matches(&self, human: &str) -> bool {
        self.0
            .matches_up_to(&human.replace("::", NATIVE_SEPARATOR), NATIVE_SEPARATOR)
    }
}

struct Scope<'a> {
    collection: &'a Collection,
    day: EngineDay,
}

impl Scope<'_> {
    fn holds(&self, query: &Query, index: usize) -> bool {
        match query {
            Query::Term(term) => self.term(term, index),
            Query::Not(inner) => !self.holds(inner, index),
            Query::And(operands) => operands.iter().all(|q| self.holds(q, index)),
            Query::Or(operands) => operands.iter().any(|q| self.holds(q, index)),
        }
    }

    fn card(&self, index: usize) -> &Card {
        &self.collection.cards[index]
    }

    fn note(&self, index: usize) -> Option<&Note> {
        self.collection.notes.get(self.card(index).note)
    }

    fn notetype(&self, note: &Note) -> Option<&Notetype> {
        self.collection.notetypes.get(note.notetype)
    }

    fn term(&self, term: &Term, index: usize) -> bool {
        let card = self.card(index);
        match term {
            Term::Text(text) => self.note(index).is_some_and(|note| self.text(note, text)),
            Term::Field { field, text } => self
                .note(index)
                .is_some_and(|note| self.field(note, field, text)),
            Term::Deck(name) => self.deck(card, name),
            Term::Tag(tag) => self.note(index).is_some_and(|note| tagged(note, tag)),
            Term::Is(state) => self.state(card, *state),
            Term::Prop(prop, compare, n) => match prop {
                Prop::Due => {
                    let due = if card.original_due == 0 {
                        card.due
                    } else {
                        card.original_due
                    };
                    (matches!(card.queue, 2 | 3)
                        && compare.holds(due, n.saturating_add(self.day.days_elapsed)))
                        || (matches!(card.queue, 1 | 4)
                            && compare.holds(due.saturating_sub(self.day.next_day_at) / DAY, *n))
                }
                Prop::Interval => compare.holds(card.interval, *n),
                Prop::Reps => compare.holds(card.reps, *n),
                Prop::Lapses => compare.holds(card.lapses, *n),
            },
            Term::Card(template) => match template {
                Template::Ordinal(ord) => card.ord == *ord,
                Template::Name(name) => self.note(index).is_some_and(|note| {
                    self.notetype(note).is_some_and(|notetype| {
                        notetype
                            .templates
                            .get(usize::from(card.ord))
                            .is_some_and(|template| named(name, template))
                    })
                }),
            },
            Term::Note(name) => self.note(index).is_some_and(|note| {
                self.notetype(note)
                    .is_some_and(|notetype| named(name, &notetype.name))
            }),
            Term::Added(days) => card.id > self.cutoff_ms(*days),
            Term::Rated { days, ease } => {
                let cutoff = self.cutoff_ms(*days);
                self.collection.revlog.iter().any(|review| {
                    review.card == index
                        && review.id >= cutoff
                        && ease.map_or(review.ease > 0, |button| review.ease == i64::from(button))
                })
            }
        }
    }

    /// The instant `days` study days before the next day begins, in milliseconds.
    fn cutoff_ms(&self, days: u32) -> i64 {
        self.day
            .next_day_at
            .saturating_sub(DAY.saturating_mul(i64::from(days)))
            .saturating_mul(1000)
    }

    fn text(&self, note: &Note, text: &str) -> bool {
        let pattern = format!("%{}%", to_like(text));
        let sort = self
            .notetype(note)
            .and_then(|notetype| note.fields.get(notetype.sort_field))
            .map(|field| strip_html(field))
            .unwrap_or_default();
        like(&pattern, &sort) || like(&pattern, &note.fields.join(FIELD_SEPARATOR))
    }

    fn field(&self, note: &Note, field: &str, text: &str) -> bool {
        let Some(notetype) = self.notetype(note) else {
            return false;
        };
        if matches!(field, "*" | "_*" | "*_") {
            let glob = Glob::new(text);
            return note.fields.iter().any(|value| glob.matches(value));
        }
        let name = Glob::new(field);
        let matched: Vec<usize> = notetype
            .fields
            .iter()
            .enumerate()
            .filter(|(_, candidate)| named_by(&name, field, candidate))
            .map(|(ordinal, _)| ordinal)
            .collect();
        let each_alone = matches!(text, "*" | "_*" | "*_");
        let value = to_like(text);
        let flds = note.fields.join(FIELD_SEPARATOR);
        ranges(&matched, !each_alone)
            .into_iter()
            .any(|(start, end)| {
                let parts: Vec<&str> = (0..notetype.fields.len())
                    .filter_map(|ordinal| {
                        if ordinal == start {
                            Some(value.as_str())
                        } else if (start..end).contains(&ordinal) {
                            None
                        } else {
                            Some("%")
                        }
                    })
                    .collect();
                like(&parts.join(FIELD_SEPARATOR), &flds)
            })
    }

    fn deck(&self, card: &Card, name: &str) -> bool {
        match name {
            "*" => true,
            "filtered" => card.original_deck_id != 0,
            name => {
                let wanted = DeckName::new(name);
                let named = |id: i64| {
                    self.collection
                        .decks
                        .iter()
                        .any(|deck| deck.id == id && wanted.matches(&deck.name))
                };
                named(card.deck_id) || (card.original_deck_id != 0 && named(card.original_deck_id))
            }
        }
    }

    fn state(&self, card: &Card, state: State) -> bool {
        match state {
            State::New => card.kind == 0,
            State::Learn => matches!(card.kind, 1 | 3),
            State::Review => matches!(card.kind, 2 | 3),
            State::Buried => matches!(card.queue, -3 | -2),
            State::Suspended => card.queue == -1,
            State::Due => {
                (matches!(card.queue, 2 | 3) && card.due <= self.day.days_elapsed)
                    || (matches!(card.queue, 1 | 4)
                        && card.due <= self.day.now.saturating_add(LEARN_AHEAD_SECS))
            }
        }
    }
}

/// Whether a note type's or template's name answers `search`: a glob when it holds a wildcard,
/// otherwise equal to it with every escape removed, case folded.
fn named(search: &str, candidate: &str) -> bool {
    named_by(&Glob::new(search), search, candidate)
}

fn named_by(glob: &Glob, search: &str, candidate: &str) -> bool {
    if glob.is_wild() {
        glob.matches(candidate)
    } else {
        let mut plain = String::with_capacity(search.len());
        let mut chars = search.chars();
        while let Some(c) = chars.next() {
            plain.push(if c == '\\' {
                chars.next().unwrap_or('\\')
            } else {
                c
            });
        }
        Glob::new(
            &plain
                .replace('\\', "\\\\")
                .replace('*', "\\*")
                .replace('_', "\\_"),
        )
        .matches(candidate)
    }
}

/// Whether one of the note's tags, or a parent of one, answers `tag`.
fn tagged(note: &Note, tag: &str) -> bool {
    match tag {
        "none" => note.tags.is_empty(),
        "*" => true,
        tag if tag.contains(' ') => false,
        tag => {
            let glob = Glob::new(tag);
            note.tags
                .iter()
                .any(|candidate| glob.matches_up_to(candidate, "::"))
        }
    }
}

/// The matched ordinals as ranges: runs of consecutive ordinals joined when `join`, each alone
/// otherwise (the engine's `collect_ranges`).
fn ranges(ordinals: &[usize], join: bool) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &ordinal in ordinals {
        match out.last_mut() {
            Some((_, end)) if join && *end == ordinal => *end = ordinal + 1,
            _ => out.push((ordinal, ordinal + 1)),
        }
    }
    out
}
