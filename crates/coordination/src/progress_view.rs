//! Road to C2's progress view (SPEC-077 R15, T5): the stored progress of each configured course,
//! ordered by name, read by `GET /api/progress` and the bot's progress command alike.
//!
//! The api and the bot hold no edge to the curriculum context, so the view re-exports the stored
//! rows' types they render.

pub use deck_streak_curriculum::store::{StoredBand, StoredProgress};
