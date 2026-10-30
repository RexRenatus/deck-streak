//! The one encoding (SPEC-509 A01 and A06; the design's T2-1 and T2-6).
//!
//! Every kind of operation is built over small values, exhaustively: each field takes each of a
//! few values, in every combination. Each operation's frame decodes to itself, and every strict
//! prefix of it is refused as a truncation. Two devices that make the same operation write equal
//! bodies, and their frames differ only in the head. Each refusal the decoder names is reached
//! by a frame built for it, and the frame bound admits its last byte and refuses one more.

// An integration test is test code: its helpers panic on a malformed sample, and it prints the
// examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_sync::codec::{HEAD_BYTES, MAX_DURATION_MS, MAX_OP_BYTES, VERSION};
use deck_streak_sync::{
    DecodeError, EncodeError, Key, NotetypePart, Op, OpHead, OpKind, decode, encode, encode_body,
};

/// The digits of one combination, read one field at a time in the base of that field's choices.
struct Digits(usize);

impl Digits {
    fn pick<T: Clone>(&mut self, choices: &[T]) -> T {
        let choice = choices[self.0 % choices.len()].clone();
        self.0 /= choices.len();
        choice
    }
}

const I64: [i64; 2] = [0, -1];
const I32: [i32; 2] = [0, i32::MIN];
const U16: [u16; 2] = [0, u16::MAX];
const KEY: Key = Key {
    t: -3,
    rank: 1,
    origin: [7; 16],
    seq: 9,
};
const KEYS: [Option<Key>; 2] = [None, Some(KEY)];

fn texts() -> [String; 2] {
    [String::new(), "e\u{301}t\u{e9}".to_owned()]
}

/// The operation of `kind` that combination `digits` names.
fn build(kind: OpKind, d: &mut Digits) -> Op {
    match kind {
        OpKind::ReviewAnswered => Op::ReviewAnswered {
            card: d.pick(&I64),
            note: d.pick(&I64),
            home_deck: d.pick(&I64),
            preset: d.pick(&I64),
            ease: d.pick(&[1, 2, 3, 4]),
            kind: d.pick(&[0, 1, 2, 3]),
            duration_ms: d.pick(&[0, MAX_DURATION_MS]),
            study_day: d.pick(&I32),
            basis: d.pick(&KEYS),
            due_hint: d.pick(&[None, Some(-1)]),
            model_version: d.pick(&[None, Some([0xAB; 32])]),
        },
        OpKind::ReviewUndone => Op::ReviewUndone {
            target: d.pick(&[KEY, Key { seq: 1, ..KEY }]),
        },
        OpKind::NoteCreated => Op::NoteCreated {
            note: d.pick(&I64),
            notetype: d.pick(&I64),
            deck: d.pick(&I64),
            guid: d.pick(&[[0; 16], [0xFF; 16]]),
            fields: d.pick(&[Vec::new(), texts().to_vec()]),
        },
        OpKind::NoteFieldSet => Op::NoteFieldSet {
            note: d.pick(&I64),
            field: d.pick(&U16),
            value: d.pick(&texts()),
            base_rev: d.pick(&KEYS),
        },
        OpKind::NoteDeleted => Op::NoteDeleted { note: d.pick(&I64) },
        OpKind::CardMoved => Op::CardMoved {
            card: d.pick(&I64),
            deck: d.pick(&I64),
            base_rev: d.pick(&KEYS),
        },
        OpKind::CardFlag => Op::CardFlag {
            card: d.pick(&I64),
            flag: d.pick(&[0, u8::MAX]),
        },
        OpKind::CardBury => Op::CardBury {
            card: d.pick(&I64),
            day: d.pick(&I32),
        },
        OpKind::CardSuspend => Op::CardSuspend {
            card: d.pick(&I64),
            on: d.pick(&[false, true]),
        },
        OpKind::DeckCreated => Op::DeckCreated {
            deck: d.pick(&I64),
            parent: d.pick(&[None, Some(-1)]),
            name: d.pick(&texts()),
        },
        OpKind::DeckRenamed => Op::DeckRenamed {
            deck: d.pick(&I64),
            name: d.pick(&texts()),
            base_rev: d.pick(&KEYS),
        },
        OpKind::NotetypeEdited => Op::NotetypeEdited {
            notetype: d.pick(&I64),
            part: d.pick(&[
                NotetypePart::Template(2),
                NotetypePart::Css,
                NotetypePart::FieldName(u16::MAX),
            ]),
            value: d.pick(&texts()),
            base_rev: d.pick(&KEYS),
        },
        OpKind::MediaBound => Op::MediaBound {
            name: d.pick(&texts()),
            sha256: d.pick(&[[0; 32], [0x5A; 32]]),
            size: d.pick(&[0, u32::MAX]),
        },
        OpKind::MediaUnbound => Op::MediaUnbound {
            name: d.pick(&texts()),
        },
    }
}

