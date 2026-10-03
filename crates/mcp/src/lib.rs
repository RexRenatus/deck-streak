//! DeckStreak's MCP adapter (SPEC-119; ADR-119, ADR-121, ADR-320).
//!
//! This part is the fail-closed bearer guard alone (#158): the grants read from the credentials
//! directory, the request layer that answers every request without a granted bearer before the
//! server sees it, the scope decision a tool asks of it, and the predecessor's limiter of failed
//! attempts. The server, its listener and its tools arrive with #157, behind this layer.
//!
//! - [`settings`]: the credentials' ids and why one refuses start (R6, R7).
//! - [`grants`]: the scopes and the grants, each token's SHA-256 digest computed once (R7, R8).
//! - [`limiter`]: the buckets of failed attempts, decided in one critical section (R13).
//! - [`guard`]: the bearer's parser, the constant-time match, the one refusal and the layer
//!   (R9 to R12, R14).

#![forbid(unsafe_code)]

pub mod grants;
pub mod guard;
pub mod limiter;
pub mod settings;

pub use grants::{Grants, Scope, Scopes};
pub use guard::{DENIED, Granted, Guard, GuardLayer, GuardService, Refusal};
pub use limiter::{Bucket, Limiter, Outcome};
pub use settings::McpError;
