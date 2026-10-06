//! The link code, the passkey sign-in's account checks, and the owner's methods (SPEC-359 R3, R4,
//! R6, R9; ADR-370).
//!
//! A link code is [`LINK_CODE_BYTES`] from the operating system's generator, answered once as
//! base64url and kept only as its SHA-256, in memory. It lives [`LINK_CODE_LIFETIME`], redeems
//! once, and at most [`MAX_LIVE_LINK_CODES`] are live. It is minted only for a `telegram` session
//! whose handshake is at most [`REAUTH_AGE`] old, and its redeem opens a `link` session, which
//! lives [`LINK_SESSION_LIFETIME`].
//!
//! The owner's methods are Telegram, the primary, then each passkey by its row id. Removing a
//! passkey needs a fresh `telegram` session and ends the `linked` sessions it opened; Telegram
//! itself is never removed.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use deck_streak_kernel::{Clock, Db, UtcMillis};
use serde::Serialize;
use serde_json::Value;
use subtle::ConstantTimeEq;
use webauthn_rs::prelude::{Base64UrlSafeData, PublicKeyCredential};

use crate::Refusal;
use crate::owner::Owner;
use crate::passkeys::{PasskeyError, Passkeys, SignedIn, State, base64url, digest, millis};
use crate::session::{Proof, SessionToken, Sessions};

pub use crate::session::LINK_SESSION_LIFETIME;

/// A link code's length: 16 bytes, 128 bits, from the operating system's generator (R3).
pub const LINK_CODE_BYTES: usize = 16;
/// A link code is refused `link_code_expired` this long after it was minted (R3).
pub const LINK_CODE_LIFETIME: Duration = Duration::from_secs(600);
/// The most link codes kept at once; minting one more evicts the oldest (R3).
pub const MAX_LIVE_LINK_CODES: usize = 8;
/// How old a `telegram` session's handshake may be for a link code or a removal (R3, R9).
pub const REAUTH_AGE: Duration = Duration::from_secs(300);
/// The Telegram method's id in the owner's methods: never a `passkeys` row id (R9).
pub const TELEGRAM_METHOD: i64 = 0;

/// A link code, as the Mini App holds it: base64url. It exists in the clear only in the answer
/// that mints it. Its `Debug` never shows it.
pub struct LinkCode(String);

impl LinkCode {
    /// The code, for the one answer that carries it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for LinkCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LinkCode(..)")
    }
}

/// A live link code, as the store keeps it: its SHA-256, never the code.
struct Live {
    digest: Vec<u8>,
    owner: Owner,
    minted: UtcMillis,
}

/// The live link codes, in memory, on the kernel's clock. A handle: every clone reads and writes
/// the same store.
#[derive(Clone)]
pub struct LinkCodes {
    live: Arc<Mutex<Vec<Live>>>,
    clock: Arc<dyn Clock>,
}

impl LinkCodes {
    /// An empty store whose codes age by `clock`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            live: Arc::default(),
            clock,
        }
    }

    /// Keeps a NEW code for `owner`, evicting the oldest when [`MAX_LIVE_LINK_CODES`] are live,
    /// and returns it: the only time it exists in the clear.
    fn insert(&self, owner: Owner) -> Result<LinkCode, PasskeyError> {
        let mut code = [0_u8; LINK_CODE_BYTES];
        getrandom::fill(&mut code).map_err(|_| PasskeyError::Random)?;
        let mut live = self.lock();
        // The store is in insertion order, so its first code is the oldest.
        if live.len() >= MAX_LIVE_LINK_CODES {
            live.remove(0);
        }
        live.push(Live {
            digest: digest(&code),
            owner,
            minted: self.clock.now(),
        });
        Ok(LinkCode(base64url(&code)))
    }

    /// Takes the code `code` names out of the store, in the critical section that finds it; the
    /// owner it was minted for.
    ///
    /// # Errors
    ///
    /// [`Refusal::LinkCodeInvalid`] when no live code is `code`, and [`Refusal::LinkCodeExpired`]
    /// when it is [`LINK_CODE_LIFETIME`] old.
    pub fn take(&self, code: &str) -> Result<Owner, Refusal> {
        let presented = digest(&code_bytes(code).ok_or(Refusal::LinkCodeInvalid)?);
        let now = self.clock.now();
        let mut live = self.lock();
        let at = live
            .iter()
            .position(|kept| bool::from(kept.digest.ct_eq(&presented)))
            .ok_or(Refusal::LinkCodeInvalid)?;
        let kept = live.remove(at);
        drop(live);
        let age = now
            .epoch_millis()
            .saturating_sub(kept.minted.epoch_millis());
        if age >= millis(LINK_CODE_LIFETIME) {
            return Err(Refusal::LinkCodeExpired);
        }
        Ok(kept.owner)
    }

    /// The SHA-256 digests the store keeps, oldest first: what R3's "kept only as its SHA-256"
    /// audits.
    #[must_use]
    pub fn digests(&self) -> Vec<Vec<u8>> {
        self.lock().iter().map(|kept| kept.digest.clone()).collect()
    }

    /// How many codes are live.
    #[must_use]
    pub fn live(&self) -> usize {
        self.lock().len()
    }

    /// The store.
    fn lock(&self) -> MutexGuard<'_, Vec<Live>> {
        self.live.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for LinkCodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkCodes")
            .field("live", &self.live())
            .finish_non_exhaustive()
    }
}

