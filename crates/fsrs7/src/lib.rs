//! The isolated FSRS-7 scheduler (ADR-338): a card's review history, replayed into FSRS-7 memory
//! state by the upstream scheduler crate at the pinned revision (ADR-353 D1). It depends on no
//! other context of the workspace, and never on the engine, which keeps the released crate.
//!
//! [`convert`] turns review-log rows into one item per card (SPEC-342 R4), [`replay`] runs the
//! items through the model card by card or in one batch (R6), and [`measure`] is the replay-time
//! harness the crate's two examples call (R5, R7 and R8; ADR-353 D6). [`stock`] projects a
//! replayed state into the values a stock client reads (SPEC-386 R6).

pub mod convert;
pub mod measure;
pub mod replay;
pub mod stock;
