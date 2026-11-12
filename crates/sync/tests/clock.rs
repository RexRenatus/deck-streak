//! A device's clock (SPEC-509 A02): each tick and each id becomes its last reading, a pulled clock
//! raises it and a lower one leaves it, and at the top of the range it refuses and keeps its
//! reading.

use deck_streak_sync::{ClockError, Hlc, IdError, Slot, tick};

#[test]
fn the_clock_keeps_what_it_made_and_saw_and_refuses_at_the_top_keeping_its_reading() {
    let mut clock = Hlc::starting_at(100);
    assert_eq!(clock.last(), 100);
    assert_eq!(
        clock.tick_at(50),
        Ok(101),
        "a wall clock behind the last reading"
    );
    assert_eq!(clock.tick_at(500), Ok(500), "a wall clock ahead of it");
    clock.recv(900);
    assert_eq!(clock.last(), 900, "a pulled clock raises the reading");
    clock.recv(10);
    assert_eq!(clock.last(), 900, "a lower one leaves it");
    let slot = Slot::new(5).expect("slot 5");
    assert_eq!(
        clock.next_id_at(0, slot),
        Ok(901),
        "the least id above 900 in slot 5"
    );
    assert_eq!(clock.last(), 901, "the id becomes the reading");
    assert_eq!(clock.tick_at(0), Ok(902));

    let mut top = Hlc::starting_at(i64::MAX);
    assert_eq!(top.tick_at(0), Err(ClockError::Exhausted));
    assert_eq!(top.next_id_at(0, slot), Err(IdError::Exhausted));
    assert_eq!(top.last(), i64::MAX, "a refusal keeps the reading");
    assert_eq!(tick(0, i64::MAX, 0), Err(ClockError::Exhausted));
    assert_eq!(tick(i64::MAX - 1, 0, i64::MIN), Ok(i64::MAX));
    assert_eq!(
        ClockError::Exhausted.to_string(),
        "the hybrid clock is at the largest instant it holds"
    );
}
