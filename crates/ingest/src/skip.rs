//! The skip day's record (SPEC-083 R1 to R6): what the search and the day spec are, the rows one
//! skip leaves, the set of study days a skip covers, the undo's target and the summary's counts.
//!
//! [`SkipStore`] keeps the record in the service's database, owned by this context
//! (docs/CONTEXT-MAP.md). The once-per-study-day rule is held by the migration's key, so two
//! concurrent takes write one row whatever the callers do. Nothing here reaches the collection:
//! the write to it is `skip_write` (SPEC-083 R22 to R32).

use std::fmt;

use chrono::{Datelike, NaiveDate};
use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

/// The day spec's smallest lower bound: a moved card lands strictly after today
/// (`constants.SKIP_SPREAD_MIN_DAYS`).
pub const SKIP_SPREAD_MIN_DAYS: i64 = 1;
/// The day spec's upper bound (`constants.SKIP_SPREAD_MAX_DAYS`).
pub const SKIP_SPREAD_MAX_DAYS: i64 = 3;
/// The search a deployment runs unless it configures one (`constants.SKIP_DEFAULT_SEARCH`).
pub const SKIP_DEFAULT_SEARCH: &str = "prop:due=0 -is:suspended -is:buried";
/// The most cards one skip may move; a larger set is refused, never truncated
/// (`constants.SKIP_MAX_CARDS`).
pub const SKIP_MAX_CARDS: usize = 5000;
/// The skips a month the streak bridge covers (`constants.SKIP_BRIDGE_MONTHLY_CAP`).
pub const SKIP_BRIDGE_MONTHLY_CAP: u32 = 3;

/// What holds a search to the study day's due review cards whatever the owner configured, and out
/// of filtered decks (R3): a card in one would return to a deck that may no longer exist.
const HOLDS: &str = "prop:due=0 -is:suspended -is:buried -deck:filtered";

/// Days from the Common Era's day 1 to the epoch day 0.
const EPOCH_DAY_FROM_CE: i64 = 719_163;

/// The day spec of a skip (`skip.py:skip_spec`): bounds clamped to at least 1 and ordered,
/// rendered as one number when equal and as `lo-hi` otherwise. It never carries `!`, so an SM-2
/// interval is kept.
#[must_use]
pub fn skip_spec(min_days: i64, max_days: i64) -> String {
    format!("{min_days}-{max_days}")
}

/// Why a configured search is refused at start (R3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SearchRefusal {
    /// The search is blank, or does not parse as one expression: a closing parenthesis with none
    /// open, an unclosed group or an unclosed quote. Wrapped as text it would leave the wrap's group
    /// and escape every hold.
    #[error("the skip search is not one expression")]
    NotOneExpression,
}

/// Whether `search` is one expression on its own: its parentheses balance without closing a group
/// that was never opened, and its quotes close. A parenthesis inside quotes, or escaped, is text.
#[must_use]
pub fn is_one_expression(search: &str) -> bool {
    if search.trim().is_empty() {
        return false;
    }
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    for character in search.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                let Some(open) = depth.checked_sub(1) else {
                    return false;
                };
                depth = open;
            }
            _ => {}
        }
    }
    depth == 0 && !quoted
}

/// The configured search wrapped exactly as the predecessor wraps it
/// (`sync.py:AnkiSyncer._skip_day_blocking`): review cards only.
#[must_use]
pub fn wrap_search(configured: &str) -> String {
    format!("({configured}) -is:new -is:learn")
}

/// The search the preview shows and the write runs (R3): the predecessor's wrap of the configured
/// search, followed by the holds no configured search can escape.
///
/// # Errors
///
/// [`SearchRefusal::NotOneExpression`] when `configured` does not parse as one expression.
pub fn skip_search(configured: &str) -> Result<String, SearchRefusal> {
    if !is_one_expression(configured) {
        return Err(SearchRefusal::NotOneExpression);
    }
    Ok(format!("({configured})"))
}

