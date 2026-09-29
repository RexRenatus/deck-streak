//! The open lapse coordination hands on is counted from the study reviews its caller read
//! (SPEC-049 A15; R15; ADR-088). Every review is synthetic and every instant is set by hand.

use std::collections::BTreeMap;

use deck_streak_coordination::lapse::open_lapse;
use deck_streak_coordination::recompute::RecomputeFacts;
use deck_streak_ingest::reader::{CollectionData, Review};
use deck_streak_kernel::{StudyDay, StudyDayRule, UtcMillis};

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// A study day near the present.
const T: i64 = 20_000;

/// Ten o'clock UTC of study day `day` under the default rule (rollover at 04:00 UTC).
const fn at(day: i64) -> i64 {
    day * DAY_MS + 10 * HOUR_MS
}

/// A review on `day` of type `kind` answered with `ease`.
const fn review(day: i64, kind: i64, ease: i64) -> Review {
    Review {
        id: at(day),
        card_id: 1,
        ease,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind,
    }
}

/// The lapse open on study day `today`, over the window `reviews`.
fn open(today: i64, reviews: Vec<Review>) -> Option<i64> {
    let data = CollectionData {
        reviews,
        cards: Vec::new(),
        created_at: UtcMillis::from_epoch_millis(at(T - 1_000)),
        deck_names: BTreeMap::new(),
    };
    let facts = RecomputeFacts::new(
        &data,
        StudyDayRule::default(),
        UtcMillis::from_epoch_millis(at(today)),
        None,
    );
    open_lapse(&facts).map(StudyDay::epoch_day)
}

#[test]
fn the_open_lapse_is_counted_from_the_study_reviews_read() {
    let studied = [review(T - 20, 1, 3), review(T - 4, 1, 3)];
    // Three silent study days after a study day open a lapse whose id is the first of them.
    assert_eq!(open(T - 1, studied.to_vec()), Some(T - 3));
    assert_eq!(open(T, studied.to_vec()), Some(T - 3));
    // Two do not.
    assert_eq!(open(T - 2, studied.to_vec()), None);
    // A study review closes it.
    let closed = [studied[0], studied[1], review(T, 1, 4)];
    assert_eq!(open(T, closed.to_vec()), None);
    assert_eq!(open(T - 1, closed.to_vec()), Some(T - 3));
    // Every kind of study review counts: a learn, a review, a relearn and a filtered answer.
    for kind in 0..=3 {
        let with_kind = [studied[0], studied[1], review(T, kind, 2)];
        assert_eq!(open(T, with_kind.to_vec()), None, "kind {kind}");
    }
    // A manual or rescheduling entry, and an answer of ease 0, are no study review: they close
    // nothing, and a window holding only them opens no lapse of its own.
    for (kind, ease) in [(4, 3), (5, 3), (1, 0)] {
        let quiet = [studied[0], studied[1], review(T, kind, ease)];
        assert_eq!(
            open(T, quiet.to_vec()),
            Some(T - 3),
            "kind {kind} ease {ease}"
        );
    }
    // An empty window holds no lapse.
    assert_eq!(open(T, Vec::new()), None);
}
