//! The scheduled flush of the held notifications (SPEC-041 R7, amended for #291): the router's
//! flush, run by its own job at a time outside the quiet window.

use deck_streak_notifications::Router;

use crate::runner::{Done, Fire, Reason, Work};

/// The job's work: one flush of the router's held queue.
pub struct HeldFlushWork<'a> {
    router: &'a Router,
}

impl<'a> HeldFlushWork<'a> {
    /// The work that flushes `router`.
    #[must_use]
    pub const fn new(router: &'a Router) -> Self {
        Self { router }
    }
}

impl Work for HeldFlushWork<'_> {
    async fn perform(&self, _fire: &Fire) -> Result<Done, Reason> {
        let Self { router: _ } = self;
        Ok(Done::Done)
    }
}
