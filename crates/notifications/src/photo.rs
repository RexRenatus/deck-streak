//! The photo the router sends and the file id Telegram answers it with (SPEC-132 R3).
//!
//! A photo is refused before any call when the Bot API would refuse it: more bytes, more pixels
//! or a longer side than it takes, or a caption past its length. The sizes are read from the
//! image's own header, by hand, because the port takes no image crate for a check of five numbers.

use std::fmt;

/// The most bytes the Bot API takes for a photo.
pub const MAX_BYTES: usize = 10_000_000;
/// The most the width and the height of a photo may add up to, in pixels.
pub const MAX_DIMENSION_SUM: u64 = 10_000;
/// The largest ratio of a photo's longer side to its shorter.
pub const MAX_RATIO: u64 = 20;
/// The longest caption, in UTF-16 units as written: Telegram counts a caption after entity parsing
/// in UTF-16 units, so a character outside the Basic Multilingual Plane counts two, and a tag or an
/// entity counted as written only over-counts (the bot's `MAX_CAPTION_UTF16`).
pub const MAX_CAPTION_UTF16: usize = 1_024;
/// The most characters a file id holds.
pub const MAX_FILE_ID_CHARS: usize = 512;

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
        let text = text.into();
        if text.is_empty() || text.chars().count() > MAX_FILE_ID_CHARS {
            return Err(PhotoError::FileId);
        }
        Ok(Self(text))
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
        if bytes.len() > MAX_BYTES {
            return Err(PhotoError::TooLarge);
        }
        let (width, height) = size_of(&bytes).ok_or(PhotoError::Format)?;
        let (width, height) = (u64::from(width), u64::from(height));
        if width == 0 || height == 0 || width + height > MAX_DIMENSION_SUM {
            return Err(PhotoError::Dimensions);
        }
        if width.max(height) > MAX_RATIO * width.min(height) {
            return Err(PhotoError::Ratio);
        }
        let caption = caption.into();
        if caption.encode_utf16().count() > MAX_CAPTION_UTF16 {
            return Err(PhotoError::Caption);
        }
        Ok(Self { bytes, caption })
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

/// The width and height the header of a PNG or a JPEG names, or `None` for anything else.
fn size_of(bytes: &[u8]) -> Option<(u32, u32)> {
    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.starts_with(&PNG) {
        let header = bytes.get(12..24)?;
        if &header[..4] != b"IHDR" {
            return None;
        }
        let width = u32::from_be_bytes(header[4..8].try_into().ok()?);
        let height = u32::from_be_bytes(header[8..12].try_into().ok()?);
        return Some((width, height));
    }
    if bytes.starts_with(&[0xff, 0xd8]) {
        return jpeg_size(bytes);
    }
    None
}

/// The size a JPEG's first frame header names: the segments are walked by their lengths until a
/// start-of-frame marker, which is any of `0xc0..=0xcf` but the table, the reserved and the
/// arithmetic-conditioning markers.
fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    loop {
        if *bytes.get(at)? != 0xff {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        if marker == 0xff {
            at += 1;
            continue;
        }
        if marker == 0xd8 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            at += 2;
            continue;
        }
        let length = usize::from(u16::from_be_bytes([
            *bytes.get(at + 2)?,
            *bytes.get(at + 3)?,
        ]));
        if (0xc0..=0xcf).contains(&marker) && ![0xc4, 0xc8, 0xcc].contains(&marker) {
            let frame = bytes.get(at + 5..at + 9)?;
            let height = u32::from(u16::from_be_bytes([frame[0], frame[1]]));
            let width = u32::from(u16::from_be_bytes([frame[2], frame[3]]));
            return Some((width, height));
        }
        if length < 2 {
            return None;
        }
        at += 2 + length;
    }
}
