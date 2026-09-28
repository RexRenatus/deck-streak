//! The one validator of Telegram's launch data: the `WebAppData` HMAC over the sorted check string,
//! a bounded `auth_date`, and a malformed payload refused (SPEC-024 A1 to A5, R1, R2).
//!
//! Payloads are signed here the way Telegram signs them, through RustCrypto's own
//! `new_from_slice`, never through anything the crate does, so each test knows exactly what it
//! changed. A1 and A5 also read payloads whose hashes Python's standard `hmac` and `hashlib`
//! computed, so the construction itself (which key signs which message) is pinned by an oracle
//! that cannot share a mistake with the crate. The bot tokens never have the Bot API token's
//! shape, and the user ids have fewer than seven digits (R11).

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::time::Duration;

use axum::http::StatusCode;
use deck_streak_identity::settings::{FUTURE_SKEW, INIT_DATA_MAX_AGE};
use deck_streak_identity::{Freshness, Refusal, WebAppKey, validate};
use deck_streak_kernel::{Environment, SettingsError, TelegramUserId, UtcMillis};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// The synthetic bot token the payloads are signed for.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// Another synthetic bot's token.
const OTHER_BOT_TOKEN: &str = "another-synthetic-signing-token";
/// The synthetic user the payloads name.
const USER: i64 = 4242;
/// The `auth_date` of the Python payloads, 2025-01-15T12:00:00Z, in seconds.
const SIGNED_AT: i64 = 1_736_942_400;
/// A payload Python signed for [`BOT_TOKEN`]: the key is
/// `hmac.new(b"WebAppData", token, sha256).digest()`, and the hash
/// `hmac.new(key, check_string, sha256).hexdigest()` over `auth_date`, `query_id` and `user`.
const PYTHON_PAYLOAD: &str = concat!(
    "auth_date=1736942400&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=306d6a4fa3169695522041c18683db5d6111ef948ba2d36a9dd2eeb566c257e0",
);
/// The same fields and a `signature`, signed by Python with the signature in the check string.
const PYTHON_SIGNATURE_PAYLOAD: &str = concat!(
    "auth_date=1736942400&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&signature=c3ludGhldGljLXNpZ25hdHVyZQ",
    "&hash=9ad2f708082378cb419f56cbe90a5d718775ae05e8a22bfd2054bee73d234c72",
);
/// The hash Python computed over the same fields with the signature LEFT OUT of the check string,
/// as only the third-party Ed25519 check builds it: the hash of [`PYTHON_PAYLOAD`].
const HASH_WITHOUT_SIGNATURE: &str =
    "306d6a4fa3169695522041c18683db5d6111ef948ba2d36a9dd2eeb566c257e0";

type HmacSha256 = Hmac<Sha256>;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The instant `seconds` after the epoch.
fn at(seconds: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(seconds * 1000)
}

/// The fields of a launch for `user`, dated `auth_date`.
fn launch(user: i64, auth_date: i64) -> Vec<(String, String)> {
    vec![
        ("auth_date".to_owned(), auth_date.to_string()),
        ("query_id".to_owned(), "synthetic-query".to_owned()),
        (
            "user".to_owned(),
            format!("{{\"id\":{user},\"first_name\":\"Ada\",\"username\":\"synthetic_owner\"}}"),
        ),
    ]
}

