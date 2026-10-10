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

/// The body's classes after the template's on a day face: none.
const DAY: &str = "";
/// The body's classes after the template's on a night face: Anki's two spellings of night mode.
const NIGHT: &str = " nightMode night_mode";
/// The attribute each video start tag gains, so an iPhone plays the video inline (SPEC-393 R11).
const PLAYS_INLINE: &str = " playsinline";
/// The opening of a video start tag, its name matched ASCII-case-insensitively.
const VIDEO_OPEN: &str = "<video";

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
        /// The voices the tag asks for, in its order: a voice's identifier, or its name with each
        /// space written `_`, either after one prefix and `_`.
        voices: Vec<String>,
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

    fn inlines_fonts(&self) -> bool {
        true
    }
}

/// The page that holds `face`'s text. Its body's classes are `card card<n>`, `<n>` the card's
/// template counted from one, then the night classes only when `night` asks for them (SPEC-393
/// R2), and every video in the text plays inline.
fn document(face: &Face, night: bool) -> String {
    let classes = if night { NIGHT } else { DAY };
    let template = u64::from(face.ordinal) + 1;
    format!(
        concat!(
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\">",
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
            "<style>{css}</style></head>",
            "<body class=\"card card{template}{classes}\">{text}</body></html>",
        ),
        css = face.css,
        template = template,
        classes = classes,
        text = plays_inline(&face.text),
    )
}

/// `text` with ` playsinline` written after the element name of each `<video` start tag, the name
/// matched in any ASCII case and ending at whitespace, `/` or `>`; every other element, a
/// `<video-note>` included, stays as written (SPEC-393 R11).
fn plays_inline(text: &str) -> String {
    let mut written = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest
        .as_bytes()
        .windows(VIDEO_OPEN.len())
        .position(|window| window.eq_ignore_ascii_case(VIDEO_OPEN.as_bytes()))
    {
        let name_end = start + VIDEO_OPEN.len();
        written.push_str(&rest[..name_end]);
        rest = &rest[name_end..];
        if rest.starts_with(|next: char| next.is_ascii_whitespace() || next == '/' || next == '>') {
            written.push_str(PLAYS_INLINE);
        }
    }
    written.push_str(rest);
    written
}

fn clip(clip: face::Clip) -> Clip {
    match clip {
        face::Clip::Sound { name, bytes } => Clip::Sound { name, bytes },
        face::Clip::Speech {
            text,
            language,
            rate,
            voices,
        } => Clip::Speech {
            text,
            language,
            rate,
            voices,
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
        }
    }
}
