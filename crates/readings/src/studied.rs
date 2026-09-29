//! When a reading is studied (SPEC-047 R3): the majority rule and the window of study days a
//! qualifying review counts in. Pure: the reviews come in, a count and a verdict go out.

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{StudyDay, StudyDayRule, UtcMillis};

/// The share of its covered cards that must be studied, in percent (the predecessor's constant).
pub const STUDIED_MAJORITY_PCT: u32 = 0;

/// Whether `studied` of `covered` cards is a majority.
#[must_use]
pub fn is_studied(studied: u32, covered: u32) -> bool {
    let _ = (studied, covered);
    false
}

/// The window of a reading: its generation instant and the study day it was generated for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// When the reading was generated.
    pub generated_at: UtcMillis,
    /// The study day it was generated for.
    pub study_day: StudyDay,
}

impl Window {
    /// Whether a review answered at `at` counts toward the reading.
    #[must_use]
    pub fn counts(&self, rule: StudyDayRule, at: UtcMillis) -> bool {
        let _ = (rule, at);
        false
    }

    /// Whether the window has closed at `now`: the rollover that starts study day d + 2 has passed.
    #[must_use]
    pub fn is_over(&self, rule: StudyDayRule, now: UtcMillis) -> bool {
        let _ = (rule, now);
        false
    }
}

/// Whether `review` is a study review: type 0 to 3 with ease 1 or more.
#[must_use]
pub fn qualifies(review: &Review) -> bool {
    let _ = review;
    false
}

/// How many of `covered` cards have a qualifying review inside `window`.
#[must_use]
pub fn studied_count(
    window: &Window,
    rule: StudyDayRule,
    covered: &[i64],
    reviews: &[Review],
) -> u32 {
    let _ = (window, rule, covered, reviews);
    0
}

/// Where a reading stands (SPEC-047 R8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Its window is open and the threshold is not crossed.
    Open,
    /// The threshold was crossed.
    Studied,
    /// The window closed below the threshold; late reviews inside it can still turn it studied.
    Retired,
}

impl Verdict {
    /// The stored text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Studied => "studied",
            Self::Retired => "retired",
        }
    }

    /// The verdict a stored text is.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "open" => Some(Self::Open),
            "studied" => Some(Self::Studied),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}
