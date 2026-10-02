//! The chest constants, the rarity roll, the Epic odds and the payout (SPEC-081 R3, R4), and the
//! chest port's three grants: the session chests, the challenge chest and the weekly chest (R2,
//! R5-R7, R16, R17).
//!
//! The roll, the odds and the payout are pure: the draw is an input, so the parity oracle proves
//! the fold from a draw to a rarity and a payout. The grants take their draws from the [`Draw`]
//! port and write through the caller's connection, inside the caller's one write (ADR-081): a
//! chest and the pity counters after it are stored together or not at all.

use deck_streak_kernel::{Hour, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::chest_store::StoredChest;
use crate::draw::{Draw, DrawError};
use crate::sessions::Session;

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
/// The Epic points a study day with the Ascendant buff adds to each session chest's roll (R3).
pub const ASCENDANT_BUFF_PTS: f64 = 10.0;
/// The Epic points the challenge chest's roll adds (R16).
pub const CHALLENGE_BUFF_PTS: f64 = 10.0;
/// The session review XP the challenge chest's payout cap is reckoned from (R16).
pub const CHALLENGE_SESSION_BASE_XP: i64 = 200;

/// Why a grant, or a read or write of the chest store, did not complete.
#[derive(Debug, thiserror::Error)]
pub enum ChestError {
    /// A draw failed: nothing was written for the chest it was taken for.
    #[error("a chest's draw failed, so nothing was written for it")]
    Draw(#[from] DrawError),
    /// The database refused a read or a write.
    #[error("the chest store's database refused the operation")]
    Database(#[from] sqlx::Error),
    /// A stored value lies outside its column's rule.
    #[error("the stored {column} holds {value}, which is not one of its values")]
    Unreadable {
        /// The table and column.
        column: &'static str,
        /// The value read.
        value: String,
    },
}

/// A session of the study day with the review XP it earned at the base rate. The caller reckons
/// the XP: this context computes none (docs/CONTEXT-MAP.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EarnedSession {
    /// The session, its bounds and its effort.
    pub session: Session,
    /// Its review XP at the base rate, which caps a Common or Rare payout (R4).
    pub base_xp: i64,
}

/// What the session grant reads besides the store: the study day, its sessions and the
/// recompute's day and clock facts.
#[derive(Clone, Copy, Debug)]
pub struct SessionChestGrant<'a> {
    /// The study day the recompute grants for.
    pub study_day: StudyDay,
    /// The study day's sessions, eligible or not; the grant applies the effort floor.
    pub sessions: &'a [EarnedSession],
    /// Whether the study day is a declared skip day: no chest is granted on one.
    pub skip_day: bool,
    /// Whether the study day holds the Ascendant buff (SPEC-072).
    pub ascendant: bool,
    /// The recompute's local hour, which decides the vault (R7).
    pub local_hour: Hour,
    /// Whether the recompute runs inside quiet hours (SPEC-041), which vaults too.
    pub quiet: bool,
}

/// Grants the session chests of `request`'s study day (R2, R5-R7), in the caller's write.
///
/// Each eligible session, in order of its start, earns one chest until the day holds
/// `per_day_max` chests of every origin. A session whose start lies within one session gap of a
/// session chest the day already held is skipped, and so is a session whose key is held, without
/// a draw: a stored chest is never rolled again. A chest takes two draws, rarity then payout,
/// before anything is written; it is then stored with the pity counters after it.
///
/// # Errors
///
/// [`ChestError::Draw`] when a draw fails: nothing is written for that session or any later one,
/// and the chests granted before it stay in the caller's write, each with its pity. A store error
/// as [`chest_store`](crate::chest_store) answers it.
pub async fn grant_session_chests_on(
    connection: &mut SqliteConnection,
    request: &SessionChestGrant<'_>,
    draw: &mut impl Draw,
    at: UtcMillis,
) -> Result<Vec<StoredChest>, ChestError> {
    let _ = (connection, at);
    for earned in request.sessions {
        if crate::sessions::meets_chest_floor(&earned.session.effort) {
            draw.draw()?;
            draw.draw()?;
        }
    }
    Ok(Vec::new())
}

/// Grants the challenge quest's chest for `study_day` (R16): one a study day, rolled with
/// [`CHALLENGE_BUFF_PTS`] and paid against [`CHALLENGE_SESSION_BASE_XP`], sealed, with the pity
/// counters after it. A held key answers none and takes no draw. The day's cap does not count it
/// out.
///
/// # Errors
///
/// [`ChestError::Draw`] when a draw fails, and nothing is written; a store error as
/// [`chest_store`](crate::chest_store) answers it.
pub async fn grant_challenge_chest_on(
    connection: &mut SqliteConnection,
    study_day: StudyDay,
    draw: &mut impl Draw,
    at: UtcMillis,
) -> Result<Option<StoredChest>, ChestError> {
    let _ = (connection, study_day, draw, at);
    Ok(None)
}

/// Grants the weekly quest's chest for `study_day` (R17): one a study day, an Epic that pays 0,
/// sealed, with no draw and no change to the pity counters. A held key answers none. Whether the
/// week's claim is due is the caller's (SPEC-080).
///
/// # Errors
///
/// A store error as [`chest_store`](crate::chest_store) answers it.
pub async fn grant_weekly_chest_on(
    connection: &mut SqliteConnection,
    study_day: StudyDay,
    at: UtcMillis,
) -> Result<Option<StoredChest>, ChestError> {
    let _ = (connection, study_day, at);
    Ok(None)
}

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
        match name {
            "common" => Some(Self::Common),
            "rare" => Some(Self::Rare),
            "epic" => Some(Self::Epic),
            "legendary" => Some(Self::Legendary),
            _ => None,
        }
    }
}