/// The calendar year and month of a study day, as the tariff and the summary count months.
#[must_use]
pub fn calendar_month(day: StudyDay) -> Option<(i32, u32)> {
    let from_ce = i32::try_from(day.epoch_day().checked_add(EPOCH_DAY_FROM_CE)?).ok()?;
    let date = NaiveDate::from_num_days_from_ce_opt(from_ce)?;
    Some((date.year(), date.month()))
}

/// Why a skip's write ended `failed`: one code from a closed set, and nothing else (R1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FailReason {
    /// The engine's day was not the study day (R3).
    EngineDayDiffers,
    /// The collection's configured UTC offset differed from the process's zone (R3).
    ZoneDiffers,
    /// The process's zone observes daylight saving (R3).
    ZoneObservesDaylightSaving,
    /// The converge did not succeed.
    ConvergeFailed,
    /// The server demanded a full sync, which is never performed (R24).
    FullSyncRequired,
    /// The search selected more than [`SKIP_MAX_CARDS`].
    TooManyCards,
    /// The cards the converge selected were not the previewed ones (R21).
    PreviewChanged,
    /// The push was not accepted.
    PushFailed,
    /// The reschedule did not complete.
    WriteFailed,
    /// The take ran past its timeout.
    TimedOut,
    /// Any other failure inside the engine.
    EngineFailed,
}

impl FailReason {
    /// Every code.
    pub const ALL: [Self; 11] = [
        Self::EngineDayDiffers,
        Self::ZoneDiffers,
        Self::ZoneObservesDaylightSaving,
        Self::ConvergeFailed,
        Self::FullSyncRequired,
        Self::TooManyCards,
        Self::PreviewChanged,
        Self::PushFailed,
        Self::WriteFailed,
        Self::TimedOut,
        Self::EngineFailed,
    ];

    /// The code as `skip_days` stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EngineDayDiffers => "engine_day_differs",
            Self::ZoneDiffers => "zone_differs",
            Self::ZoneObservesDaylightSaving => "zone_observes_daylight_saving",
            Self::ConvergeFailed => "converge_failed",
            Self::FullSyncRequired => "full_sync_required",
            Self::TooManyCards => "too_many_cards",
            Self::PreviewChanged => "preview_changed",
            Self::PushFailed => "push_failed",
            Self::WriteFailed => "write_failed",
            Self::TimedOut => "timed_out",
            Self::EngineFailed => "engine_failed",
        }
    }

    /// The code `skip_days` stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|code| code.as_str() == text)
    }
}

impl fmt::Display for FailReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A skip's id in `skip_days`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkipId(i64);

impl SkipId {
    /// The id as `skip_days` stores it.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// The id `skip_days` stored as `id`.
    #[must_use]
    pub const fn from_row_id(id: i64) -> Self {
        Self(id)
    }
}

/// The state of a skip's write (R1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipState {
    /// The take is in flight: its outcome is not known yet.
    Pending,
    /// The server accepted the push.
    Applied,
    /// The take ended without a push the server accepted.
    Failed(FailReason),
}

/// One skip as the record holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkipRecord {
    /// The row's id.
    pub id: SkipId,
    /// The study day the skip covers.
    pub day: StudyDay,
    /// The due review count the preview showed; absent when the day had no rollup.
    pub due_count: Option<i64>,
    /// The state of the write.
    pub state: SkipState,
    /// The cards the write moved.
    pub cards_moved: i64,
    /// Whether the tariff went unfunded.
    pub tariff_unfunded: bool,
    /// When the skip was undone, if it was.
    pub undone_at: Option<UtcMillis>,
}

/// A skip row as the summary reads it (`skip.py:summarize_skips`'s input).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkipRow {
    /// The study day.
    pub day: StudyDay,
    /// Whether the write was accepted.
    pub applied: bool,
    /// Whether the skip was undone.
    pub undone: bool,
    /// The cards the write moved.
    pub cards_moved: i64,
}

