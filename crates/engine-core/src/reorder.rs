//! The reorder-only input of due-card scores (SPEC-414, ADR-428, over ADR-339): an outside
//! scorer's score for each card due today, keyed by the engine's day count and the collection's
//! schema stamp it was made for, and the one function that orders today's stock queue by it.
//!
//! The function moves entries and has no way to change one: it is generic over the entry, and
//! reads only a card id through the caller's accessor. A stale input (another day or another
//! schema stamp), an absent one, a switched-off one and an invalid score each leave the stock
//! order. No client calls it yet; the wiring is the study screens' (ADR-428 D5).

use std::cmp::Ordering;
use std::collections::BTreeMap;

/// An outside scorer's scores for the cards due on one day of one collection snapshot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DueScores {
    /// The engine's day count the scores were made for, as the engine counts today.
    pub day: u32,
    /// The schema stamp (`col.scm`) of the collection snapshot the scorer read.
    pub snapshot: i64,
    /// Each scored card id's score; a card with none is unscored.
    pub scores: BTreeMap<i64, f64>,
}

/// The order the review presents today's stock `queue` in.
///
/// The queue comes back exactly when `on` is false, `input` is `None`, or the input was made for
/// another day than `today` or another snapshot than `snapshot`: a stale input cannot be told
/// from a fresh one by anything else it holds (ADR-428 D1). Otherwise every entry whose card has
/// a score in `[0, 1]` comes first, highest first, entries with equal scores in their stock order
/// (`-0.0` equals `0.0` under `partial_cmp`), then every other entry in its stock order: NaN, the
/// infinities and a score outside `[0, 1]` count as no score (D4). `card` reads an entry's card id
/// and is the only way the function looks into an entry.
#[must_use]
pub fn reorder<T>(
    on: bool,
    today: u32,
    snapshot: i64,
    input: Option<&DueScores>,
    queue: Vec<T>,
    card: impl Fn(&T) -> i64,
) -> Vec<T> {
    let applied = match input {
        Some(input) if on && input.day == today && input.snapshot == snapshot => input,
        _ => return queue,
    };
    let mut scored = Vec::new();
    let mut unscored = Vec::new();
    for entry in queue {
        match applied.scores.get(&card(&entry)) {
            Some(&score) if (0.0..=1.0).contains(&score) => scored.push((score, entry)),
            _ => unscored.push(entry),
        }
    }
    // A used score is never NaN, so `partial_cmp` always orders two of them.
    scored.sort_by(|(first, _), (second, _)| second.partial_cmp(first).unwrap_or(Ordering::Equal));
    scored
        .into_iter()
        .map(|(_, entry)| entry)
        .chain(unscored)
        .collect()
}
