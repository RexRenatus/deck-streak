//! The relight (SPEC-076 R16, R17), after the predecessor's
//! `pipeline_layers/showcase.py:ShowcaseLayer._relight` at `27ee2bc`.

use deck_streak_kernel::StudyDay;

use crate::constants::{RELIGHT_CARDS, RELIGHT_XP};

/// What a return day earns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relight {
    /// XP granted.
    pub amount: u32,
    /// The grant's source: `relight:<epoch day>`.
    pub source: String,
    /// The predecessor's event name for the celebration.
    pub event_type: &'static str,
    /// The celebration's key.
    pub event_key: String,
}

/// The relight a return day is due, from its review count and the XP already granted for it.
#[must_use]
pub fn relight(today: StudyDay, reviews: Option<u32>, granted: u32) -> Option<Relight> {
    let qualifies = reviews.is_some_and(|n| n >= RELIGHT_CARDS) && granted == 0;
    qualifies.then(|| {
        let key = format!("relight:{}", today.epoch_day());
        Relight {
            amount: RELIGHT_XP,
            source: key.clone(),
            event_type: "record",
            event_key: key,
        }
    })
}