/// The check string of `fields`: each `key=value`, sorted by key, joined by a line feed.
fn check_string(fields: &[(String, String)]) -> String {
    let mut sorted = fields.to_vec();
    sorted.sort();
    sorted
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The hash Telegram gives `fields` for the bot `token`: the check string's HMAC-SHA-256 under the
/// HMAC-SHA-256 of the token keyed with `WebAppData`.
fn sign(token: &str, fields: &[(String, String)]) -> String {
    let mut secret = HmacSha256::new_from_slice(b"WebAppData").expect("HMAC takes any key");
    secret.update(token.as_bytes());
    let key = secret.finalize().into_bytes();
    let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC takes any key");
    mac.update(check_string(fields).as_bytes());
    hex(&mac.finalize().into_bytes())
}

/// A hash made the naive way: the check string's HMAC keyed with the bot token itself.
fn sign_with_the_token_itself(token: &str, fields: &[(String, String)]) -> String {
    let mut mac = HmacSha256::new_from_slice(token.as_bytes()).expect("HMAC takes any key");
    mac.update(check_string(fields).as_bytes());
    hex(&mac.finalize().into_bytes())
}

/// `text` percent-encoded as a form encodes it: every byte but the unreserved ones escaped.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The raw launch data of `fields` and `hash`, as Telegram hands it to the Mini App.
fn payload(fields: &[(String, String)], hash: &str) -> String {
    fields
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name), encode(value)))
        .chain([format!("hash={hash}")])
        .collect::<Vec<_>>()
        .join("&")
}

/// `fields` with the field `name` set to `value`.
fn with(fields: &[(String, String)], name: &str, value: &str) -> Vec<(String, String)> {
    fields
        .iter()
        .map(|(field, old)| {
            let kept = if field == name { value } else { old };
            (field.clone(), kept.to_owned())
        })
        .collect()
}

#[test]
fn a_payload_signed_with_the_webappdata_key_is_accepted() {
    let key = WebAppKey::from_bot_token(BOT_TOKEN);
    let now = at(SIGNED_AT + 10);

    // The payload Python signed is accepted as the user it names.
    let caller = validate(PYTHON_PAYLOAD, &key, Freshness::default(), now)
        .expect("the payload Python signed with the WebAppData key is accepted");
    assert_eq!(caller.user(), TelegramUserId::new(USER));

    // So is one signed here the same way for another user, as that user.
    let fields = launch(777, SIGNED_AT);
    let signed = payload(&fields, &sign(BOT_TOKEN, &fields));
    assert_eq!(
        validate(&signed, &key, Freshness::default(), now).map(|caller| caller.user()),
        Ok(TelegramUserId::new(777))
    );

    // Keyed with the bot token itself instead of its WebAppData key, the same fields are refused.
    let naive = payload(&fields, &sign_with_the_token_itself(BOT_TOKEN, &fields));
    assert_eq!(
        validate(&naive, &key, Freshness::default(), now),
        Err(Refusal::InitDataInvalid)
    );
}

#[test]
fn a_tampered_field_is_refused_with_401() {
    let key = WebAppKey::from_bot_token(BOT_TOKEN);
    let now = at(SIGNED_AT + 10);
    let fields = launch(USER, SIGNED_AT);
    let hash = sign(BOT_TOKEN, &fields);
    assert!(
        validate(&payload(&fields, &hash), &key, Freshness::default(), now).is_ok(),
        "the payload as signed is accepted"
    );

    // One field changed after signing, the hash kept.
    let changes = [
        (
            "user",
            "{\"id\":4243,\"first_name\":\"Ada\",\"username\":\"synthetic_owner\"}",
        ),
        ("auth_date", "1736942401"),
        ("query_id", "synthetic-query-2"),
    ];
    for (name, value) in examined("field change(s)", changes.to_vec()) {
        let tampered = payload(&with(&fields, name, value), &hash);
        let refusal = validate(&tampered, &key, Freshness::default(), now)
            .expect_err("a field changed after signing is refused");
        assert_eq!(refusal, Refusal::InitDataInvalid, "{name}");
        assert_eq!(refusal.status(), StatusCode::UNAUTHORIZED, "{name}");
        assert_eq!(refusal.reason(), "init_data_invalid", "{name}");
    }

    // So is the hash with one digit changed.
    let mut digits = hash.clone().into_bytes();
    digits[0] = if digits[0] == b'0' { b'1' } else { b'0' };
    let changed = String::from_utf8(digits).expect("hex is ASCII");
    assert_eq!(
        validate(&payload(&fields, &changed), &key, Freshness::default(), now),
        Err(Refusal::InitDataInvalid)
    );
}

