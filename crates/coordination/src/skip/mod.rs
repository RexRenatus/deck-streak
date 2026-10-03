//! The skip day's coordination (SPEC-083 R7 to R9, R11; ADR-321 D6, D10, D13): the preview, the
//! settlement of an applied take with its tariff, and the undo's refund.

pub mod days;

use deck_streak_ingest::skip::{SearchRefusal, SkipId};
use deck_streak_kernel::{Db, KernelError, StudyDay, StudyDayRule, UtcMillis};

/// What the preview shows (R7): the golden's fields, and no card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkipPreview {
    /// The study day a skip would cover.
    pub day: StudyDay,
    /// The day's due review count; absent when the day has no rollup.
    pub due_count: Option<i64>,
    /// Whether the day already holds an applied skip not undone.
    pub already_skipped: bool,
    /// The search the write would run.
    pub search: String,
    /// The day spec the write would set.
    pub spec: String,
    /// The tariff a skip of the day would cost.
    pub tariff: i64,
    /// Whether the wallet holds the tariff.
    pub tariff_funded: bool,
}

/// Why the preview could not be shown.
#[derive(Debug, thiserror::Error)]
pub enum PreviewRefusal {
    /// The configured search is not one expression.
    #[error(transparent)]
    Search(#[from] SearchRefusal),
    /// The database refused a read.
    #[error(transparent)]
    Database(#[from] KernelError),
}

/// What one applied take's settlement charged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settlement {
    /// The skip's price.
    pub price: i64,
    /// The coins the wallet paid.
    pub paid: i64,
    /// Whether the skip went unfunded.
    pub unfunded: bool,
}

/// The preview of a skip of `today`.
///
/// # Errors
///
/// [`PreviewRefusal`] when the search is refused or a read fails.
#[allow(
    clippy::unused_async,
    reason = "the red stub keeps the green signature, which reads the ledger"
)]
pub async fn preview(
    db: &Db,
    today: StudyDay,
    configured: &str,
) -> Result<SkipPreview, PreviewRefusal> {
    let _ = (db, configured);
    Ok(SkipPreview {
        day: today,
        due_count: None,
        already_skipped: false,
        search: String::new(),
        spec: String::new(),
        tariff: 0,
        tariff_funded: false,
    })
}

/// Settles the skip `id` applied, with its tariff.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
#[allow(
    clippy::unused_async,
    reason = "the red stub keeps the green signature, which writes the ledger"
)]
pub async fn settle_applied(
    db: &Db,
    id: SkipId,
    cards_moved: i64,
    now: UtcMillis,
) -> Result<Option<Settlement>, KernelError> {
    let _ = (db, id, cards_moved, now);
    Ok(None)
}

/// Marks the skip `id` undone and refunds what it paid.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
#[allow(
    clippy::unused_async,
    reason = "the red stub keeps the green signature, which writes the ledger"
)]
pub async fn settle_undone(
    db: &Db,
    rule: StudyDayRule,
    id: SkipId,
    now: UtcMillis,
) -> Result<i64, KernelError> {
    let _ = (db, rule, id, now);
    Ok(0)
}
