//! The minimum-client handshake (SPEC-374 R1, R4 to R8; ADR-385).
//!
//! The sync service states the oldest client level it accepts at [`STATEMENT_PATH`]. A client
//! reads that statement and hands it to its dispatcher, which keeps the latest outcome and refuses
//! every sync call until a statement that admits [`CLIENT_LEVEL`] is read, so a client older than
//! the service accepts stops before any sync request, and says why.

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

/// The URL of the statement for an encoded sync login, or `None` when there is none to read.
#[must_use]
pub fn statement_url(login: &[u8]) -> Option<String> {
    let _ = login;
    None
}
