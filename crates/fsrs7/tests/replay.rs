//! SPEC-342 A8 and A9 (R6): the replay from the pinned revision's FSRS-7 defaults, card by card and
//! in one batch.

use deck_streak_fsrs7::convert;
use deck_streak_fsrs7::measure::history;
use deck_streak_fsrs7::replay::{self, Method};
use fsrs7::{FSRS, FSRSItem, FSRSReview};

/// FSRS-7's initial stability for Good at the pinned revision: the model takes a first review's
/// stability from its parameters by rating (`init_stability`, src/model.rs:173-175), and Good's is
/// the third of the FSRS-7 defaults (`DEFAULT_PARAMETERS`, src/inference_v7.rs:1-5). Typed from
/// that line, never read from the crate.
const INITIAL_STABILITY_FOR_GOOD: f32 = 3.9221;
/// The most two methods' values for one card may differ by, relative to the larger.
const RELATIVE: f64 = 1e-6;

fn model() -> FSRS {
    FSRS::new(&[]).expect("no parameters select FSRS-7's defaults")
}

fn agree(left: f64, right: f64) -> bool {
    (left - right).abs() <= RELATIVE * left.abs().max(right.abs())
}

#[test]
fn one_good_review_replays_to_the_models_initial_stability_for_good() {
    let model = model();
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
    let model = model();
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
