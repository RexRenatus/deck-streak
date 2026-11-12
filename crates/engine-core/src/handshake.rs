//! The minimum-client handshake (SPEC-374 R1, R4 to R8; ADR-385).
//!
//! The sync service states the oldest client level it accepts at [`STATEMENT_PATH`]. A client
//! reads that statement and hands it to its dispatcher, which keeps the latest outcome and refuses
//! every sync call until a statement that admits [`CLIENT_LEVEL`] is read, so a client older than
//! the service accepts stops before any sync request, and says why.

use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::sync::SyncLoginRequest;
use prost::Message;
use url::Url;

use crate::login_guard;

/// This client's level: one positive integer, defined once, which both clients compile (R1).
pub const CLIENT_LEVEL: u32 = 1;

/// The path at the sync endpoint's origin where the service states its minimum client level (R2).
pub const STATEMENT_PATH: &str = "/api/sync/minimum-client";

/// The refusal when the statement names a minimum above this client's level (R6).
pub const BELOW: &str = "This version of DeckStreak is older than the oldest the sync service accepts. Update DeckStreak to sync.";

/// The refusal when no statement was read (R6).
pub const UNREAD: &str = "A network error occurred. The sync service's oldest accepted version could not be read, so nothing was synced.";

/// The refusal when a statement was read and does not decode (R6).
pub const UNDECODABLE: &str = "The sync service's statement of the oldest version it accepts could not be read, so nothing was synced.";

/// The statement's one field: the oldest client level the service accepts.
const MINIMUM_FIELD: &str = "minimum_client_level";

/// What the latest statement decided (R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Outcome {
    /// The statement decodes and its minimum is at most [`CLIENT_LEVEL`].
    Admitted,
    /// The statement decodes and its minimum is above [`CLIENT_LEVEL`].
    Below,
    /// A statement was read and does not decode, an empty one included.
    Undecodable,
    /// No statement was read: the state a dispatcher starts in.
    #[default]
    Unread,
}

/// The outcome of `statement`: the body the service answered, or `None` when nothing answered
/// (R4). A body decodes when it is a JSON object whose `minimum_client_level` is a non-negative
/// integer; anything else, an empty body included, is undecodable.
#[must_use]
pub fn decide(statement: Option<&[u8]>) -> Outcome {
    let Some(statement) = statement else {
        return Outcome::Unread;
    };
    match minimum(statement) {
        Some(minimum) if minimum <= u64::from(CLIENT_LEVEL) => Outcome::Admitted,
        Some(_) => Outcome::Below,
        None => Outcome::Undecodable,
    }
}

/// The minimum a statement's body names, when it decodes.
fn minimum(statement: &[u8]) -> Option<u64> {
    serde_json::from_slice::<serde_json::Value>(statement)
        .ok()?
        .get(MINIMUM_FIELD)?
        .as_u64()
}

/// Whether `outcome` lets a sync call reach the engine (R5).
///
/// # Errors
///
/// Unless the outcome is admitted, an encoded `BackendError` of kind `INVALID_INPUT`, the login
/// guard's shape, whose message is the outcome's sentence (R6).
pub fn admits(outcome: Outcome) -> Result<(), Vec<u8>> {
    let sentence = match outcome {
        Outcome::Admitted => return Ok(()),
        Outcome::Below => BELOW,
        Outcome::Undecodable => UNDECODABLE,
        Outcome::Unread => UNREAD,
    };
    Err(BackendError {
        message: sentence.to_owned(),
        kind: Kind::InvalidInput.into(),
        ..BackendError::default()
    }
    .encode_to_vec())
}

/// The URL of the statement for an encoded sync login: [`STATEMENT_PATH`] at the scheme, host and
/// port of the login's endpoint (R8). `None` when the login guard refuses the login, so a login the
/// rule refuses is never read and keeps the guard's own refusal.
#[must_use]
pub fn statement_url(login: &[u8]) -> Option<String> {
    login_guard::check(login).ok()?;
    let endpoint = SyncLoginRequest::decode(login).ok()?.endpoint?;
    let origin = Url::parse(&endpoint).ok()?.origin().ascii_serialization();
    Some(format!("{origin}{STATEMENT_PATH}"))
}
