//! The obligations registry (SPEC-023 R10): the port through which a context's delivery registers a
//! named source of open deadlines, which every sync cycle collects and hands the change gate.
//!
//! A deadline-bearing obligation (a window that closes unreviewed, a wager that expires, a free spin
//! that lapses) comes due precisely on a cycle in which nothing in the collection changed, so the
//! gate hears of it here rather than from the collection. The gate runs the recompute for a deadline
//! that lies after its last recompute and at or before now, so a deadline not yet due, or one a
//! recompute already served, never holds it open: each costs exactly the one recompute that serves
//! it. It ports the predecessor's `pipeline.py:GamifyPipeline._obligation_deadlines`, one source per
//! context instead of one list, so the term stays open to every later wave's deadlines. No source is
//! registered at W0: each deadline-bearing feature registers its own in the delivery that builds it.

use std::fmt;

use deck_streak_ingest::gate::Deadline;
use deck_streak_kernel::{KernelError, PortFuture, UtcMillis};

/// A named source of open deadlines, registered by the context that owns them.
pub trait ObligationSource: Send + Sync {
    /// The source's name, as a log line names it.
    fn name(&self) -> &'static str;

    /// Every open obligation's deadline, as the source reads it at `now`. A deadline already due
    /// and not yet settled is included: the gate decides which ones a recompute has served.
    fn deadlines(&self, now: UtcMillis) -> PortFuture<'_, Vec<Deadline>>;
}

/// Every registered source, in the order they were registered.
#[derive(Default)]
pub struct Obligations {
    sources: Vec<Box<dyn ObligationSource>>,
}

impl Obligations {
    /// No source.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `source`: every cycle from now on collects its deadlines.
    pub fn register(&mut self, source: impl ObligationSource + 'static) {
        self.sources.push(Box::new(source));
    }

    /// The registry with `source` registered.
    #[must_use]
    pub fn with(mut self, source: impl ObligationSource + 'static) -> Self {
        self.register(source);
        self
    }

    /// The registered sources' names, in registration order.
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        self.sources.iter().map(|source| source.name()).collect()
    }

    /// Every registered source's open deadlines at `now`, source by source in registration order.
    ///
    /// # Errors
    ///
    /// The first source's error: a cycle that cannot read an obligation cannot know it may skip.
    pub async fn collect(&self, now: UtcMillis) -> Result<Vec<Deadline>, KernelError> {
        let mut deadlines = Vec::new();
        for source in &self.sources {
            deadlines.extend(source.deadlines(now).await?);
        }
        Ok(deadlines)
    }
}

impl fmt::Debug for Obligations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Obligations")
            .field("sources", &self.names())
            .finish()
    }
}
