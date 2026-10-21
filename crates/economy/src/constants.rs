//! The coin and shop constants, each the predecessor's own value (SPEC-082 R9).
//!
//! A test holds every one equal to the constants golden, and holds `economy.json`'s coin and shop
//! values equal to them, so the declaration the game-economy pack judges cannot drift from the
//! engine.

/// The base XP that buys one coin: a day mints one coin for each this many base XP.
pub const COIN_MINT_XP_DIVISOR: i64 = 25;
/// The most coins a day mints.
pub const COIN_MINT_DAILY_CAP: i64 = 40;
/// The most coins a day may lose to debits, whatever the wallet.
pub const DAILY_LOSS_CAP_COINS: i64 = 100;
/// The share of the wallet at the day's start that the day's loss cap allows.
pub const DAILY_LOSS_CAP_WALLET_FRAC: f64 = 0.30;
/// The share of the wallet a fine takes at least, so a fine keeps its bite as the wallet grows.
pub const FINE_WALLET_FRAC: f64 = 0.02;
/// The streak freeze's price in coins.
pub const SHOP_FREEZE_PRICE: i64 = 150;
/// The scroll pass's price in coins.
pub const SHOP_SCROLL_PASS_PRICE: i64 = 40;
/// The minutes a scroll pass lasts.
pub const SCROLL_PASS_MINUTES: i64 = 30;
/// The multiple of the scroll pass's price while a surcharge lies ahead.
pub const PASS_SURCHARGE_MULT: f64 = 1.5;
/// The hours a pass surcharge lasts once a fine sets it.
pub const PASS_SURCHARGE_HOURS: i64 = 48;
/// The freezes a language streak holds at most.
pub const STREAK_FREEZE_CAP: i64 = 3;
/// The lowest balance a wallet reaches: coins are never owed.
pub const WALLET_FLOOR: i64 = 0;