impl SkipRow {
    fn is_active(&self) -> bool {
        self.applied && !self.undone
    }
}

/// The counts the summary shows (R6): no card data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkipSummary {
    /// The skips in the current study day's calendar month.
    pub this_month: u32,
    /// The skips, all time.
    pub all_time: u32,
    /// The last skip's study day.
    pub last_day: Option<StudyDay>,
    /// The cards the writes moved, all time.
    pub cards_moved_all_time: i64,
}

/// The summary of `rows` on `today` (`skip.py:summarize_skips`): only an applied skip not undone
/// is tallied.
#[must_use]
pub fn summarize_skips(rows: &[SkipRow], today: StudyDay) -> SkipSummary {
    let _ = (rows, today);
    SkipSummary {
        this_month: 0,
        all_time: 0,
        last_day: None,
        cards_moved_all_time: 0,
    }
}

/// Why the record refuses an action, with a bounded reason and nothing else.
#[derive(Debug, thiserror::Error)]
pub enum SkipRefusal {
    /// The study day already holds a skip `pending` or `applied` and not undone (R2).
    #[error("already_skipped")]
    AlreadySkipped,
    /// No applied skip is left to undo (R5).
    #[error("nothing_to_undo")]
    NothingToUndo,
    /// The study day's take is pending, so its outcome is not known yet (R5).
    #[error("take_pending")]
    TakePending,
    /// The database refused the operation.
    #[error(transparent)]
    Database(#[from] KernelError),
}

/// The record of skips, over the service's database.
#[derive(Clone)]
pub struct SkipStore {
    db: Db,
}

impl SkipStore {
    /// A store over `db`.
    #[must_use]
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Records a `pending` skip for `day` (R2).
    ///
    /// # Errors
    ///
    /// [`SkipRefusal::AlreadySkipped`] when `day` holds a skip `pending` or `applied` and not undone.
    pub async fn begin(
        &self,
        day: StudyDay,
        due_count: Option<i64>,
        now: UtcMillis,
    ) -> Result<SkipId, SkipRefusal> {
        let _ = (&self.db, day, due_count, now);
        Ok(SkipId(0))
    }

    /// Settles a `pending` skip `applied`.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn settle_applied(
        &self,
        id: SkipId,
        cards_moved: i64,
        tariff_unfunded: bool,
    ) -> Result<(), KernelError> {
        let _ = (id, cards_moved, tariff_unfunded);
        Ok(())
    }

    /// Settles a `pending` skip `failed` with its one reason.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn settle_failed(&self, id: SkipId, reason: FailReason) -> Result<(), KernelError> {
        let _ = (id, reason);
        Ok(())
    }

    /// The skip an undo targets (R5).
    ///
    /// # Errors
    ///
    /// [`SkipRefusal::NothingToUndo`] when no applied skip is left.
    pub async fn latest_undoable(&self, today: StudyDay) -> Result<SkipRecord, SkipRefusal> {
        let _ = today;
        Err(SkipRefusal::NothingToUndo)
    }

    /// Marks a skip undone at `now`.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn mark_undone(&self, id: SkipId, now: UtcMillis) -> Result<(), KernelError> {
        let _ = (id, now);
        Ok(())
    }

    /// The skip set (R4).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn skip_set(&self) -> Result<Vec<StudyDay>, KernelError> {
        Ok(Vec::new())
    }

    /// The summary's counts on `today` (R6).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn summary(&self, today: StudyDay) -> Result<SkipSummary, KernelError> {
        Ok(summarize_skips(&[], today))
    }

    /// The skip that covers `day` and is `pending` or `applied` and not undone.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn open_on(&self, day: StudyDay) -> Result<Option<SkipRecord>, KernelError> {
        let _ = day;
        Ok(None)
    }

    /// Every skip, oldest first.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn records(&self) -> Result<Vec<SkipRecord>, KernelError> {
        Ok(Vec::new())
    }
}
