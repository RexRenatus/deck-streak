//! The historical landmarks of the owner's review log (SPEC-102 R1 to R4, ADR-318): the
//! anniversaries of the first study day and every 25th earned study day, the rule that selects
//! the ones dated today, and their texts. It ports the predecessor's `landmarks.py`
//! (`compute_landmarks`, `due_today`, `render_landmark`, `_ordinal_label`, at `27ee2bc`).
//!
//! Everything here is a pure function of integers. The study days arrive already read, by
//! ingest's own read of the whole scoped log, because notifications has no edge to ingest.

use deck_streak_kernel::StudyDay;

/// Every this many earned study days is a landmark.
pub const LANDMARK_DAY_STEP: usize = 25;
/// The event name of an anniversary landmark.
pub const ANNIVERSARY_EVENT_TYPE: &str = "landmark_anniversary";
/// The event name of an earned-study-day landmark.
pub const STUDY_DAY_EVENT_TYPE: &str = "landmark_study_day";
/// The anniversary's text.
pub const ANNIVERSARY_TEMPLATE: &str = "\u{1f5d3}\u{fe0f} <b>{ordinal} anniversary</b> on {day}";
/// The anniversary's text when the streak is not current.
pub const ANNIVERSARY_GAP_TEMPLATE: &str = "\u{1f5d3}\u{fe0f} <b>{ordinal} anniversary</b> on {day}\nThe anniversary does not need a streak.";
/// The earned-study-day landmark's text.
pub const STUDY_DAY_TEMPLATE: &str = "\u{1f4da} <b>{ordinal} earned study days</b> on {day}";

/// The most anniversaries the walk takes. It bounds the walk's work for any input, since a study
/// day can carry a year of eighteen digits; the predecessor's calendar ends at year 9999, so no
/// walk it can hold reaches this many and the cap binds none of them.
const ANNIVERSARY_WALK_CAP: i128 = 10_000;

/// One historical event; it carries no text derived from the collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Landmark {
    /// The dedupe key, `landmark:anniv:N` or `landmark:day:N`.
    pub key: String,
    /// The event name, [`ANNIVERSARY_EVENT_TYPE`] or [`STUDY_DAY_EVENT_TYPE`].
    pub event: &'static str,
    /// The anniversary's or the study day's ordinal.
    pub ordinal: u32,
    /// The study day the event is dated.
    pub day: StudyDay,
}

/// The landmarks that have landed by `today`, oldest first.
#[must_use]
pub fn compute_landmarks(study_days: &[StudyDay], today: StudyDay) -> Vec<Landmark> {
    let mut days: Vec<StudyDay> = study_days.to_vec();
    days.sort();
    days.dedup();
    let Some(&origin) = days.first() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    // The anniversaries rise year by year, so the exits below (no date, or the first one past
    // `today`) end the walk, and the cap bounds it should any exit fail to.
    for ordinal in 1_i128..=ANNIVERSARY_WALK_CAP {
        let Some(day) = anniversary(origin, ordinal) else {
            break;
        };
        if day > today {
            break;
        }
        let Ok(ordinal) = u32::try_from(ordinal) else {
            break;
        };
        found.push(Landmark {
            key: format!("landmark:anniv:{ordinal}"),
            event: ANNIVERSARY_EVENT_TYPE,
            ordinal,
            day,
        });
    }
    for ordinal in (LANDMARK_DAY_STEP..=days.len()).step_by(LANDMARK_DAY_STEP) {
        let day = days[ordinal - 1];
        if day > today {
            break;
        }
        let Ok(ordinal) = u32::try_from(ordinal) else {
            break;
        };
        found.push(Landmark {
            key: format!("landmark:day:{ordinal}"),
            event: STUDY_DAY_EVENT_TYPE,
            ordinal,
            day,
        });
    }
    found.sort_by(|a, b| a.day.cmp(&b.day).then_with(|| a.key.cmp(&b.key)));
    found
}

/// The setting the predecessor stores once, on its first run (`landmarks.py:LANDMARK_HIGH_WATER_KEY`):
/// the mark that a run has happened, never read again to suppress a landmark (SPEC-102 R6).
pub const LANDMARK_HIGH_WATER_KEY: &str = "landmark_high_water";

