//! SPEC-358 A4: the review's flag and bury rules, judged in the core, where both clients take them
//! (SPEC-358 R2). The two tests moved here from the web engine's study tests with their names kept,
//! and still pin what SPEC-350 A4 and A5 hold.
//! A third pins the bytes of the two requests the native adapter sends for them (SPEC-358 R3).

use deck_streak_engine_core::review::{BuryOf, bury_of, bury_request, flag_request, toggled_red};

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

#[test]
fn the_bury_and_flag_requests_are_the_engines_bytes() {
    // SPEC-358 R3: spelled by hand from the engine's messages at the pinned rev, never by the
    // encoder under test. BuryOrSuspendCardsRequest { card_ids (1): [7], mode (3): 2 } is the
    // packed list `0a 01 07`, then the mode `18 02`; SetFlagRequest { card_ids (1): [7], flag
    // (2): 1 } is `0a 01 07`, then the flag `10 01`.
    assert_eq!(
        (bury_request(bury_of(7)), flag_request(7, 1)),
        (
            vec![0x0a, 0x01, 0x07, 0x18, 0x02],
            vec![0x0a, 0x01, 0x07, 0x10, 0x01]
        ),
        "the native bury's and flag's requests, encoded"
    );
}
