//! The change gate (SPEC-023 R8, R9, R11): the sync always runs, and the recompute after it is
//! skipped only when nothing it could observe would change.
//!
//! It ports the predecessor's `pipeline.py:GamifyPipeline._maybe_skip_recompute`,
//! `_first_due_obligation` and `anki_reader.py:probe_change_signal` (predecessor `27ee2bc`).
//! [`probe`] reads the copy's cheap signal: the newest revlog id, and the count and weighted
//! fingerprint of the cards, from integer columns only (`goldens/change_probe.json`). [`decide`] is
//! a pure function of the gate's inputs: it runs the recompute, with the first reason that holds, or
//! skips. A deadline registered through `coordination::obligations` is its own term, because a
//! deadline comes due precisely on a cycle in which nothing in the collection changed: an input-keyed
//! gate would skip the recompute that settles it (the defect the predecessor once shipped).
//! [`ChangeGate`] reads the inputs, decides, records a skip, and writes the anchor a full recompute
//! leaves.

use std::sync::Arc;

use deck_streak_kernel::{Clock, Db, KernelError, StudyDay, StudyDayRule, UtcMillis};

use crate::reader::{CollectionReader, ReadError, read_failed};
use crate::state::SqliteIngestState;
use crate::sync_runs::{RunHistory, RunStatus, SkippedRun, SqliteSyncRuns, Trigger};

/// The weights of a card's `due`, `ivl`, `queue`, `factor`, `lapses`, `type` and `odid` in the
/// fingerprint (`anki_reader.py:_CARD_FIELD_WEIGHTS`).
pub const CARD_FIELD_WEIGHTS: [i64; 7] = [131, 137, 139, 149, 151, 157, 163];
/// Each card's weighted term is reduced by this modulus before the sum, so the sum over a large
/// collection stays far below `SQLite`'s 64-bit ceiling (`anki_reader.py:_CARDS_FINGERPRINT_MOD`).
pub const CARD_FINGERPRINT_MODULUS: i64 = 1_000_000_007;

/// The newest revlog id, by the log's integer primary key.
const NEWEST_REVIEW: &str = "SELECT max(id) FROM revlog";

/// The cards' count and fingerprint: one scan of integer columns, each card's weighted term reduced
/// by the modulus before the sum, as the predecessor's `_CARDS_FINGERPRINT_SQL` computes it. The
/// weights are bound as `?1` to `?7`, in [`CARD_FIELD_WEIGHTS`]' order, and the modulus as `?8`:
/// bound integers take the same integer arithmetic as the predecessor's literals.
const CARD_FINGERPRINT: &str = "SELECT count(id), coalesce(sum((due * ?1 + ivl * ?2 + queue * ?3 \
     + factor * ?4 + lapses * ?5 + type * ?6 + odid * ?7) % ?8), 0) FROM cards";

/// The copy's cheap change signal (R9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe {
    /// The newest revlog id, or 0 when the log is empty.
    pub newest_review_id: i64,
    /// How many cards the collection holds.
    pub card_count: i64,
    /// The cards' weighted fingerprint.
    pub card_fingerprint: i64,
}

/// Reads the probe from the copy, read-only (R9). It reads every card and review whatever the
/// scope: a change outside the scope still opens the gate, which costs an unneeded recompute at
/// worst and never a missed one.
///
/// # Errors
///
/// Every [`ReadError`] of the reader.
pub async fn probe(reader: &CollectionReader) -> Result<Probe, ReadError> {
    reader
        .with_copy("probe_change_signal", |copy| async move {
            let newest: Option<i64> = sqlx::query_scalar(NEWEST_REVIEW)
                .fetch_one(copy.reader())
                .await
                .map_err(read_failed)?;
            let [due, ivl, queue, factor, lapses, kind, odid] = CARD_FIELD_WEIGHTS;
            let (card_count, card_fingerprint): (i64, i64) = sqlx::query_as(CARD_FINGERPRINT)
                .bind(due)
                .bind(ivl)
                .bind(queue)
                .bind(factor)
                .bind(lapses)
                .bind(kind)
                .bind(odid)
                .bind(CARD_FINGERPRINT_MODULUS)
                .fetch_one(copy.reader())
                .await
                .map_err(read_failed)?;
            Ok(Probe {
                newest_review_id: newest.unwrap_or(0),
                card_count,
                card_fingerprint,
            })
        })
        .await
}

/// What the last full recompute saw (R11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// The probe the recompute's cycle read.
    pub probe: Probe,
    /// The study day it ran in.
    pub study_day: StudyDay,
    /// When its cycle decided to run it: a deadline at or before this was seen by it.
    pub recomputed_at: UtcMillis,
    /// The settings generation it read.
    pub settings_generation: i64,
}

/// The anchor as `ingest_state` holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchorState {
    /// No full recompute has written one, or an erase cleared it.
    Missing,
    /// The row is gone, or holds only part of an anchor.
    Unreadable,
    /// A whole anchor.
    Present(Anchor),
}

