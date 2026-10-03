//! The obligation horizon (SPEC-091 R6, #91): a fixed 365-day forward histogram of the reviews
//! each card owes, and the two-part readout that names the true 30-day obligation and the two
//! levers that create more. The predecessor's `horizon.py:compute_horizon` and `build_readout`,
//! proved by `goldens/horizon.json`, `goldens/horizon_readout.json` and
//! `goldens/horizon.constants.json`.
//!
//! Pure and read-only: every input is a card already read by ingest and the collection's day
//! number; nothing here reads a clock, a table or a file.

use deck_streak_ingest::reader::Card;

/// The histogram's length in days: always exactly this long, even for an empty collection.
pub const HORIZON_DAYS: usize = 0;

/// The headline window in days; named once so the label and the number cannot disagree.
pub const WINDOW_DAYS: usize = 0;

/// A forward day carrying more reviews than this is a real daily obligation, not a flat line.
pub const FLAT_PEAK_REVIEWS: u64 = 0;

/// What a new card costs over its lifetime, in reviews (`divest.py:NEW_CARD_LIFETIME_REVIEWS`).
/// Curriculum may not depend on insights, so it holds the price itself, and A19 holds it equal.
pub const NEW_CARD_LIFETIME_REVIEWS: u64 = 0;

/// One pass over the collection's cards: every card lands in exactly one of an index of the
/// curve, `beyond_horizon`, `new_count` or `excluded_count`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HorizonScan {
    /// Reviews per forward day; index 0 holds what is owed now.
    pub curve: Vec<u64>,
    /// Cards due on day 365 or later.
    pub beyond_horizon: u64,
    /// Cards never started, owned and not owed.
    pub new_count: u64,
    /// Suspended and buried cards.
    pub excluded_count: u64,
    /// Every card scanned.
    pub total_cards: u64,
}

/// Scans `cards` as of `today`, the collection's day number.
#[must_use]
pub fn compute_horizon(cards: &[Card], today: i64) -> HorizonScan {
    let _ = (cards, today);
    HorizonScan {
        curve: Vec::new(),
        beyond_horizon: 0,
        new_count: 0,
        excluded_count: 0,
        total_cards: 0,
    }
}

/// The sum of the first `days` entries of `curve`.
#[must_use]
pub fn window_total(curve: &[u64], days: usize) -> u64 {
    let _ = (curve, days);
    0
}

/// The busiest forward day, `(day, reviews)`, over days 1 to the end, arrears excluded.
#[must_use]
pub fn forward_peak(curve: &[u64]) -> (usize, u64) {
    let _ = curve;
    (0, 0)
}

/// The readout: a relief half and an honest half, with the numbers they state.
#[derive(Clone, Debug, PartialEq)]
pub struct HorizonReadout {
    /// The width of the headline window, in days.
    pub window_days: usize,
    /// The true obligation for that window, arrears included.
    pub window_reviews: u64,
    /// Every card scanned.
    pub total_cards: u64,
    /// The cards the relief sentence calls "the other N cards".
    pub other_cards: u64,
    /// Cards never started.
    pub new_count: u64,
    /// Suspended and buried cards.
    pub excluded_count: u64,
    /// Cards due after the window: the in-horizon tail and the beyond-horizon count.
    pub due_later: u64,
    /// Cards due on day 365 or later.
    pub beyond_horizon: u64,
    /// Whether the forward curve's peak is at most [`FLAT_PEAK_REVIEWS`].
    pub is_flat: bool,
    /// The busiest forward day, or 0 when nothing is scheduled forward.
    pub peak_day: usize,
    /// The reviews on `peak_day`.
    pub peak_reviews: u64,
    /// [`NEW_CARD_LIFETIME_REVIEWS`], as the readout states it.
    pub new_card_lifetime_reviews: u64,
    /// The desired retention the caller supplied, in percent.
    pub desired_retention_pct: f64,
    /// The relief half: the true windowed number and a breakdown of the remainder.
    pub relief_text: String,
    /// The honest half: the flatness claim when it holds, and both levers.
    pub honest_text: String,
}

/// Assembles the readout over `cards` as of `today`, given the collection's desired retention.
#[must_use]
pub fn build_readout(cards: &[Card], today: i64, desired_retention_pct: f64) -> HorizonReadout {
    let _ = (cards, today);
    HorizonReadout {
        window_days: 0,
        window_reviews: 0,
        total_cards: 0,
        other_cards: 0,
        new_count: 0,
        excluded_count: 0,
        due_later: 0,
        beyond_horizon: 0,
        is_flat: false,
        peak_day: 0,
        peak_reviews: 0,
        new_card_lifetime_reviews: 0,
        desired_retention_pct,
        relief_text: String::new(),
        honest_text: String::new(),
    }
}
