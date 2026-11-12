//! A11 (SPEC-602 R11): the wire walk refuses a truncation, an unsupported wire type, a varint at
//! seventy bits and a step that makes no progress, and reads fixed vectors and seeded messages.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use deck_streak_anki_format::wire::{
    NO_PROGRESS, WireError, WireField, WireValue, walk, walk_with,
};
use support::examined;

fn refused(text: &str) -> Result<Vec<WireField<'static>>, WireError> {
    Err(WireError(text.to_owned()))
}

#[test]
fn the_walk_refuses_a_truncated_varint_and_field() {
    let cases: Vec<(&[u8], &str)> = vec![
        (&[0x08], "truncated varint"),
        (&[0x08, 0x80], "truncated varint"),
        (&[0x0a, 0x05, 0x01], "truncated length-delimited field"),
        (&[0x09, 0x01, 0x02, 0x03], "truncated 64-bit field"),
        (&[0x0d, 0x01], "truncated 32-bit field"),
    ];
    for (data, text) in examined("truncated message(s)", cases) {
        assert_eq!(walk(data), refused(text), "{data:02x?}");
    }
}

#[test]
fn the_walk_refuses_an_unsupported_wire_type() {
    for wire_type in examined("unsupported wire type(s)", vec![3_u8, 4, 6, 7]) {
        let data = [0x08 | wire_type, 0x00];
        assert_eq!(
            walk(&data),
            refused(&format!("unsupported wire type {wire_type}")),
            "{data:02x?}"
        );
    }
}

#[test]
fn the_walk_refuses_a_varint_at_seventy_bits() {
    // Ten continuation bytes reach a shift of 70 bits: refused there.
    let mut at_seventy = vec![0x08];
    at_seventy.extend_from_slice(&[0x80; 10]);
    at_seventy.push(0x01);
    assert_eq!(walk(&at_seventy), refused("varint too long"), "70 bits");
    // Nine continuation bytes stay under the limit: read.
    let mut under = vec![0x08];
    under.extend_from_slice(&[0x80; 9]);
    under.push(0x01);
    assert_eq!(
        walk(&under),
        Ok(vec![WireField {
            number: 1,
            value: WireValue::Varint(1 << 63)
        }]),
        "63 bits"
    );
}

#[test]
fn the_walk_refuses_a_step_that_makes_no_progress() {
    let data = [0x08, 0x01, 0x10, 0x02];
    let mut calls = 0;
    let stalled = walk_with(&data, |_, at| {
        calls += 1;
        assert!(calls < 3, "the walk stops at the first stalled step");
        Ok((
            WireField {
                number: 1,
                value: WireValue::Varint(0),
            },
            at,
        ))
    });
    assert_eq!(stalled, refused(NO_PROGRESS), "a step that stays");
    assert_eq!(calls, 1, "one step was taken");

    let mut calls = 0;
    let back = walk_with(&data, |_, at| {
        calls += 1;
        assert!(calls < 4, "the walk stops at the first step back");
        let next = if at == 0 { 2 } else { 1 };
        Ok((
            WireField {
                number: 1,
                value: WireValue::Varint(0),
            },
            next,
        ))
    });
    assert_eq!(back, refused(NO_PROGRESS), "a step back");
    assert_eq!(calls, 2, "two steps were taken");
}

/// A fixed-seed generator (xorshift64), so every run walks the same messages.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = u8::try_from(value & 0x7f).expect("seven bits");
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Each field of a message: its number, wire type, value bytes and varint value.
type Fields = Vec<(u64, u8, Vec<u8>, u64)>;

/// One message of 1 to 6 fields, each field's number, wire type and value drawn from `rng`.
fn message(rng: &mut Seeded) -> (Vec<u8>, Fields) {
    let mut data = Vec::new();
    let mut fields = Vec::new();
    for _ in 0..=rng.next() % 6 {
        let number = 1 + rng.next() % 2000;
        let wire_type = [0_u8, 1, 2, 5][usize::try_from(rng.next() % 4).expect("an index")];
        varint(&mut data, (number << 3) | u64::from(wire_type));
        match wire_type {
            0 => {
                let value = rng.next() >> (rng.next() % 64);
                varint(&mut data, value);
                fields.push((number, wire_type, Vec::new(), value));
            }
            1 | 5 => {
                let width = if wire_type == 1 { 8 } else { 4 };
                let bytes: Vec<u8> = rng.next().to_le_bytes()[..width].to_vec();
                data.extend_from_slice(&bytes);
                fields.push((number, wire_type, bytes, 0));
            }
            _ => {
                let length = usize::try_from(rng.next() % 12).expect("a length");
                let bytes: Vec<u8> = (0..length).map(|_| rng.next().to_le_bytes()[0]).collect();
                varint(&mut data, u64::try_from(length).expect("a length"));
                data.extend_from_slice(&bytes);
                fields.push((number, wire_type, bytes, 0));
            }
        }
    }
    (data, fields)
}

#[test]
fn the_walk_reads_fixed_vectors_and_seeded_messages() {
    let fixed: Vec<(&[u8], Vec<WireField<'static>>)> = vec![
        (&[], vec![]),
        (
            &[0x08, 0x96, 0x01],
            vec![WireField {
                number: 1,
                value: WireValue::Varint(150),
            }],
        ),
        (
            &[0x12, 0x02, 0x68, 0x69, 0x1d, 1, 2, 3, 4],
            vec![
                WireField {
                    number: 2,
                    value: WireValue::Length(b"hi"),
                },
                WireField {
                    number: 3,
                    value: WireValue::Fixed32(&[1, 2, 3, 4]),
                },
            ],
        ),
    ];
    for (data, wanted) in examined("fixed vector(s)", fixed) {
        assert_eq!(walk(data), Ok(wanted), "{data:02x?}");
    }
    let mut rng = Seeded(0x5eed_0602);
    let mut walked = Vec::new();
    for _ in 0..500 {
        let (data, wanted) = message(&mut rng);
        let found = walk(&data).expect("a seeded message walks");
        let found: Fields = found
            .iter()
            .map(|field| {
                let number = u64::try_from(field.number).expect("a field number");
                match field.value {
                    WireValue::Varint(value) => (
                        number,
                        0,
                        Vec::new(),
                        u64::try_from(value).expect("a 64-bit value"),
                    ),
                    WireValue::Fixed64(bytes) => (number, 1, bytes.to_vec(), 0),
                    WireValue::Length(bytes) => (number, 2, bytes.to_vec(), 0),
                    WireValue::Fixed32(bytes) => (number, 5, bytes.to_vec(), 0),
                }
            })
            .collect();
        assert_eq!(found, wanted, "{data:02x?}");
        walked.push(data);
    }
    examined("seeded message(s)", walked);
}
