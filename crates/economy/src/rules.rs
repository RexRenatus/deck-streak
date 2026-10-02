//! The coin rules, pure.

/// The coins a day's base XP mints.
#[must_use]
pub fn mint_for_base_xp(_base_xp: i64) -> i64 {
    0
}

/// The most coins a day may lose, from the wallet at the day's start.
#[must_use]
pub fn daily_loss_cap(_wallet_at_rollover: i64) -> i64 {
    0
}

/// A fine's amount, scaled to the wallet.
#[must_use]
pub fn scaled_fine(_configured: i64, _wallet: i64) -> i64 {
    0
}

/// A debit clipped to the wallet and the day's remaining cap: the amount allowed and whether any
/// part was forgiven.
#[must_use]
pub fn clip_debit(_requested: i64, _wallet: i64, _cap_remaining: i64) -> (i64, bool) {
    (0, false)
}
