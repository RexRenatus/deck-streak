//! The engine core: the one client-side holder of Anki's engine (SPEC-345, ADR-356).
//!
//! Both clients reach the engine through a [`dispatch::Dispatcher`] started on their own
//! [`table::Transport`]: the native adapter (`deck-streak-ffi`) on `Native`, the web engine
//! (`deck-streak-web-engine`) on `Web`. A call reaches the engine only when the core's table admits
//! its pair on that transport; an exempt write is held for an owner's gesture, and every other pair
//! is refused before the engine sees it. The engine's database door takes no SQL from an adapter:
//! a read is one of a closed set whose statements the core holds.
//!
//! - [`table`]: the transports, the ordinary table with its transport columns, the exempt table
//!   and the decision for a pair (R2, R3).
//! - [`answer`]: the owner's answer, one press on one card with one grade, Again or Good, its
//!   refusal, and the codec steps of a native press (SPEC-365 R1 to R3, R6).
//! - [`dispatch`]: the dispatcher, its refusal and its fixed reads (R1, R4).
//! - [`face`]: a card's face, completed as the engine's own reviewer completes it: its text, its
//!   sound and speech clips, and what autoplay and replay play (SPEC-348 R2, R4).
//! - [`full_sync`]: the full-sync choice both clients share: the offer, the counts by id, and the
//!   states from the counts to the write, with the reads they compare (SPEC-357 R3-R9).
//! - [`gesture`]: the owner's gesture, one exempt write and its one target, and its refusal
//!   (SPEC-345 R7, R8).
//! - [`handshake`]: the minimum-client handshake: this client's level, the outcome of the
//!   latest statement of the service's minimum, its sentences and the statement's URL
//!   (SPEC-374 R1, R4 to R8).
//! - [`login_guard`]: the endpoint guard on the engine's sync login (SPEC-347 R2).
//! - [`media`]: the rules a face's media references pass: the name rule, the closed type table,
//!   the two caps and the `data:` rewrite (SPEC-348 R3).
//! - [`credential`]: the rule that keeps, sends and drops the sync key: its generation, a login
//!   kept only at the generation it started at, a send only at the held one, and a drop only on
//!   the server's refusal of the current one (SPEC-363 R3).
//! - [`one_way`]: the one-way sync's steps in the order the choice's model checks them, each
//!   reading its side from its file: the server copy, the backup, the re-check and the write
//!   (SPEC-364 R4-R8).
//! - [`review`]: the review's flag and bury rules both clients share: red, the flag the flag
//!   action sets, the user's bury mode and the bury of one card (SPEC-358 R2), and the two
//!   requests the native adapter sends for them, encoded (R3).
//! - [`undo_answer`]: the rule an undo of the review's own last answer passes: its record, the
//!   review row it names, the refusals and the state the card returns to (SPEC-371 R3).
//! - [`late`]: whether a review cannot count toward the streak for its card's due day: the
//!   engine's day, and the rule that judges a card past its due day in it (SPEC-376 R1, R2).
//!
//! The core depends on the engine and on no crate of this workspace; only the two client
//! adapters depend on it (ADR-356 D4, held by the graph census).

#![forbid(unsafe_code)]

pub mod answer;
pub mod credential;
pub mod dispatch;
pub mod face;
pub mod full_sync;
pub mod gesture;
pub mod handshake;
pub mod late;
pub mod login_guard;
pub mod media;
pub mod one_way;
pub mod review;
pub mod table;
pub mod undo_answer;
