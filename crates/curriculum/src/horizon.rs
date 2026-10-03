//! The obligation horizon (SPEC-091 R6, #91): a fixed 365-day forward histogram of the reviews
//! each card owes, and the two-part readout that names the true 30-day obligation and the two
//! levers that create more. The predecessor's `horizon.py:compute_horizon` and `build_readout`,
//! proved by `goldens/horizon.json`, `goldens/horizon_readout.json` and
//! `goldens/horizon.constants.json`.
//!
//! Pure and read-only: every input is a card already read by ingest and the collection's day
//! number; nothing here reads a clock, a table or a file.

use deck_streak_ingest::reader::Card;
use deck_streak_kernel::pynum;

/// The queues whose `due` is a day number, the only form that can be bucketed by `due - today`.
const DAY_NUMBER_QUEUES: [i64; 2] = [2, 3];

/// The queues whose `due` is epoch seconds: an intraday step, owed now.
const INTRADAY_QUEUES: [i64; 2] = [1, 4];

/// Suspended (-1) and buried (-2 sibling, -3 manual): not part of the forward obligation.
const EXCLUDED_QUEUES: [i64; 3] = [-1, -2, -3];

/// The histogram's length in days: always exactly this long, even for an empty collection.
pub const HORIZON_DAYS: usize = 365;

/// The headline window in days; named once so the label and the number cannot disagree.
pub const WINDOW_DAYS: usize = 30;

/// A forward day carrying more reviews than this is a real daily obligation, not a flat line.
pub const FLAT_PEAK_REVIEWS: u64 = 25;

/// What a new card costs over its lifetime, in reviews (`divest.py:NEW_CARD_LIFETIME_REVIEWS`).
/// Curriculum may not depend on insights, so it holds the price itself, and A19 holds it equal.
pub const NEW_CARD_LIFETIME_REVIEWS: u64 = 8;

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

/// Scans `cards` as of `today`, the collection's day number (never a date).
///
/// The guard chain classifies by queue and type FIRST and reads `due` as a day only at its tail,
/// because `due` is a day number in queues 2 and 3, epoch seconds in 1 and 4 and a position in
/// the new queue. Each branch ends the card's turn, so a card lands in exactly one of an index of
/// the curve, `beyond_horizon`, `new_count` and `excluded_count`:
///
/// 1. a suspended or buried card is counted as excluded;
/// 2. a card of type 0 or in queue 0 is new, whatever its `reps` say, because a reset card keeps
///    its answers and a position read as a day would invent an obligation;
/// 3. an intraday card is owed now;
/// 4. a borrowed card with a negative `due` holds a filtered position, not a day, and is owed now;
/// 5. any queue the port does not know is owed now, never bucketed on a `due` of unknown meaning;
/// 6. the rest is bucketed by `due - today`, arrears clamped to day 0, and a day 365 or later is
///    counted beyond the horizon and never folded into the last index.
#[must_use]
pub fn compute_horizon(cards: &[Card], today: i64) -> HorizonScan {
    let mut curve = vec![0_u64; HORIZON_DAYS];
    let mut beyond_horizon = 0_u64;
    let mut new_count = 0_u64;
    let mut excluded_count = 0_u64;
    for card in cards {
        if EXCLUDED_QUEUES.contains(&card.queue) {
            excluded_count += 1;
            continue;
        }
        if card.kind == 0 || card.queue == 0 {
            new_count += 1;
            continue;
        }
        if INTRADAY_QUEUES.contains(&card.queue) {
            bump(&mut curve, 0);
            continue;
        }
        if card.original_deck_id != 0 && card.due < 0 {
            bump(&mut curve, 0);
            continue;
        }
        if !DAY_NUMBER_QUEUES.contains(&card.queue) {
            bump(&mut curve, 0);
            continue;
        }
        let offset = card.due.saturating_sub(today).max(0);
        match usize::try_from(offset) {
            Ok(day) if day < HORIZON_DAYS => bump(&mut curve, day),
            _ => beyond_horizon += 1,
        }
    }
    HorizonScan {
        curve,
        beyond_horizon,
        new_count,
        excluded_count,
        total_cards: u64::try_from(cards.len()).unwrap_or(u64::MAX),
    }
}