/// The bytes a link code's base64url spells, or `None` for any other text.
fn code_bytes(code: &str) -> Option<Vec<u8>> {
    serde_json::from_value::<Base64UrlSafeData>(Value::String(code.to_owned()))
        .ok()
        .map(Vec::from)
}

/// The owner of the `telegram` session `token` names, when its handshake is at most
/// [`REAUTH_AGE`] old (SPEC-359 R3, R9).
///
/// # Errors
///
/// [`Refusal::NoSession`] when `token` names no live session, and [`Refusal::ReauthRequired`]
/// for any other proof or an older handshake.
pub fn fresh_telegram(sessions: &Sessions, token: &str) -> Result<Owner, Refusal> {
    let admitted = sessions.admit_proof(token).ok_or(Refusal::NoSession)?;
    let age = admitted
        .now
        .epoch_millis()
        .saturating_sub(admitted.began.epoch_millis());
    if admitted.proof != Proof::Telegram || age > millis(REAUTH_AGE) {
        return Err(Refusal::ReauthRequired);
    }
    Ok(admitted.owner)
}

/// The `passkeys` row holding the credential `id`, with the Telegram user id it belongs to: read
/// before the library verifies, so an unknown credential is `not_linked` and another user's is
/// `not_owner` (SPEC-359 R6).
async fn credential_row(db: &Db, id: &[u8]) -> Result<Option<(i64, i64)>, PasskeyError> {
    let found = sqlx::query!(
        "SELECT id, telegram_user_id FROM passkeys WHERE credential_id = ?1",
        id
    )
    .fetch_optional(db.reader())
    .await?;
    Ok(found.map(|row| (row.id, row.telegram_user_id)))
}

/// One of the owner's ways in, as the methods list names it: never a credential id, a public key
/// or the user handle (SPEC-359 R9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Method {
    /// The method's id: [`TELEGRAM_METHOD`] for Telegram, else the passkey's row id.
    pub id: i64,
    /// `telegram` or `passkey`.
    pub kind: &'static str,
    /// When the passkey was registered, epoch milliseconds; `None` for Telegram.
    pub created_at: Option<i64>,
    /// When the passkey last signed in, epoch milliseconds; `None` until it has.
    pub last_used_at: Option<i64>,
}

impl Passkeys {
    /// The link codes.
    #[must_use]
    pub const fn link_codes(&self) -> &LinkCodes {
        &self.codes
    }

    /// Mints a link code for the fresh `telegram` session `session` (SPEC-359 R3).
    ///
    /// # Errors
    ///
    /// `linking_off`, `no_session`, `reauth_required`, or a random failure.
    pub fn mint_link_code(&self, session: &str) -> Result<LinkCode, PasskeyError> {
        self.relying_party()?;
        let owner = fresh_telegram(&self.sessions, session)?;
        let code = self.codes.insert(owner)?;
        tracing::info!(event = "link_code_minted", "a link code was minted");
        Ok(code)
    }

