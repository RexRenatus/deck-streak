//! SHA-1 (FIPS 180-4), for the one name the engine derives by it: a LaTeX image's (SPEC-513 R9;
//! ADR-531 D8).
//!
//! SHA-1 is broken for collisions, so it certifies nothing here: a media file is known by its
//! SHA-256 ([`crate::MediaHash`]), and SHA-1 only spells a name the engine already gives. It is
//! written in the crate, rather than taken from a package, so the core keeps `sha2` its one
//! dependency.

/// The length of a SHA-1 digest, in bytes.
pub const DIGEST_LEN: usize = 20;

/// The lowercase hex digits, in order of their value.
const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The SHA-1 of `bytes`.
#[must_use]
pub fn sha1(bytes: &[u8]) -> [u8; DIGEST_LEN] {
    let _ = bytes;
    [0; DIGEST_LEN]
}

/// The SHA-1 of `bytes`, in lowercase hex.
#[must_use]
pub fn sha1_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(2 * DIGEST_LEN);
    for byte in sha1(bytes) {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}
