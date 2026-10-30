//! The MCP adapter (SPEC-119; ADR-119, ADR-121, ADR-320, ADR-329).
//!
//! The fail-closed bearer guard (#158): the grants read from the credentials directory, the
//! request layer that answers every request without a granted bearer before the server sees it,
//! the scope decision a tool asks of it, and the predecessor's limiter of failed attempts. Behind
//! it, the server's first slice (#157): the streamable HTTP transport on a loopback address under
//! the house serving stack, and one tool, `get_law_track`.
//!
//! - [`settings`]: the credentials' ids, the listen address, and why one refuses start (R3, R6,
//!   R7).
//! - [`grants`]: the scopes and the grants, each token's SHA-256 digest computed once (R7, R8).
//! - [`limiter`]: the buckets of failed attempts, decided in one critical section (R13).
//! - [`guard`]: the bearer's parser, the constant-time match, the one refusal and the layer
//!   (R9 to R12, R14).
//! - [`server`]: the one path, the serving stack, the listener and the drain (R2 to R4).
//! - [`tools`]: the tools, each behind its scope, and the law track's port (R15 to R17).

#![forbid(unsafe_code)]

pub mod grants;
pub mod guard;
pub mod limiter;
pub mod server;
pub mod settings;
pub mod tools;

pub use grants::{Grants, Scope, Scopes};
pub use guard::{DENIED, Granted, Guard, GuardLayer, GuardService, Refusal};
pub use limiter::{Bucket, Limiter, Outcome};
pub use settings::{ListenAddress, ListenRefusal, McpError};
pub use tools::{LawTrack, LawTrackFuture, LawTrackSource, LedgerLawTrack, McpServer};
