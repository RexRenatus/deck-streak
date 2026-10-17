//! The ingest window (SPEC-023 R6): only the last [`INGEST_WINDOW_DAYS`] of reviews are read each
//! cycle, and every study event older than the window is one persisted count.
//!
//! It ports the predecessor's `pipeline.py:GamifyPipeline._maybe_rebase_ingest` and
//! `anki_reader.py:count_study_reviews_before` (predecessor `27ee2bc`): reviews newer than the floor
//! are read; the floor is the persisted base's, or [`INGEST_WINDOW_DAYS`] days before now when there
//! is none; the base is recounted in SQL when it is more than [`INGEST_REBASE_DAYS`] days staler than
//! a fresh floor; and a recount lower than the stored count is the self-check, logged once (cards
//! deleted or the collection replaced). `goldens/ingest_rebase.json` and
//! `goldens/ingest.constants.json` prove the rule and the constants. The bounded window keeps the
//! read inside the memory budget of the unit that runs it (ADR-032).

use deck_streak_kernel::{KernelError, UtcMillis};

use crate::reader::{
    CollectionData, CollectionReader, ReadError, allowed_deck_ids, deck_names, read_failed,
    scope_ids,
};
use crate::state::SqliteIngestState;

/// The days of reviews each cycle reads (`constants.py:INGEST_WINDOW_DAYS`).
pub const INGEST_WINDOW_DAYS: i64 = 400;
/// How many days staler than a fresh floor a base may be before it is recounted
/// (`constants.py:INGEST_REBASE_DAYS`).
pub const INGEST_REBASE_DAYS: i64 = 7;

const DAY_MS: i64 = 86_400_000;

/// The study events at or before the floor `?1` of the cards whose home deck is one of the ids in
/// `?2`: the SQL mirror of `reader::is_study_event` and `reader::Card::home_deck_id`, as the
/// predecessor's `anki_reader.py:count_study_reviews_before` wrote it. A card deleted since takes its
/// reviews out of the count, which is what the self-check sees. `r.id <= ?1` is the exact complement
/// of the read's `r.id > ?1`, so a read above the floor and this count partition the log.
const STUDY_EVENTS_BEFORE: &str = "SELECT count(*) FROM revlog r JOIN cards c ON c.id = r.cid \
     WHERE r.id <= ?1 AND r.type IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) IN (SELECT value FROM json_each(?2))";

/// The window's base: the floor, a review id (which is epoch milliseconds), and the count of study
/// events of the cards in scope at or before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowBase {
    /// The floor: reviews newer than it are read.
    pub floor: i64,
    /// The study events at or before the floor.
    pub count: i64,
}

/// What a cycle does with the base (R6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rebase {
    /// The base is fresh enough: kept as it is.
    Keep(WindowBase),
    /// There is no base, or it is stale: recount the study events at or before `floor`.
    Recount {
        /// The fresh floor to recount before.
        floor: i64,
    },
}

/// The self-check (R6): a recount lower than the stored count, which only deleted cards or a
/// replaced collection explain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelfCheck {
    /// The count the base held.
    pub stored: i64,
    /// The count the recount found.
    pub recounted: i64,
}

/// A fresh floor at `now`: [`INGEST_WINDOW_DAYS`] days before it, never before the epoch.
#[must_use]
pub fn fresh_floor(now: UtcMillis) -> i64 {
    now.epoch_millis()
        .saturating_sub(INGEST_WINDOW_DAYS * DAY_MS)
        .max(0)
}

/// The floor a read at `now` reads above: the base's, or a fresh one when there is none.
#[must_use]
pub fn read_floor(base: Option<WindowBase>, now: UtcMillis) -> i64 {
    base.map_or_else(|| fresh_floor(now), |base| base.floor)
}

/// Whether the base at `now` is kept or recounted: it is recounted when there is none, or when it
/// is more than [`INGEST_REBASE_DAYS`] days staler than a fresh floor.
#[must_use]
pub fn rebase(base: Option<WindowBase>, now: UtcMillis) -> Rebase {
    let limit = now
        .epoch_millis()
        .saturating_sub((INGEST_WINDOW_DAYS + INGEST_REBASE_DAYS) * DAY_MS);
    match base {
        Some(base) if base.floor >= limit => Rebase::Keep(base),
        _ => Rebase::Recount {
            floor: fresh_floor(now),
        },
    }
}

/// The base a recount of `count` study events at or before `floor` writes, and the self-check when
/// `count` is lower than what `previous` held. A failed self-check logs one WARN.
#[must_use]
pub fn rebased(
    previous: Option<WindowBase>,
    floor: i64,
    count: i64,
) -> (WindowBase, Option<SelfCheck>) {
    let self_check = previous
        .filter(|previous| count < previous.count)
        .map(|previous| SelfCheck {
            stored: previous.count,
            recounted: count,
        });
    if let Some(check) = self_check {
        tracing::warn!(
            stored = check.stored,
            recounted = check.recounted,
            "ingest self-check: the study events before the window fell (cards deleted or the \
             collection replaced)"
        );
    }
    (WindowBase { floor, count }, self_check)
}

/// One cycle's read inside the window (R6): the data, the base the cycle kept or wrote, and the
/// self-check when the recount shrank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowRead {
    /// What the read returned.
    pub data: CollectionData,
    /// The base after the cycle.
    pub base: WindowBase,
    /// The self-check, when a recount came back lower than the stored count.
    pub self_check: Option<SelfCheck>,
}

/// Why the window could not be read.
#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    /// The copy could not be read.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The window's base could not be read or written.
    #[error("the window's base could not be read or written")]
    State(#[from] KernelError),
}

/// Reads the window at `now` (R6): the reviews above the floor, then the base kept or recounted,
/// and a recount written back.
///
/// # Errors
///
/// [`WindowError::Read`] when the copy cannot be read, and [`WindowError::State`] when the base
/// cannot be read or written.
pub async fn read_window(
    reader: &CollectionReader,
    state: &SqliteIngestState,
    now: UtcMillis,
) -> Result<WindowRead, WindowError> {
    let stored = state.load().await?.window_base;
    let data = reader.read(read_floor(stored, now)).await?;
    let (base, self_check) = match rebase(stored, now) {
        Rebase::Keep(base) => (base, None),
        Rebase::Recount { floor } => {
            let (base, self_check) = rebased(stored, floor, recount(reader, floor).await?);
            state.write_window_base(base, now).await?;
            (base, self_check)
        }
    };
    Ok(WindowRead {
        data,
        base,
        self_check,
    })
}

/// Counts the study events of the cards in `reader`'s scope at or before `floor`, in SQL, on the
/// read-only copy: one indexed count, on the offload, once per rebase period.
async fn recount(reader: &CollectionReader, floor: i64) -> Result<i64, ReadError> {
    let prefixes = reader.scope().include().prefixes().to_vec();
    reader
        .with_copy("count_study_reviews_before", move |copy| async move {
            let scope = scope_ids(&allowed_deck_ids(&deck_names(&copy).await?, &prefixes));
            sqlx::query_scalar(STUDY_EVENTS_BEFORE)
                .bind(floor)
                .bind(scope)
                .fetch_one(copy.reader())
                .await
                .map_err(read_failed)
        })
        .await
}
