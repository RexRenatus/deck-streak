//! SPEC-094 A12: every instrument has one row, and an inert row never runs.

use deck_streak_insights::dark_fields;
use deck_streak_insights::instrument::Cadence;
use deck_streak_insights::registry::{ROWS, Row, State, find, runnable};

#[test]
fn an_inert_instrument_never_runs() {
    let mut ids: Vec<&str> = ROWS.iter().map(|row| row.id).collect();
    let all = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), all, "an instrument has two rows");
    assert!(all >= 1, "the registry is empty");
    let dark = find(ROWS, dark_fields::ID).expect("Dark Fields has a row");
    assert_eq!(dark.cadence, Cadence::Weekly);
    assert_eq!(dark.state, State::Live);

    let inert = Row {
        id: "sleeper",
        cadence: Cadence::Weekly,
        state: State::Inert,
    };
    let rows = [dark, inert];
    let ids: Vec<&str> = runnable(&rows).iter().map(|row| row.id).collect();
    assert_eq!(ids, [dark_fields::ID]);
    assert_eq!(find(&rows, "sleeper"), None);
    assert_eq!(find(&rows, dark_fields::ID), Some(dark));

    let revived = Row {
        state: State::Live,
        ..inert
    };
    let rows = [dark, revived];
    let ids: Vec<&str> = runnable(&rows).iter().map(|row| row.id).collect();
    assert_eq!(ids, [dark_fields::ID, "sleeper"]);
}
