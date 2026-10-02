//! The coin and shop constants.

/// The XP a coin costs: one coin is minted for each this many base XP.
pub const COIN_MINT_XP_DIVISOR: i64 = 0;
/// The most coins a day mints.
pub const COIN_MINT_DAILY_CAP: i64 = 0;
/// The most coins a day may lose.
pub const DAILY_LOSS_CAP_COINS: i64 = 0;
/// The share of the day-start wallet the loss cap allows.
pub const DAILY_LOSS_CAP_WALLET_FRAC: f64 = 0.0;
/// The share of the wallet a fine takes at least.
pub const FINE_WALLET_FRAC: f64 = 0.0;
/// The freeze's price in coins.
pub const SHOP_FREEZE_PRICE: i64 = 0;
/// The scroll pass's price in coins.
pub const SHOP_SCROLL_PASS_PRICE: i64 = 0;
/// The minutes a scroll pass lasts.
pub const SCROLL_PASS_MINUTES: i64 = 0;
/// The multiple of the pass's price while a surcharge lies ahead.
pub const PASS_SURCHARGE_MULT: f64 = 0.0;
/// The hours a surcharge lasts.
pub const PASS_SURCHARGE_HOURS: i64 = 0;
/// The freezes a language streak holds at most.
pub const STREAK_FREEZE_CAP: i64 = 0;
/// The lowest balance a wallet reaches.
pub const WALLET_FLOOR: i64 = 0;
