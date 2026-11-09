//! The pity counters saturate at the top and the bottom of their range (SPEC-435 section 11,
//! A14 to A20, #903).
//!
//! Each counter stays below its guarantee under the invariants `roll_rarity` keeps, so no stored
//! counter reaches either end of `i64`. Each test plants a counter at that end, where an unchecked
//! sum panics under the test profile and wraps in the release profile, and asserts the answer the
//! saturating sum gives: a counter at the top still forces its guarantee, and a counter at the
//! bottom earns no ramp. Every expected value is a literal.

use deck_streak_quests::chests::{Rarity, epic_odds_pts, roll_rarity};
use deck_streak_quests::pity::Pity;

#[test]
fn a_legendary_counter_at_the_top_of_its_range_still_forces_the_legendary_guarantee() {
    assert_eq!(
        roll_rarity(0.99, 0, i64::MAX, 0.0),
        Rarity::Legendary,
        "a counter at the top of its range is past the Legendary guarantee"
    );
}

#[test]
fn an_epic_counter_at_the_top_of_its_range_still_forces_the_epic_guarantee() {
    assert_eq!(
        roll_rarity(0.99, i64::MAX, 0, 0.0),
        Rarity::Epic,
        "a counter at the top of its range is past the Epic guarantee"
    );
}

#[test]
fn an_epic_counter_at_the_bottom_of_its_range_earns_no_ramp() {
    assert_eq!(
        epic_odds_pts(i64::MIN, 0.0).to_bits(),
        7.0_f64.to_bits(),
        "a counter at the bottom of its range earns the base Epic points and no ramp"
    );
}

#[test]
fn an_epic_at_the_top_of_the_legendary_counter_holds_it_there() {
    let pity = Pity {
        since_epic: 5,
        since_legendary: i64::MAX,
    };

    assert_eq!(
        pity.after(Rarity::Epic),
        Pity {
            since_epic: 0,
            since_legendary: i64::MAX,
        },
        "an Epic resets its counter and holds the Legendary counter at the top"
    );
}

#[test]
fn a_legendary_at_the_top_of_the_epic_counter_holds_it_there() {
    let pity = Pity {
        since_epic: i64::MAX,
        since_legendary: 5,
    };

    assert_eq!(
        pity.after(Rarity::Legendary),
        Pity {
            since_epic: i64::MAX,
            since_legendary: 0,
        },
        "a Legendary resets its counter and holds the Epic counter at the top"
    );
}

#[test]
fn a_common_chest_at_the_top_of_the_epic_counter_holds_it_there() {
    let pity = Pity {
        since_epic: i64::MAX,
        since_legendary: 5,
    };

    assert_eq!(
        pity.after(Rarity::Common),
        Pity {
            since_epic: i64::MAX,
            since_legendary: 6,
        },
        "a Common holds the Epic counter at the top and adds one to the Legendary counter"
    );
}

#[test]
fn a_rare_chest_at_the_top_of_the_legendary_counter_holds_it_there() {
    let pity = Pity {
        since_epic: 5,
        since_legendary: i64::MAX,
    };

    assert_eq!(
        pity.after(Rarity::Rare),
        Pity {
            since_epic: 6,
            since_legendary: i64::MAX,
        },
        "a Rare adds one to the Epic counter and holds the Legendary counter at the top"
    );
}
