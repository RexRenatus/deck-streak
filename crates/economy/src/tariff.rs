//! The skip day's tariff (SPEC-083 R8 to R10, T3, T4; ADR-321 D11, D13): the ladder `economy.json`
//! holds, the price of a skip at the count of its month's earlier skips, and what one skip paid.
//!
//! The ladder is read once from the embedded file, so no tariff number is typed in code (R10). The
//! price is a pure function. The charge and the refund are the wallet's floor-clipped debit and its
//! refund, which coordination calls inside its own write. What a skip paid is read from the coin
//! ledger by the tariff's source and the skip's reference, since the wallet's ports answer no read
//! by reference and the refund owes exactly that amount (T4).

use std::sync::LazyLock;

use deck_streak_kernel::KernelError;
use serde_json::Value;
use sqlx::SqliteConnection;

/// The coin ledger's source of a skip's tariff (T3).
pub const TARIFF_SOURCE: &str = "skip_tariff";
/// The coin ledger's source of a skip's refund (T4).
pub const REFUND_SOURCE: &str = "skip_tariff_refund";

/// `economy.json`, embedded at build time.
const ECONOMY_FILE: &str = include_str!("../../../economy.json");

/// `streak.skip_tariff_coins` in `economy.json`: the price of a month's first skip, of its second,
/// and of every later one (R8).
///
/// # Panics
///
/// When the embedded file lacks the ladder: a build-time fact, held by A14's test.
#[must_use]
pub fn ladder() -> &'static [i64] {
    static LADDER: LazyLock<Vec<i64>> = LazyLock::new(parse);
    &LADDER
}

#[allow(
    clippy::expect_used,
    reason = "the embedded file is the build's own, and A14's test reads the ladder"
)]
fn parse() -> Vec<i64> {
    let file: Value = serde_json::from_str(ECONOMY_FILE).expect("economy.json is JSON");
    file["streak"]["skip_tariff_coins"]
        .as_array()
        .expect("a ladder of prices in economy.json")
        .iter()
        .map(|coins| coins.as_i64().expect("a whole price in economy.json"))
        .collect()
}

/// The price of a skip whose month already holds `earlier` applied skips not undone on an earlier
/// study day (D13): the ladder's entry at that count, the last price repeating, and nothing for an
/// empty ladder.
#[must_use]
pub fn price(ladder: &[i64], earlier: usize) -> i64 {
    let Some(last) = ladder.len().checked_sub(1) else {
        return 0;
    };
    ladder.get(earlier.min(last)).copied().unwrap_or(0)
}

/// What the skip `reference` paid: the coins its tariff's movements took, read on the caller's
/// connection, 0 when none did.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn paid_on(
    connection: &mut SqliteConnection,
    reference: &str,
) -> Result<i64, KernelError> {
    let paid: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(-delta), 0) FROM coin_ledger WHERE source = ?1 AND reference = ?2",
    )
    .bind(TARIFF_SOURCE)
    .bind(reference)
    .fetch_one(connection)
    .await?;
    Ok(paid)
}
