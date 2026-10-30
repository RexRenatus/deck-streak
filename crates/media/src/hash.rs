//! A media file's identity: the SHA-256 of its bytes (SPEC-513 R1; ADR-531 D1). STUB: every
//! function keeps its shape and answers wrongly, so the crate's tests are seen red first.

use std::fmt;

/// The length of a hash's lowercase hex form.
pub const HEX_LEN: usize = 64;

/// The length of a blob path: two hex digits, a separator, then the whole hex form.
pub const BLOB_PATH_LEN: usize = 67;

/// The SHA-256 of a media file's bytes: the one identity the media store knows a file by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaHash([u8; 32]);

impl MediaHash {
    /// The SHA-256 of `bytes`. STUB: all zeros.
    #[must_use]
    pub fn of(_bytes: &[u8]) -> Self {
        Self([0; 32])
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

    /// The hash's lowercase hex form, as ASCII bytes. STUB: all zero digits.
    #[must_use]
    pub fn hex(&self) -> [u8; HEX_LEN] {
        [b'0'; HEX_LEN]
    }

    /// The hash's lowercase hex form.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.hex().iter().map(|&digit| char::from(digit)).collect()
    }

    /// The hash whose lowercase hex form is `text`. STUB: none.
    #[must_use]
    pub fn from_hex(_text: &str) -> Option<Self> {
        None
    }

    /// Where a store keeps the bytes. STUB: all zeros.
    #[must_use]
    pub fn blob_path(&self) -> [u8; BLOB_PATH_LEN] {
        [0; BLOB_PATH_LEN]
    }

    /// The hash a blob path names. STUB: none.
    #[must_use]
    pub fn from_blob_path(_path: &[u8]) -> Option<Self> {
        None
    }
}

impl fmt::Display for MediaHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}
