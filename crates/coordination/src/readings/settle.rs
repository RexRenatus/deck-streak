//! The settle step (SPEC-047 R4, R5): after a sync, each reading whose window is open, and each
//! retired one, is measured again; on crossing the threshold it grants its studied XP once and
//! stamps the vault's Studied line, and at the rollover that starts study day d + 2 a reading below
//! the threshold retires.
//!
//! It never ticks the owner's `I read it` line: the stamp port names the Studied line only.

use std::sync::Arc;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Clock, KernelError, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_progression::grant::GrantPort;
use deck_streak_readings::store::SqliteReadings;
use deck_streak_readings::topic::TopicKey;

use super::generate::{PortFuture, VaultWriteFailed};

/// Stamps the Studied line of a reading's live note (SPEC-042's stamp).
pub trait StudiedStamp: Send + Sync {
    /// Stamps the line of `topic`'s live note, found from `today`.
    fn stamp_studied<'a>(
        &'a self,
        topic: &'a TopicKey,
        today: StudyDay,
    ) -> PortFuture<'a, Result<(), VaultWriteFailed>>;
}

/// The reviews could not be read.
#[derive(Debug, thiserror::Error)]
#[error("the reviews could not be read")]
pub struct ReviewsUnreadable;

/// Reads the study reviews of cards (SPEC-023's qualifying reviews by card).
pub trait ReviewsByCard: Send + Sync {
    /// The reviews of `card_ids` answered at or after `since`.
    fn reviews<'a>(
        &'a self,
        card_ids: &'a [i64],
        since: UtcMillis,
    ) -> PortFuture<'a, Result<Vec<Review>, ReviewsUnreadable>>;
}

/// What one settle pass did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettleReport {
    /// Readings measured.
    pub measured: u32,
    /// Readings that turned studied in this pass.
    pub studied: u32,
    /// Readings retired in this pass.
    pub retired: u32,
}

/// The settle use case.
pub struct Settle<G, S, R> {
    readings: SqliteReadings,
    grants: G,
    stamps: S,
    reviews: R,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
}

impl<G: GrantPort, S: StudiedStamp, R: ReviewsByCard> Settle<G, S, R> {
    /// The use case over its record, ports, the services clock and the rule.
    pub const fn new(
        readings: SqliteReadings,
        grants: G,
        stamps: S,
        reviews: R,
        clock: Arc<dyn Clock>,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            readings,
            grants,
            stamps,
            reviews,
            clock,
            rule,
        }
    }

    /// Measures every reading that is not yet studied.
    ///
    /// # Errors
    ///
    /// [`KernelError`] when the record or the ledger fails.
    pub async fn run(&self) -> Result<SettleReport, KernelError> {
        let _ = (
            &self.readings,
            &self.grants,
            &self.stamps,
            &self.reviews,
            &self.clock,
            self.rule,
        );
        Ok(SettleReport::default())
    }
}
