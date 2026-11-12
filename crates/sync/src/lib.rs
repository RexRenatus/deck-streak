//! # deck-streak-sync
//!
//! What this crate owns: the sync core (SPEC-509, ADR-527). The operations every device and the
//! server write in ([`op`]), their one byte encoding ([`codec`]), the hybrid clock ([`clock`]),
//! the one id allocator beside it ([`ids`]), the key and the order every party folds by
//! ([`key`]), the rule for concurrent edits of one register ([`merge`]), and a device's queue of
//! operations the server has not judged yet ([`pending`]).
//!
//! What it does not own: any I/O. No source here reads a clock, a file, the environment, the
//! network, a process or a thread, and the crate depends on nothing, so the server, the web
//! client's `wasm32` module and the native client's static library link one encoding and one
//! clock. A caller hands in the wall clock and the bytes; the server's judge, its log and the
//! routes that carry operations live with their callers.

pub mod clock;
pub mod codec;
pub mod ids;
pub mod key;
pub mod merge;
pub mod op;
pub mod pending;

#[cfg(kani)]
mod kani_proofs;

pub use clock::{ClockError, Hlc, tick};
pub use codec::{DecodeError, EncodeError, decode, encode, encode_body};
pub use ids::{IdError, SLOTS, Slot, next_id};
pub use key::{Key, Origin, compare, sort_keys};
pub use merge::{Conflict, Edit, Register, apply, merge};
pub use op::{NotetypePart, Op, OpHead, OpKind};
pub use pending::{PendingOp, PendingQueue, Refusal, Rejected, Rejection, Settled, Stage, Verdict};
