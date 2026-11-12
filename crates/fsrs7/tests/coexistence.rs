//! SPEC-342 A4 (R3; ADR-338's coexistence, ADR-353 D5): the released scheduler the engine pins and
//! the pinned FSRS-7 package link into one test binary, and each answers from its own model.

/// One Good review, as the first of a card's history.
const GOOD: u32 = 3;

#[test]
fn both_schedulers_link_into_one_binary_and_answer_from_their_own_models() {
    let released = fsrs6::FSRS::new(&fsrs6::DEFAULT_PARAMETERS)
        .expect("the released package's own defaults are a valid model");
    let pinned = fsrs7::FSRS::new(&[]).expect("no parameters select FSRS-7's defaults");

    let six = released
        .memory_state(
            fsrs6::FSRSItem {
                reviews: vec![fsrs6::FSRSReview {
                    rating: GOOD,
                    delta_t: 0u32,
                }],
            },
            None,
        )
        .expect("the released package replays one Good review");
    let seven = pinned
        .memory_state(
            fsrs7::FSRSItem {
                reviews: vec![fsrs7::FSRSReview {
                    rating: GOOD,
                    delta_t: 0.0f32,
                }],
            },
            None,
        )
        .expect("the pinned package replays one Good review");

    assert!(
        seven.stability_fast.is_finite() && seven.stability_fast > 0.0,
        "FSRS-7's memory state carries a fast stability trace, read {}",
        seven.stability_fast
    );
    assert_ne!(
        six.stability.to_bits(),
        seven.stability.to_bits(),
        "the two packages answered one Good review with the same stability {}: one model answered \
         for both",
        six.stability
    );
    assert_eq!(
        fsrs6::DEFAULT_PARAMETERS.len(),
        21,
        "the released package's defaults are FSRS-6's 21 parameters"
    );
}
