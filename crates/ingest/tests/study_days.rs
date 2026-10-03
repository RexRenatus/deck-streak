//! The study days of the whole scoped log (SPEC-102 A24): every day with a scoped study event,
//! once each and oldest first, with no window, no out-of-scope card, no entry that is not a study
//! event and no deleted card.

mod support;

use deck_streak_kernel::StudyDay;
use deck_streak_kernel::StudyDayRule;
use support::Fixture;
use support::synthetic::{self, PlannedCard, PlannedReview};

/// An endpoint no test contacts: the reader never syncs.
const ENDPOINT: &str = "http://127.0.0.1:9/";
const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

const fn card(id: i64, deck: &'static str) -> PlannedCard {
    PlannedCard {
        id,
        deck,
        filtered: false,
    }
}

/// A review of `card` at `hour` of the UTC day `day` (the default rule's rollover is 04:00).
const fn review(day: i64, hour: i64, card: i64, kind: i64, ease: i64) -> PlannedReview {
    PlannedReview {
        id: day * DAY_MS + hour * HOUR_MS,
        card,
        kind,
        ease,
    }
}

const CARDS: [PlannedCard; 3] = [card(2001, "Law"), card(2002, "Maths"), card(2003, "Law")];

const REVIEWS: [PlannedReview; 10] = [
    // A study event older than any ingest window: the whole log is read.
    review(10_000, 12, 2001, 1, 3),
    // Two reviews on one study day.
    review(19_001, 5, 2001, 0, 1),
    review(19_001, 23, 2001, 1, 3),
    // Out of scope: a card of a deck the include list does not name.
    review(19_003, 12, 2002, 1, 3),
    // Not study events: a manual entry, a rescheduling entry and an unanswered row.
    review(19_004, 12, 2001, 4, 0),
    review(19_002, 12, 2001, 5, 0),
    review(19_007, 12, 2001, 1, 0),
    // A study event at 00:30, before the 04:00 rollover: the previous study day.
    review(19_006, 0, 2001, 2, 2),
    // The card deleted below: its review leaves the log's scope with it.
    review(19_010, 12, 2003, 1, 3),
    review(19_005, 12, 2001, 3, 4),
];

#[tokio::test]
async fn the_study_days_are_every_scoped_study_event_day_once() {
    let fixture = Fixture::new(ENDPOINT);
    synthetic::build_planned(&fixture.copy(), &["Law", "Maths"], &CARDS, &REVIEWS);
    synthetic::delete_cards(&fixture.copy(), &[2003]);
    synthetic::add_reviews(&fixture.copy(), &[review(19_020, 12, 2001, 1, 3)]);
    let reader = synthetic::reader(&fixture.settings(), "Law", None, support::clock_at(0));

    let days = reader
        .study_days(StudyDayRule::default())
        .await
        .expect("the study days read");

    let expected: Vec<StudyDay> = [10_000, 19_001, 19_005, 19_020]
        .into_iter()
        .map(StudyDay::from_epoch_day)
        .collect();
    assert_eq!(days, expected);
}
