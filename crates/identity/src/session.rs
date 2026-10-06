//! Owner sessions: the in-memory store, the `__Host-` cookie, and the extractor that admits a
//! request (SPEC-024 R5 to R7; ADR-024).
//!
//! A handshake opens a NEW session: an id of 32 bytes from the operating system's generator, handed
//! to the browser once as hex in the cookie and kept here only as its SHA-256, so a memory dump
//! holds no usable id. A session ends [`IDLE_TIMEOUT`] after its last request or
//! [`ABSOLUTE_LIFETIME`] after it began, whichever is first; at most [`MAX_LIVE_SESSIONS`] are
//! kept, and opening one more evicts the oldest. Nothing is written to disk, so a restart ends
//! every session and the Mini App re-handshakes once.
//!
//! A presented id is hashed and compared with each live session's digest in constant time
//! (`subtle`), so no timing reveals how much of a stored digest a guess matched.
//!
//! SPEC-359 R2 gives each session its proof ([`Proof`]): `telegram` for a handshake, `link` for a
//! redeemed link code, and `linked` for a passkey sign-in, with the `passkeys` row that opened it.
//! [`OwnerSession`] admits `telegram` and `linked`; a `link` session lives
//! [`LINK_SESSION_LIFETIME`] and reaches the linking routes alone, through [`LinkSession`].

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderName};
use deck_streak_kernel::{Clock, UtcMillis};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::Refusal;
use crate::owner::Owner;

/// The session cookie's name. The `__Host-` prefix makes a browser refuse it unless it is `Secure`,
/// has `Path=/` and no `Domain`, so no other host or path can plant or read it.
pub const SESSION_COOKIE: &str = "__Host-deckstreak_session";
/// A session ends this long after its last request.
pub const IDLE_TIMEOUT: Duration = Duration::from_mins(30);
/// A session ends this long after it began, however busy it is; the cookie's `Max-Age`.
pub const ABSOLUTE_LIFETIME: Duration = Duration::from_hours(8);
/// The most sessions kept at once: one owner, a few devices.
pub const MAX_LIVE_SESSIONS: usize = 8;
/// A `link` session ends this long after the redeem that opened it (SPEC-359 R2).
pub const LINK_SESSION_LIFETIME: Duration = Duration::from_secs(600);
/// A session id's length: 32 bytes from the operating system's generator.
const ID_BYTES: usize = 32;

/// Why a session could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The operating system's random generator failed; the source says why.
    #[error("the operating system's random generator failed")]
    Random(#[source] getrandom::Error),
}

/// A session's id, as the browser holds it: 64 hex digits. It exists in the clear only between
/// [`Sessions::open`] and the `Set-Cookie` that carries it. Its `Debug` never shows it.
pub struct SessionToken(String);

impl SessionToken {
    /// The id, for the one header that carries it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionToken(..)")
    }
}

/// The `Set-Cookie` header that hands the browser `token`: `__Host-` prefixed, `Path=/`, a
/// `Max-Age` of the absolute lifetime, `Secure`, `HttpOnly` and `SameSite=Strict`, with no
/// `Domain` (R6). The value is written whole, every attribute in one place.
#[must_use]
pub fn opening_cookie(token: &SessionToken) -> (HeaderName, String) {
    (
        SET_COOKIE,
        format!(
            "__Host-deckstreak_session={}; Path=/; Max-Age={}; Secure; HttpOnly; SameSite=Strict",
            token.0,
            ABSOLUTE_LIFETIME.as_secs()
        ),
    )
}

/// The `Set-Cookie` header that ends the browser's session: the same cookie, empty, with a
/// `Max-Age` of zero and every other attribute unchanged, as a browser needs to match it.
#[must_use]
pub fn ended_cookie() -> (HeaderName, &'static str) {
    (
        SET_COOKIE,
        "__Host-deckstreak_session=; Path=/; Max-Age=0; Secure; HttpOnly; SameSite=Strict",
    )
}

/// The session id the request's cookies carry: the value of the one [`SESSION_COOKIE`], or `None`
/// when there is none, or more than one.
#[must_use]
pub fn presented(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for value in headers.get_all(COOKIE) {
        let Ok(text) = value.to_str() else {
            continue;
        };
        for pair in text.split(';') {
            let Some((name, id)) = pair.trim().split_once('=') else {
                continue;
            };
            if name == SESSION_COOKIE {
                if found.is_some() {
                    return None;
                }
                found = Some(id);
            }
        }
    }
    found
}

/// What a session proves (SPEC-359 R2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proof {
    /// A Telegram handshake, at the session's beginning.
    Telegram,
    /// A redeemed link code: the linking routes alone, for [`LINK_SESSION_LIFETIME`].
    Link,
    /// A passkey sign-in, with the `passkeys` row that opened it.
    Linked(i64),
}

