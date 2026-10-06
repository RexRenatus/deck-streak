//! Passkey ceremonies, the counter rule and the `passkeys` table (SPEC-359 R5 to R8, R11; ADR-370).
//!
//! A ceremony's state stays on the server, in memory: [`Ceremonies`] keeps at most
//! [`MAX_LIVE_CEREMONIES`], each under the SHA-256 of a 32-byte flow id that reaches the browser
//! only in the ceremony cookie, and [`Ceremonies::take`] removes an entry in the same critical
//! section that finds it, so a ceremony is used at most once. The state is never serialized and a
//! ceremony's `Debug` is `Ceremony { .. }`.
//!
//! The relying party is R1's one origin ([`crate::linking_config`]). A registration runs inside a
//! `link` session and inserts one row for the configured owner; a sign-in verifies an assertion
//! over the owner's passkeys and then advances the stored counter by ONE compare-and-swap
//! `UPDATE` on the value [`counter_advances`] read.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use deck_streak_kernel::{Clock, Db, KernelError, UtcMillis};
use serde_json::Value;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use webauthn_rs::prelude::{
    CredentialID, Passkey, PasskeyAuthentication, PasskeyRegistration, RegisterPublicKeyCredential,
    Uuid, WebauthnError,
};

use crate::Refusal;
use crate::linking::LinkCodes;
use crate::linking_config::{LinkingConfig, RelyingParty};
use crate::owner::Owner;
use crate::session::{Proof, SessionError, SessionToken, Sessions};

/// A ceremony is refused `challenge_expired` this long after it started (SPEC-359 R7).
pub const CEREMONY_LIFETIME: Duration = Duration::from_mins(5);
/// The most ceremonies kept at once; starting one more evicts the oldest (SPEC-359 R7).
pub const MAX_LIVE_CEREMONIES: usize = 8;
/// A flow id's length: 32 bytes from the operating system's generator (SPEC-359 R7).
pub const FLOW_ID_BYTES: usize = 32;
/// The passkey's user name and display name: fixed text that carries no personal data (R5).
pub const USER_NAME: &str = "DeckStreak";

/// Why a linking or passkey call did not complete.
#[derive(Debug, thiserror::Error)]
pub enum PasskeyError {
    /// The request was refused, by its reason code.
    #[error("the request was refused: {}", .0.reason())]
    Refused(Refusal),
    /// The `passkeys` table could not be read or written; the source says why.
    #[error("the passkeys table could not be read or written")]
    Store(#[from] KernelError),
    /// A session could not be opened; the source says why.
    #[error(transparent)]
    Session(#[from] SessionError),
    /// The operating system's random generator failed.
    #[error("the operating system's random generator failed")]
    Random,
}

impl PasskeyError {
    /// The refusal, when the call was refused.
    #[must_use]
    pub const fn refusal(&self) -> Option<Refusal> {
        match self {
            Self::Refused(refusal) => Some(*refusal),
            Self::Store(_) | Self::Session(_) | Self::Random => None,
        }
    }
}

impl From<Refusal> for PasskeyError {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
}

impl From<sqlx::Error> for PasskeyError {
    fn from(error: sqlx::Error) -> Self {
        Self::Store(KernelError::from(error))
    }
}

/// A flow id, as the browser holds it in the ceremony cookie: 64 hex digits. Its `Debug` never
/// shows it.
pub struct FlowId(String);

impl FlowId {
    /// The id, for the one cookie that carries it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for FlowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FlowId(..)")
    }
}

/// A started ceremony: its flow id, for the cookie, and the options the browser's call takes.
#[derive(Debug)]
pub struct Started {
    /// The flow id.
    pub flow: FlowId,
    /// The `publicKey` options, as the library answers them.
    pub options: Value,
}

/// A passkey sign-in that succeeded: the new `linked` session, and what the page may signal back
/// to the browser (SPEC-359 R6).
#[derive(Debug)]
pub struct SignedIn {
    /// The new session.
    pub session: SessionToken,
    /// The `passkeys` row that signed in.
    pub row: i64,
    /// The owner's accepted credential ids, base64url.
    pub credential_ids: Vec<String>,
    /// The owner's user handle, base64url.
    pub user_handle: String,
}

/// One of the owner's passkeys as a sign-in ceremony read it at its start.
#[derive(Clone)]
pub(crate) struct Held {
    pub(crate) row: i64,
    pub(crate) credential_id: Vec<u8>,
    pub(crate) user_handle: Vec<u8>,
    pub(crate) counter: u32,
    pub(crate) passkey: Passkey,
}

/// A ceremony's state, kept in memory and never serialized.
#[derive(Clone)]
pub(crate) enum State {
    /// A registration, with the SHA-256 of the `link` session that started it and its handle.
    Registration {
        state: PasskeyRegistration,
        session: Vec<u8>,
        user_handle: Uuid,
    },
    /// A sign-in, with the owner's passkeys as it read them.
    SignIn {
        state: PasskeyAuthentication,
        held: Vec<Held>,
    },
}

/// A live ceremony, as the store keeps it: the SHA-256 of its flow id, never the id.
#[derive(Clone)]
pub struct Ceremony {
    digest: Vec<u8>,
    began: UtcMillis,
    pub(crate) state: State,
}

impl fmt::Debug for Ceremony {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Ceremony { .. }")
    }
}

