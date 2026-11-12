//! The XP exchange readout's view (SPEC-075 R4 to R6, R8, R10; #80): each source bucket's XP per
//! graduated card over a window of study days, the readout the API's `GET /api/xp/exchange`
//! answers.
//!
//! The XP rows are progression's (both XP tables, which only progression names) and the
//! graduations analytics'; this view joins them on ONE read transaction, because the recompute
//! writes a day's rollup and its settlement in one transaction and a rate divides one by the other
//! (ADR-075). It writes nothing.

use deck_streak_kernel::{Db, KernelError, StudyDay};
use deck_streak_progression::exchange::{SourceRate, exchange_rates, xp_rows};

/// The most days a window spans, as the predecessor's `get_xp_exchange_rates` tool caps it. A
/// read-window parameter, never an economy constant.
pub const EXCHANGE_WINDOW_CAP: i64 = 3650;

/// The readout over a window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExchangeView {
    /// The window's first and last day, or none when it reads every day.
    pub window: Option<(StudyDay, StudyDay)>,
    /// Each bucket's rate, in byte order.
    pub rates: Vec<SourceRate>,
}

/// The window of the last `days` days ending `today`, at most [`EXCHANGE_WINDOW_CAP`] of them; none,
/// meaning every day, when `days` is 0 or less.
#[must_use]
pub fn exchange_window(days: i64, today: StudyDay) -> Option<(StudyDay, StudyDay)> {
    if days <= 0 {
        return None;
    }
    let span = days.min(EXCHANGE_WINDOW_CAP);
    Some((
        StudyDay::from_epoch_day(today.epoch_day() - (span - 1)),
        today,
    ))
}

/// The readout of the last `days` days ending `today` (every day when `days` is 0 or less).
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn exchange_view(
    db: &Db,
    today: StudyDay,
    days: i64,
) -> Result<ExchangeView, KernelError> {
    let window = exchange_window(days, today);
    let mut transaction = db.reader().begin().await?;
    let connection = &mut *transaction;
    let rows = xp_rows(connection, window).await?;
    let graduations = deck_streak_analytics::rollup::graduations(connection, window).await?;
    Ok(ExchangeView {
        window,
        rates: exchange_rates(&rows, &graduations),
    })
}
