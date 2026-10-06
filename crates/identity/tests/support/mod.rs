//! Test support for identity's linking and passkey tests (SPEC-359): a `tracing` capture that keeps
//! every event and span it is given, field by field, so a test can read both what was logged and
//! that nothing secret was; a software authenticator whose responses each test shapes (the origin,
//! user verification, the counter and the signing key); and the fixture every linking test builds
//! on, over a database migrated in a temporary directory and a manual clock.
//!
//! The authenticator builds its responses by hand: ES256 over P-256, and a minimal CBOR writer for
//! the registration's `none` attestation object and the COSE key, so a test can present a counter
//! it chooses and omit user verification.

// Each test binary uses a part of this module, and an integration test's helpers panic on a failed
// fixture.
#![allow(dead_code, clippy::expect_used)]

use std::fmt::{self, Write as _};
use std::sync::{Arc, Mutex, PoisonError};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use deck_streak_identity::{LinkingConfig, Owner, Passkeys, Sessions};
use deck_streak_kernel::{Db, ManualClock, TelegramUserId, UtcMillis};
use p256::ecdsa::signature::Signer as _;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// One captured event or span: where it came from, and each field as it was recorded.
#[derive(Clone, Debug)]
pub struct Line {
    /// The event's level, or `span` for a span's fields.
    pub level: String,
    /// The module path the event or span was emitted from.
    pub target: String,
    /// Each field's name and its recorded text, in the order they were recorded.
    pub fields: Vec<(String, String)>,
}

impl Line {
    /// The recorded text of the field `name`, if the line carries it.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.as_str())
    }

    /// The whole line as one text: its level, its target and every field, for a search that must
    /// not miss a value wherever it was recorded.
    #[must_use]
    pub fn rendered(&self) -> String {
        let mut text = format!("{} {}", self.level, self.target);
        for (name, value) in &self.fields {
            let _ = write!(text, " {name}={value}");
        }
        text
    }
}

/// A subscriber that keeps every event and span it is given.
#[derive(Clone, Default)]
pub struct Captured(Arc<Mutex<Vec<Line>>>);

impl Captured {
    /// Every line captured so far.
    #[must_use]
    pub fn lines(&self) -> Vec<Line> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn push(&self, line: Line) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// The fields of one event or span, as they are recorded.
#[derive(Default)]
struct Fields(Vec<(String, String)>);

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_owned(), value.to_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.push((field.name().to_owned(), format!("{value:?}")));
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut fields = Fields::default();
        span.record(&mut fields);
        self.push(Line {
            level: "span".to_owned(),
            target: span.metadata().target().to_owned(),
            fields: fields.0,
        });
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, values: &Record<'_>) {
        let mut fields = Fields::default();
        values.record(&mut fields);
        self.push(Line {
            level: "span".to_owned(),
            target: String::new(),
            fields: fields.0,
        });
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.push(Line {
            level: metadata.level().to_string(),
            target: metadata.target().to_owned(),
            fields: fields.0,
        });
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

/// The synthetic owner, a user id of fewer than seven digits.
pub const OWNER: i64 = 4242;
/// Another Telegram user, never the owner.
pub const STRANGER: i64 = 4343;
/// 2025-01-15T03:30:10Z, in milliseconds.
pub const STARTED_AT: i64 = 1_736_911_810_000;
/// The configured origin: a reserved name.
pub const ORIGIN: &str = "https://app.example";
/// Its relying party id.
pub const RP_ID: &str = "app.example";
/// Another origin: a reserved name.
pub const ELSEWHERE: &str = "https://elsewhere.example";

/// The authenticator data's flags (`WebAuthn` 6.1).
pub const USER_PRESENT: u8 = 0x01;
/// User verified.
pub const USER_VERIFIED: u8 = 0x04;
/// Attested credential data included.
pub const ATTESTED: u8 = 0x40;

/// `bytes` as base64url without padding.
#[must_use]
pub fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// The bytes base64url `text` spells.
#[must_use]
pub fn unb64(text: &str) -> Vec<u8> {
    URL_SAFE_NO_PAD
        .decode(text.trim_end_matches('='))
        .expect("base64url text")
}

/// A minimal CBOR writer: the major types the attestation object and the COSE key need.
#[derive(Default)]
pub struct Cbor(Vec<u8>);

impl Cbor {
    fn head(&mut self, major: u8, value: u64) {
        let major = major << 5;
        if value < 24 {
            self.0
                .push(major | u8::try_from(value).expect("a small value"));
        } else if let Ok(byte) = u8::try_from(value) {
            self.0.extend([major | 0x18, byte]);
        } else if let Ok(short) = u16::try_from(value) {
            self.0.push(major | 0x19);
            self.0.extend(short.to_be_bytes());
        } else {
            self.0.push(major | 0x1a);
            self.0
                .extend(u32::try_from(value).expect("a 32-bit value").to_be_bytes());
        }
    }