/// The live ceremonies, in memory, on the kernel's clock. A handle: every clone reads and writes
/// the same store.
#[derive(Clone)]
pub struct Ceremonies {
    live: Arc<Mutex<Vec<Ceremony>>>,
    clock: Arc<dyn Clock>,
}

impl Ceremonies {
    /// An empty store whose ceremonies age by `clock`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            live: Arc::default(),
            clock,
        }
    }

    /// Keeps `state` under a NEW flow id, evicting the oldest ceremony when
    /// [`MAX_LIVE_CEREMONIES`] are live, and returns the id: the only time it exists in the clear.
    fn insert(&self, state: State) -> Result<FlowId, PasskeyError> {
        let mut id = [0_u8; FLOW_ID_BYTES];
        getrandom::fill(&mut id).map_err(|_| PasskeyError::Random)?;
        let mut live = self.lock();
        // The store is in insertion order, so its first ceremony is the oldest.
        if live.len() >= MAX_LIVE_CEREMONIES {
            live.remove(0);
        }
        live.push(Ceremony {
            digest: digest(&id),
            began: self.clock.now(),
            state,
        });
        Ok(FlowId(hex(&id)))
    }

    /// Takes the ceremony `flow` names out of the store, in the critical section that finds it.
    ///
    /// # Errors
    ///
    /// [`Refusal::ChallengeInvalid`] when no live ceremony has that id, and
    /// [`Refusal::ChallengeExpired`] when it is [`CEREMONY_LIFETIME`] old.
    pub fn take(&self, flow: &str) -> Result<Ceremony, Refusal> {
        let presented = digest(&id_bytes(flow).ok_or(Refusal::ChallengeInvalid)?);
        let now = self.clock.now();
        let mut live = self.lock();
        let at = live
            .iter()
            .position(|ceremony| bool::from(ceremony.digest.ct_eq(&presented)))
            .ok_or(Refusal::ChallengeInvalid)?;
        let ceremony = live.remove(at);
        drop(live);
        let age = now
            .epoch_millis()
            .saturating_sub(ceremony.began.epoch_millis());
        if age >= millis(CEREMONY_LIFETIME) {
            return Err(Refusal::ChallengeExpired);
        }
        Ok(ceremony)
    }

    /// How many ceremonies are live.
    #[must_use]
    pub fn live(&self) -> usize {
        self.lock().len()
    }

    /// The store.
    fn lock(&self) -> MutexGuard<'_, Vec<Ceremony>> {
        self.live.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for Ceremonies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ceremonies")
            .field("live", &self.live())
            .finish_non_exhaustive()
    }
}

