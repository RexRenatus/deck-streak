//! The chest constants, the rarity roll, the Epic odds and the payout (SPEC-081 R3, R4).
//!
//! Every function is pure: the draw is an input, never read here, so the parity oracle proves the
//! fold from a draw to a rarity and a payout and the draw's source is a later part's port.

#![allow(unused_variables)]

/// Common's base odds, in percent points.
pub const BASE_ODDS_COMMON: f64 = 70.0;
/// Rare's base odds, in percent points.
pub const BASE_ODDS_RARE: f64 = 22.0;
/// Epic's base odds, in percent points.
pub const BASE_ODDS_EPIC: f64 = 7.0;
/// Legendary's base odds, in percent points.
pub const BASE_ODDS_LEGENDARY: f64 = 1.0;
/// The most Epic points a ramp and its buffs can reach.
pub const EPIC_ODDS_CEILING_PCT: f64 = 40.0;
/// Chests since the last Epic before the Epic odds start to ramp.
pub const PITY_EPIC_RAMP_AFTER: i64 = 8;
/// Epic points gained for each chest beyond the ramp's start.
pub const PITY_EPIC_RAMP_PTS: f64 = 5.0;
/// The chest since the last Epic that is an Epic whatever the draw.
pub const PITY_EPIC_GUARANTEE: i64 = 14;
/// The chest since the last Legendary that is a Legendary whatever the draw.
pub const PITY_LEGENDARY_GUARANTEE: i64 = 40;
/// A Common's payout band, in XP.
pub const COMMON_XP: (i64, i64) = (10, 25);
/// A Rare's payout band, in XP.
pub const RARE_XP: (i64, i64) = (30, 60);
/// A Legendary's payout, in XP.
pub const LEGENDARY_XP: i64 = 150;
/// What an Epic whose choice was never made resolves to, in XP.
pub const EPIC_FALLBACK_XP: i64 = 50;
/// The share of a session's review XP a Common or Rare payout may reach.
pub const PAYOUT_SESSION_FRAC: f64 = 0.30;
/// The least a Common or Rare payout's cap allows, in XP.
pub const PAYOUT_CAP_FLOOR_XP: i64 = 25;

/// A chest's rarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rarity {
    /// The most frequent.
    Common,
    /// The second tier.
    Rare,
    /// The third tier: its prize is a choice.
    Epic,
    /// The rarest.
    Legendary,
}

impl Rarity {
    /// The lowercase name the stored row and the goldens carry.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Rare => "rare",
            Self::Epic => "epic",
            Self::Legendary => "legendary",
        }
    }

    /// The rarity named `name`, or none when it is not one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        None
    }
}

/// The Epic percent points: base plus the ramp plus the buffs, never past the ceiling.
#[must_use]
pub fn epic_odds_pts(since_epic: i64, buff_pts: f64) -> f64 {
    0.0
}

/// One draw `u` in [0, 1) folded into a rarity, with the pity counters' guarantees first.
#[must_use]
pub fn roll_rarity(u: f64, since_epic: i64, since_legendary: i64, buff_pts: f64) -> Rarity {
    Rarity::Common
}

/// A chest's XP payout: a Common or Rare draws from its band, capped by the session's review XP;
/// a Legendary pays its fixed amount; an Epic pays 0, its choice being the prize.
#[must_use]
pub fn payout_xp(rarity: Rarity, u: f64, session_base_xp: i64) -> i64 {
    0
}