    /// An integer.
    pub fn int(&mut self, value: i64) -> &mut Self {
        if value >= 0 {
            self.head(0, value.unsigned_abs());
        } else {
            self.head(1, value.unsigned_abs() - 1);
        }
        self
    }

    /// A byte string.
    pub fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.head(2, value.len() as u64);
        self.0.extend_from_slice(value);
        self
    }

    /// A text string.
    pub fn text(&mut self, value: &str) -> &mut Self {
        self.head(3, value.len() as u64);
        self.0.extend_from_slice(value.as_bytes());
        self
    }

    /// A map of `entries` pairs; the caller writes each key, then its value.
    pub fn map(&mut self, entries: u64) -> &mut Self {
        self.head(5, entries);
        self
    }

    /// The encoding.
    #[must_use]
    pub fn done(&self) -> Vec<u8> {
        self.0.clone()
    }
}

/// What a response presents: its origin, its relying party, its flags and its counter, and whether
/// it is signed by a key the credential does not hold.
#[derive(Clone, Debug)]
pub struct Presented {
    /// The origin the client data names.
    pub origin: String,
    /// The relying party id whose SHA-256 the authenticator data carries.
    pub rp_id: String,
    /// The authenticator data's flags.
    pub flags: u8,
    /// The authenticator data's signature counter.
    pub counter: u32,
    /// Whether an assertion is signed by another key.
    pub foreign_signature: bool,
}

impl Default for Presented {
    fn default() -> Self {
        Self {
            origin: ORIGIN.to_owned(),
            rp_id: RP_ID.to_owned(),
            flags: USER_PRESENT | USER_VERIFIED,
            counter: 0,
            foreign_signature: false,
        }
    }
}

impl Presented {
    /// The default response with the counter `counter`.
    #[must_use]
    pub fn counter(counter: u32) -> Self {
        Self {
            counter,
            ..Self::default()
        }
    }
}

/// A software authenticator holding one ES256 credential.
pub struct Authenticator {
    key: SigningKey,
    credential_id: Vec<u8>,
}

impl Authenticator {
    /// The authenticator whose key and credential id derive from `seed`.
    #[must_use]
    pub fn new(seed: u8) -> Self {
        Self {
            key: SigningKey::from_slice(&[seed; 32]).expect("a valid P-256 scalar"),
            credential_id: vec![seed; 16],
        }
    }

    /// The credential id.
    #[must_use]
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    /// The COSE encoding of the credential's public key.
    #[must_use]
    pub fn cose_key(&self) -> Vec<u8> {
        let point = self.key.verifying_key().to_encoded_point(false);
        Cbor::default()
            .map(5)
            .int(1)
            .int(2)
            .int(3)
            .int(-7)
            .int(-1)
            .int(1)
            .int(-2)
            .bytes(point.x().expect("an x coordinate"))
            .int(-3)
            .bytes(point.y().expect("a y coordinate"))
            .done()
    }

    /// The client data JSON of a ceremony of `kind` over the challenge `options` carries.
    fn client_data(kind: &str, options: &Value, presented: &Presented) -> Vec<u8> {
        let challenge = options["publicKey"]["challenge"]
            .as_str()
            .expect("the options carry a challenge");
        json!({
            "type": kind,
            "challenge": challenge,
            "origin": presented.origin,
            "crossOrigin": false,
        })
        .to_string()
        .into_bytes()
    }

    /// The authenticator data: the relying party's SHA-256, the flags and the counter.
    fn authenticator_data(presented: &Presented) -> Vec<u8> {
        let mut data = Sha256::digest(presented.rp_id.as_bytes()).to_vec();
        data.push(presented.flags);
        data.extend(presented.counter.to_be_bytes());
        data
    }

    /// The registration response to `options`, as a browser posts it.
    #[must_use]
    pub fn register(&self, options: &Value, presented: &Presented) -> Value {
        let client_data = Self::client_data("webauthn.create", options, presented);
        let mut data = Self::authenticator_data(&Presented {
            flags: presented.flags | ATTESTED,
            ..presented.clone()
        });
        data.extend([0_u8; 16]);
        data.extend(
            u16::try_from(self.credential_id.len())
                .expect("a short id")
                .to_be_bytes(),
        );
        data.extend(&self.credential_id);
        data.extend(self.cose_key());
        let attestation = Cbor::default()
            .map(3)
            .text("fmt")
            .text("none")
            .text("attStmt")
            .map(0)
            .text("authData")
            .bytes(&data)
            .done();
        json!({
            "id": b64(&self.credential_id),
            "rawId": b64(&self.credential_id),
            "type": "public-key",
            "response": {
                "attestationObject": b64(&attestation),
                "clientDataJSON": b64(&client_data),
            },
            "extensions": {},
        })
    }

