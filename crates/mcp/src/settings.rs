//! The guard's credentials: their ids, the shortest token a grant accepts, and why a credential
//! refuses start (SPEC-119 R6, R7; ADR-320).
//!
//! Every refusal names a credential's id and never its value: the value is a secret the loader has
//! already registered with the redactor (SPEC-066).

use deck_streak_kernel::CredentialError;

/// The core token's credential id. It is required: every loader error refuses start (R6).
pub const CORE_CREDENTIAL: &str = "mcp-core-token";

/// The law-track token's credential id. It is optional: a missing file grants nothing, and any
/// other loader error refuses start (R6).
pub const LAW_TRACK_CREDENTIAL: &str = "mcp-law-track-token";

/// The fewest characters a loaded token may hold (R7).
pub const MIN_CREDENTIAL_CHARS: usize = 32;

/// Why the guard refuses to start. Each variant names a credential's id, never its value.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// The credential loader refused the credential (R6).
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// The credential holds a byte outside 0x21 to 0x7E, which no request can present (R9), so its
    /// grant could never be used (R7 as amended by T15).
    #[error("the credential {id} holds a character no request can present")]
    UnpresentableCredential {
        /// The credential's id.
        id: &'static str,
    },
    /// The credential is shorter than [`MIN_CREDENTIAL_CHARS`] characters (R7).
    #[error("the credential {id} is shorter than {MIN_CREDENTIAL_CHARS} characters")]
    WeakCredential {
        /// The credential's id.
        id: &'static str,
    },
    /// Two credentials hold one value, which would give one token both grants' scopes (R7).
    #[error("the credentials {first} and {second} hold one value")]
    SharedCredential {
        /// The first credential's id.
        first: &'static str,
        /// The second credential's id.
        second: &'static str,
    },
}
