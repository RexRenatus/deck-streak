//! The seed a reading is written from (SPEC-046 R1, R5): the day set's cards, its distinct notes
//! with their field text in field order, and for a language topic the new words.
//!
//! The seed is what the engine holds; the model is shown the notes as fenced card text and never
//! anything else about the owner.

use crate::coverage::{anchor_for_note, is_anchor_usable};
use crate::state::FailedReason;

/// Which of the two forms a topic's reading takes (SPEC-046 R3, R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track {
    /// A law topic: the issue, rule, application, conclusion form.
    Law,
    /// A language topic: the glosses, grammar, pronunciation, culture form.
    Language,
}

/// One distinct note of the day set: its id and its field text in field order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedNote {
    /// The note's id in the collection.
    pub id: i64,
    /// The note's fields, joined in field order.
    pub text: String,
}

/// The seed of one topic's reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seed {
    /// The new cards of the day set.
    pub card_ids: Vec<i64>,
    /// The day set's distinct notes.
    pub notes: Vec<SeedNote>,
    /// A language topic's new words, markup removed; empty for a law topic.
    pub new_words: Vec<String>,
}

impl Seed {
    /// The count of new cards, the `n` of the word target.
    #[must_use]
    pub fn new_cards(&self) -> u32 {
        u32::try_from(self.card_ids.len()).unwrap_or(u32::MAX)
    }

    /// The citation key of a note: `n` and its id.
    #[must_use]
    pub fn key_of(note_id: i64) -> String {
        format!("n{note_id}")
    }

    /// Every note's citation key, in note order.
    #[must_use]
    pub fn source_keys(&self) -> Vec<String> {
        self.notes
            .iter()
            .map(|note| Self::key_of(note.id))
            .collect()
    }
}

/// Whether the seed can be written from at all, decided before any model call (SPEC-046 R5).
///
/// # Errors
///
/// [`FailedReason::SeedEmpty`] when the seed holds no note, and
/// [`FailedReason::AnchorUnusableAll`] when a law seed's anchors are all unusable.
pub fn screen(seed: &Seed, track: Track) -> Result<(), FailedReason> {
    if seed.notes.is_empty() {
        return Err(FailedReason::SeedEmpty);
    }
    if track == Track::Law
        && !seed
            .notes
            .iter()
            .any(|note| is_anchor_usable(&anchor_for_note(&note.text)))
    {
        return Err(FailedReason::AnchorUnusableAll);
    }
    Ok(())
}
