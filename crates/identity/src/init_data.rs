//! The one validator of Telegram's Mini App launch data (SPEC-024 R1, R2; ADR-006).
//!
//! Telegram's rule (core.telegram.org/bots/webapps, "Validating data received via the Mini App"):
//! the data-check-string is every received field but `hash`, sorted by key, written `key=value`
//! and joined by a line feed; the secret key is the HMAC-SHA-256 of the bot token keyed with the
//! constant `WebAppData`; and `hash` is the hex HMAC-SHA-256 of the data-check-string under that
//! key. The `signature` field stays in the string: only Telegram's third-party Ed25519 check leaves
//! it out. Checking `auth_date` is left to the service; here it must be inside the freshness window
//! ([`Freshness`]) on the kernel's clock, and it is read only after the signature holds.
//!
//! The launch data is read strictly, by one decoder: a segment with no `=`, an empty segment, a
//! repeated key, an escape that is not `%` and two hex digits, or bytes that are not UTF-8 refuse
//! it. A lenient decoder would read each of them as something, and a check string built from what
//! it read could still match the hash; one decoder is also one reading, so no second parse can
//! disagree with the one the hash was checked against (ADR-024).
//!
//! The hash is compared through `Mac::verify_slice` alone, which compares in constant time, and no
//! other comparison of hash bytes exists in this crate: A6's census reads the source for one. Of
//! the user object only the id is kept.

use std::collections::BTreeMap;
use std::fmt;

use deck_streak_kernel::{TelegramUserId, UtcMillis};
use hmac::digest::Key;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::Refusal;
use crate::settings::Freshness;

/// The constant Telegram keys the bot token's HMAC with to make the Mini App's secret key.
const WEB_APP_DATA: &[u8; 10] = b"WebAppData";
/// The field that carries the hex HMAC: the one field the check string leaves out.
const HASH: &str = "hash";
/// The field that carries the launch's Unix time, in seconds.
const AUTH_DATE: &str = "auth_date";
/// The field that carries Telegram's user object, as JSON.
const USER: &str = "user";

type HmacSha256 = Hmac<Sha256>;

/// The Mini App's secret key: the HMAC-SHA-256 of the bot token keyed with `WebAppData`. It is
/// derived once, at start, and the bot token itself is not kept. Its `Debug` never shows it.
#[derive(Clone)]
pub struct WebAppKey([u8; 32]);

impl WebAppKey {
    /// The secret key of `bot_token`.
    #[must_use]
    pub fn from_bot_token(bot_token: &str) -> Self {
        let mut mac = keyed(WEB_APP_DATA);
        mac.update(bot_token.as_bytes());
        Self(mac.finalize().into_bytes().into())
    }
}

impl fmt::Debug for WebAppKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WebAppKey(..)")
    }
}

/// Who validly signed launch data names: of Telegram's user object, only the id is kept (R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller {
    user: TelegramUserId,
}

impl Caller {
    /// The caller's Telegram user id.
    #[must_use]
    pub const fn user(self) -> TelegramUserId {
        self.user
    }
}

/// The caller the raw launch data `raw` names, when it is signed with `key`, and fresh by
/// `freshness` at `now`.
///
/// # Errors
///
/// [`Refusal::InitDataInvalid`] when `raw` is missing a field, repeats a key, has a field that does
/// not decode, carries no hash or a forged one, or has no integer `auth_date` or no `user.id`; and
/// [`Refusal::InitDataStale`] when it is validly signed and its `auth_date` is older than the
/// bound, or further ahead of `now` than the skew allows.
pub fn validate(
    raw: &str,
    key: &WebAppKey,
    freshness: Freshness,
    now: UtcMillis,
) -> Result<Caller, Refusal> {
    let mut fields = fields(raw).ok_or(Refusal::InitDataInvalid)?;
    let hex_hash = fields.remove(HASH).ok_or(Refusal::InitDataInvalid)?;
    let hash = hex_bytes(&hex_hash).ok_or(Refusal::InitDataInvalid)?;
    let mut mac = keyed(&key.0);
    mac.update(check_string(&fields).as_bytes());
    mac.verify_slice(&hash)
        .map_err(|_| Refusal::InitDataInvalid)?;

    // The signature holds: only now is anything the launch data says believed.
    let auth_date = fields
        .get(AUTH_DATE)
        .and_then(|text| seconds(text))
        .ok_or(Refusal::InitDataInvalid)?;
    fresh(auth_date, freshness, now)?;
    let user = fields
        .get(USER)
        .and_then(|json| user_id(json))
        .ok_or(Refusal::InitDataInvalid)?;
    Ok(Caller { user })
}

