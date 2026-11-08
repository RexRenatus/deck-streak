//! The double-XP token's settle source (SPEC-411 A1; R1): `2x:<id>` is derived exactly when its id
//! is a positive decimal integer with no sign and no leading zero that fits `i64`, and no other
//! source changes its answer. Every source here is written by hand.

use deck_streak_progression::settle::is_derived;

#[test]
fn a_double_xp_source_is_derived_only_with_a_positive_token_id() {
    let sources = [
        "2x:1",
        "2x:42",
        "2x:9223372036854775807",
        "2x:0",
        "2x:-1",
        "2x:+1",
        "2x:01",
        "2x:",
        "2x:9223372036854775808",
        "2x:1a",
        "2x: 1",
        "2X:1",
        "x2:1",
        "chest:1",
        "reviews",
        "consistency",
        "read:qaa",
        "read:",
    ];
    let answers: Vec<(&str, bool)> = sources
        .iter()
        .map(|source| (*source, is_derived(source)))
        .collect();
    assert_eq!(
        answers,
        [
            ("2x:1", true),
            ("2x:42", true),
            ("2x:9223372036854775807", true),
            ("2x:0", false),
            ("2x:-1", false),
            ("2x:+1", false),
            ("2x:01", false),
            ("2x:", false),
            ("2x:9223372036854775808", false),
            ("2x:1a", false),
            ("2x: 1", false),
            ("2X:1", false),
            ("x2:1", false),
            ("chest:1", false),
            ("reviews", true),
            ("consistency", true),
            ("read:qaa", true),
            ("read:", false),
        ],
        "a 2x source is derived only with a positive token id, and no other source moved"
    );
}
