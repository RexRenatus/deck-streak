//! SPEC-358 A4: the review's flag and bury rules, judged in the core, where both clients take them
//! (SPEC-358 R2). The two tests moved here from the web engine's study tests with their names kept,
//! and still pin what SPEC-350 A4 and A5 hold.

use deck_streak_engine_core::review::{BuryOf, bury_of, toggled_red};

#[test]
fn the_flag_toggles_red() {
    // SPEC-350 R7: no flag turns red, red turns to none, and any other flag turns red.
    assert_eq!(toggled_red(0), 1);
    assert_eq!(toggled_red(1), 0);
    assert_eq!(toggled_red(2), 1);
    assert_eq!(toggled_red(7), 1);
}

#[test]
fn bury_is_the_users_bury_of_the_shown_card() {
    // SPEC-350 R2: one card, no note, and the user's bury, the engine's mode 2.
    assert_eq!(
        bury_of(42),
        BuryOf {
            card_ids: vec![42],
            note_ids: vec![],
            mode: 2,
        }
    );
}
