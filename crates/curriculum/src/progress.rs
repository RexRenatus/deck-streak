//! Road to C2: a card's mastery, a deck name's unit, and each course's progress (SPEC-077).

use std::collections::BTreeMap;

use deck_streak_ingest::reader::Card;
use deck_streak_kernel::{CourseCode, Courses};

/// The share of a band's cards' mastery, in percent, at which the band is achieved.
pub const CEFR_BAND_ACHIEVED_PCT: f64 = 80.0;
/// The interval in days at which a card with no memory state counts as mature.
pub const PROGRESS_MATURE_IVL_DAYS: i64 = 21;
/// The stability in days at which a card's stability score reaches 1.
pub const PROGRESS_STABILITY_TARGET_DAYS: f64 = 100.0;
/// The mastery at or above which a card counts as mature.
pub const MATURE_MASTERY_THRESHOLD: f64 = 0.5;
/// The smallest decay magnitude the forgetting curve accepts.
pub const MIN_ABS_DECAY: f64 = 1e-3;
/// The largest decay magnitude the forgetting curve accepts.
pub const MAX_ABS_DECAY: f64 = 10.0;
/// The XP a band-up pays: the predecessor kept it outside its economy file, so it is this
/// context's own constant (ADR-047, SPEC-077 ruling 4).
pub const XP_BONUS_BAND_UP: i64 = 0;

/// One CEFR band's counts within a course.
#[derive(Clone, Debug, PartialEq)]
pub struct BandProgress {
    /// The band, one of the kernel's CEFR bands.
    pub band: &'static str,
    /// Cards of the course whose unit falls in the band.
    pub total: u32,
    /// Of those, the cards at or above the mature threshold.
    pub mature: u32,
    /// The band's mean mastery in percent.
    pub pct: f64,
    /// Whether the band is achieved.
    pub achieved: bool,
}

/// One course's progress.
#[derive(Clone, Debug, PartialEq)]
pub struct CourseProgress {
    /// The course's code.
    pub code: CourseCode,
    /// The course's name.
    pub name: String,
    /// The course's flag.
    pub flag: String,
    /// Cards counted.
    pub total_cards: u32,
    /// Mature cards counted.
    pub mature_cards: u32,
    /// Mean mastery in percent.
    pub mastery_pct: f64,
    /// The last band of the achieved run from A1, or A1.
    pub current_band: &'static str,
    /// Every band, in order.
    pub bands: Vec<BandProgress>,
    /// The highest unit with a mature card.
    pub current_unit: Option<u32>,
}

/// A card's mastery in 0..=1 at `now_sec`.
#[must_use]
pub fn card_mastery(_card: &Card, _now_sec: i64, _mature_ivl: i64) -> f64 {
    -1.0
}

/// The unit number a deck name carries.
#[must_use]
pub fn parse_unit(_deck_name: &str) -> Option<u32> {
    Some(u32::MAX)
}

/// Each course's progress, ordered by name.
#[must_use]
pub fn course_progress(
    _cards: &[Card],
    _deck_names: &BTreeMap<i64, String>,
    _courses: &Courses,
    _now_sec: i64,
) -> Vec<CourseProgress> {
    Vec::new()
}
