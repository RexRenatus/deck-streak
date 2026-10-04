//! SPEC-338 A19: the synthetic notes the web engine's `seed` writes have the shape of ADR-022's
//! measured collection, judged natively.

// The examined helper prints its count on purpose; clippy.toml's in-test allowances cover only
// `#[test]` bodies.
#![allow(clippy::print_stdout)]

use std::collections::HashSet;

use deck_streak_web_engine::synthetic::fields;

fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn each_synthetic_note_carries_two_fields_of_two_hundred_characters() {
    // ADR-022's collection: 250,000 notes, each with two text fields of 200 characters.
    let numbers: Vec<u32> = (0..250_000).chain([u32::MAX]).collect();
    let mut fronts = HashSet::new();
    for number in examined("synthetic note(s)", numbers) {
        let [front, back] = fields(number);
        assert_eq!(
            (front.chars().count(), back.chars().count()),
            (200, 200),
            "note {number}'s fields"
        );
        assert!(
            front.is_ascii() && back.is_ascii(),
            "note {number}'s fields are not ASCII"
        );
        assert_ne!(front, back, "note {number}'s front is its back");
        assert!(
            fronts.insert(front),
            "note {number}'s front repeats another note's"
        );
    }
    // The fields are the same on every run and every target: a figure measured over them can be
    // measured again.
    assert_eq!(fields(4242), fields(4242));
}
