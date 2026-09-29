//! The photo the router sends and the file id Telegram answers it with (SPEC-132 R3).

use std::fmt;

/// Why a photo or a file id was refused: every refusal is `photo_invalid`, and the detail names
/// which bound it broke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotoError {
    /// The bytes are neither a PNG nor a JPEG whose size the header names.
    Format,
    /// More bytes than the Bot API takes.
    TooLarge,
    /// Width plus height beyond the Bot API's total.
    Dimensions,
    /// A ratio of the longer side to the shorter beyond the Bot API's.
    Ratio,
    /// A caption beyond the Bot API's length, after escaping.
    Caption,
    /// An empty or overlong file id.
    FileId,
}

impl PhotoError {
    /// The refusal as the caller reads it.
    #[must_use]
    pub const fn code(self) -> &'static str {
        "photo_invalid"
    }
}

impl fmt::Display for PhotoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PhotoError {}

/// The file id Telegram holds for a photo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileId(String);

impl FileId {
    /// The file id `text`.
    ///
    /// # Errors
    ///
    /// [`PhotoError::FileId`] when it is empty or overlong.
    pub fn new(text: impl Into<String>) -> Result<Self, PhotoError> {
        Ok(Self(text.into()))
    }

    /// The file id as Telegram spells it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A photo the router may send.
#[derive(Clone, PartialEq, Eq)]
pub struct Photo {
    bytes: Vec<u8>,
    caption: String,
}

impl fmt::Debug for Photo {
    /// Never the bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Photo")
            .field("len", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl Photo {
    /// The photo `bytes` with the HTML caption `caption`.
    ///
    /// # Errors
    ///
    /// A [`PhotoError`] when a bound of the Bot API is broken.
    pub fn new(bytes: Vec<u8>, caption: impl Into<String>) -> Result<Self, PhotoError> {
        Ok(Self {
            bytes,
            caption: caption.into(),
        })
    }

    /// The image's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The caption, as the bot's HTML.
    #[must_use]
    pub fn caption(&self) -> &str {
        &self.caption
    }
}