/// Every operation of `kind` over the small values, each combination once.
fn samples(kind: OpKind) -> Vec<Op> {
    let mut found = Vec::new();
    for combination in 0.. {
        let mut digits = Digits(combination);
        let op = build(kind, &mut digits);
        if digits.0 != 0 {
            break;
        }
        found.push(op);
    }
    found
}

const HEAD: OpHead = OpHead {
    origin: [0x11; 16],
    seq: 1,
    hlc: 1_761_955_200_000,
};

#[test]
fn every_variant_round_trips_over_small_values_and_every_truncation_is_refused() {
    let (mut operations, mut cuts) = (0_u64, 0_u64);
    let mut kinds = BTreeSet::new();
    for kind in OpKind::ALL {
        for op in samples(kind) {
            assert_eq!(op.kind(), kind, "{op:?}");
            let frame = encode(&HEAD, &op).expect("a sample encodes");
            assert_eq!(decode(&frame), Ok((HEAD, op.clone())), "{op:?}");
            for cut in 0..frame.len() {
                assert_eq!(
                    decode(&frame[..cut]),
                    Err(DecodeError::Truncated),
                    "{op:?} cut to {cut} byte(s)"
                );
                cuts += 1;
            }
            operations += 1;
            kinds.insert(kind);
        }
    }
    println!(
        "examined {operations} operation(s) of {} kind(s) and {cuts} truncation(s)",
        kinds.len()
    );
    assert_eq!(kinds.len(), OpKind::ALL.len(), "every kind is examined");
    assert!(
        operations >= 8_192,
        "every combination of the answer's fields"
    );
}

#[test]
fn two_devices_encode_one_operation_to_bodies_equal_apart_from_the_head() {
    let one = OpHead {
        origin: [1; 16],
        seq: 7,
        hlc: 1_761_955_200_000,
    };
    let two = OpHead {
        origin: [2; 16],
        seq: 3,
        hlc: 1_761_955_260_000,
    };
    let mut examined = 0_u64;
    for kind in OpKind::ALL {
        for op in samples(kind) {
            let (first, second) = (
                encode(&one, &op).expect("a sample encodes"),
                encode(&two, &op).expect("a sample encodes"),
            );
            let body = encode_body(&op).expect("a sample encodes");
            assert_eq!(&first[HEAD_BYTES..], body.as_slice(), "{op:?}");
            assert_eq!(&second[HEAD_BYTES..], body.as_slice(), "{op:?}");
            assert_ne!(first[..HEAD_BYTES], second[..HEAD_BYTES], "{op:?}");
            examined += 1;
        }
    }
    println!("examined {examined} operation(s) made on two devices");
    assert!(examined > 0, "examined 0 operations");
}

/// A note field set, as a frame to plant a fault in.
fn field_set() -> (Op, Vec<u8>) {
    let op = Op::NoteFieldSet {
        note: 5,
        field: 1,
        value: "ab".to_owned(),
        base_rev: None,
    };
    let frame = encode(&HEAD, &op).expect("a field set encodes");
    (op, frame)
}

#[test]
fn another_version_an_unknown_kind_and_a_trailing_byte_are_refused_by_name() {
    let (_, frame) = field_set();
    for found in [0, VERSION + 1, u8::MAX] {
        let mut planted = frame.clone();
        planted[0] = found;
        assert_eq!(decode(&planted), Err(DecodeError::Version { found }));
    }
    for tag in [0, 15, u8::MAX] {
        let mut planted = frame.clone();
        planted[HEAD_BYTES] = tag;
        assert_eq!(decode(&planted), Err(DecodeError::UnknownKind { tag }));
    }
    for kind in OpKind::ALL {
        assert_eq!(OpKind::from_tag(kind.tag()), Some(kind), "{kind:?}");
    }
    let mut planted = frame;
    planted.extend_from_slice(&[0, 0]);
    assert_eq!(decode(&planted), Err(DecodeError::Trailing { extra: 2 }));
}