    /// The assertion response to `options`, as a browser posts it.
    #[must_use]
    pub fn assert(&self, options: &Value, presented: &Presented) -> Value {
        let client_data = Self::client_data("webauthn.get", options, presented);
        let data = Self::authenticator_data(presented);
        let mut signed = data.clone();
        signed.extend(Sha256::digest(&client_data));
        let key = if presented.foreign_signature {
            SigningKey::from_slice(&[0x77; 32]).expect("a valid P-256 scalar")
        } else {
            self.key.clone()
        };
        let signature: Signature = key.sign(&signed);
        json!({
            "id": b64(&self.credential_id),
            "rawId": b64(&self.credential_id),
            "type": "public-key",
            "response": {
                "authenticatorData": b64(&data),
                "clientDataJSON": b64(&client_data),
                "signature": b64(signature.to_der().as_bytes()),
                "userHandle": null,
            },
            "extensions": {},
        })
    }
}

/// The world a linking test runs in: a migrated database, a manual clock, the owner's sessions and
/// the linking use cases over the configured origin.
pub struct Fixture {
    /// The database.
    pub db: Db,
    /// The clock.
    pub clock: Arc<ManualClock>,
    /// The owner's sessions.
    pub sessions: Sessions,
    /// The use cases.
    pub passkeys: Passkeys,
    /// The owner.
    pub owner: Owner,
    _dir: tempfile::TempDir,
}

/// The fixture, linking on at [`ORIGIN`].
pub async fn fixture() -> Fixture {
    fixture_with(LinkingConfig::from_setting(Some(ORIGIN)).expect("the origin is an https origin"))
        .await
}

/// The fixture under `config`.
pub async fn fixture_with(config: LinkingConfig) -> Fixture {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck-streak.db"))
        .await
        .expect("the database opens and migrates");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let sessions = Sessions::new(clock.clone());
    let owner = Owner::new(TelegramUserId::new(OWNER));
    let passkeys = Passkeys::new(config, sessions.clone(), owner, clock.clone());
    Fixture {
        db,
        clock,
        sessions,
        passkeys,
        owner,
        _dir: dir,
    }
}

impl Fixture {
    /// A NEW `link` session.
    #[must_use]
    pub fn link_session(&self) -> String {
        self.sessions
            .open_link(self.owner)
            .expect("a link session opens")
            .expose()
            .to_owned()
    }

    /// Registers `authenticator`'s credential inside `session`; its row id.
    pub async fn register(&self, session: &str, authenticator: &Authenticator) -> i64 {
        let started = self
            .passkeys
            .start_registration(&self.db, session)
            .await
            .expect("a registration starts");
        let response = authenticator.register(&started.options, &Presented::default());
        self.passkeys
            .finish_registration(&self.db, session, started.flow.expose(), &response)
            .await
            .expect("the registration finishes")
    }

    /// Inserts a `passkeys` row for `user` holding `authenticator`'s credential at `counter`,
    /// with the same serialized shape a registration writes; its row id.
    pub async fn seed(&self, user: i64, authenticator: &Authenticator, counter: u32) -> i64 {
        let session = self.link_session();
        let row = self.register(&session, authenticator).await;
        let mut transaction = self.db.write().await.expect("a write");
        sqlx::query(
            "UPDATE passkeys SET telegram_user_id = ?1, counter = ?2, \
             credential = json_set(credential, '$.cred.counter', ?2) WHERE id = ?3",
        )
        .bind(user)
        .bind(i64::from(counter))
        .bind(row)
        .execute(&mut *transaction)
        .await
        .expect("the seed's update");
        transaction.commit().await.expect("the seed commits");
        row
    }

    /// The row `row`'s counter, backup state and last use.
    pub async fn row(&self, row: i64) -> Option<(i64, i64, Option<i64>)> {
        sqlx::query_as::<_, (i64, i64, Option<i64>)>(
            "SELECT counter, backup_state, last_used_at FROM passkeys WHERE id = ?1",
        )
        .bind(row)
        .fetch_optional(self.db.reader())
        .await
        .expect("a read")
    }

    /// How many `passkeys` rows exist.
    pub async fn rows(&self) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM passkeys")
            .fetch_one(self.db.reader())
            .await
            .expect("a count")
    }
}
