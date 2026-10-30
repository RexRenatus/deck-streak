//! A media file's identity: the SHA-256 of its bytes (SPEC-513 R1; ADR-531 D1).
//!
//! The store knows a file by [`MediaHash`] alone. Its lowercase hex form is what a certified name
//! carries ([`crate::name`]), and its blob path is where a store keeps the bytes: two hex digits,
//! a separator and the whole hex form, so the path names its hash and nothing else.

use std::fmt;

use sha2::{Digest, Sha256};

/// The length of a hash's lowercase hex form.
pub const HEX_LEN: usize = 64;

/// The length of a blob path: two hex digits, a separator, then the whole hex form.
pub const BLOB_PATH_LEN: usize = 67;

/// The lowercase hex digits, in order of their value.
const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The SHA-256 of a media file's bytes: the one identity the media store knows a file by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaHash([u8; 32]);

impl MediaHash {
    /// The SHA-256 of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The hash whose 32 bytes are `bytes`.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The hash's 32 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The hash's lowercase hex form, as ASCII bytes.
    #[must_use]
    pub fn hex(&self) -> [u8; HEX_LEN] {
        let mut out = [0; HEX_LEN];
        for (index, byte) in self.0.iter().enumerate() {
            out[2 * index] = DIGITS[usize::from(byte >> 4)];
            out[2 * index + 1] = DIGITS[usize::from(byte & 0x0f)];
        }
        out
    }

    /// The hash's lowercase hex form.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.hex().iter().map(|&digit| char::from(digit)).collect()
    }

    /// The hash whose lowercase hex form is `text`: exactly 64 digits, `0`-`9` and `a`-`f`. Any
    /// other text, an uppercase digit included, is `None`.
    #[must_use]
    pub fn from_hex(text: &str) -> Option<Self> {
        Self::from_hex_bytes(text.as_bytes())
    }

    /// [`MediaHash::from_hex`] over ASCII bytes.
    fn from_hex_bytes(digits: &[u8]) -> Option<Self> {
        if digits.len() != HEX_LEN {
            return None;
        }
        let mut out = [0; 32];
        for (index, pair) in digits.as_chunks::<2>().0.iter().enumerate() {
            out[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
        }
        Some(Self(out))
    }

    /// Where a store keeps the bytes: the first two hex digits, `/`, then the whole hex form.
    #[must_use]
    pub fn blob_path(&self) -> [u8; BLOB_PATH_LEN] {
        let hex = self.hex();
        let mut path = [0; BLOB_PATH_LEN];
        #[cfg(not(feature = "kani-witness-blob-path"))]
        let fan = [hex[0], hex[1]];
        #[cfg(feature = "kani-witness-blob-path")]
        let fan = [hex[1], hex[0]];
        path[..2].copy_from_slice(&fan);
        path[2] = b'/';
        path[3..].copy_from_slice(&hex);
        path
    }

    /// The hash a blob path names: its fan-out must be its hex form's first two digits, so a path
    /// is read back to exactly one hash and every other path is `None`.
    #[must_use]
    pub fn from_blob_path(path: &[u8]) -> Option<Self> {
        if path.len() != BLOB_PATH_LEN || path[2] != b'/' || path[..2] != path[3..5] {
            return None;
        }
        Self::from_hex_bytes(&path[3..])
    }
}

impl fmt::Display for MediaHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// The value of one lowercase hex digit.
fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}