#[test]
fn a_value_out_of_its_range_is_refused_by_both_ends() {
    let answer = |ease: u8, kind: u8, duration_ms: u32| Op::ReviewAnswered {
        card: 1,
        note: 2,
        home_deck: 3,
        preset: 4,
        ease,
        kind,
        duration_ms,
        study_day: 5,
        basis: None,
        due_hint: None,
        model_version: None,
    };
    let frame = encode(&HEAD, &answer(1, 0, 0)).expect("an answer encodes");
    // The ease, the kind and the duration follow the tag and four ids.
    let ease_at = HEAD_BYTES + 1 + 4 * 8;
    for (ease, kind, duration_ms, field) in [
        (0, 0, 0, "ease"),
        (5, 0, 0, "ease"),
        (1, 4, 0, "kind"),
        (4, 0, 3_600_001, "duration_ms"),
    ] {
        assert_eq!(
            encode(&HEAD, &answer(ease, kind, duration_ms)),
            Err(EncodeError::OutOfRange { field }),
            "{field}"
        );
        let mut planted = frame.clone();
        planted[ease_at] = ease;
        planted[ease_at + 1] = kind;
        planted[ease_at + 2..ease_at + 6].copy_from_slice(&duration_ms.to_le_bytes());
        assert_eq!(
            decode(&planted),
            Err(DecodeError::OutOfRange { field }),
            "{field}"
        );
    }
    assert_eq!(
        decode(&encode(&HEAD, &answer(4, 3, 3_600_000)).expect("the top of each range")),
        Ok((HEAD, answer(4, 3, 3_600_000)))
    );

    let suspend = encode(&HEAD, &Op::CardSuspend { card: 1, on: true }).expect("a suspend");
    let mut planted = suspend;
    *planted.last_mut().expect("a flag byte") = 2;
    assert_eq!(
        decode(&planted),
        Err(DecodeError::OutOfRange { field: "on" })
    );

    let (_, frame) = field_set();
    let mut planted = frame.clone();
    *planted.last_mut().expect("an option byte") = 2;
    assert_eq!(
        decode(&planted),
        Err(DecodeError::OutOfRange { field: "option" })
    );
    // The value's two bytes follow the tag, the note, the field and the length.
    let value_at = HEAD_BYTES + 1 + 8 + 2 + 4;
    let mut planted = frame;
    planted[value_at] = 0xFF;
    assert_eq!(decode(&planted), Err(DecodeError::NotUtf8));

    // A note holds at most as many fields as the 16-bit count can say.
    let crowded = Op::NoteCreated {
        note: 1,
        notetype: 2,
        deck: 3,
        guid: [0; 16],
        fields: vec![String::new(); usize::from(u16::MAX) + 1],
    };
    assert_eq!(
        encode(&HEAD, &crowded),
        Err(EncodeError::OutOfRange { field: "fields" })
    );

    let css = Op::NotetypeEdited {
        notetype: 1,
        part: NotetypePart::Css,
        value: String::new(),
        base_rev: None,
    };
    let mut planted = encode(&HEAD, &css).expect("a notetype edit");
    planted[HEAD_BYTES + 1 + 8] = 3;
    assert_eq!(
        decode(&planted),
        Err(DecodeError::OutOfRange { field: "part" })
    );
}

#[test]
fn a_frame_at_the_bound_is_admitted_and_one_byte_more_is_refused() {
    // A media unbinding is the head, the tag, the name's length and the name.
    let longest = MAX_OP_BYTES - HEAD_BYTES - 1 - 4;
    let unbind = |len: usize| Op::MediaUnbound {
        name: "n".repeat(len),
    };
    let frame = encode(&HEAD, &unbind(longest)).expect("a frame at the bound");
    assert_eq!(frame.len(), MAX_OP_BYTES);
    assert_eq!(decode(&frame), Ok((HEAD, unbind(longest))));
    assert_eq!(
        encode(&HEAD, &unbind(longest + 1)),
        Err(EncodeError::TooLarge {
            bytes: MAX_OP_BYTES + 1
        })
    );
    let mut over = frame;
    over.push(0);
    assert_eq!(
        decode(&over),
        Err(DecodeError::TooLarge {
            bytes: MAX_OP_BYTES + 1
        })
    );
}

#[test]
fn each_refusal_reads_as_its_sentence() {
    for (refusal, sentence) in [
        (
            EncodeError::TooLarge { bytes: 9 }.to_string(),
            "an operation of 9 bytes is over 262144",
        ),
        (
            EncodeError::OutOfRange { field: "ease" }.to_string(),
            "the field ease is out of its range",
        ),
        (
            DecodeError::TooLarge { bytes: 9 }.to_string(),
            "9 bytes is over 262144",
        ),
        (
            DecodeError::Truncated.to_string(),
            "the frame ends inside a field",
        ),
        (
            DecodeError::Version { found: 2 }.to_string(),
            "wire version 2 is not 1",
        ),
        (
            DecodeError::UnknownKind { tag: 15 }.to_string(),
            "tag 15 names no operation",
        ),
        (
            DecodeError::OutOfRange { field: "on" }.to_string(),
            "the field on is out of its range",
        ),
        (
            DecodeError::NotUtf8.to_string(),
            "a text field is not UTF-8",
        ),
        (
            DecodeError::Trailing { extra: 2 }.to_string(),
            "2 byte(s) follow the last field",
        ),
    ] {
        assert_eq!(refusal, sentence);
    }
}