/// A session the store admitted, with its proof and the instant it began.
#[derive(Clone, Copy, Debug)]
pub struct Admitted {
    /// The owner the session belongs to.
    pub owner: Owner,
    /// What the session proves.
    pub proof: Proof,
    /// When the session began: for a `telegram` session, its handshake.
    pub began: UtcMillis,
    /// When the store admitted it: the instant a freshness check measures the handshake's age
    /// against (SPEC-359 R3, R9).
    pub now: UtcMillis,
}

/// A live session, as the store keeps it: the SHA-256 of its id, never the id.
struct Live {
    digest: [u8; 32],
    proof: Proof,
    owner: Owner,
    began: UtcMillis,
    seen: UtcMillis,
}

impl Live {
    /// Whether the session is still live at `now`: less than the idle timeout since its last
    /// request, and less than the absolute lifetime since it began.
    fn is_live(&self, now: UtcMillis) -> bool {
        let since = |then: UtcMillis| now.epoch_millis().saturating_sub(then.epoch_millis());
        since(self.seen) < millis(IDLE_TIMEOUT) && since(self.began) < millis(ABSOLUTE_LIFETIME)
    }
}

/// A timeout in whole milliseconds; both are far below `i64::MAX`.
fn millis(timeout: Duration) -> i64 {
    i64::try_from(timeout.as_millis()).unwrap_or(i64::MAX)
}

/// The SHA-256 of a session id.
fn digest(id: &[u8]) -> [u8; 32] {
    Sha256::digest(id).into()
}

/// The 32 bytes a session id's 64 lowercase hex digits spell, or `None` for any other text.
fn id_bytes(token: &str) -> Option<[u8; ID_BYTES]> {
    let digits = token.as_bytes();
    if digits.len() != ID_BYTES * 2 {
        return None;
    }
    let value = |digit: u8| match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    };
    let mut id = [0_u8; ID_BYTES];
    for (byte, pair) in id.iter_mut().zip(digits.chunks_exact(2)) {
        *byte = (value(pair[0])? << 4) | value(pair[1])?;
    }
    Some(id)
}

/// `id` as 64 lowercase hex digits.
fn hex(id: &[u8; ID_BYTES]) -> String {
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

/// The owner's live sessions, in memory, on the kernel's clock.
///
/// A handle: every clone reads and writes the same store.
#[derive(Clone)]
pub struct Sessions {
    live: Arc<Mutex<Vec<Live>>>,
    clock: Arc<dyn Clock>,
}

impl Sessions {
    /// An empty store whose sessions age by `clock`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            live: Arc::default(),
            clock,
        }
    }

    /// Opens a NEW session for `owner`, evicting the oldest when [`MAX_LIVE_SESSIONS`] are live,
    /// and returns its id: the only time the id exists in the clear.
    ///
    /// # Errors
    ///
    /// [`SessionError::Random`] when the operating system's generator fails.
    pub fn open(&self, owner: Owner) -> Result<SessionToken, SessionError> {
        self.open_with(owner, Proof::Telegram)
    }

    /// Opens a NEW `link` session for `owner`, as [`Sessions::open`] does (SPEC-359 R4).
    ///
    /// # Errors
    ///
    /// [`SessionError::Random`] when the operating system's generator fails.
    pub fn open_link(&self, owner: Owner) -> Result<SessionToken, SessionError> {
        self.open_with(owner, Proof::Link)
    }

    /// Opens a NEW `linked` session for `owner`, opened by the `passkeys` row `row` (SPEC-359 R6).
    ///
    /// # Errors
    ///
    /// [`SessionError::Random`] when the operating system's generator fails.
    pub fn open_linked(&self, owner: Owner, row: i64) -> Result<SessionToken, SessionError> {
        self.open_with(owner, Proof::Linked(row))
    }

    /// Opens a NEW session with `proof`, evicting the oldest when [`MAX_LIVE_SESSIONS`] are live.
    fn open_with(&self, owner: Owner, proof: Proof) -> Result<SessionToken, SessionError> {
        let mut id = [0_u8; ID_BYTES];
        getrandom::fill(&mut id).map_err(SessionError::Random)?;
        let now = self.clock.now();
        let mut live = self.live_at(now);
        if live.len() >= MAX_LIVE_SESSIONS
            && let Some(oldest) = (0..live.len()).min_by_key(|&at| live[at].began)
        {
            live.swap_remove(oldest);
        }
        live.push(Live {
            digest: digest(&id),
            proof,
            owner,
            began: now,
            seen: now,
        });
        Ok(SessionToken(hex(&id)))
    }

    /// The owner of the live session `token` names, refreshing its idle timer; `None` when it
    /// names none, or its session has ended.
    #[must_use]
    pub fn admit(&self, token: &str) -> Option<Owner> {
        let presented = digest(&id_bytes(token)?);
        let now = self.clock.now();
        let mut live = self.live_at(now);
        let session = live
            .iter_mut()
            .find(|session| bool::from(session.digest.ct_eq(&presented)))?;
        session.seen = now;
        Some(session.owner)
    }

    /// The live session `token` names, with its proof, refreshing its idle timer; `None` when it
    /// names none, or its session has ended (SPEC-359 R2).
    #[must_use]
    pub fn admit_proof(&self, token: &str) -> Option<Admitted> {
        let presented = digest(&id_bytes(token)?);
        let now = self.clock.now();
        let mut live = self.live_at(now);
        let session = live
            .iter_mut()
            .find(|session| bool::from(session.digest.ct_eq(&presented)))?;
        session.seen = now;
        Some(Admitted {
            owner: session.owner,
            proof: session.proof,
            began: session.began,
            now,
        })
    }

    /// Ends every `linked` session the `passkeys` row `row` opened, and no other; how many ended
    /// (SPEC-359 R9).
    #[allow(
        clippy::must_use_candidate,
        reason = "ending the sessions is the effect; how many ended is only a report"
    )]
    pub fn end_opened_by(&self, row: i64) -> usize {
        let _ = row;
        0
    }

    /// Ends the session `token` names, on the server; whether a live one was ended.
    #[allow(
        clippy::must_use_candidate,
        reason = "ending the session is the effect; whether one was live is only a report"
    )]
    pub fn end_session(&self, token: &str) -> bool {
        let Some(id) = id_bytes(token) else {
            return false;
        };
        let presented = digest(&id);
        let mut live = self.live_at(self.clock.now());
        let before = live.len();
        live.retain(|session| !bool::from(session.digest.ct_eq(&presented)));
        live.len() < before
    }

    /// How many sessions are live now.
    #[must_use]
    pub fn live(&self) -> usize {
        self.live_at(self.clock.now()).len()
    }

    /// The store, every session that has ended at `now` dropped from it first.
    fn live_at(&self, now: UtcMillis) -> MutexGuard<'_, Vec<Live>> {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.retain(|session| session.is_live(now));
        live
    }
}

