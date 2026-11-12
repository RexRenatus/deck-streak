//! A review's XP is the predecessor's (SPEC-072 A1; R1, R4).
//!
//! The golden `review_xp.json` holds what `gamification/xp.py:review_xp` returned at `27ee2bc` over
//! every ease, maturity, type and tier, the ties and the rows that are not study events.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::reader::Review;
use deck_streak_ingest::tier::parse_tier;
use deck_streak_progression::review_xp::review_xp;

#[test]
fn review_xp_matches_the_parity_golden_for_every_combination() {
    golden::each_case("review_xp", |case| {
        let field = |name: &str| {
            case.input[name]
                .as_i64()
                .unwrap_or_else(|| panic!("{name} is an integer in {}", case.input))
        };
        let review = Review {
            id: 0,
            card_id: 0,
            ease: field("ease"),
            interval: field("ivl"),
            last_interval: 0,
            factor: 0,
            taken_ms: 0,
            kind: field("rtype"),
        };
        let tier = case.input["tier"].as_str().and_then(parse_tier);
        let expected = case.output.as_u64().expect("XP is an unsigned integer");
        assert_eq!(
            u64::from(review_xp(&review, tier)),
            expected,
            "the XP of {}",
            case.input
        );
    });
}
