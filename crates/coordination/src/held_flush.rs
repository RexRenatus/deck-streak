//! The scheduled flush of the held notifications (SPEC-041 R7, amended for #291): the router's
//! flush, run by its own job at a time outside the quiet window.

use deck_streak_notifications::{Flushed, Router};

use crate::ladder_facts;
use crate::runner::{Done, Fire, Reason, Work};

/// The job's work: one flush of the router's held queue. A flush that finds the window closed or
/// another flush running delivers nothing and is done: the next fire flushes again.
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
        match ladder_facts::flush_re_capped(self.router).await {
            Ok(Flushed::Ran { .. } | Flushed::QuietHours | Flushed::Busy) => Ok(Done::Done),
            Ok(Flushed::BreakerOpen) => Ok(Done::NotDelivered),
            Ok(Flushed::NoNotifier) => Err(Reason::new("no_notifier")),
            Err(_) => Err(Reason::new("flush_failed")),
        }
    }
}
