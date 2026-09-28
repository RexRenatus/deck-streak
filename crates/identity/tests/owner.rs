//! The owner pin: valid launch data for anyone else is refused with 403, nothing of the launch data
//! reaches the log, and a missing credential refuses start by its id (SPEC-024 A7, A8, A16; R3, R4).
//!
//! Payloads are signed here the way Telegram signs them, with a synthetic token that never has the
//! Bot API token's shape, for user ids of fewer than seven digits (R11).

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fmt::{self, Write as _};
use std::fs;
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::StatusCode;
use deck_streak_identity::owner::{OWNER_USER_ID, TELEGRAM_BOT_TOKEN};
use deck_streak_identity::{Freshness, IdentityError, Owner, OwnerGate, Refusal, WebAppKey};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Redactor, TelegramUserId, UtcMillis,
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// The synthetic bot token the payloads are signed for.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// Another synthetic bot's token, for a forged payload.
const OTHER_BOT_TOKEN: &str = "another-synthetic-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// A synthetic user who is not the owner.
const STRANGER: i64 = 777;
/// When the payloads were signed, in seconds: 2025-01-15T12:00:00Z.
const SIGNED_AT: i64 = 1_736_942_400;

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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The hash Telegram gives `fields` for the bot `token`.
fn sign(token: &str, fields: &[(String, String)]) -> String {
    let mut sorted = fields.to_vec();
    sorted.sort();
    let check_string = sorted
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut secret = HmacSha256::new_from_slice(b"WebAppData").expect("HMAC takes any key");
    secret.update(token.as_bytes());
    let key = secret.finalize().into_bytes();
    let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC takes any key");
    mac.update(check_string.as_bytes());
    hex(&mac.finalize().into_bytes())
}

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

/// The raw launch data of `fields` and `hash`.
fn payload(fields: &[(String, String)], hash: &str) -> String {
    fields
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name), encode(value)))
        .chain([format!("hash={hash}")])
        .collect::<Vec<_>>()
        .join("&")
}

/// The gate for the synthetic owner and bot.
fn gate() -> OwnerGate {
    OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    )
}

/// A subscriber that keeps every event and span it is given, each as one line of its level,
/// target, name and fields, each field's text as recorded.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<String>>>);