/// The mark a first run stores, exactly the predecessor's `json.dumps(state, sort_keys=True)`:
/// `{"anniversary": A, "seeded": true, "study_day": S}`, A and S the highest anniversary and
/// study-day ordinals among `landmarks`, 0 when there are none (`landmarks.py:run_landmarks`).
/// These are the bytes the v9 import carries as stored, so they are written by hand rather than
/// by a serializer whose separators could differ.
#[must_use]
pub fn high_water_mark(landmarks: &[Landmark]) -> String {
    let anniversary = highest_ordinal(landmarks, ANNIVERSARY_EVENT_TYPE);
    let study_day = highest_ordinal(landmarks, STUDY_DAY_EVENT_TYPE);
    format!("{{\"anniversary\": {anniversary}, \"seeded\": true, \"study_day\": {study_day}}}")
}

/// The highest ordinal among the landmarks of `event`, or 0.
fn highest_ordinal(landmarks: &[Landmark], event: &str) -> u32 {
    landmarks
        .iter()
        .filter(|landmark| landmark.event == event)
        .map(|landmark| landmark.ordinal)
        .max()
        .unwrap_or(0)
}

/// The landmarks dated exactly `today`.
#[must_use]
pub fn due_today(landmarks: &[Landmark], today: StudyDay) -> Vec<&Landmark> {
    landmarks
        .iter()
        .filter(|landmark| landmark.day == today)
        .collect()
}

/// The text of `landmark`; `gap_honest` drops the anniversary's congratulation of a streak.
#[must_use]
pub fn render_landmark(landmark: &Landmark, gap_honest: bool) -> String {
    let template = if landmark.event == ANNIVERSARY_EVENT_TYPE {
        if gap_honest {
            ANNIVERSARY_GAP_TEMPLATE
        } else {
            ANNIVERSARY_TEMPLATE
        }
    } else {
        STUDY_DAY_TEMPLATE
    };
    template
        .replace("{ordinal}", &ordinal_label(landmark.ordinal))
        .replace("{day}", &landmark.day.to_string())
}

/// `1st`, `2nd`, `3rd` and `Nth`, with 11 to 13 always `th` (`landmarks.py:_ordinal_label`).
fn ordinal_label(value: u32) -> String {
    let suffix = if (value % 100) > 10 && (value % 100) < 14 {
        "th"
    } else {
        match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{value}{suffix}")
}

/// The year and the `-MM-DD` tail of `day`'s ISO rendering.
fn split_iso(day: StudyDay) -> (i128, String) {
    let text = day.to_string();
    let cut = text.len().saturating_sub(6);
    let year = text[..cut].parse::<i128>().unwrap_or(0);
    (year, text[cut..].to_string())
}

/// The `n`th anniversary of `origin`: the same month and day `n` years on, falling back to the
/// 28th when that date does not exist (a 29 February); `None` when even that is refused.
fn anniversary(origin: StudyDay, n: i128) -> Option<StudyDay> {
    let (year, tail) = split_iso(origin);
    let year = year + n;
    let head = if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{year:+05}")
    };
    if let Ok(day) = format!("{head}{tail}").parse::<StudyDay>() {
        return Some(day);
    }
    let fallback = format!("{head}{}28", &tail[..4]);
    fallback.parse::<StudyDay>().ok()
}

#[cfg(test)]
mod tests {
    use super::ordinal_label;

    /// The suffix the `match value % 10` arm alone gives.
    fn match_arm(value: u32) -> &'static str {
        match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    }

    #[test]
    fn the_th_guard_and_the_match_arm_agree_at_the_ten_and_fourteen_edges() {
        for value in [10_u32, 14, 110, 114] {
            assert_eq!(match_arm(value), "th", "match arm at {value}");
            assert_eq!(
                ordinal_label(value),
                format!("{value}th"),
                "label at {value}"
            );
        }
        for value in [11_u32, 12, 13, 111, 112, 113] {
            assert_ne!(match_arm(value), "th", "the guard is needed at {value}");
            assert_eq!(
                ordinal_label(value),
                format!("{value}th"),
                "label at {value}"
            );
        }
    }
}
