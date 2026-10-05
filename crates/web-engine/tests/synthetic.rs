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

/// An independent `SplitMix64` (the published generator and its constants), so the module's
/// draws are judged against the algorithm itself and not against themselves.
struct Reference(u64);

impl Reference {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A field as the module's doc comment describes it: the label and the note's number, then words
/// of one to three syllables drawn until the text reaches 200 characters, cut to exactly 200.
fn reference_field(label: &str, number: u32, draw: &mut Reference) -> String {
    const SYLLABLES: [&str; 16] = [
        "ka", "lo", "mi", "ne", "ru", "sa", "te", "vi", "zo", "pa", "de", "fu", "gi", "ho", "ja",
        "be",
    ];
    let mut text = format!("{label} {number}");
    while text.len() < 200 {
        text.push(' ');
        let syllables = draw.next() % 3 + 1;
        for _ in 0..syllables {
            let at = usize::try_from(draw.next() % 16).unwrap_or(0);
            text.push_str(SYLLABLES[at]);
        }
    }
    text.truncate(200);
    text
}

#[test]
fn the_fields_follow_the_published_generator_draw_for_draw() {
    // The published SplitMix64 vectors for seed 0 pin the reference itself.
    let mut vectors = Reference(0);
    assert_eq!(
        [vectors.next(), vectors.next(), vectors.next()],
        [
            0xE220_A839_7B1D_CDAF,
            0x6E78_9E6A_A1B9_65F4,
            0x06C4_5D18_8009_454F
        ]
    );
    // Over a population wide enough that some front ends exactly on 200 characters, which is the
    // case where the loop's bound decides how many draws the back field starts after.
    let numbers: Vec<u32> = (0..5_000).chain([u32::MAX]).collect();
    for number in examined("synthetic note(s) against the generator", numbers) {
        let mut draw = Reference(u64::from(number));
        let front = reference_field("front", number, &mut draw);
        let back = reference_field("back", number, &mut draw);
        assert_eq!(fields(number), [front, back], "note {number}'s fields");
    }
}
