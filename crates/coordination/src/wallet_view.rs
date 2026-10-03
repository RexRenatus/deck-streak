//! The wallet view (SPEC-082 R5, R15; ADR-315 ruling 3): the balance, the day's loss cap and what
//! is left of it, and one page of the coin movements, newest first.
//!
//! Every rule is economy's; this module reads the wallet's ports and says what they hold. The
//! API's `GET /api/wallet` reads [`wallet_view`], so the numbers the screen shows are the ones the
//! ports decide on.

use deck_streak_economy::rules::daily_loss_cap;
use deck_streak_economy::wallet::{MovementsPage, SqliteWallet};
use deck_streak_kernel::{Db, KernelError, StudyDay};

/// The owner's wallet, as the surfaces show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletView {
    /// The study day the view is for.
    pub study_day: StudyDay,
    /// The balance: the sum of every movement.
    pub balance: i64,
    /// The most the day may lose, from the wallet at the day's start.
    pub loss_cap: i64,
    /// What the day's debits leave of the cap, never below 0.
    pub loss_cap_left: i64,
    /// One page of the movements, newest first, and the next page's cursor.
    pub page: MovementsPage,
}

/// The view for `today`: the page after the movement `before` names, or the newest page when it is
/// `None`.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn wallet_view(
    db: &Db,
    today: StudyDay,
    before: Option<i64>,
) -> Result<WalletView, KernelError> {
    let wallet = SqliteWallet::new(db.clone());
    let balance = wallet.balance().await?;
    let loss_cap = daily_loss_cap(wallet.balance_before(today).await?);
    let loss_cap_left = (loss_cap - wallet.debits_for_day(today).await?).max(0);
    let page = wallet.movements(before).await?;
    Ok(WalletView {
        study_day: today,
        balance,
        loss_cap,
        loss_cap_left,
        page,
    })
}
