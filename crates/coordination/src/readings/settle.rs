//! The settle step (SPEC-047 R4, R5): after a sync, each reading whose window is open, and each
//! retired one, is measured again; on crossing the threshold it grants its studied XP once and
//! stamps the vault's Studied line, and at the rollover that starts study day d + 2 a reading below
//! the threshold retires.
//!
//! It never ticks the owner's `I read it` line: the stamp port names the Studied line only.

use std::sync::Arc;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Clock, KernelError, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_progression::grant::{GrantPort, GrantRequest, GrantScope, GrantSource};
use deck_streak_progression::xp::XpAmount;
use deck_streak_readings::store::{SqliteReadings, StoreError};
use deck_streak_readings::studied::{Verdict, Window, is_studied, studied_count};
use deck_streak_readings::topic::TopicKey;
use deck_streak_readings::xp::{STUDIED_XP, studied_source, track_of};

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
    /// A reading whose reviews cannot be read is left as it was and measured on the next pass.
    ///
    /// # Errors
    ///
    /// [`KernelError`] when the record or the ledger fails.
    pub async fn run(&self) -> Result<SettleReport, KernelError> {
        let mut report = SettleReport::default();
        let now = self.clock.now();
        let today = self.rule.study_day(now);
        for reading in self.readings.unsettled().await.map_err(store_failed)? {
            let Ok(reviews) = self
                .reviews
                .reviews(&reading.card_ids, reading.generated_at)
                .await
            else {
                continue;
            };
            let window = Window {
                generated_at: reading.generated_at,
                study_day: reading.study_day,
            };
            let count = studied_count(&window, self.rule, &reading.card_ids, &reviews);
            let covered = u32::try_from(reading.card_ids.len()).unwrap_or(u32::MAX);
            report.measured += 1;
            if is_studied(count, covered) {
                let request = GrantRequest {
                    study_day: today,
                    source: GrantSource::new(&studied_source(&reading.id)).map_err(|_| {
                        KernelError::Offload {
                            operation: "a reading's grant source",
                        }
                    })?,
                    track: track_of(&reading.topic),
                    amount: XpAmount::new(STUDIED_XP),
                    scope: GrantScope::Once,
                };
                let _ = self.grants.grant(&request, now).await?;
                // The verdict turns only once the line is stamped, so a failed stamp is retried by
                // the next pass; the grant above is once-scoped, so the retry earns nothing twice.
                if self
                    .stamps
                    .stamp_studied(&reading.topic, today)
                    .await
                    .is_ok()
                {
                    self.readings
                        .record_measure(&reading.id, count, Verdict::Studied, Some(now))
                        .await?;
                    report.studied += 1;
                } else {
                    self.readings
                        .record_measure(&reading.id, count, reading.verdict, None)
                        .await?;
                }
            } else {
                let verdict = if window.is_over(self.rule, now) {
                    Verdict::Retired
                } else {
                    Verdict::Open
                };
                if verdict == Verdict::Retired && reading.verdict != Verdict::Retired {
                    report.retired += 1;
                }
                self.readings
                    .record_measure(&reading.id, count, verdict, None)
                    .await?;
            }
        }
        Ok(report)
    }
}

/// A failed read of the record, as the kernel's error the pass answers with.
fn store_failed(error: StoreError) -> KernelError {
    match error {
        StoreError::Kernel(error) => error,
        StoreError::Unreadable { .. } => KernelError::Offload {
            operation: "reading a stored reading",
        },
    }
}
