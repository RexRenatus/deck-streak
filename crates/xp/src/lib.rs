//! The per-review XP rule (SPEC-360, ADR-371): what one answer earns, and the table it reads.
//!
//! [`review_xp`] holds the rule the predecessor's `gamification/xp.py:review_xp` states (SPEC-072
//! R1 and R4) and its own study-event guard; [`table`] holds the nine per-review constants of
//! `economy.json`'s `xp` section, embedded when the crate is built and parsed once.
//!
//! The crate does no I/O: no file, socket, clock, environment, process or thread is reached from
//! its source, and it depends on no other context of the workspace. That is what lets it sit
//! beside the kernel and be linked by the server and, later, by both clients. Progression links it
//! now and translates an ingest review into its [`review_xp::ReviewFacts`] at its own edge; the
//! umbrella FFI crate and the web engine link it when their delivery draws those edges.

pub mod review_xp;
pub mod table;