impl Captured {
    fn lines(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn push(&self, line: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// A line of recorded fields.
struct Fields(String);

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        let _ = write!(self.0, " {}={value}", field.name());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let _ = write!(self.0, " {}={value:?}", field.name());
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut fields = Fields(format!("span {}", span.metadata().name()));
        span.record(&mut fields);
        self.push(fields.0);
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, values: &Record<'_>) {
        let mut fields = Fields("span record".to_owned());
        values.record(&mut fields);
        self.push(fields.0);
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let mut fields = Fields(format!("{} {}", metadata.level(), metadata.target()));
        event.record(&mut fields);
        self.push(fields.0);
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

/// A credentials directory holding `credentials`, each as systemd writes one: a file named by its
/// id, ending in a line feed.
fn credentials(credentials: &[(&str, &str)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for (id, value) in credentials {
        fs::write(directory.path().join(id), format!("{value}\n")).expect("a credential file");
    }
    directory
}

fn loader(directory: &tempfile::TempDir, redactor: &Redactor) -> CredentialLoader {
    let path = CredentialsDirectory::new(directory.path()).expect("an absolute path");
    CredentialLoader::new(path, redactor.clone())
}

#[test]
fn a_valid_payload_for_another_user_is_refused_with_403() {
    let gate = gate();
    let now = at(SIGNED_AT + 10);

    let theirs = launch(STRANGER, SIGNED_AT);
    let refusal = gate
        .admit(&payload(&theirs, &sign(BOT_TOKEN, &theirs)), now)
        .expect_err("valid, fresh launch data for another user is refused");
    assert_eq!(refusal, Refusal::NotOwner);
    assert_eq!(refusal.status(), StatusCode::FORBIDDEN);
    assert_eq!(refusal.reason(), "not_owner");

    let ours = launch(OWNER, SIGNED_AT);
    let owner = gate
        .admit(&payload(&ours, &sign(BOT_TOKEN, &ours)), now)
        .expect("the owner's launch data is admitted");
    assert_eq!(owner.user(), TelegramUserId::new(OWNER));
}

#[test]
fn init_data_never_reaches_the_log() {
    let gate = gate();
    let now = at(SIGNED_AT + 10);
    let ours = launch(OWNER, SIGNED_AT);
    let our_hash = sign(BOT_TOKEN, &ours);
    let theirs = launch(STRANGER, SIGNED_AT);
    let their_hash = sign(BOT_TOKEN, &theirs);
    let forged_hash = sign(OTHER_BOT_TOKEN, &ours);
    let admitted = payload(&ours, &our_hash);
    let stranger = payload(&theirs, &their_hash);
    let forged = payload(&ours, &forged_hash);

    let captured = Captured::default();
    let outcomes = tracing::subscriber::with_default(captured.clone(), || {
        [
            gate.admit(&admitted, now),
            gate.admit(&stranger, now),
            gate.admit(&forged, now),
        ]
    });
    let lines = examined("log line(s)", captured.lines());

    // Nothing of the launch data reached a line: the raw payloads, their hashes, the user object,
    // the query, and the bot's token.
    let user_object = &ours[2].1;
    let secrets = [
        admitted.as_str(),
        stranger.as_str(),
        forged.as_str(),
        our_hash.as_str(),
        their_hash.as_str(),
        forged_hash.as_str(),
        user_object.as_str(),
        "synthetic_owner",
        "synthetic-query",
        BOT_TOKEN,
    ];
    for line in &lines {
        for secret in secrets {
            assert!(
                !line.contains(secret),
                "the launch data reached the log: {line}"
            );
        }
    }

    // The gate logged each outcome, a refusal by its reason code.
    assert_eq!(
        outcomes.map(|outcome| outcome.map(Owner::user)),
        [
            Ok(TelegramUserId::new(OWNER)),
            Err(Refusal::NotOwner),
            Err(Refusal::InitDataInvalid),
        ]
    );
    for reason in ["not_owner", "init_data_invalid"] {
        assert!(
            lines.iter().any(|line| line.contains(reason)),
            "the refusal {reason} was not logged: {lines:#?}"
        );
    }
    assert!(
        lines.iter().any(|line| line.contains("admitted")),
        "the admission was not logged: {lines:#?}"
    );
}

#[test]
fn a_missing_owner_credential_refuses_start_by_its_id() {
    let owner_id = OWNER.to_string();
    // Each credential missing in turn refuses start, naming the missing credential's id.
    let missing = [
        (OWNER_USER_ID, vec![(TELEGRAM_BOT_TOKEN, BOT_TOKEN)]),
        (TELEGRAM_BOT_TOKEN, vec![(OWNER_USER_ID, owner_id.as_str())]),
    ];
    for (id, present) in examined("missing credential(s)", missing.to_vec()) {
        let directory = credentials(&present);
        let refusal = OwnerGate::load(&loader(&directory, &Redactor::new()), Freshness::default())
            .expect_err("a missing credential refuses start");
        let said = refusal.to_string();
        assert!(said.contains(id), "the refusal does not name {id}: {said}");
        assert!(
            matches!(refusal, IdentityError::Credential(_)),
            "{refusal:?}"
        );
    }

    // A malformed one refuses by its id too, and never shows its value.
    let malformed = [
        (OWNER_USER_ID, "the-owner", BOT_TOKEN),
        (OWNER_USER_ID, "-4242", BOT_TOKEN),
        (TELEGRAM_BOT_TOKEN, owner_id.as_str(), "   "),
    ];
    for (id, owner_value, token_value) in examined("malformed credential(s)", malformed.to_vec()) {
        let directory = credentials(&[
            (OWNER_USER_ID, owner_value),
            (TELEGRAM_BOT_TOKEN, token_value),
        ]);
        let refusal = OwnerGate::load(&loader(&directory, &Redactor::new()), Freshness::default())
            .expect_err("a malformed credential refuses start");
        let said = refusal.to_string();
        assert!(said.contains(id), "the refusal does not name {id}: {said}");
        let value = if id == OWNER_USER_ID {
            owner_value
        } else {
            token_value
        };
        assert!(
            !said.contains(value.trim()) || value.trim().is_empty(),
            "{said}"
        );
    }

    // Both present: the gate loads through the kernel's loader, which registered both values with
    // the redactor, and admits the owner's launch data signed for that bot.
    let directory = credentials(&[(OWNER_USER_ID, &owner_id), (TELEGRAM_BOT_TOKEN, BOT_TOKEN)]);
    let redactor = Redactor::new();
    let gate = OwnerGate::load(&loader(&directory, &redactor), Freshness::default())
        .expect("both credentials load");
    assert_eq!(redactor.registered(), 2);
    assert_eq!(gate.owner().user(), TelegramUserId::new(OWNER));
    let ours = launch(OWNER, SIGNED_AT);
    assert_eq!(
        gate.admit(&payload(&ours, &sign(BOT_TOKEN, &ours)), at(SIGNED_AT + 10))
            .map(Owner::user),
        Ok(TelegramUserId::new(OWNER))
    );
}
