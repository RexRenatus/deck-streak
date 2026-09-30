//! The form a reading takes and the word target it is written to (SPEC-046 R3, R4).

use std::fmt::Write as _;

use crate::seed::{Seed, Track};

/// The band's floor, in primer words: the `reading-length` gate's lower bound.
pub const BAND_FLOOR_WORDS: u32 = 800;
/// The band's ceiling, in primer words: the `reading-length` gate's upper bound.
pub const BAND_CEILING_WORDS: u32 = 1500;
/// The words a new card adds to the target beyond the first.
pub const WORDS_PER_NEW_CARD: u32 = 50;

/// The prose target for a topic with `new_cards` new cards.
#[must_use]
pub fn word_target(new_cards: u32) -> u32 {
    let extra = new_cards
        .saturating_sub(1)
        .saturating_mul(WORDS_PER_NEW_CARD);
    BAND_FLOOR_WORDS
        .saturating_add(extra)
        .min(BAND_CEILING_WORDS)
}

/// The law form's sections.
const LAW_SECTIONS: &[&str] = &[
    "reading",
    "issue",
    "rule",
    "application",
    "conclusion",
    "retrieval",
];
/// The language form's sections.
const LANGUAGE_SECTIONS: &[&str] = &[
    "reading",
    "glosses",
    "grammar",
    "pronunciation",
    "culture",
    "retrieval",
];
/// The law sections that are prose.
const LAW_PROSE: &[&str] = &["reading", "issue", "rule", "application", "conclusion"];
/// The language sections that are prose: the glosses and the retrieval are lists by their packs'
/// contracts, and the others are not held to the marker check.
const LANGUAGE_PROSE: &[&str] = &["reading"];

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
        match self.track {
            Track::Law => LAW_SECTIONS,
            Track::Language => LANGUAGE_SECTIONS,
        }
    }

    /// The sections whose prose may hold no list marker.
    #[must_use]
    pub fn list_checked(&self) -> &'static [&'static str] {
        match self.track {
            Track::Law => LAW_PROSE,
            Track::Language => LANGUAGE_PROSE,
        }
    }

    /// The form's instruction for `{{form}}`.
    #[must_use]
    pub fn instruction(&self, seed: &Seed) -> String {
        let mut text = String::from(
            "Write the reading as Markdown with these sections, in this order, each opened by a \
             heading of the form `## Title <!-- section:NAME -->`:\n",
        );
        for name in self.sections() {
            let _ = writeln!(text, "- `<!-- section:{name} -->`");
        }
        match self.track {
            Track::Law => {
                text.push_str(
                    "Cite each of these notes at least once as `[@key]`, and cite nothing else. \
                     Quote each note's opening words in the prose:\n",
                );
                for key in seed.source_keys() {
                    let _ = writeln!(text, "- `[@{key}]`");
                }
            }
            Track::Language => {
                text.push_str(
                    "Gloss each of these new words in the glosses section and use it in the \
                     reading:\n",
                );
                for word in &seed.new_words {
                    let _ = writeln!(text, "- {word}");
                }
            }
        }
        text.push_str(
            "Write the prose of every section except the retrieval and the glosses as \
             paragraphs: no line begins with a bullet or a number.",
        );
        text
    }

    /// The engine-written frontmatter keys after the persona's, one per line.
    #[must_use]
    pub fn frontmatter_extra(&self, seed: &Seed) -> String {
        match self.track {
            Track::Law => format!(
                "sources: {}\nx-new-cards: {}\n",
                json_list(&seed.source_keys()),
                seed.new_cards()
            ),
            Track::Language => format!("x-new-words: {}\n", json_list(&seed.new_words)),
        }
    }
}

/// The `corpus.json` the law gates resolve citations against.
#[must_use]
pub fn corpus_json(seed: &Seed, subject: &str) -> String {
    let sources: Vec<serde_json::Value> = seed
        .notes
        .iter()
        .map(|note| {
            serde_json::json!({
                "id": Seed::key_of(note.id),
                "title": Seed::key_of(note.id),
                "text": note.text,
            })
        })
        .collect();
    let corpus = serde_json::json!({
        "schema": "phx.law.corpus.v1",
        "subject": subject,
        "sources": sources,
    });
    serde_json::to_string_pretty(&corpus).unwrap_or_default()
}

/// A JSON list of strings, on one line.
fn json_list(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|item| serde_json::to_string(item).unwrap_or_else(|_| "\"\"".to_owned()))
        .collect();
    format!("[{}]", quoted.join(", "))
}