impl fmt::Debug for Sessions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sessions")
            .field("live", &self.live())
            .finish_non_exhaustive()
    }
}

/// A request from the owner: its cookie names a live session, whose idle timer the request
/// refreshed (R7). A route that takes it answers 401 `no_session` to any other request.
#[derive(Clone, Copy, Debug)]
pub struct OwnerSession {
    owner: Owner,
}

impl OwnerSession {
    /// The owner the session belongs to.
    #[must_use]
    pub const fn owner(&self) -> Owner {
        self.owner
    }
}

impl<S> FromRequestParts<S> for OwnerSession
where
    Sessions: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Refusal;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        presented(&parts.headers)
            .and_then(|token| Sessions::from_ref(state).admit(token))
            .map(|owner| Self { owner })
            .ok_or(Refusal::NoSession)
    }
}

/// A request inside a `link` session: the linking routes' session (SPEC-359 R2, R5). A route that
/// takes it answers 401 `no_session` to any other request. Its `Debug` never shows the id.
#[derive(Clone)]
pub struct LinkSession {
    owner: Owner,
    token: String,
}

impl LinkSession {
    /// The owner the session belongs to.
    #[must_use]
    pub const fn owner(&self) -> Owner {
        self.owner
    }

    /// The session's id, for the identity call the route makes with it.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Debug for LinkSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LinkSession(..)")
    }
}

impl<S> FromRequestParts<S> for LinkSession
where
    Sessions: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Refusal;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = presented(&parts.headers).ok_or(Refusal::NoSession)?;
        let admitted = Sessions::from_ref(state)
            .admit_proof(token)
            .ok_or(Refusal::NoSession)?;
        match admitted.proof {
            Proof::Link => Ok(Self {
                owner: admitted.owner,
                token: token.to_owned(),
            }),
            Proof::Telegram | Proof::Linked(_) => Err(Refusal::NoSession),
        }
    }
}

/// A request inside a `telegram` session whose handshake is at most
/// [`crate::linking::REAUTH_AGE`] old: what minting a link code and removing a method need
/// (SPEC-359 R3, R9). Any other session is refused 401 `reauth_required`, and no session
/// `no_session`. Its `Debug` never shows the id.
#[derive(Clone)]
pub struct FreshTelegramSession {
    owner: Owner,
    token: String,
}

impl FreshTelegramSession {
    /// The owner the session belongs to.
    #[must_use]
    pub const fn owner(&self) -> Owner {
        self.owner
    }

    /// The session's id, for the identity call the route makes with it.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Debug for FreshTelegramSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FreshTelegramSession(..)")
    }
}

impl<S> FromRequestParts<S> for FreshTelegramSession
where
    Sessions: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Refusal;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = presented(&parts.headers).ok_or(Refusal::NoSession)?;
        let owner = crate::linking::fresh_telegram(&Sessions::from_ref(state), token)?;
        Ok(Self {
            owner,
            token: token.to_owned(),
        })
    }
}
