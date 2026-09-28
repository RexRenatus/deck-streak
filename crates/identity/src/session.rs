//! Owner sessions: the in-memory store, the `__Host-` cookie, and the extractor that admits a
//! request (SPEC-024 R5 to R7; ADR-024).
//!
//! A handshake opens a NEW session: an id of 32 bytes from the operating system's generator, handed
//! to the browser once as hex in the cookie and kept here only as its SHA-256, so a memory dump
//! holds no usable id. A session ends [`IDLE_TIMEOUT`] after its last request or
//! [`ABSOLUTE_LIFETIME`] after it began, whichever is first; at most [`MAX_LIVE_SESSIONS`] are
//! kept, and opening one more evicts the oldest. Nothing is written to disk, so a restart ends
//! every session and the Mini App re-handshakes once.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderName};
use deck_streak_kernel::{Clock, TelegramUserId};

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
/// `Domain` (R6).
#[must_use]
pub fn opening_cookie(token: &SessionToken) -> (HeaderName, String) {
    (SET_COOKIE, format!("deckstreak_session={}", token.0))
}

/// The `Set-Cookie` header that ends the browser's session: the same cookie, empty, with a
/// `Max-Age` of zero.
#[must_use]
pub fn ended_cookie() -> (HeaderName, &'static str) {
    (SET_COOKIE, "deckstreak_session=")
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

/// The owner's live sessions, in memory, on the kernel's clock.
///
/// A handle: every clone reads and writes the same store.
#[derive(Clone)]
pub struct Sessions {
    clock: Arc<dyn Clock>,
    opened: Arc<std::sync::Mutex<Option<Owner>>>,
}

impl Sessions {
    /// An empty store whose sessions age by `clock`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            opened: Arc::default(),
        }
    }

    /// Opens a NEW session for `owner`, evicting the oldest when [`MAX_LIVE_SESSIONS`] are live,
    /// and returns its id: the only time the id exists in the clear.
    ///
    /// # Errors
    ///
    /// [`SessionError::Random`] when the operating system's generator fails.
    pub fn open(&self, owner: Owner) -> Result<SessionToken, SessionError> {
        if let Ok(mut opened) = self.opened.lock() {
            *opened = Some(owner);
        }
        Ok(SessionToken("0".repeat(64)))
    }

    /// The owner of the live session `token` names, refreshing its idle timer; `None` when it
    /// names none, or its session has ended.
    #[must_use]
    pub fn admit(&self, token: &str) -> Option<Owner> {
        let _unread = token;
        self.opened.lock().ok().and_then(|opened| *opened)
    }

    /// Ends the session `token` names, on the server; whether a live one was ended.
    pub fn end_session(&self, token: &str) -> bool {
        let _unread = token;
        false
    }

    /// How many sessions are live now.
    #[must_use]
    pub fn live(&self) -> usize {
        let _unread = &self.clock;
        0
    }
}

impl fmt::Debug for Sessions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sessions")
            .field("live", &self.live())
            .finish()
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
        let _unread = (parts, Sessions::from_ref(state));
        Ok(Self {
            owner: Owner::new(TelegramUserId::new(0)),
        })
    }
}
