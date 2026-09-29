//! The form a reading takes and the word target it is written to (SPEC-046 R3, R4).

use crate::seed::{Seed, Track};

/// The band's floor, in primer words: the `reading-length` gate's lower bound.
pub const BAND_FLOOR_WORDS: u32 = 0;
/// The band's ceiling, in primer words: the `reading-length` gate's upper bound.
pub const BAND_CEILING_WORDS: u32 = 0;
/// The words a new card adds to the target beyond the first.
pub const WORDS_PER_NEW_CARD: u32 = 0;

/// The prose target for a topic with `new_cards` new cards.
#[must_use]
pub fn word_target(new_cards: u32) -> u32 {
    let _ = new_cards;
    0
}

/// One of the two forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Form {
    track: Track,
}

impl Form {
    /// The form of a track.
    #[must_use]
    pub const fn of(track: Track) -> Self {
        Self { track }
    }

    /// The track this form is for.
    #[must_use]
    pub const fn track(&self) -> Track {
        self.track
    }

    /// The sections, in order, the last being `retrieval`.
    #[must_use]
    pub fn sections(&self) -> &'static [&'static str] {
        &[]
    }

    /// The sections whose prose may hold no list marker.
    #[must_use]
    pub fn list_checked(&self) -> &'static [&'static str] {
        &[]
    }

    /// The form's instruction for `{{form}}`.
    #[must_use]
    pub fn instruction(&self, seed: &Seed) -> String {
        let _ = seed;
        String::new()
    }

    /// The engine-written frontmatter keys after the persona's, one per line.
    #[must_use]
    pub fn frontmatter_extra(&self, seed: &Seed) -> String {
        let _ = seed;
        String::new()
    }
}

/// The `corpus.json` the law gates resolve citations against.
#[must_use]
pub fn corpus_json(seed: &Seed, subject: &str) -> String {
    let _ = (seed, subject);
    String::new()
}
