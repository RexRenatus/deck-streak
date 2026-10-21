//! The XP crate's review XP is the predecessor's (SPEC-360 A1, A12; R1).
//!
//! The golden `review_xp.json` holds what `gamification/xp.py:review_xp` returned at `27ee2bc` over
//! every ease, maturity, type and tier, the ties and the rows that are not study events. The crate
//! is held to it directly, over its own `ReviewFacts` and `Tier`, before any caller translates.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_xp::review_xp::{ReviewFacts, Tier, review_xp};
use deck_streak_xp::table::table;
use serde_json::Value;

/// A golden case's tier: `null` or a tag from `T1` to `T4`. The crate parses no tag (tier parsing
/// stays in ingest), so this local match maps the golden's own spelling and refuses any other.
fn tier_of(tag: &Value) -> Option<Tier> {
    if tag.is_null() {
        return None;
    }
    match tag.as_str() {
        Some("T1") => Some(Tier::T1),
        Some("T2") => Some(Tier::T2),
        Some("T3") => Some(Tier::T3),
        Some("T4") => Some(Tier::T4),
        _ => panic!("a golden tier is null or T1 to T4, not {tag}"),
    }
}

#[test]
fn the_xp_crates_review_xp_matches_the_parity_golden_for_every_case() {
    golden::each_case("review_xp", |case| {
        let field = |name: &str| {
            case.input[name]
                .as_i64()
                .unwrap_or_else(|| panic!("{name} is an integer in {}", case.input))
        };
        let review = ReviewFacts {
            ease: field("ease"),
            interval: field("ivl"),
            kind: field("rtype"),
        };
        let tier = tier_of(&case.input["tier"]);
        let expected = case.output.as_u64().expect("XP is an unsigned integer");
        assert_eq!(
            u64::from(review_xp(&review, tier)),
            expected,
            "the XP of {}",
            case.input
        );
    });
}

/// An ease past four multiplies by one, as the predecessor's `_ => 1.0` arm does. The golden holds
/// no such answer, so this pins the arm for every study type (ruling 448 (6)).
#[test]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the rounded product of a base and multipliers of a few digits is a small non-negative \
              number, as the rule's own cast is"
)]
fn an_ease_past_four_multiplies_by_one_for_every_study_type() {
    let t = table();
    for kind in 0_i64..=3 {
        let index = usize::try_from(kind).expect("a study type from 0 to 3 indexes the table");
        let review = ReviewFacts {
            ease: 5,
            interval: 0,
            kind,
        };
        let expected =
            (t.base * 1.0 * t.fresh * t.types[index] * t.untagged).round_ties_even() as u32;
        assert_eq!(
            review_xp(&review, None),
            expected,
            "the XP of ease 5, interval 0, type {kind} and no tier"
        );
    }
}