/// The counter rule (SPEC-359 R8, ADR-370 D3): an assertion is accepted when the presented and
/// stored counters are both zero, or the presented one is greater.
#[must_use]
pub const fn counter_advances(stored: u32, presented: u32) -> bool {
    (stored == 0 && presented == 0) || presented > stored
}

/// The refusal a library error answers (SPEC-359 R8).
const fn refusal_of(error: &WebauthnError) -> Refusal {
    match error {
        WebauthnError::InvalidRPOrigin
        | WebauthnError::CredentialCrossOrigin
        | WebauthnError::InvalidRPIDHash => Refusal::OriginMismatch,
        WebauthnError::UserNotVerified => Refusal::UvRequired,
        WebauthnError::CredentialPossibleCompromise => Refusal::CounterRegressed,
        WebauthnError::MismatchedChallenge | WebauthnError::ChallengeNotFound => {
            Refusal::ChallengeInvalid
        }
        _ => Refusal::PasskeyInvalid,
    }
}

/// A lifetime in whole milliseconds; every lifetime here is far below `i64::MAX`.
pub(crate) fn millis(lifetime: Duration) -> i64 {
    i64::try_from(lifetime.as_millis()).unwrap_or(i64::MAX)
}

/// The SHA-256 of `bytes`.
pub(crate) fn digest(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

/// `id` as lowercase hex digits.
fn hex(id: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    id.iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 0x0f)]),
            ]
        })
        .collect()
}

/// The bytes a flow id's 64 lowercase hex digits spell, or `None` for any other text.
fn id_bytes(flow: &str) -> Option<Vec<u8>> {
    let digits = flow.as_bytes();
    if digits.len() != FLOW_ID_BYTES * 2 {
        return None;
    }
    let value = |digit: u8| match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    };
    digits
        .chunks_exact(2)
        .map(|pair| Some((value(pair[0])? << 4) | value(pair[1])?))
        .collect()
}

/// `bytes` as base64url without padding.
pub(crate) fn base64url(bytes: &[u8]) -> String {
    match serde_json::to_value(webauthn_rs::prelude::Base64UrlSafeData::from(bytes)) {
        Ok(Value::String(text)) => text,
        _ => String::new(),
    }
}

/// The linking and passkey use cases over one relying party, the owner's sessions and the owner
/// (SPEC-359 R3 to R9). A handle: every clone shares the same stores.
#[derive(Clone)]
pub struct Passkeys {
    pub(crate) config: LinkingConfig,
    pub(crate) sessions: Sessions,
    pub(crate) owner: Owner,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) ceremonies: Ceremonies,
    pub(crate) codes: LinkCodes,
}

impl fmt::Debug for Passkeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Passkeys")
            .field("ceremonies", &self.ceremonies)
            .finish_non_exhaustive()
    }
}

