//! The coin rules, pure: the mint, the daily loss cap, the wealth-scaled fine and the debit clip.
//!
//! Each is the predecessor's function, and the two that round a wallet's share compute it over
//! `f64` in the predecessor's order: the cap's share rounds down (`int(wallet * 0.3)`) and the
//! fine's rounds up (`ceil(wallet * 0.02)`). A test holds every one equal to its golden.

use crate::constants::{
    COIN_MINT_DAILY_CAP, COIN_MINT_XP_DIVISOR, DAILY_LOSS_CAP_COINS, DAILY_LOSS_CAP_WALLET_FRAC,
    FINE_WALLET_FRAC,
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
/// less, otherwise the lesser of [`DAILY_LOSS_CAP_COINS`] and the wallet's share rounded down.
#[must_use]
pub fn daily_loss_cap(wallet_at_rollover: i64) -> i64 {
    if wallet_at_rollover <= 0 {
        return 0;
    }
    // The wallet is positive and far below 2^53, so the conversion is exact, and the share is
    // positive, so truncation is the predecessor's `int()`.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let share = (wallet_at_rollover as f64 * DAILY_LOSS_CAP_WALLET_FRAC) as i64;
    DAILY_LOSS_CAP_COINS.min(share)
}

/// A fine's amount: none for a configured fine of 0 or less, the configured fine for a wallet of
/// 0 or less, otherwise the greater of the configured fine and the wallet's share rounded up.
#[must_use]
pub fn scaled_fine(configured: i64, wallet: i64) -> i64 {
    if configured <= 0 {
        return 0;
    }
    if wallet <= 0 {
        return configured;
    }
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let share = (wallet as f64 * FINE_WALLET_FRAC).ceil() as i64;
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
