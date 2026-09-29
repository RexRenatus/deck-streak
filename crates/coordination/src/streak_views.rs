//! The streak view and the governor view (SPEC-076 R20, R21): what the fold stored, as the surfaces
//! show it. The API's `GET /api/streak` and `GET /api/governor` and the bot's `/streak` all read
//! these, so the numbers cannot drift between them.

use deck_streak_kernel::{Db, KernelError, StudyDay};
use deck_streak_streaks::constants::{RELIGHT_CARDS, STREAK_FREEZE_CAP};
use deck_streak_streaks::store;
use deck_streak_streaks::streak::{StreakState, heat_for, heat_tier};

/// What the day still open puts at risk on a track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtStake {
    /// A missed day would spend a freeze (the language track, while it holds one).
    Freeze,
    /// A missed day would break the streak.
    Break,
    /// Nothing: there is no streak to lose.
    Nothing,
}

impl AtStake {
    /// The word the views use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Freeze => "freeze",
            Self::Break => "break",
            Self::Nothing => "none",
        }
    }
}

/// Both tracks and what is at stake for the open day.
#[derive(Clone, Debug, PartialEq)]
pub struct StreakView {
    /// The study day the view is for.
    pub study_day: StudyDay,
    /// The language track.
    pub language: StreakState,
    /// The law track; it holds no freezes.
    pub law: StreakState,
    /// The language track's heat emoji, empty below the first threshold.
    pub language_heat: &'static str,
    /// The language track's heat tier, from 0.
    pub language_tier: u32,
    /// The law track's heat tier, from 0.
    pub law_tier: u32,
    /// The most freezes a track holds.
    pub freeze_cap: u32,
    /// What a missed day costs the language track.
    pub language_at_stake: AtStake,
    /// What a missed day costs the law track.
    pub law_at_stake: AtStake,
}

/// The view for `today`.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn streak_view(db: &Db, today: StudyDay) -> Result<StreakView, KernelError> {
    let mut connection = db.reader().acquire().await?;
    let language = store::state(&mut connection, "language")
        .await?
        .unwrap_or_else(StreakState::start);
    let law = store::state(&mut connection, "law")
        .await?
        .unwrap_or_else(StreakState::start);
    let language_at_stake = match (language.current, language.freezes) {
        (0, _) => AtStake::Nothing,
        (_, 0) => AtStake::Break,
        _ => AtStake::Freeze,
    };
    let law_at_stake = if law.current == 0 {
        AtStake::Nothing
    } else {
        AtStake::Break
    };
    Ok(StreakView {
        study_day: today,
        language,
        law,
        language_heat: heat_for(language.current),
        language_tier: heat_tier(language.current),
        law_tier: heat_tier(law.current),
        freeze_cap: STREAK_FREEZE_CAP,
        language_at_stake,
        law_at_stake,
    })
}

/// The governor's verdict as stored, and the strength behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GovernorView {
    /// The latest stored strength, zero before the first day settles.
    pub strength: f64,
    /// Whether a lapse is open.
    pub lapse: bool,
    /// The open lapse's anchor.
    pub lapse_since: Option<StudyDay>,
    /// The reviews on one day that relight a lapse.
    pub relight_reviews: u32,
    /// Whether the governor stands by.
    pub standby: bool,
}

impl GovernorView {
    /// `lapse`, `standby` or `armed`.
    #[must_use]
    pub const fn verdict(&self) -> &'static str {
        if self.lapse {
            "lapse"
        } else if self.standby {
            "standby"
        } else {
            "armed"
        }
    }
}

/// The governor view.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn governor_view(db: &Db) -> Result<GovernorView, KernelError> {
    let mut connection = db.reader().acquire().await?;
    let row = store::governor(&mut connection).await?;
    let strength = store::latest_strength(&mut connection)
        .await?
        .unwrap_or(0.0);
    Ok(GovernorView {
        strength,
        lapse: row.lapse_since.is_some(),
        lapse_since: row.lapse_since,
        relight_reviews: RELIGHT_CARDS,
        standby: row.standby,
    })
}