impl Passkeys {
    /// The use cases for `owner`, over `sessions`, as `config` turns linking on or off.
    #[must_use]
    pub fn new(
        config: LinkingConfig,
        sessions: Sessions,
        owner: Owner,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            config,
            sessions,
            owner,
            ceremonies: Ceremonies::new(clock.clone()),
            codes: LinkCodes::new(clock.clone()),
            clock,
        }
    }

    /// The live ceremonies.
    #[must_use]
    pub const fn ceremonies(&self) -> &Ceremonies {
        &self.ceremonies
    }

    /// The relying party, or `linking_off`.
    pub(crate) fn relying_party(&self) -> Result<&RelyingParty, Refusal> {
        self.config.relying_party()
    }

    /// The owner's passkeys, oldest first.
    async fn held(&self, db: &Db) -> Result<Vec<Held>, PasskeyError> {
        let owner = self.owner.user().get();
        let rows = sqlx::query!(
            "SELECT id, credential_id, user_handle, counter, credential \
             FROM passkeys WHERE telegram_user_id = ?1 ORDER BY id",
            owner
        )
        .fetch_all(db.reader())
        .await?;
        rows.into_iter()
            .map(|row| {
                let passkey = serde_json::from_str::<Passkey>(&row.credential)
                    .map_err(|_| Refusal::PasskeyInvalid)?;
                Ok(Held {
                    row: row.id,
                    credential_id: row.credential_id,
                    user_handle: row.user_handle,
                    counter: u32::try_from(row.counter).map_err(|_| Refusal::PasskeyInvalid)?,
                    passkey,
                })
            })
            .collect()
    }

    /// Starts a registration inside the `link` session `session` (SPEC-359 R5).
    ///
    /// # Errors
    ///
    /// `linking_off`, `no_session` for any session but a `link` one, or a store or random
    /// failure.
    pub async fn start_registration(
        &self,
        db: &Db,
        session: &str,
    ) -> Result<Started, PasskeyError> {
        let relying_party = self.relying_party()?;
        self.link_session(session)?;
        let held = self.held(db).await?;
        let excluded = held
            .iter()
            .map(|passkey| CredentialID::from(passkey.credential_id.clone()))
            .collect::<Vec<_>>();
        // One user handle for the owner: the held passkeys' own, or a NEW random one for the first.
        let held_handle = held
            .first()
            .map(|passkey| Uuid::from_slice(&passkey.user_handle))
            .transpose()
            .map_err(|_| Refusal::PasskeyInvalid)?;
        let user_handle = held_handle.unwrap_or_else(Uuid::new_v4);
        let (options, state) = relying_party
            .webauthn()
            .start_passkey_registration(user_handle, USER_NAME, USER_NAME, Some(excluded))
            .map_err(|error| refusal_of(&error))?;
        let flow = self.ceremonies.insert(State::Registration {
            state,
            session: digest(session.as_bytes()),
            user_handle,
        })?;
        let options = serde_json::to_value(&options).map_err(|_| Refusal::PasskeyInvalid)?;
        Ok(Started { flow, options })
    }

    /// Finishes the registration `flow` names inside the `link` session `session`, and inserts
    /// the owner's `passkeys` row; its row id (SPEC-359 R5, R8).
    ///
    /// # Errors
    ///
    /// The refusals of R8, `already_linked` for a credential already present, or a store failure.
    pub async fn finish_registration(
        &self,
        db: &Db,
        session: &str,
        flow: &str,
        response: &Value,
    ) -> Result<i64, PasskeyError> {
        let relying_party = self.relying_party()?;
        self.link_session(session)?;
        let ceremony = self.ceremonies.take(flow)?;
        let State::Registration {
            state,
            session: started_in,
            user_handle,
        } = ceremony.state
        else {
            return Err(Refusal::ChallengeInvalid.into());
        };
        if !bool::from(digest(session.as_bytes()).ct_eq(&started_in)) {
            return Err(Refusal::ChallengeInvalid.into());
        }
        let credential = serde_json::from_value::<RegisterPublicKeyCredential>(response.clone())
            .map_err(|_| Refusal::PasskeyInvalid)?;
        let passkey = relying_party
            .webauthn()
            .finish_passkey_registration(&credential, &state)
            .map_err(|error| refusal_of(&error))?;
        let stored = serde_json::to_value(&passkey).map_err(|_| Refusal::PasskeyInvalid)?;
        let counter = stored["cred"]["counter"].as_i64().unwrap_or(0);
        let backup_state = i64::from(stored["cred"]["backup_state"].as_bool().unwrap_or(false));
        let serialized = stored.to_string();
        let owner = self.owner.user().get();
        let credential_id = passkey.cred_id().as_ref().to_vec();
        let user_handle = user_handle.as_bytes().to_vec();
        let now = self.clock.now().epoch_millis();
        let mut transaction = db.write().await?;
        let row = sqlx::query!(
            "INSERT INTO passkeys \
             (telegram_user_id, credential_id, user_handle, credential, counter, backup_state, \
              created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
            owner,
            credential_id,
            user_handle,
            serialized,
            counter,
            backup_state,
            now
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(database) if database.is_unique_violation() => {
                PasskeyError::from(Refusal::AlreadyLinked)
            }
            other => PasskeyError::from(other),
        })?;
        transaction.commit().await?;
        tracing::info!(
            event = "passkey_registered",
            row = row.id,
            "a passkey was registered"
        );
        Ok(row.id)
    }

    /// Starts a sign-in over the owner's passkeys; no session is needed (SPEC-359 R6).
    ///
    /// # Errors
    ///
    /// `linking_off`, `not_linked` when the owner holds no passkey, or a store or random
    /// failure.
    pub async fn start_sign_in(&self, db: &Db) -> Result<Started, PasskeyError> {
        let relying_party = self.relying_party()?;
        let held = self.held(db).await?;
        if held.is_empty() {
            return Err(Refusal::NotLinked.into());
        }
        let passkeys = held
            .iter()
            .map(|passkey| passkey.passkey.clone())
            .collect::<Vec<_>>();
        let (options, state) = relying_party
            .webauthn()
            .start_passkey_authentication(&passkeys)
            .map_err(|error| refusal_of(&error))?;
        let flow = self.ceremonies.insert(State::SignIn { state, held })?;
        let options = serde_json::to_value(&options).map_err(|_| Refusal::PasskeyInvalid)?;
        Ok(Started { flow, options })
    }

    /// Verifies a sign-in's assertion over the owner's passkeys as the ceremony read them, applies
    /// the counter rule and advances the stored counter; the row that signed in (SPEC-359 R8).
    pub(crate) async fn verify_assertion(
        &self,
        db: &Db,
        state: &PasskeyAuthentication,
        held: &[Held],
        credential: &webauthn_rs::prelude::PublicKeyCredential,
    ) -> Result<i64, PasskeyError> {
        let relying_party = self.relying_party()?;
        let result = relying_party
            .webauthn()
            .finish_passkey_authentication(credential, state)
            .map_err(|error| refusal_of(&error))?;
        let passkey = held
            .iter()
            .find(|passkey| passkey.credential_id.as_slice() == result.cred_id().as_ref())
            .ok_or(Refusal::NotLinked)?;
        let presented = result.counter();
        if !counter_advances(passkey.counter, presented) {
            return Err(Refusal::CounterRegressed.into());
        }
        let mut updated = passkey.passkey.clone();
        let _ = updated.update_credential(&result);
        let serialized = serde_json::to_string(&updated).map_err(|_| Refusal::PasskeyInvalid)?;
        self.advance(
            db,
            passkey.row,
            passkey.counter,
            presented,
            result.backup_state(),
            &serialized,
        )
        .await?;
        Ok(passkey.row)
    }

    /// Advances the row's counter from `read`, the value the rule read, to `presented`, by ONE
    /// compare-and-swap `UPDATE`; `counter_regressed` when the row no longer holds `read`.
    async fn advance(
        &self,
        db: &Db,
        row: i64,
        read: u32,
        presented: u32,
        backup: bool,
        credential: &str,
    ) -> Result<(), PasskeyError> {
        let counter = i64::from(presented);
        let read = i64::from(read);
        let backup_state = i64::from(backup);
        let now = self.clock.now().epoch_millis();
        let mut transaction = db.write().await?;
        let advanced = sqlx::query!(
            "UPDATE passkeys SET credential = ?1, counter = ?2, backup_state = ?3, \
             last_used_at = ?4 WHERE id = ?5 AND counter = ?6",
            credential,
            counter,
            backup_state,
            now,
            row,
            read
        )
        .execute(&mut *transaction)
        .await?;
        if advanced.rows_affected() != 1 {
            return Err(Refusal::CounterRegressed.into());
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Whether `session` is a live `link` session.
    fn link_session(&self, session: &str) -> Result<(), Refusal> {
        match self
            .sessions
            .admit_proof(session)
            .map(|admitted| admitted.proof)
        {
            Some(Proof::Link) => Ok(()),
            _ => Err(Refusal::NoSession),
        }
    }
}
