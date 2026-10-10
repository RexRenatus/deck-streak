//! The native face (SPEC-348 R5; ADR-359 D1, D2).
//!
//! The core completes a card's face; this module reads its media from the opened collection's
//! media folder and wraps its text in one closed HTML page the card's isolated frame shows. The
//! page holds a viewport, the note type's CSS and the text, every media reference in it a `data:`
//! URL the core wrote or emptied. The shell names no script, no `base` element and no URL of any
//! other scheme; a card author's own markup stays the frame's to block.

use std::fs::File;
use std::io::Read as _;
use std::path::PathBuf;

use deck_streak_engine_core::face::{self, Face};
use deck_streak_engine_core::media::Reader;

/// The body's classes on a day face: the class every Anki card template styles.
const DAY: &str = "card";
/// The body's classes on a night face: Anki's two spellings of night mode beside the card's.
const NIGHT: &str = "card nightMode night_mode";
/// The line a withheld face's page holds in the card's place (SPEC-380 R4, R6): the web's `en` line,
/// since the native review shows English alone.
const WITHHELD: &str = "This image occlusion card cannot be shown here, because this app does not draw its masks. You can still bury or flag it.";

/// One thing a face plays, as a native client receives it.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum Clip {
    /// A sound file, by its name and bytes, which the client plays from memory.
    Sound {
        /// The file's name, as the card names it.
        name: String,
        /// The file's bytes.
        bytes: Vec<u8>,
    },
    /// Text the platform speaks.
    Speech {
        /// The text, as one plain line.
        text: String,
        /// The language, in the platform's spelling (`en-US`).
        language: String,
        /// The platform's rate, between its minimum and maximum.
        rate: f32,
    },
}

/// A card's face, as a native client shows and plays it.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CardFace {
    /// One closed HTML page: a viewport, the note type's CSS and the side's text.
    pub document: String,
    /// The clips to play when the face is shown.
    pub autoplay: Vec<Clip>,
    /// The clips a replay plays.
    pub replay: Vec<Clip>,
    /// The media names the face left out, each once.
    pub omitted: Vec<String>,
    /// Whether the card's question is an image occlusion question whose masks this app does not
    /// draw, so the page shows a line in the card's place (SPEC-380 R4).
    pub withheld: bool,
}

/// The media folder of the collection the engine opened, read at most a cap and a byte at a time;
/// the core's comparison decides what that length admits.
pub(crate) struct MediaFolder(pub(crate) Option<PathBuf>);

impl Reader for MediaFolder {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        let file = File::open(self.0.as_ref()?.join(name)).ok()?;
        let mut bytes = Vec::new();
        file.take(limit).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }
}

/// The page that holds `face`'s text, with the night classes only when `night` asks for them; a
/// withheld face's page holds the one line in the text's place, and its CSS is already empty.
fn document(face: &Face, night: bool) -> String {
    let classes = if night { NIGHT } else { DAY };
    let text = if face.withheld {
        WITHHELD
    } else {
        face.text.as_str()
    };
    format!(
        concat!(
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\">",
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
            "<style>{css}</style></head><body class=\"{classes}\">{text}</body></html>",
        ),
        css = face.css,
        classes = classes,
        text = text,
    )
}

fn clip(clip: face::Clip) -> Clip {
    match clip {
        face::Clip::Sound { name, bytes } => Clip::Sound { name, bytes },
        face::Clip::Speech {
            text,
            language,
            rate,
        } => Clip::Speech {
            text,
            language,
            rate,
        },
    }
}

impl CardFace {
    /// The native face of the core's `face`, its page lit for `night`.
    pub(crate) fn new(face: Face, night: bool) -> Self {
        let document = document(&face, night);
        Self {
            document,
            autoplay: face.autoplay.into_iter().map(clip).collect(),
            replay: face.replay.into_iter().map(clip).collect(),
            omitted: face.omitted,
            withheld: face.withheld,
        }
    }
}
