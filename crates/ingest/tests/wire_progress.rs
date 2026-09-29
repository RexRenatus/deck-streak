//! SPEC-094 R3, ADR-095 amendment: the wire walk refuses a pass that does not advance, and keeps
//! nothing from it. Every reader here is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use deck_streak_ingest::wire::{NO_PROGRESS, WireError, WireField, WireValue, walk, walk_with};

fn field(number: u128) -> WireField<'static> {
    WireField {
        number,
        value: WireValue::Varint(0),
    }
}

#[test]
fn a_reader_that_stays_is_refused_with_nothing_kept() {
    let data = [1_u8, 2, 3];
    let mut calls = 0;
    let got = walk_with(&data, |_, at| {
        calls += 1;
        assert!(calls < 3, "the walk went on past a stalled pass");
        Ok((field(9), at))
    });
    assert_eq!(got, Err(WireError(NO_PROGRESS.to_owned())));
    assert_eq!(calls, 1, "refused on the first pass");
}

#[test]
fn a_reader_that_steps_back_is_refused() {
    let data = [1_u8, 2, 3];
    let got = walk_with(&data, |_, at| Ok((field(9), at.saturating_sub(1))));
    assert_eq!(got, Err(WireError(NO_PROGRESS.to_owned())));
    let mut position = 0;
    let later = walk_with(&data, |_, at| {
        position += 1;
        Ok((field(9), if position == 1 { at + 2 } else { at - 1 }))
    });
    assert_eq!(later, Err(WireError(NO_PROGRESS.to_owned())));
}

#[test]
fn a_reader_that_advances_by_one_keeps_every_field_in_order() {
    let data = [1_u8, 2, 3];
    let got = walk_with(&data, |_, at| Ok((field(at as u128), at + 1))).expect("walk");
    assert_eq!(got, vec![field(0), field(1), field(2)]);
    assert_eq!(walk_with(&[], |_, at| Ok((field(0), at + 1))), Ok(vec![]));
}

#[test]
fn a_reader_error_passes_through_untouched() {
    let data = [1_u8];
    let got = walk_with(&data, |_, _| Err(WireError("the read failed".to_owned())));
    assert_eq!(got, Err(WireError("the read failed".to_owned())));
}

#[test]
fn the_real_walk_reads_a_varint_field_and_a_length_field() {
    let data = [0x08_u8, 0x96, 0x01, 0x12, 0x02, b'h', b'i'];
    let got = walk(&data).expect("walk");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].number, 1);
    assert_eq!(got[0].value, WireValue::Varint(150));
    assert_eq!(got[1].number, 2);
    assert_eq!(got[1].value, WireValue::Length(b"hi"));
}
