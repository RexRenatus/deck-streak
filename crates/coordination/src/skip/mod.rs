//! The skip day's coordination (SPEC-083 R7 to R9, R11; ADR-321 D6, D10, D13): the preview, the
//! settlement of an applied take with its tariff, and the undo's refund.
//!
//! The record is ingest's, the tariff's ladder and price are economy's, and the coins move through
//! the wallet's ports; this module joins them inside one write, so a settled record and its charge
//! (or an undone record and its refund) are committed together or not at all (D10). A skip is
//! priced by the applied skips not undone on an EARLIER study day of its calendar month (D13), so
//! a retry is priced as the first attempt was, whatever applied after it. The charge is keyed to
//! the skip's own study day and the refund to the undo's (D6), so each is taken once per skip.

pub mod days;

use deck_streak_analytics::rollup::stored;
use deck_streak_economy::tariff::{self, REFUND_SOURCE, TARIFF_SOURCE};
use deck_streak_economy::wallet::{self, DebitAnswer, DepositAnswer, SqliteWallet};
use deck_streak_ingest::skip::{
    SKIP_SPREAD_MAX_DAYS, SKIP_SPREAD_MIN_DAYS, SearchRefusal, SkipId, SkipRecord, SkipState,
    calendar_month, mark_undone_on, records_on, settle_applied_on, skip_search, skip_spec,
};
use deck_streak_kernel::{Db, KernelError, StudyDay, StudyDayRule, UtcMillis};

/// What the preview shows (R7): the golden's fields, and no card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkipPreview {
    /// The study day a skip would cover.
    pub day: StudyDay,
    /// The day's due review count; absent when the day has no rollup.
    pub due_count: Option<i64>,
    /// Whether the day already holds a skip `pending` or `applied` and not undone, which a take
    /// would refuse.
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

/// The applied skips not undone on a study day before `day` in `day`'s calendar month (D13).
fn earlier_in_month(records: &[SkipRecord], day: StudyDay) -> usize {
    let month = calendar_month(day);
    records
        .iter()
        .filter(|record| record.state == SkipState::Applied && record.undone_at.is_none())
        .filter(|record| record.day < day && month.is_some())
        .filter(|record| calendar_month(record.day) == month)
        .count()
}

/// The coin ledger's reference of the skip `id`: its row id, so each skip is its own key.
fn reference_of(id: SkipId) -> String {
    id.get().to_string()
}

/// The preview of a skip of `today` (R7): the day, its due count, whether it is already skipped,
/// the search and the day spec the write would use, and the tariff with whether the wallet holds
/// it. Nothing is written.
///
/// # Errors
///
/// [`PreviewRefusal::Search`] when the configured search is refused, and
/// [`PreviewRefusal::Database`] when a read fails.
pub async fn preview(
    db: &Db,
    today: StudyDay,
    configured: &str,
) -> Result<SkipPreview, PreviewRefusal> {
    let search = skip_search(configured)?;
    let mut reader = db.reader().acquire().await.map_err(KernelError::from)?;
    let due_count = stored(&mut reader, today)
        .await?
        .and_then(|rolled| rolled.card_state)
        .map(|state| state.due_today);
    let records = records_on(&mut reader).await?;
    drop(reader);
    let already_skipped = records.iter().any(|record| {
        record.day == today
            && record.undone_at.is_none()
            && matches!(record.state, SkipState::Pending | SkipState::Applied)
    });
    let tariff = tariff::price(tariff::ladder(), earlier_in_month(&records, today));
    let balance = SqliteWallet::new(db.clone()).balance().await?;
    Ok(SkipPreview {
        day: today,
        due_count,
        already_skipped,
        search,
        spec: skip_spec(SKIP_SPREAD_MIN_DAYS, SKIP_SPREAD_MAX_DAYS),
        tariff,
        tariff_funded: balance >= tariff,
    })
}

/// Settles the skip `id` applied with `cards_moved`, and charges its tariff, in one write (R8, R9;
/// D10). The charge is the wallet's floor-clipped debit keyed to the skip's own study day, so a
/// retry is answered by the debit already held; a skip the wallet cannot fully fund still applies,
/// and its record says the tariff went unfunded.
///
/// Answers `None`, writing nothing, when no such skip exists or it failed or was undone.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn settle_applied(
    db: &Db,
    id: SkipId,
    cards_moved: i64,
    now: UtcMillis,
) -> Result<Option<Settlement>, KernelError> {
    let mut write = db.write().await?;
    let records = records_on(&mut write).await?;
    let Some(skip) = records.iter().find(|record| record.id == id) else {
        return Ok(None);
    };
    if skip.undone_at.is_some() || matches!(skip.state, SkipState::Failed(_)) {
        return Ok(None);
    }
    let price = tariff::price(tariff::ladder(), earlier_in_month(&records, skip.day));
    let reference = reference_of(id);
    let charged_on = skip.day;
    let answer = wallet::debit_floored_on(
        &mut write,
        charged_on,
        TARIFF_SOURCE,
        &reference,
        price,
        now,
    )
    .await?;
    let paid = match answer {
        DebitAnswer::Debited { paid, .. } | DebitAnswer::AlreadyDebited { paid } => paid,
        DebitAnswer::NothingRequested => 0,
    };
    let unfunded = paid < price;
    settle_applied_on(&mut write, id, cards_moved, unfunded).await?;
    write.commit().await?;
    Ok(Some(Settlement {
        price,
        paid,
        unfunded,
    }))
}

/// Marks the skip `id` undone and refunds what its tariff paid, in one write (R11; D6, D10). The
/// refund is keyed to the study day the undo was recorded on, so a retry refunds nothing more.
/// Answers the coins this call refunded: 0 when the skip paid nothing, was not applied, or was
/// already refunded.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn settle_undone(
    db: &Db,
    rule: StudyDayRule,
    id: SkipId,
    now: UtcMillis,
) -> Result<i64, KernelError> {
    let mut write = db.write().await?;
    mark_undone_on(&mut write, id, now).await?;
    let records = records_on(&mut write).await?;
    let Some(skip) = records.iter().find(|record| record.id == id) else {
        return Ok(0);
    };
    let Some(undone_at) = skip.undone_at else {
        return Ok(0);
    };
    let reference = reference_of(id);
    // A skip that paid nothing asks the wallet for nothing, which it answers as not positive.
    let paid = tariff::paid_on(&mut write, &reference).await?;
    let refunded_on = rule.study_day(undone_at);
    let answer = wallet::refund_on(
        &mut write,
        refunded_on,
        REFUND_SOURCE,
        &reference,
        paid,
        now,
    )
    .await?;
    write.commit().await?;
    Ok(match answer {
        DepositAnswer::Deposited(coins) => coins,
        DepositAnswer::AlreadyDeposited | DepositAnswer::NotPositive => 0,
    })
}
