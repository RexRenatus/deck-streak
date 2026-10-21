//! Road to C2: a card's mastery, a deck name's unit, and each course's progress (SPEC-077).

use std::collections::BTreeMap;

use deck_streak_ingest::memory_state::DEFAULT_DECAY;
use deck_streak_ingest::reader::Card;
use deck_streak_kernel::courses::CEFR_BANDS;
use deck_streak_kernel::{CourseCode, Courses};

use crate::unit_bands::band_for_unit;

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
pub const XP_BONUS_BAND_UP: i64 = 500;

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

/// The probability of recall `elapsed_days` after the last review, in 0..=1.
///
/// The decay's magnitude is clamped into `MIN_ABS_DECAY..=MAX_ABS_DECAY` so the curve is total for a
/// corrupt value, as the predecessor's is.
fn retrievability(stability: f64, elapsed_days: f64, decay: f64) -> f64 {
    if stability <= 0.0 {
        return 0.0;
    }
    let magnitude = if decay == 0.0 {
        DEFAULT_DECAY
    } else {
        decay.abs()
    };
    let exponent = -magnitude.clamp(MIN_ABS_DECAY, MAX_ABS_DECAY);
    let factor = 0.9_f64.powf(1.0 / exponent) - 1.0;
    let recall = (1.0 + factor * elapsed_days.max(0.0) / stability).powf(exponent);
    recall.clamp(0.0, 1.0)
}

/// A card's mastery in 0..=1 at `now_sec`.
///
/// A buried card (queue -1) is 0. With a memory state the mastery is the stability score times the
/// recall now; without one a review card at `mature_ivl` days or more is 1 and any other 0.
#[must_use]
pub fn card_mastery(card: &Card, now_sec: i64, mature_ivl: i64) -> f64 {
    if card.queue == -1 {
        return 0.0;
    }
    if let Some(memory) = card.memory.filter(|memory| memory.stability > 0.0) {
        let elapsed = match memory.last_review_sec {
            #[allow(
                clippy::cast_precision_loss,
                reason = "epoch seconds sit far below 2^52"
            )]
            Some(last) if last != 0 => ((now_sec - last) as f64 / 86_400.0).max(0.0),
            _ => 0.0,
        };
        let recall = retrievability(memory.stability, elapsed, memory.decay);
        let score = (memory.stability.ln_1p() / PROGRESS_STABILITY_TARGET_DAYS.ln_1p()).min(1.0);
        return score * recall;
    }
    if card.kind == 2 && card.interval >= mature_ivl {
        1.0
    } else {
        0.0
    }
}

/// The unit number a deck name carries: the first `Unit` followed by white space and digits.
///
/// A unit beyond `u32` is no unit: every configured band is a range of `u32` units, so no band
/// holds it and its card is not counted, as the predecessor counts none (SPEC-077 A4's
/// `unit_beyond_u32` case).
#[must_use]
pub fn parse_unit(deck_name: &str) -> Option<u32> {
    let mut rest = deck_name;
    while let Some(at) = rest.find("Unit") {
        let after = &rest[at + 4..];
        let trimmed = after.trim_start_matches(|c: char| c.is_whitespace());
        if trimmed.len() < after.len() {
            let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
            if !digits.is_empty() {
                let significant = digits.trim_start_matches('0');
                return if significant.is_empty() {
                    Some(0)
                } else {
                    significant.parse().ok()
                };
            }
        }
        rest = after;
    }
    None
}

/// Each course's progress over `cards`, ordered by name.
///
/// A card counts when it carries a course the file names, its home deck is named, that name
/// carries a unit, and the unit falls in one of the course's bands.
#[must_use]
pub fn course_progress(
    cards: &[Card],
    deck_names: &BTreeMap<i64, String>,
    courses: &Courses,
    now_sec: i64,
) -> Vec<CourseProgress> {
    let mut results = Vec::new();
    for course in courses.courses() {
        // count, mastery sum and mature count per band, in the bands' order.
        let mut cells = [(0_u32, 0.0_f64, 0_u32); CEFR_BANDS.len()];
        let mut current_unit: Option<u32> = None;
        let mut counted = false;
        for card in cards
            .iter()
            .filter(|card| card.course.as_ref() == Some(&course.code))
        {
            let home = if card.original_deck_id == 0 {
                card.deck_id
            } else {
                card.original_deck_id
            };
            let Some(name) = deck_names.get(&home) else {
                continue;
            };
            let Some(unit) = parse_unit(name) else {
                continue;
            };
            let Some(band) = band_for_unit(course, unit) else {
                continue;
            };
            let Some(slot) = CEFR_BANDS.iter().position(|candidate| *candidate == band) else {
                continue;
            };
            let mastery = card_mastery(card, now_sec, PROGRESS_MATURE_IVL_DAYS);
            counted = true;
            cells[slot].0 += 1;
            cells[slot].1 += mastery;
            if mastery >= MATURE_MASTERY_THRESHOLD {
                cells[slot].2 += 1;
                current_unit = Some(current_unit.map_or(unit, |held| held.max(unit)));
            }
        }
        if !counted {
            continue;
        }
        let mut bands = Vec::with_capacity(CEFR_BANDS.len());
        let (mut total, mut mature, mut total_mastery) = (0_u32, 0_u32, 0.0_f64);
        let (mut contiguous, mut current) = (true, CEFR_BANDS[0]);
        for (band, (count, mastery_sum, mature_count)) in CEFR_BANDS.iter().zip(cells) {
            let pct = if count == 0 {
                0.0
            } else {
                mastery_sum / f64::from(count) * 100.0
            };
            let achieved = count > 0 && pct >= CEFR_BAND_ACHIEVED_PCT;
            bands.push(BandProgress {
                band,
                total: count,
                mature: mature_count,
                pct,
                achieved,
            });
            total += count;
            mature += mature_count;
            total_mastery += mastery_sum;
            if contiguous && achieved {
                current = band;
            } else {
                contiguous = false;
            }
        }
        let mastery_pct = if total == 0 {
            0.0
        } else {
            total_mastery / f64::from(total) * 100.0
        };
        results.push(CourseProgress {
            code: course.code,
            name: course.name.clone(),
            flag: course.flag.clone(),
            total_cards: total,
            mature_cards: mature,
            mastery_pct,
            current_band: current,
            bands,
            current_unit,
        });
    }
    results.sort_by(|left, right| left.name.cmp(&right.name));
    results
}

/// What a course's current band is against the band stored for it before the recompute (R7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandStep {
    /// The course has no stored band: its current band is recorded as a silent baseline, with no
    /// XP, no badge and no celebration.
    FirstSighting(&'static str),
    /// The current band comes later in [`CEFR_BANDS`] than the stored one: it is recorded once, and
    /// only a band recorded for the first time is paid.
    BandUp(&'static str),
    /// The current band is the stored one or an earlier one: nothing is recorded.
    Unchanged,
}

/// The step of a course whose stored band is `stored` (none before its first recompute) and whose
/// current band is `current`, by the order of [`CEFR_BANDS`].
///
/// A band outside [`CEFR_BANDS`] has no place in the order, so it never reads as a band-up.
#[must_use]
pub fn band_step(stored: Option<&str>, current: &'static str) -> BandStep {
    let Some(stored) = stored else {
        return BandStep::FirstSighting(current);
    };
    let place = |band: &str| CEFR_BANDS.iter().position(|candidate| *candidate == band);
    match (place(stored), place(current)) {
        (Some(held), Some(reached)) if reached > held => BandStep::BandUp(current),
        _ => BandStep::Unchanged,
    }
}
