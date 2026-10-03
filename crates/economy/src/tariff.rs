//! The skip day's tariff (SPEC-083 R8 to R10, T3, T4; ADR-321 D11, D13): the ladder `economy.json`
//! holds, the price of a skip at the count of its month's earlier skips, and what one skip paid.

use deck_streak_kernel::KernelError;
use sqlx::SqliteConnection;

/// The coin ledger's source of a skip's tariff (T3).
pub const TARIFF_SOURCE: &str = "skip_tariff";
/// The coin ledger's source of a skip's refund (T4).
pub const REFUND_SOURCE: &str = "skip_tariff_refund";

/// `streak.skip_tariff_coins` in `economy.json`.
#[must_use]
pub fn ladder() -> &'static [i64] {
    &[]
}

/// The price of a skip whose month already holds `earlier` applied skips not undone on an earlier
/// study day (D13).
#[must_use]
pub fn price(ladder: &[i64], earlier: usize) -> i64 {
    let _ = (ladder, earlier);
    0
}

/// What the skip `reference` paid.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
#[allow(
    clippy::unused_async,
    reason = "the red stub keeps the green signature, which reads the ledger"
)]
pub async fn paid_on(
    connection: &mut SqliteConnection,
    reference: &str,
) -> Result<i64, KernelError> {
    let _ = (connection, reference);
    Ok(0)
}