    /// Redeems `code`: ends the session the request arrived with, if any, and opens a `link`
    /// session for the code's owner (SPEC-359 R4).
    ///
    /// # Errors
    ///
    /// `linking_off`, `link_code_invalid`, `link_code_expired`, or a session failure.
    pub fn redeem_link_code(
        &self,
        code: &str,
        arriving: Option<&str>,
    ) -> Result<SessionToken, PasskeyError> {
        self.relying_party()?;
        let owner = self.codes.take(code)?;
        if let Some(arriving) = arriving {
            self.sessions.end_session(arriving);
        }
        let session = self.sessions.open_link(owner)?;
        tracing::info!(event = "link_code_redeemed", "a link code was redeemed");
        Ok(session)
    }

    /// Finishes the sign-in `flow` names: the credential must be one of the configured owner's
    /// passkeys, the assertion must verify and advance the counter, and the session the request
    /// arrived with is replaced by a NEW `linked` one (SPEC-359 R6).
    ///
    /// # Errors
    ///
    /// The refusals of R6 and R8, or a store or session failure.
    pub async fn finish_sign_in(
        &self,
        db: &Db,
        arriving: Option<&str>,
        flow: &str,
        response: &Value,
    ) -> Result<SignedIn, PasskeyError> {
        self.relying_party()?;
        let ceremony = self.ceremonies.take(flow)?;
        let State::SignIn { state, held } = ceremony.state else {
            return Err(Refusal::ChallengeInvalid.into());
        };
        let credential = serde_json::from_value::<PublicKeyCredential>(response.clone())
            .map_err(|_| Refusal::PasskeyInvalid)?;
        let Some((_, user)) = credential_row(db, credential.get_credential_id()).await? else {
            return Err(Refusal::NotLinked.into());
        };
        if user != self.owner.user().get() {
            return Err(Refusal::NotOwner.into());
        }
        let row = self
            .verify_assertion(db, &state, &held, &credential)
            .await?;
        if let Some(arriving) = arriving {
            self.sessions.end_session(arriving);
        }
        let session = self.sessions.open_linked(self.owner, row)?;
        tracing::info!(event = "passkey_signed_in", row, "a passkey signed in");
        Ok(SignedIn {
            session,
            row,
            credential_ids: held
                .iter()
                .map(|passkey| base64url(&passkey.credential_id))
                .collect(),
            user_handle: held
                .first()
                .map(|passkey| base64url(&passkey.user_handle))
                .unwrap_or_default(),
        })
    }

    /// The owner's methods: Telegram first, then each passkey, oldest first (SPEC-359 R9).
    ///
    /// # Errors
    ///
    /// `linking_off`, or a store failure.
    pub async fn methods(&self, db: &Db) -> Result<Vec<Method>, PasskeyError> {
        self.relying_party()?;
        let owner = self.owner.user().get();
        let rows = sqlx::query!(
            "SELECT id, created_at, last_used_at FROM passkeys \
             WHERE telegram_user_id = ?1 ORDER BY id",
            owner
        )
        .fetch_all(db.reader())
        .await?;
        let telegram = Method {
            id: TELEGRAM_METHOD,
            kind: "telegram",
            created_at: None,
            last_used_at: None,
        };
        Ok(std::iter::once(telegram)
            .chain(rows.into_iter().map(|row| Method {
                id: row.id,
                kind: "passkey",
                created_at: Some(row.created_at),
                last_used_at: row.last_used_at,
            }))
            .collect())
    }

    /// Removes the passkey `id` from inside the fresh `telegram` session `session`, and ends the
    /// `linked` sessions it opened (SPEC-359 R9).
    ///
    /// # Errors
    ///
    /// `linking_off`, `no_session`, `reauth_required`, `last_method` for Telegram,
    /// `identity_unknown` for an id no passkey of the owner has, or a store failure.
    pub async fn remove(&self, db: &Db, session: &str, id: i64) -> Result<(), PasskeyError> {
        self.relying_party()?;
        fresh_telegram(&self.sessions, session)?;
        if id == TELEGRAM_METHOD {
            return Err(Refusal::LastMethod.into());
        }
        let owner = self.owner.user().get();
        let mut transaction = db.write().await?;
        let removed = sqlx::query!(
            "DELETE FROM passkeys WHERE id = ?1 AND telegram_user_id = ?2",
            id,
            owner
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        if removed.rows_affected() != 1 {
            return Err(Refusal::IdentityUnknown.into());
        }
        self.sessions.end_opened_by(id);
        tracing::info!(event = "passkey_removed", row = id, "a passkey was removed");
        Ok(())
    }
}
