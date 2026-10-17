//! SPEC-342 A8 and A9 (R6): the replay from the pinned revision's FSRS-7 defaults, card by card and
//! in one batch; SPEC-386 A5 and A6 (R5): the replay equals the pinned revision's own values.

use deck_streak_fsrs7::convert;
use deck_streak_fsrs7::measure::history;
use deck_streak_fsrs7::replay::{self, Method};
use fsrs7::{FSRS, FSRSError, FSRSItem, FSRSReview, MemoryState};

/// FSRS-7's initial stability for Good at the pinned revision: the model takes a first review's
/// stability from its parameters by rating (`init_stability`, src/model.rs:173-175), and Good's is
/// the third of the FSRS-7 defaults (`DEFAULT_PARAMETERS`, src/inference_v7.rs:1-5). Typed from
/// that line, never read from the crate.
const INITIAL_STABILITY_FOR_GOOD: f32 = 3.9221;
/// The most two methods' values for one card may differ by, relative to the larger.
const RELATIVE: f64 = 1e-6;

fn agree(left: f64, right: f64) -> bool {
    (left - right).abs() <= RELATIVE * left.abs().max(right.abs())
}

#[test]
fn one_good_review_replays_to_the_models_initial_stability_for_good() {
    let model = FSRS::new(&[]).expect("no parameters select FSRS-7's defaults");
    for method in Method::ALL {
        let item = FSRSItem {
            reviews: vec![FSRSReview {
                rating: 3,
                delta_t: 0.0,
            }],
        };
        let states = replay::replay(method, &model, vec![item]).expect("one Good review replays");
        let [state] = states.as_slice() else {
            panic!(
                "{}: {} state(s) for one item, not one",
                method.name(),
                states.len()
            );
        };
        assert_eq!(
            state.stability.to_bits(),
            INITIAL_STABILITY_FOR_GOOD.to_bits(),
            "{}: one Good review replayed to stability {}, not {INITIAL_STABILITY_FOR_GOOD}",
            method.name(),
            state.stability
        );
        assert_eq!(
            replay::checksum(&states).to_bits(),
            f64::from(INITIAL_STABILITY_FOR_GOOD).to_bits(),
            "{}: the checksum of one state is its stability",
            method.name()
        );
    }
}

#[test]
fn the_single_and_batch_methods_agree_on_every_card() {
    let items: Vec<FSRSItem> = convert::histories(&history::rows(200, 8))
        .into_iter()
        .map(|card| card.item)
        .collect();
    let cards = items.len();
    let model = FSRS::new(&[]).expect("no parameters select FSRS-7's defaults");
    let single = replay::replay(Method::Single, &model, items.clone()).expect("single replays");
    let batch = replay::replay(Method::Batch, &model, items).expect("batch replays");

    assert_eq!(single.len(), cards, "single: one state per card");
    assert_eq!(batch.len(), cards, "batch: one state per card");
    for (card, (one, all)) in single.iter().zip(&batch).enumerate() {
        for (field, one, all) in [
            ("stability", one.stability, all.stability),
            ("difficulty", one.difficulty, all.difficulty),
            ("stability_fast", one.stability_fast, all.stability_fast),
        ] {
            assert!(
                agree(f64::from(one), f64::from(all)),
                "card {card}: {field} is {one} by single and {all} by batch"
            );
        }
    }
    let (one, all) = (replay::checksum(&single), replay::checksum(&batch));
    assert!(one > 0.0, "the checksum of {cards} cards is {one}");
    assert!(
        agree(one, all),
        "the checksums are {one} by single and {all} by batch"
    );
    println!("examined {cards} card(s)");
}

/// The four ratings, Again to Easy.
const RATINGS: [u32; 4] = [1, 2, 3, 4];

/// The four first-review states, Again to Easy, as the pinned revision's `next_states` example
/// asserts them (src/inference.rs:334-352, values at :346-349): `FSRS::default()`, no previous
/// state, a desired retention of 0.9 and 0 days elapsed, compared exactly (`assert_eq!`). A
/// first review replays at the delta 0, which is that example's elapsed time.
const FIRST_STATES: [(f32, f32, f32); 4] = [
    (0.1104, 6.1686, 0.08832),
    (2.2395, 5.261_278, 1.791_600_1),
    (3.9221, 3.530_724_3, 3.13768),
    (11.7841, 1.0, 9.427_279),
];

/// FSRS-7's four initial stabilities, Again to Easy: the first four of the pinned revision's
/// `DEFAULT_PARAMETERS` (src/inference_v7.rs:1-5), typed from that line.
const INITIAL_STABILITIES: [f32; 4] = [0.1104, 2.2395, 3.9221, 11.7841];

/// Each rating's one-review item replayed by `method` from the pinned defaults, Again to Easy.
fn first_reviews(method: Method) -> Result<Vec<MemoryState>, FSRSError> {
    let items = RATINGS
        .iter()
        .map(|&rating| FSRSItem {
            reviews: vec![FSRSReview {
                rating,
                delta_t: 0.0,
            }],
        })
        .collect();
    replay::replay(method, &FSRS::default(), items)
}

#[test]
fn the_replay_equals_the_pinned_revisions_own_vectors() {
    for method in Method::ALL {
        assert_eq!(
            first_reviews(method).expect("four first reviews replay"),
            FIRST_STATES
                .iter()
                .map(|&(stability, difficulty, stability_fast)| MemoryState {
                    stability,
                    difficulty,
                    stability_fast,
                })
                .collect::<Vec<_>>(),
            "{}: the four first reviews, Again to Easy",
            method.name()
        );
    }
}

#[test]
fn each_first_rating_replays_to_its_initial_stability() {
    for method in Method::ALL {
        let stabilities: Vec<u32> = first_reviews(method)
            .expect("four first reviews replay")
            .iter()
            .map(|state| state.stability.to_bits())
            .collect();
        assert_eq!(
            stabilities,
            INITIAL_STABILITIES.map(f32::to_bits).to_vec(),
            "{}: the four first ratings' stabilities, Again to Easy",
            method.name()
        );
    }
}
