//! The open lapse coordination hands on is counted from the study reviews its caller read
//! (SPEC-049 A15; R15; ADR-088). Every review is synthetic and every instant is set by hand.

use std::collections::BTreeMap;

use deck_streak_coordination::lapse::open_lapse;
use deck_streak_coordination::recompute::RecomputeFacts;
use deck_streak_ingest::reader::{CollectionData, Review};
use deck_streak_kernel::{Hour, StudyDay, StudyDayRule, UtcMillis, UtcOffset};

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
    // nothing.
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

const MINUTE_MS: i64 = 60_000;

/// The instant study day `d` begins under an offset of `offset` minutes east and a rollover at
/// `hour`: the day is the local calendar day of the offset, shifted to start at the rollover hour,
/// so it begins `hour` hours after local midnight, which is `offset` minutes before UTC midnight.
const fn begins(offset: i64, hour: i64, d: i64) -> i64 {
    d * DAY_MS - offset * MINUTE_MS + hour * HOUR_MS
}

/// The study day of instant `t` from the definition alone: the day whose span, from its beginning
/// to the next day's, holds `t`.
fn day_of(offset: i64, hour: i64, t: i64) -> i64 {
    let mut d = t.div_euclid(DAY_MS);
    while begins(offset, hour, d) > t {
        d -= 1;
    }
    while begins(offset, hour, d + 1) <= t {
        d += 1;
    }
    d
}

/// A study review answered at instant `t`.
const fn review_at(t: i64) -> Review {
    Review {
        id: t,
        card_id: 1,
        ease: 3,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

// A helper of test code: a constant outside the configurable range is a malformed population.
#[allow(clippy::expect_used)]
fn rule(offset: i64, hour: i64) -> StudyDayRule {
    StudyDayRule::new(
        Hour::new(u8::try_from(hour).expect("an hour")).expect("an hour of the day"),
        UtcOffset::from_minutes(i16::try_from(offset).expect("minutes"))
            .expect("an offset in range"),
    )
}

/// The lapse open at instant `now` over `reviews`, under `rule`.
fn open_under(rule: StudyDayRule, now: i64, reviews: Vec<Review>) -> Option<i64> {
    let data = CollectionData {
        reviews,
        cards: Vec::new(),
        created_at: UtcMillis::from_epoch_millis(now - 1_000 * DAY_MS),
        deck_names: BTreeMap::new(),
    };
    let facts = RecomputeFacts::new(&data, rule, UtcMillis::from_epoch_millis(now), None);
    open_lapse(&facts).map(StudyDay::epoch_day)
}

#[test]
fn a_review_counts_on_the_study_day_the_rule_gives_at_every_boundary() {
    let mut examined: u64 = 0;
    let mut opened: u64 = 0;
    for offset in [-720_i64, -300, -210, 0, 330, 345, 540, 840] {
        for hour in [0_i64, 4] {
            let the_rule = rule(offset, hour);
            // Local midnight of local calendar date `T` in this offset.
            let midnight = T * DAY_MS - offset * MINUTE_MS;
            let mut instants = vec![
                begins(offset, hour, T) - 1,
                begins(offset, hour, T),
                begins(offset, hour, T) + 1,
            ];
            for (h, m) in [(23, 59), (0, 1), (3, 59), (4, 1)] {
                instants.push(midnight + h * HOUR_MS + m * MINUTE_MS);
            }
            for t in instants {
                let d = day_of(offset, hour, t);
                assert_eq!(
                    the_rule
                        .study_day(UtcMillis::from_epoch_millis(t))
                        .epoch_day(),
                    d,
                    "offset {offset}, rollover {hour}, instant {t}: the rule's day"
                );
                // One study review long ago and one at `t`; the run after `t` is what the day
                // decides: silent on d+1, d+2, d+3 opens a lapse with id d+1.
                let reviews = vec![
                    review_at(begins(offset, hour, d - 10) + HOUR_MS),
                    review_at(t),
                ];
                for (now, want) in [
                    (begins(offset, hour, d + 3), Some(d + 1)),
                    (begins(offset, hour, d + 4) - 1, Some(d + 1)),
                    (begins(offset, hour, d + 3) - 1, None),
                    (begins(offset, hour, d + 2), None),
                ] {
                    examined += 1;
                    opened += u64::from(want.is_some());
                    assert_eq!(
                        open_under(the_rule, now, reviews.clone()),
                        want,
                        "offset {offset}, rollover {hour}, review at {t} (day {d}), now {now}"
                    );
                }
            }
        }
    }
    println!("examined {examined} rollover member(s)");
    assert_eq!(examined, 448, "the population is generated: {examined}");
    assert!(
        opened > 0 && opened < examined,
        "both answers occur: {opened} open of {examined}"
    );
    // The zone west of UTC with a 04:00 rollover: 03:59 local is still the day before, 04:01 is
    // the day itself.
    let west = rule(-300, 4);
    let midnight = T * DAY_MS + 300 * MINUTE_MS;
    let local =
        |h: i64, m: i64| UtcMillis::from_epoch_millis(midnight + h * HOUR_MS + m * MINUTE_MS);
    assert_eq!(west.study_day(local(23, 59)).epoch_day(), T);
    assert_eq!(west.study_day(local(0, 1)).epoch_day(), T - 1);
    assert_eq!(west.study_day(local(3, 59)).epoch_day(), T - 1);
    assert_eq!(west.study_day(local(4, 1)).epoch_day(), T);
}