/// An HMAC-SHA-256 keyed with `key`, which is at most one SHA-256 block (64 bytes) long, as both
/// keys here are: `WebAppData`'s 10 bytes and the secret key's 32. RFC 2104 pads a key shorter than
/// the block with zeros, so the zero-padded block IS the key, and `KeyInit::new` takes it without
/// the `Result` of `new_from_slice`, which HMAC never refuses anyway.
fn keyed<const N: usize>(key: &[u8; N]) -> HmacSha256 {
    const {
        assert!(
            N <= 64,
            "an HMAC-SHA-256 key longer than a block is hashed first"
        );
    }
    let mut block = Key::<HmacSha256>::default();
    block[..N].copy_from_slice(key);
    HmacSha256::new(&block)
}

/// The launch data's fields, each name and value decoded, or `None` when a segment is empty, has
/// no `=`, does not decode, or repeats a name.
fn fields(raw: &str) -> Option<BTreeMap<String, String>> {
    let mut fields = BTreeMap::new();
    for segment in raw.split('&') {
        let (name, value) = segment.split_once('=')?;
        if fields.insert(decode(name)?, decode(value)?).is_some() {
            return None;
        }
    }
    Some(fields)
}

/// `text` decoded as `application/x-www-form-urlencoded` writes it: `+` is a space, and `%` begins
/// an escape of exactly two hex digits. `None` when an escape is malformed or the bytes it spells
/// are not UTF-8.
fn decode(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        bytes.push(match byte {
            b'+' => b' ',
            b'%' => (hex_digit(rest.next()?)? << 4) | hex_digit(rest.next()?)?,
            other => other,
        });
    }
    String::from_utf8(bytes).ok()
}

/// The value of one hex digit, in either case.
fn hex_digit(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// The bytes the hex digits of `text` spell, two digits to a byte, or `None` when it is not hex.
fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    let pairs = text.as_bytes().chunks_exact(2);
    if !pairs.remainder().is_empty() {
        return None;
    }
    pairs
        .map(|pair| Some((hex_digit(pair[0])? << 4) | hex_digit(pair[1])?))
        .collect()
}

/// The data-check-string: every field, sorted by name (the map's order), written `name=value`, one
/// to a line.
fn check_string(fields: &BTreeMap<String, String>) -> String {
    fields
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whole seconds since the epoch, as `auth_date` writes them: decimal digits and nothing else.
fn seconds(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Whether launch data dated `auth_date` is inside the freshness window at `now`: no older than
/// the bound, and no further ahead than the skew allows.
fn fresh(auth_date: i64, freshness: Freshness, now: UtcMillis) -> Result<(), Refusal> {
    let issued = auth_date.checked_mul(1000).ok_or(Refusal::InitDataStale)?;
    let age = now.epoch_millis().saturating_sub(issued);
    let max_age = millis(freshness.max_age());
    let skew = millis(freshness.future_skew());
    if age > max_age || age < -skew {
        return Err(Refusal::InitDataStale);
    }
    Ok(())
}

/// A bound in whole milliseconds; the settings keep every bound far below `i64::MAX`.
fn millis(bound: std::time::Duration) -> i64 {
    i64::try_from(bound.as_millis()).unwrap_or(i64::MAX)
}

/// Telegram's user object, of which only the id is read.
#[derive(Deserialize)]
struct User {
    id: i64,
}

/// The id of the user object `json`, or `None` when it is not one.
fn user_id(json: &str) -> Option<TelegramUserId> {
    serde_json::from_str::<User>(json)
        .ok()
        .map(|user| TelegramUserId::new(user.id))
}
