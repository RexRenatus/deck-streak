//! The sealing key the API releases to the owner's session, so the web client can keep its sync key
//! sealed at rest (SPEC-363 R5; ADR-374 D2).
//!
//! The service holds one seal secret, read once at start. A browser names a seal id of 16 random
//! bytes it chose, and the sealing key for it is the HMAC-SHA-256 under the secret over
//! [`SEAL_LABEL`] then the id. The service stores nothing per browser: the same secret and id give
//! the same key again, after a restart included. No `Debug` here prints a byte of the secret, an id
//! or a key.

use std::fmt;

use hmac::{Hmac, KeyInit, Mac};
use serde_json::Value;
use sha2::Sha256;
use webauthn_rs::prelude::Base64UrlSafeData;

use crate::passkeys::base64url;

/// The fixed label the sealing key's HMAC reads before the seal id, so a key made under the seal
/// secret for any other purpose can never equal a sealing key.
pub const SEAL_LABEL: &[u8] = b"deck-streak sync seal v1";
/// The fewest bytes a seal secret may have: one HMAC-SHA-256 output's worth.
pub const MIN_SECRET_BYTES: usize = 32;
/// A seal id's length in bytes.
pub const SEAL_ID_BYTES: usize = 16;

/// Why a seal secret refuses start. It never carries a byte of the secret.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// The secret is shorter than [`MIN_SECRET_BYTES`].
    #[error("the seal secret is shorter than {MIN_SECRET_BYTES} bytes")]
    TooShort,
}

type HmacSha256 = Hmac<Sha256>;

/// The service's seal secret, held only as the HMAC keyed with it. Its `Debug` prints
/// `SealSecret(..)`.
#[derive(Clone)]
pub struct SealSecret(HmacSha256);

impl SealSecret {
    /// The seal secret `bytes`.
    ///
    /// # Errors
    ///
    /// [`SealError::TooShort`] when `bytes` holds fewer than [`MIN_SECRET_BYTES`].
    pub fn new(bytes: &[u8]) -> Result<Self, SealError> {
        if bytes.len() < MIN_SECRET_BYTES {
            return Err(SealError::TooShort);
        }
        // RFC 2104 takes a key of any length, hashing one longer than a block, so HMAC refuses no
        // key here; the one refusal its keying could give is a length, and the length is this one.
        HmacSha256::new_from_slice(bytes)
            .map(Self)
            .map_err(|_| SealError::TooShort)
    }

    /// The sealing key for `id`: the HMAC-SHA-256 under the seal secret over [`SEAL_LABEL`] then
    /// the id's 16 bytes.
    #[must_use]
    pub fn key_for(&self, id: &SealId) -> SealKey {
        let mut mac = self.0.clone();
        mac.update(SEAL_LABEL);
        mac.update(&id.0);
        SealKey(mac.finalize().into_bytes().into())
    }
}

impl fmt::Debug for SealSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SealSecret(..)")
    }
}

/// A browser's seal id: [`SEAL_ID_BYTES`] bytes. Its `Debug` prints `SealId(..)`.
#[derive(Clone, PartialEq, Eq)]
pub struct SealId([u8; SEAL_ID_BYTES]);

impl SealId {
    /// The seal id `text` spells, or `None` unless it is exactly 22 unpadded base64url characters
    /// that decode to [`SEAL_ID_BYTES`] bytes.
    ///
    /// The decoder webauthn-rs carries also reads padding and the standard alphabet, so the id is
    /// written back and must spell `text` again: only the one canonical spelling of 16 bytes, 22
    /// characters whose last carries no bit past the 128th, parses.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let decoded = serde_json::from_value::<Base64UrlSafeData>(Value::String(text.to_owned()))
            .ok()
            .map(Vec::from)?;
        if decoded.len() == SEAL_ID_BYTES {
            let mut id = [0_u8; SEAL_ID_BYTES];
            id.copy_from_slice(&decoded);
            (base64url(&id) == text).then_some(Self(id))
        } else {
            None
        }
    }
}

impl fmt::Debug for SealId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SealId(..)")
    }
}

/// A sealing key: 32 bytes. Its `Debug` prints `SealKey(..)`.
pub struct SealKey([u8; 32]);

impl SealKey {
    /// The key as 43 unpadded base64url characters, the one form it leaves the service in.
    #[must_use]
    pub fn encoded(&self) -> String {
        base64url(&self.0)
    }
}

impl fmt::Debug for SealKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SealKey(..)")
    }
}
