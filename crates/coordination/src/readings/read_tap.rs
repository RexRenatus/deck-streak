//! The owner's tap on a reading (SPEC-047 R1, R2, R6): the first tap sets `read_at`, grants the
//! reading's read XP once and ticks the vault note's `I read it` line; a later tap changes nothing
//! and retries only a vault tick that failed.
//!
//! This is the one caller of the vault's read tick. The vault and the grant come through ports, so
//! no reading code writes the note or the ledger itself.

use std::sync::Arc;

use deck_streak_kernel::{Clock, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_progression::grant::{GrantPort, GrantRequest, GrantScope, GrantSource};
use deck_streak_progression::xp::XpAmount;
use deck_streak_readings::reading::ReadingId;
use deck_streak_readings::store::{ReadingProgress, SqliteReadings, VaultTick};
pub use deck_streak_readings::studied::Verdict;
use deck_streak_readings::topic::TopicKey;
use deck_streak_readings::xp::{READ_XP, read_source, track_of};

use super::generate::{PortFuture, VaultWriteFailed};

/// Ticks the owner's `I read it` line of a reading's live note (SPEC-042's read tick).
pub trait ReadTick: Send + Sync {
    /// Ticks the line of `topic`'s live note, found from `today`.
    fn tick_read<'a>(
        &'a self,
        topic: &'a TopicKey,
        today: StudyDay,
    ) -> PortFuture<'a, Result<(), VaultWriteFailed>>;
}

/// Why a tap was refused.
#[derive(Debug, thiserror::Error)]
pub enum TapError {
    /// No stored ready reading has that id: nothing was granted or written.
    #[error("no such reading")]
    NotFound,
    /// The record or the ledger could not be reached.
    #[error("the reading could not be read or written")]
    Unavailable,
}

/// What a tap answers: the reading's first `read_at` and where its measure stands (R9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapAnswer {
    /// The reading's id.
    pub id: String,
    /// When the owner first tapped it.
    pub read_at: UtcMillis,
    /// Whether this tap was the first.
    pub first: bool,
    /// The covered card count.
    pub covered: u32,
    /// The covered cards found studied at the last settle.
    pub studied: u32,
    /// The verdict at the last settle.
    pub verdict: Verdict,
}

/// The tap as the API sees it, by the reading's id as text.
pub trait ReadTapPort: Send + Sync {
    /// Taps the reading `id`.
    fn tap<'a>(&'a self, id: &'a str) -> PortFuture<'a, Result<TapAnswer, TapError>>;
}

/// The tap use case.
pub struct ReadTap<G, V> {
    readings: SqliteReadings,
    grants: G,
    vault: V,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
}

impl<G: GrantPort, V: ReadTick> ReadTap<G, V> {
    /// The use case over its record, its grant port, its vault, the services clock and the rule.
    pub const fn new(
        readings: SqliteReadings,
        grants: G,
        vault: V,
        clock: Arc<dyn Clock>,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            readings,
            grants,
            vault,
            clock,
            rule,
        }
    }

    /// Taps the reading `id`.
    ///
    /// # Errors
    ///
    /// [`TapError::NotFound`] for an unknown id, [`TapError::Unavailable`] when the record or the
    /// ledger fails.
    pub async fn tap_reading(&self, id: &str) -> Result<TapAnswer, TapError> {
        let id = ReadingId::parse(id).ok_or(TapError::NotFound)?;
        let progress = self.progress(&id).await?;
        let now = self.clock.now();
        let today = self.rule.study_day(now);

        // A later tap changes nothing: it answers the first instant, and retries only a vault
        // tick that never landed.
        if let Some(read_at) = progress.read_at {
            if progress.vault_tick != VaultTick::Written {
                self.tick(&id, &progress.topic, today).await?;
            }
            return Ok(answer(&progress, read_at, false));
        }

        // The grant is once-scoped and idempotent, so a crash between it and the mark is safe: the
        // next tap grants nothing new and marks.
        let request = GrantRequest {
            study_day: today,
            source: GrantSource::new(&read_source(&id)).map_err(|_| TapError::Unavailable)?,
            track: track_of(&progress.topic),
            amount: XpAmount::new(READ_XP),
            scope: GrantScope::Once,
        };
        let _ = self
            .grants
            .grant(&request, now)
            .await
            .map_err(|_| TapError::Unavailable)?;
        let first = self
            .readings
            .mark_read(&id, now)
            .await
            .map_err(|_| TapError::Unavailable)?;
        // The first writer wins: a tap that lost the race answers the winner's instant.
        let stored = if first {
            progress
        } else {
            self.progress(&id).await?
        };
        let read_at = stored.read_at.unwrap_or(now);
        self.tick(&id, &stored.topic, today).await?;
        Ok(answer(&stored, read_at, first))
    }

    async fn progress(&self, id: &ReadingId) -> Result<ReadingProgress, TapError> {
        self.readings
            .progress(id)
            .await
            .map_err(|_| TapError::Unavailable)?
            .ok_or(TapError::NotFound)
    }

    /// Ticks the vault line and records whether it landed. A failed tick is not a failed tap.
    async fn tick(
        &self,
        id: &ReadingId,
        topic: &TopicKey,
        today: StudyDay,
    ) -> Result<(), TapError> {
        let landed = self.vault.tick_read(topic, today).await.is_ok();
        let state = if landed {
            VaultTick::Written
        } else {
            VaultTick::Pending
        };
        self.readings
            .set_vault_tick(id, state)
            .await
            .map_err(|_| TapError::Unavailable)
    }
}

/// A reading's tap answer at `read_at`.
fn answer(progress: &ReadingProgress, read_at: UtcMillis, first: bool) -> TapAnswer {
    TapAnswer {
        id: progress.id.as_str().to_owned(),
        read_at,
        first,
        covered: u32::try_from(progress.card_ids.len()).unwrap_or(u32::MAX),
        studied: progress.studied_count,
        verdict: progress.verdict,
    }
}

impl<G: GrantPort, V: ReadTick> ReadTapPort for ReadTap<G, V> {
    fn tap<'a>(&'a self, id: &'a str) -> PortFuture<'a, Result<TapAnswer, TapError>> {
        Box::pin(self.tap_reading(id))
    }
}