/// One open deadline of a time-driven obligation (R10): its label and the instant it comes due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadline {
    /// What comes due, for the log.
    pub label: &'static str,
    /// When it comes due.
    pub at: UtcMillis,
}

/// Everything [`decide`] reads (R8).
#[derive(Clone, Copy, Debug)]
pub struct GateInputs<'a> {
    /// Whether an owner's rescore is pending.
    pub rescore_pending: bool,
    /// Whether this cycle's sync succeeded.
    pub sync_ok: bool,
    /// The record as the cycle found it before its own sync.
    pub history: RunHistory,
    /// The persisted anchor.
    pub anchor: AnchorState,
    /// The kernel's settings generation now.
    pub settings_generation: i64,
    /// The study day now, by the kernel's rule.
    pub study_day: StudyDay,
    /// The probe this cycle read.
    pub probe: Probe,
    /// Now, by the kernel's clock.
    pub now: UtcMillis,
    /// Every registered obligation's open deadlines.
    pub deadlines: &'a [Deadline],
}

/// Why the gate runs the recompute: the first term of R8 that holds, in R8's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunReason {
    /// The owner asked for a rescore.
    RescorePending,
    /// This cycle's sync failed.
    SyncFailed,
    /// No run on record succeeded.
    NoSuccessfulRun,
    /// The last run on record failed.
    LastRunFailed,
    /// No anchor is persisted.
    AnchorMissing,
    /// The persisted anchor cannot be read whole.
    AnchorUnreadable,
    /// The settings generation moved since the anchor.
    SettingsChanged,
    /// The study day changed since the anchor.
    StudyDayChanged,
    /// The newest review id changed since the anchor.
    NewestReviewChanged,
    /// The card count changed since the anchor.
    CardCountChanged,
    /// The card fingerprint changed since the anchor.
    CardFingerprintChanged,
    /// A registered deadline came due after the anchor's recompute and at or before now.
    DeadlineDue {
        /// The deadline's label.
        label: &'static str,
    },
}

impl RunReason {
    /// The reason as a log line names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RescorePending => "rescore_pending",
            Self::SyncFailed => "sync_failed",
            Self::NoSuccessfulRun => "no_successful_run",
            Self::LastRunFailed => "last_run_failed",
            Self::AnchorMissing => "anchor_missing",
            Self::AnchorUnreadable => "anchor_unreadable",
            Self::SettingsChanged => "settings_changed",
            Self::StudyDayChanged => "study_day_changed",
            Self::NewestReviewChanged => "newest_review_changed",
            Self::CardCountChanged => "card_count_changed",
            Self::CardFingerprintChanged => "card_fingerprint_changed",
            Self::DeadlineDue { .. } => "deadline_due",
        }
    }
}

/// The gate's decision (R8): run the recompute for a reason, or skip it. A caller matches on it.
#[must_use = "a decision is matched on: dropping one runs, or skips, nothing"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Run the recompute.
    Run(RunReason),
    /// Nothing the recompute could observe changed: skip it.
    Skip,
}

/// Decides whether the recompute runs (R8): it runs, with the first reason that holds, when an
/// owner rescore is pending, when this cycle's sync failed, when no run on record succeeded or the
/// last one failed, when the anchor is missing or unreadable, when the settings generation, the
/// study day, or the probe's newest review id, card count or fingerprint changed, or when a deadline
/// lies after the anchor's recompute and at or before now; otherwise it skips.
pub fn decide(inputs: &GateInputs<'_>) -> Decision {
    let reason = if inputs.rescore_pending {
        Some(RunReason::RescorePending)
    } else if !inputs.sync_ok {
        Some(RunReason::SyncFailed)
    } else if !inputs.history.any_success {
        Some(RunReason::NoSuccessfulRun)
    } else if inputs.history.last == Some(RunStatus::Error) {
        Some(RunReason::LastRunFailed)
    } else {
        match inputs.anchor {
            AnchorState::Missing => Some(RunReason::AnchorMissing),
            AnchorState::Unreadable => Some(RunReason::AnchorUnreadable),
            AnchorState::Present(anchor) => changed_since(&anchor, inputs),
        }
    };
    reason.map_or(Decision::Skip, Decision::Run)
}

