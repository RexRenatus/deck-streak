//! The coin rules, pure: the mint, the daily loss cap, the wealth-scaled fine and the debit clip.
//!
//! Each is the predecessor's function, in whole coins. The two that take a share of the wallet
//! compute it in integers, the cap's rounded down and the fine's rounded up, as the predecessor's
//! `int(wallet * 0.3)` and `ceil(wallet * 0.02)` round theirs. Each share is taken over the
//! wallet's whole hundreds and its remainder apart, so no step leaves `i64` for any wallet. A test
//! holds every rule equal to its golden, and `formal/lean/Formal/CoinShares.lean` proves the two
//! shares' rounding and their bounds.

use crate::constants::{
    COIN_MINT_DAILY_CAP, COIN_MINT_XP_DIVISOR, DAILY_LOSS_CAP_COINS, DAILY_LOSS_CAP_WALLET_PERCENT,
    FINE_WALLET_PERCENT,
};

/// The coins a study day's base XP mints: one for each [`COIN_MINT_XP_DIVISOR`] base XP, at most
/// [`COIN_MINT_DAILY_CAP`], and none for a base of 0 or less.
#[must_use]
pub fn mint_for_base_xp(base_xp: i64) -> i64 {
    if base_xp <= 0 {
        return 0;
    }
    COIN_MINT_DAILY_CAP.min(base_xp.div_euclid(COIN_MINT_XP_DIVISOR))
}

/// The most coins a day may lose, from the wallet at the day's start: none for a wallet of 0 or
/// less, otherwise the lesser of [`DAILY_LOSS_CAP_COINS`] and the wallet's
/// [`DAILY_LOSS_CAP_WALLET_PERCENT`] percent, rounded down.
#[must_use]
pub fn daily_loss_cap(wallet_at_rollover: i64) -> i64 {
    if wallet_at_rollover <= 0 {
        return 0;
    }
    // The share of the whole hundreds, then of the remainder rounded down, as the predecessor's
    // `int()` rounds; no step overflows for any wallet `i64` holds.
    let share = wallet_at_rollover / 100 * DAILY_LOSS_CAP_WALLET_PERCENT
        + wallet_at_rollover % 100 * DAILY_LOSS_CAP_WALLET_PERCENT / 100;
    DAILY_LOSS_CAP_COINS.min(share)
}

/// A fine's amount: none for a configured fine of 0 or less, the configured fine for a wallet of
/// 0 or less, otherwise the greater of the configured fine and the wallet's
/// [`FINE_WALLET_PERCENT`] percent, rounded up.
#[must_use]
pub fn scaled_fine(configured: i64, wallet: i64) -> i64 {
    if configured <= 0 {
        return 0;
    }
    if wallet <= 0 {
        return configured;
    }
    // The share of the whole hundreds, then of the remainder rounded up, as the predecessor's
    // `ceil()` rounds; no step overflows for any wallet `i64` holds.
    let share =
        wallet / 100 * FINE_WALLET_PERCENT + (wallet % 100 * FINE_WALLET_PERCENT + 99) / 100;
    configured.max(share)
}

/// A debit clipped to the wallet and the day's remaining cap: the amount allowed, and whether any
/// part of the request was forgiven. A request of 0 or less debits nothing and forgives nothing.
#[must_use]
pub fn clip_debit(requested: i64, wallet: i64, cap_remaining: i64) -> (i64, bool) {
    if requested <= 0 {
        return (0, false);
    }
    let allowed = requested.min(wallet).min(cap_remaining).max(0);
    (allowed, allowed < requested)
}
