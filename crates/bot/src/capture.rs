//! The owner's media for the vault inbox (SPEC-118 R6 to R9, #154): which file of a message is
//! captured, under which extension and caption, whether its declared size may be fetched, and the
//! line the owner gets back.
//!
//! The choice is the predecessor's (`bot.py:CommandBot._maybe_capture_media`; golden
//! `media_capture_choice`): the first present of a photo, a voice note and a document. A photo is
//! its last size, as `.jpg`; a voice note is `.ogg`, stored as it came; a document keeps its name's
//! extension only where R7 allows one, and its caption defaults to its file name. The replies are
//! the predecessor's (`bot.py:CommandBot._capture_file`; golden `media_capture_replies`), the saved
//! file's name escaped as the predecessor escaped it.

use deck_streak_coordination::inbox_capture::CaptureKind;
use frankenstein::types::Message;

/// The Bot API's download limit (R8): a file declared over 20 × 1024 × 1024 bytes is never
/// fetched, and a stream that passes that count is stopped.
pub const MAX_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;

/// One file of the owner's message, as the capture takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The Bot API's file id, which `getFile` takes; empty when Telegram sent none.
    pub file_id: String,
    /// What it is.
    pub kind: CaptureKind,
    /// The extension the attachment is stored with, its dot included.
    pub ext: String,
    /// The caption the stub carries.
    pub caption: String,
    /// Telegram's unique id of the file, which may be empty.
    pub file_unique_id: String,
    /// The size Telegram declared, when it declared one.
    pub size: Option<u64>,
}

impl Choice {
    /// Whether the media is ignored without a word (R6): Telegram sent no file id.
    #[must_use]
    pub fn silent(&self) -> bool {
        self.file_id.is_empty()
    }

    /// The capture's unique, its retry key: Telegram's unique id of the file, or its file id when
    /// Telegram sent none (the predecessor's fallback).
    #[must_use]
    pub fn unique(&self) -> &str {
        if self.file_unique_id.is_empty() {
            &self.file_id
        } else {
            &self.file_unique_id
        }
    }
}

/// The file of `message` the capture takes, or `None` when it carries no photo, voice note or
/// document.
#[must_use]
pub fn choose(message: &Message) -> Option<Choice> {
    let caption = message.caption.clone().unwrap_or_default();
    if let Some(photo) = message.photo.as_ref().and_then(|sizes| sizes.last()) {
        return Some(Choice {
            file_id: photo.file_id.clone(),
            kind: CaptureKind::Photo,
            ext: ".jpg".to_owned(),
            caption,
            file_unique_id: photo.file_unique_id.clone(),
            size: photo.file_size,
        });
    }
    if let Some(voice) = &message.voice {
        return Some(Choice {
            file_id: voice.file_id.clone(),
            kind: CaptureKind::Voice,
            ext: ".ogg".to_owned(),
            caption,
            file_unique_id: voice.file_unique_id.clone(),
            size: voice.file_size,
        });
    }
    let document = message.document.as_ref()?;
    let name = document.file_name.clone().unwrap_or_default();
    Some(Choice {
        file_id: document.file_id.clone(),
        kind: CaptureKind::Document,
        ext: extension(&name),
        caption: if caption.is_empty() { name } else { caption },
        file_unique_id: document.file_unique_id.clone(),
        size: document.file_size,
    })
}

/// The extension a document named `name` is stored with (R7): the text after its last dot when it
/// is 1 to 10 ASCII letters and digits, its dot included; `.bin` for any other.
fn extension(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((_, ext))
            if (1..=10).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_alphanumeric()) =>
        {
            format!(".{ext}")
        }
        _ => ".bin".to_owned(),
    }
}

/// Whether a file of the declared `size` may be fetched (R8): at most [`MAX_DOWNLOAD_BYTES`], or
/// no size declared, since the stream's own count still stops it.
#[must_use]
pub fn may_fetch(size: Option<u64>) -> bool {
    size.is_none_or(|size| size <= MAX_DOWNLOAD_BYTES)
}

/// What became of a capture the owner sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The file was not fetched from Telegram: its size, its `getFile` or its download failed.
    NotFetched,
    /// The capture is in the inbox as `name`, or was already, as `name`.
    Saved {
        /// The file the owner looks for.
        name: String,
    },
    /// The vault refused the save, a missing vault among the reasons.
    NotSaved,
}

/// The line the owner gets for `outcome` (R9), as HTML.
#[must_use]
pub fn reply(outcome: &Outcome) -> String {
    match outcome {
        Outcome::NotFetched => {
            "⚠️ Couldn't fetch that file from Telegram — nothing saved.".to_owned()
        }
        Outcome::Saved { name } => format!(
            "📥 Saved to Inbox as <code>{}</code> — the nightly pass will file it.",
            escape(name)
        ),
        Outcome::NotSaved => "⚠️ Couldn't save that capture to the vault inbox.".to_owned(),
    }
}

/// `text` escaped as the predecessor escaped a saved file's name (Python's `html.escape`): `&`,
/// `<`, `>`, `"` and `'`. The bot's own escape takes only the first three.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            c => escaped.push(c),
        }
    }
    escaped
}