#[test]
fn a_payload_signed_with_another_bot_token_is_refused_with_401() {
    let key = WebAppKey::from_bot_token(BOT_TOKEN);
    let now = at(SIGNED_AT + 10);
    let fields = launch(USER, SIGNED_AT);
    let ours = payload(&fields, &sign(BOT_TOKEN, &fields));
    assert!(
        validate(&ours, &key, Freshness::default(), now).is_ok(),
        "our bot's payload is accepted"
    );

    let theirs = payload(&fields, &sign(OTHER_BOT_TOKEN, &fields));
    let refusal = validate(&theirs, &key, Freshness::default(), now)
        .expect_err("another bot's payload is refused");
    assert_eq!(refusal, Refusal::InitDataInvalid);
    assert_eq!(refusal.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn a_stale_or_future_auth_date_is_refused_with_401() {
    let key = WebAppKey::from_bot_token(BOT_TOKEN);
    let now_seconds = SIGNED_AT + 7200;
    let now = at(now_seconds);
    let hour = 3600;
    let skew = i64::try_from(FUTURE_SKEW.as_secs()).expect("seconds");
    // Validly signed launches dated relative to now, and whether each is fresh by the default bound
    // of an hour and the 60-second skew.
    let dates = [
        (now_seconds - hour - 1, false),
        (now_seconds - hour, true),
        (now_seconds, true),
        (now_seconds + skew, true),
        (now_seconds + skew + 1, false),
        (now_seconds + 86_400, false),
    ];
    for (auth_date, fresh) in examined("launch date(s)", dates.to_vec()) {
        let fields = launch(USER, auth_date);
        let signed = payload(&fields, &sign(BOT_TOKEN, &fields));
        let outcome = validate(&signed, &key, Freshness::default(), now);
        if fresh {
            assert!(outcome.is_ok(), "{auth_date}: refused: {outcome:?}");
        } else {
            let refusal = outcome.expect_err("a date outside the window is refused");
            assert_eq!(refusal, Refusal::InitDataStale, "{auth_date}");
            assert_eq!(refusal.status(), StatusCode::UNAUTHORIZED, "{auth_date}");
            assert_eq!(refusal.reason(), "init_data_stale", "{auth_date}");
        }
    }

    // A shorter bound is the bound: two minutes admits a launch two minutes old, and no older.
    let two_minutes = Freshness::new(Duration::from_secs(120)).expect("a bound in range");
    for (age, fresh) in [(120, true), (121, false)] {
        let fields = launch(USER, now_seconds - age);
        let signed = payload(&fields, &sign(BOT_TOKEN, &fields));
        let outcome = validate(&signed, &key, two_minutes, now);
        assert_eq!(outcome.is_ok(), fresh, "{age} s old: {outcome:?}");
    }
}

#[test]
fn a_payload_with_a_signature_field_validates_with_it_in_the_check_string() {
    let key = WebAppKey::from_bot_token(BOT_TOKEN);
    let now = at(SIGNED_AT + 10);

    // A payload carrying a signature whose hash left the signature out of the check string, as
    // only the third-party Ed25519 check builds it, is refused: the signature is a field like any
    // other here.
    let without = PYTHON_SIGNATURE_PAYLOAD
        .rsplit_once("&hash=")
        .map(|(fields, _)| format!("{fields}&hash={HASH_WITHOUT_SIGNATURE}"))
        .expect("the payload carries a hash");
    assert_eq!(
        validate(&without, &key, Freshness::default(), now),
        Err(Refusal::InitDataInvalid)
    );

    // Python's payload, whose hash covers the signature, is accepted as the user it names.
    let caller = validate(PYTHON_SIGNATURE_PAYLOAD, &key, Freshness::default(), now)
        .expect("a hash over the check string with its signature is accepted");
    assert_eq!(caller.user(), TelegramUserId::new(USER));

    // And one signed here with a signature field, the same way.
    let mut fields = launch(USER, SIGNED_AT);
    fields.push((
        "signature".to_owned(),
        "c3ludGhldGljLXNpZ25hdHVyZQ".to_owned(),
    ));
    let signed = payload(&fields, &sign(BOT_TOKEN, &fields));
    assert!(validate(&signed, &key, Freshness::default(), now).is_ok());
}

/// `fields` signed, then written out with every `from` in the raw text replaced by `to`: launch
/// data whose hash covers what a lenient decoder would read from it.
fn signed_then_rewritten(fields: &[(String, String)], from: &str, to: &str) -> String {
    let raw = payload(fields, &sign(BOT_TOKEN, fields));
    assert!(raw.contains(from), "{from} is not in {raw}");
    raw.replace(from, to)
}

#[test]
fn a_malformed_payload_is_refused_with_401() {
    let key = WebAppKey::from_bot_token(BOT_TOKEN);
    let now = at(SIGNED_AT + 10);
    let fields = launch(USER, SIGNED_AT);
    let hash = sign(BOT_TOKEN, &fields);
    let good = payload(&fields, &hash);
    assert!(
        validate(&good, &key, Freshness::default(), now).is_ok(),
        "the well-formed payload is accepted"
    );

    // From the repeated key down, each hash is valid for what a lenient decoder reads, so only the
    // strict reading, or a check after the signature, refuses it.
    let mut stray = fields.clone();
    stray.push(("stray".to_owned(), String::new()));
    let mut truncated = fields.clone();
    truncated.push(("tail".to_owned(), "%4".to_owned()));
    let no_user: Vec<(String, String)> = fields
        .iter()
        .filter(|(name, _)| name != "user")
        .cloned()
        .collect();
    let not_an_integer = with(&fields, "auth_date", "soon");
    let malformed = [
        ("empty", String::new()),
        ("no hash", good.replace(&format!("&hash={hash}"), "")),
        ("a hash that is not hex", payload(&fields, &"zz".repeat(32))),
        ("a short hash", payload(&fields, &hash[..62])),
        ("a repeated key", format!("{good}&query_id=synthetic-query")),
        ("an empty segment", good.replacen('&', "&&", 1)),
        (
            "a segment with no =",
            signed_then_rewritten(&stray, "&stray=&", "&stray&"),
        ),
        (
            "a malformed escape",
            signed_then_rewritten(
                &with(&fields, "query_id", "synthetic%zzquery"),
                "synthetic%25zzquery",
                "synthetic%zzquery",
            ),
        ),
        (
            "a truncated escape",
            signed_then_rewritten(&truncated, "tail=%254", "tail=%4"),
        ),
        (
            "bytes that are not UTF-8",
            signed_then_rewritten(
                &with(&fields, "query_id", "synthetic\u{fffd}query"),
                "%EF%BF%BD",
                "%FF",
            ),
        ),
        ("no user", payload(&no_user, &sign(BOT_TOKEN, &no_user))),
        (
            "an auth_date that is not an integer",
            payload(&not_an_integer, &sign(BOT_TOKEN, &not_an_integer)),
        ),
    ];
    for (what, raw) in examined("malformed payload(s)", malformed.to_vec()) {
        assert_eq!(
            validate(&raw, &key, Freshness::default(), now),
            Err(Refusal::InitDataInvalid),
            "{what}"
        );
    }
}

#[test]
fn the_freshness_bound_is_read_from_its_setting() {
    let read =
        |value: &str| Freshness::from_env(&Environment::from_vars([(INIT_DATA_MAX_AGE, value)]));
    assert_eq!(
        read("120").map(Freshness::max_age),
        Ok(Duration::from_secs(120))
    );
    assert_eq!(
        Freshness::from_env(&Environment::default()).map(Freshness::max_age),
        Ok(Duration::from_secs(3600))
    );
    for malformed in examined("malformed bound(s)", vec!["0", "86401", "an hour", "-5"]) {
        assert_eq!(
            read(malformed),
            Err(SettingsError::Malformed {
                setting: INIT_DATA_MAX_AGE,
                expected: "a whole number of seconds from 1 to 86400",
            }),
            "{malformed}"
        );
    }
}