/// Counts one review on `day`, a day the curve holds.
fn bump(curve: &mut [u64], day: usize) {
    if let Some(slot) = curve.get_mut(day) {
        *slot += 1;
    }
}

/// The sum of the first `days` entries of `curve`: the headline obligation for that window.
#[must_use]
pub fn window_total(curve: &[u64], days: usize) -> u64 {
    curve.iter().take(days).sum()
}

/// The busiest forward day, `(day, reviews)`, over days 1 to the end of `curve`.
///
/// Index 0 holds everything overdue, and a large backlog is no evidence that new material is being
/// started, so the flatness claim is measured on the days ahead. The first strict maximum wins a
/// tie. A curve with nothing scheduled forward reads `(0, 0)`.
#[must_use]
pub fn forward_peak(curve: &[u64]) -> (usize, u64) {
    let mut peak_day = 0;
    let mut peak_reviews = 0;
    for day in 1..curve.len() {
        if let Some(&reviews) = curve.get(day)
            && reviews > peak_reviews
        {
            peak_day = day;
            peak_reviews = reviews;
        }
    }
    (peak_day, peak_reviews)
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

/// Assembles the readout over `cards` as of `today`, given the collection's desired retention in
/// percent as the caller read it.
///
/// The relief half states a partition: `window_reviews + other_cards` is `total_cards`, and
/// `other_cards` is the sum of the three counts the sentence names, so the subtraction it implies
/// is one a reader can perform. The flatness claim is a claim about the curve and is stated only
/// when the forward peak supports it; both levers are named either way.
#[must_use]
pub fn build_readout(cards: &[Card], today: i64, desired_retention_pct: f64) -> HorizonReadout {
    let scan = compute_horizon(cards, today);
    let reviews = window_total(&scan.curve, WINDOW_DAYS);
    let curve_total: u64 = scan.curve.iter().sum();
    let due_later = curve_total - reviews + scan.beyond_horizon;
    let other_cards = scan.new_count + scan.excluded_count + due_later;
    let (peak_day, peak_reviews) = forward_peak(&scan.curve);
    let is_flat = peak_reviews <= FLAT_PEAK_REVIEWS;
    let relief_text = format!(
        "Your real obligation for the next {WINDOW_DAYS} days is {reviews} reviews. Total. \
         Of the other {} cards \u{2014} {} never started, {} suspended or buried, {} due later \
         than {WINDOW_DAYS} days out \u{2014} every one is something you own, not something you owe.",
        grouped(other_cards),
        grouped(scan.new_count),
        grouped(scan.excluded_count),
        grouped(due_later),
    );
    let attribution = if is_flat {
        "That curve stays flat because nothing is being started: a flat forward curve is not an \
         achievement, it is the signature of a collection that has stopped taking on new material."
            .to_owned()
    } else {
        format!(
            "That curve is not flat \u{2014} day {peak_day} alone carries {} reviews \u{2014} so \
             this collection has not stopped taking on new material, and the number above is a \
             {WINDOW_DAYS}-day window rather than a verdict on the year.",
            grouped(peak_reviews),
        )
    };
    let dial = format!("{:.0}", pynum::round(desired_retention_pct, 0));
    let honest_text = format!(
        "{attribution} Two levers create future obligation \u{2014} the daily new-card dial \
         (priced at ~{NEW_CARD_LIFETIME_REVIEWS} reviews over a new card's lifetime) and your \
         desired_retention (currently {dial}%)."
    );
    HorizonReadout {
        window_days: WINDOW_DAYS,
        window_reviews: reviews,
        total_cards: scan.total_cards,
        other_cards,
        new_count: scan.new_count,
        excluded_count: scan.excluded_count,
        due_later,
        beyond_horizon: scan.beyond_horizon,
        is_flat,
        peak_day,
        peak_reviews,
        new_card_lifetime_reviews: NEW_CARD_LIFETIME_REVIEWS,
        desired_retention_pct,
        relief_text,
        honest_text,
    }
}

/// `n` with a comma before each group of three digits: Python's `{n:,}`.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}