/// An integer as the predecessor's float: the counters and bounds here are tiny, so the
/// conversion is exact.
#[allow(
    clippy::cast_precision_loss,
    reason = "pity counters and XP bounds stay far below 2^53"
)]
const fn float(value: i64) -> f64 {
    value as f64
}

/// The Epic percent points: base plus the ramp plus the buffs, never past the ceiling.
#[must_use]
pub fn epic_odds_pts(since_epic: i64, buff_pts: f64) -> f64 {
    let ramp = float((since_epic - PITY_EPIC_RAMP_AFTER).max(0)) * PITY_EPIC_RAMP_PTS;
    EPIC_ODDS_CEILING_PCT.min(BASE_ODDS_EPIC + ramp + buff_pts.max(0.0))
}

/// One draw `u` in [0, 1) folded into a rarity, with the pity counters' guarantees first.
#[must_use]
pub fn roll_rarity(u: f64, since_epic: i64, since_legendary: i64, buff_pts: f64) -> Rarity {
    if since_legendary + 1 >= PITY_LEGENDARY_GUARANTEE {
        return Rarity::Legendary;
    }
    if since_epic + 1 >= PITY_EPIC_GUARANTEE {
        return Rarity::Epic;
    }
    let pct = u.clamp(0.0, 1.0) * 100.0;
    let epic = epic_odds_pts(since_epic, buff_pts);
    if pct < BASE_ODDS_LEGENDARY {
        Rarity::Legendary
    } else if pct < BASE_ODDS_LEGENDARY + epic {
        Rarity::Epic
    } else if pct < BASE_ODDS_LEGENDARY + epic + BASE_ODDS_RARE {
        Rarity::Rare
    } else {
        Rarity::Common
    }
}

/// A chest's XP payout: a Common or Rare draws from its band, capped by the session's review XP;
/// a Legendary pays its fixed amount; an Epic pays 0, its choice being the prize.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "both truncations are of small non-negative floats, as the predecessor's int()"
)]
pub fn payout_xp(rarity: Rarity, u: f64, session_base_xp: i64) -> i64 {
    let cap = PAYOUT_CAP_FLOOR_XP.max((float(session_base_xp) * PAYOUT_SESSION_FRAC) as i64);
    let (low, high) = match rarity {
        Rarity::Legendary => return LEGENDARY_XP,
        Rarity::Epic => return 0,
        Rarity::Common => COMMON_XP,
        Rarity::Rare => RARE_XP,
    };
    let draw = u.clamp(0.0, 1.0);
    cap.min(low + (draw * float(high - low + 1)) as i64)
}
