//! The push service fake's own decryption of an `aes128gcm` body, written from RFC 8291 sections
//! 3.3 and 3.4 and RFC 8188 section 2, over hkdf, aes-gcm and p256's ECDH. It never calls the
//! library the sender encrypts with, so a key-schedule mistake shared by an encryption and its own
//! decryption cannot pass (ADR-354 D5).

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use hkdf::Hkdf;
use p256::{PublicKey, SecretKey};
use sha2::Sha256;

/// The header RFC 8188 puts before the ciphertext: a 16-byte salt, a 4-byte record size and a
/// 1-byte key id length.
const HEADER: usize = 21;
/// RFC 8291's key id: the application server's uncompressed P-256 public key.
const KEY_ID: usize = 65;
/// RFC 8188's delimiter of the last record.
const LAST_RECORD: u8 = 2;

/// Why a body did not decrypt.
#[derive(Debug, PartialEq, Eq)]
pub enum Undecrypted {
    /// The header is short, or its key id is not a 65-byte P-256 point.
    Header,
    /// The record size names a body other than the one record this body is.
    RecordSize,
    /// The authenticated decryption failed.
    Tag,
    /// The plaintext ends in no last-record delimiter.
    Padding,
}

/// The plaintext `body` holds, decrypted with the browser's private key `secret` and its
/// authentication secret `auth`.
pub fn decrypt(body: &[u8], secret: &SecretKey, auth: &[u8; 16]) -> Result<Vec<u8>, Undecrypted> {
    if body.len() < HEADER + KEY_ID {
        return Err(Undecrypted::Header);
    }
    let salt = &body[..16];
    let record_size = u32::from_be_bytes([body[16], body[17], body[18], body[19]]) as usize;
    if usize::from(body[20]) != KEY_ID {
        return Err(Undecrypted::Header);
    }
    let server_public_bytes = &body[HEADER..HEADER + KEY_ID];
    let ciphertext = &body[HEADER + KEY_ID..];
    if ciphertext.len() > record_size || ciphertext.len() < 17 {
        return Err(Undecrypted::RecordSize);
    }
    let server_public =
        PublicKey::from_sec1_bytes(server_public_bytes).map_err(|_| Undecrypted::Header)?;
    let browser_public = secret.public_key().to_sec1_bytes();

    // RFC 8291 section 3.3: the shared secret and the input keying material.
    let shared = p256::ecdh::diffie_hellman(secret.to_nonzero_scalar(), server_public.as_affine());
    let mut key_info = b"WebPush: info\0".to_vec();
    key_info.extend_from_slice(&browser_public);
    key_info.extend_from_slice(server_public_bytes);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth), shared.raw_secret_bytes().as_ref())
        .expand(&key_info, &mut ikm)
        .map_err(|_| Undecrypted::Header)?;

    // RFC 8188 section 2.2 and 2.3: the content-encryption key and the nonce, from the salt.
    let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let mut cek = [0u8; 16];
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .map_err(|_| Undecrypted::Header)?;
    let mut nonce = [0u8; 12];
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce)
        .map_err(|_| Undecrypted::Header)?;

    let cipher = Aes128Gcm::new_from_slice(&cek).map_err(|_| Undecrypted::Header)?;
    let padded = cipher
        .decrypt(&Nonce::from(nonce), ciphertext)
        .map_err(|_| Undecrypted::Tag)?;

    // RFC 8188 section 2: the last record's plaintext is followed by 2, then any zero padding.
    let end = padded
        .iter()
        .rposition(|byte| *byte != 0)
        .ok_or(Undecrypted::Padding)?;
    if padded[end] != LAST_RECORD {
        return Err(Undecrypted::Padding);
    }
    Ok(padded[..end].to_vec())
}