/// What changed since `anchor`, in R8's order, or `None` when nothing did. A deadline counts once:
/// after the anchor's recompute, which saw every deadline at or before it, and at or before now,
/// never earlier.
fn changed_since(anchor: &Anchor, inputs: &GateInputs<'_>) -> Option<RunReason> {
    if inputs.settings_generation != anchor.settings_generation {
        Some(RunReason::SettingsChanged)
    } else if inputs.study_day != anchor.study_day {
        Some(RunReason::StudyDayChanged)
    } else if inputs.probe.newest_review_id != anchor.probe.newest_review_id {
        Some(RunReason::NewestReviewChanged)
    } else if inputs.probe.card_count != anchor.probe.card_count {
        Some(RunReason::CardCountChanged)
    } else if inputs.probe.card_fingerprint != anchor.probe.card_fingerprint {
        Some(RunReason::CardFingerprintChanged)
    } else {
        inputs
            .deadlines
            .iter()
            .find(|deadline| anchor.recomputed_at < deadline.at && deadline.at <= inputs.now)
            .map(|deadline| RunReason::DeadlineDue {
                label: deadline.label,
            })
    }
}

/// Why the gate could not decide or record.
#[derive(Debug, thiserror::Error)]
pub enum GateError {
    /// The copy could not be probed.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The gate's record could not be read or written.
    #[error("the change gate's record could not be read or written")]
    Record(#[from] KernelError),
}

/// What a cycle brings to the gate from its sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CycleFacts {
    /// What asked for the cycle.
    pub trigger: Trigger,
    /// Whether the cycle's sync succeeded.
    pub sync_ok: bool,
    /// The record as the cycle found it before its sync.
    pub history: RunHistory,
}

/// One decision, with what it was decided on: a full recompute writes its anchor from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checked {
    /// The decision.
    pub decision: Decision,
    /// The probe it read.
    pub probe: Probe,
    /// The study day it decided in.
    pub study_day: StudyDay,
    /// When it decided.
    pub now: UtcMillis,
    /// The settings generation it read.
    pub settings_generation: i64,
}

/// The gate over the service's database: `ingest_state`, `sync_runs` and the kernel's settings
/// generation.
#[derive(Clone)]
pub struct ChangeGate {
    state: SqliteIngestState,
    runs: SqliteSyncRuns,
    db: Db,
    rule: StudyDayRule,
    clock: Arc<dyn Clock>,
}

impl ChangeGate {
    /// The gate over `db`, deciding study days by `rule` on `clock`.
    #[must_use]
    pub fn new(db: Db, rule: StudyDayRule, clock: Arc<dyn Clock>) -> Self {
        Self {
            state: SqliteIngestState::new(db.clone()),
            runs: SqliteSyncRuns::new(db.clone()),
            db,
            rule,
            clock,
        }
    }

    /// The anchor, the rescore flag and the window's base.
    #[must_use]
    pub const fn state(&self) -> &SqliteIngestState {
        &self.state
    }

    /// The record of every run.
    #[must_use]
    pub const fn runs(&self) -> &SqliteSyncRuns {
        &self.runs
    }

    /// The record as it stands, which a cycle reads before its sync.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn history(&self) -> Result<RunHistory, KernelError> {
        self.runs.history().await
    }

    /// Probes the copy through `reader`, reads the persisted inputs, and decides (R8, R9); a skip is
    /// recorded as a `skipped` row and leaves the anchor as it was (R11).
    ///
    /// # Errors
    ///
    /// [`GateError::Read`] when the copy cannot be probed, and [`GateError::Record`] when the
    /// state, the settings generation or the record cannot be read or the skip written.
    pub async fn check(
        &self,
        reader: &CollectionReader,
        facts: &CycleFacts,
        deadlines: &[Deadline],
    ) -> Result<Checked, GateError> {
        let now = self.clock.now();
        let probe = probe(reader).await?;
        let state = self.state.load().await?;
        let settings_generation = self.db.settings_generation().await?;
        let study_day = self.rule.study_day(now);
        let decision = decide(&GateInputs {
            rescore_pending: state.rescore_pending,
            sync_ok: facts.sync_ok,
            history: facts.history,
            anchor: state.anchor,
            settings_generation,
            study_day,
            probe,
            now,
            deadlines,
        });
        match decision {
            Decision::Skip => {
                self.runs
                    .record_skipped(&SkippedRun {
                        trigger: facts.trigger,
                        started_at: now,
                        finished_at: self.clock.now(),
                        study_day,
                    })
                    .await?;
            }
            Decision::Run(reason) => {
                tracing::info!(
                    reason = reason.as_str(),
                    "the change gate runs the recompute"
                );
            }
        }
        Ok(Checked {
            decision,
            probe,
            study_day,
            now,
            settings_generation,
        })
    }

    /// Writes the anchor a full recompute leaves (R11): what its cycle decided on, which it saw at
    /// least, and clears the rescore flag the recompute served.
    ///
    /// # Errors
    ///
    /// [`GateError::Record`] when the anchor cannot be written.
    pub async fn recomputed(&self, checked: &Checked) -> Result<(), GateError> {
        let anchor = Anchor {
            probe: checked.probe,
            study_day: checked.study_day,
            recomputed_at: checked.now,
            settings_generation: checked.settings_generation,
        };
        self.state.write_anchor(&anchor, self.clock.now()).await?;
        Ok(())
    }
}
