//! The codec (SPEC-501 C2 and C3): every request and reply decodes to itself after encoding, over
//! a seeded generator that reaches every kind; the frames are byte-equal to the committed
//! layout in `formal/vectors/companion-frames.jsonl`; and the decoder refuses each malformed
//! frame by name. Every enumerating test prints `examined N` and refuses zero.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_companion::codec::{
    DecodeError, Frame, decode_reply, decode_request, encode_reply, encode_request, reply_kind_of,
    request_kind_of,
};
use deck_streak_companion::wire::{
    Adapter, CheckpointReply, FOLD_MAX, FRAME_MAX, HelloReply, MAJOR, MINOR, Model, Preview, Reply,
    Request, Restore, ReviewEvent, Status,
};
use serde_json::Value;

/// The committed frames, one JSON line per message: its name and its bytes in hex.
const FRAMES: &str = include_str!("../../../formal/vectors/companion-frames.jsonl");

/// How many messages of each direction the generator draws.
const DRAWS: usize = 3_000;

/// `SplitMix64`: a small seeded generator, so every run draws the same messages.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn u8(&mut self) -> u8 {
        self.next().to_le_bytes()[0]
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes([self.u8(), self.u8()])
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes([self.u8(), self.u8(), self.u8(), self.u8()])
    }

    fn i32(&mut self) -> i32 {
        i32::from_le_bytes(self.u32().to_le_bytes())
    }

    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(self.next().to_le_bytes())
    }

    fn words(&mut self) -> [u64; 4] {
        [self.next(), self.next(), self.next(), self.next()]
    }

    fn status(&mut self) -> Status {
        [
            Status::Ok,
            Status::Absent,
            Status::FixtureRefuses,
            Status::Busy,
            Status::Error,
            Status::Conflict,
            Status::Unsupported,
        ][usize::from(self.u8() % 7)]
    }

    fn adapter(&mut self) -> Adapter {
        if self.u8().is_multiple_of(2) {
            Adapter::Off
        } else {
            Adapter::OwnerOnly
        }
    }

    fn bytes(&mut self) -> Vec<u8> {
        let count = self.below(65);
        (0..count).map(|_| self.u8()).collect()
    }

    fn event(&mut self) -> ReviewEvent {
        ReviewEvent {
            t_ms: self.i64(),
            origin: [self.next(), self.next()],
            seq: self.next(),
            card: self.i64(),
            note: self.i64(),
            home_deck: self.i64(),
            preset: self.i64(),
            grade: self.u8(),
            kind: self.u8(),
            duration_ms: self.u32(),
            study_day: self.i32(),
        }
    }

    fn checkpoint(&mut self) -> CheckpointReply {
        CheckpointReply {
            status: self.status(),
            model_id: self.words(),
            adapter: self.adapter(),
            bytes: self.bytes(),
        }
    }

    fn request(&mut self) -> Request {
        match self.below(9) {
            0 => Request::Hello {
                major: self.u16(),
                minor: self.u16(),
                release: [self.u16(), self.u16(), self.u16()],
            },
            1 => {
                let count = self.below(9);
                Request::Fold {
                    events: (0..count).map(|_| self.event()).collect(),
                }
            }
            2 => Request::Preview(Preview {
                event: self.event(),
                target_ppm: self.u32(),
            }),
            3 => Request::Commit {
                event: self.event(),
            },
            4 => Request::Retract {
                t_ms: self.i64(),
                origin: [self.next(), self.next()],
                seq: self.next(),
            },
            5 => Request::Checkpoint,
            6 => Request::Restore(Restore::from(self.checkpoint())),
            7 => Request::Purge,
            _ => Request::StateHash,
        }
    }

    fn reply(&mut self) -> Reply {
        match self.below(6) {
            0 => Reply::Hello(HelloReply {
                status: self.status(),
                major: self.u16(),
                minor: self.u16(),
                build_id: self.words(),
                model_id: self.words(),
                model: if self.u8().is_multiple_of(2) {
                    Model::Trained
                } else {
                    Model::Fixture
                },
                adapter: self.adapter(),
            }),
            1 => Reply::Ack {
                status: self.status(),
                folded: self.next(),
                head_t_ms: self.i64(),
            },
            2 => Reply::Due {
                status: self.status(),
                card: self.i64(),
                staged_t_ms: self.i64(),
                interval_days: self.u32(),
                due_day: self.i32(),
                model_id: self.words(),
            },
            3 => Reply::Checkpoint(self.checkpoint()),
            4 => Reply::Hash {
                status: self.status(),
                words: self.words(),
            },
            _ => Reply::Refused {
                status: self.status(),
            },
        }
    }
}

#[test]
fn every_generated_request_and_reply_decodes_to_itself() {
    let mut draw = Draw(0x5EED_0501);
    let mut kinds: BTreeMap<u8, usize> = BTreeMap::new();
    for _ in 0..DRAWS {
        let epoch = draw.u32();
        let request = draw.request();
        let frame = encode_request(epoch, &request).expect("a generated request encodes");
        let expected = Frame {
            major: MAJOR,
            minor: MINOR,
            epoch,
            message: request.clone(),
        };
        assert_eq!(decode_request(&frame), Ok(expected), "{request:?}");
        *kinds.entry(request_kind_of(&request)).or_default() += 1;
        let reply = draw.reply();
        let frame = encode_reply(epoch, &reply).expect("a generated reply encodes");
        let expected = Frame {
            major: MAJOR,
            minor: MINOR,
            epoch,
            message: reply.clone(),
        };
        assert_eq!(decode_reply(&frame), Ok(expected), "{reply:?}");
        *kinds.entry(reply_kind_of(&reply)).or_default() += 1;
    }
    println!(
        "examined {} message(s) over {} kind(s): {kinds:?}",
        2 * DRAWS,
        kinds.len()
    );
    assert_eq!(
        kinds.len(),
        15,
        "the generator reaches every request and reply kind"
    );
}

/// A frame written by hand from the layout, independent of the crate's encoder: the header with
/// `len`, major 1, minor 0, `kind` and epoch 7, then `body`.
fn raw_frame(kind: u8, body: &[u8]) -> Vec<u8> {
    let len = u32::try_from(13 + body.len()).expect("a frame's length");
    let mut frame = Vec::with_capacity(13 + body.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&1_u16.to_le_bytes());
    frame.extend_from_slice(&0_u16.to_le_bytes());
    frame.push(kind);
    frame.extend_from_slice(&7_u32.to_le_bytes());
    frame.extend_from_slice(body);
    frame
}

/// The layout's kind bytes this file writes by hand.
const HELLO: u8 = 1;
const FOLD: u8 = 2;
const PURGE: u8 = 8;
const REFUSED: u8 = 0x86;

#[test]
fn a_frame_of_another_minor_decodes_and_keeps_its_minor() {
    // A `Refused` reply with status Busy (3), written at minor 7.
    let mut frame = raw_frame(REFUSED, &[3]);
    frame[6..8].copy_from_slice(&7_u16.to_le_bytes());
    let decoded = decode_reply(&frame).expect("a minor difference is admitted");
    assert_eq!((decoded.major, decoded.minor, decoded.epoch), (MAJOR, 7, 7));
    assert_eq!(
        decoded.message,
        Reply::Refused {
            status: Status::Busy
        }
    );
}

#[test]
fn random_bytes_never_panic_the_decoder() {
    let mut draw = Draw(0xF00D_0501);
    let mut decoded = 0_usize;
    let rounds = 20_000;
    for round in 0..rounds {
        let length = usize::try_from(draw.below(96)).expect("a small length");
        let mut bytes: Vec<u8> = (0..length).map(|_| draw.u8()).collect();
        // Half the rounds carry a well-formed header, so the bodies are read too.
        if round % 2 == 0 && bytes.len() >= 13 {
            let len = u32::try_from(bytes.len()).expect("a small frame");
            bytes[..4].copy_from_slice(&len.to_le_bytes());
            bytes[4..6].copy_from_slice(&MAJOR.to_le_bytes());
        }
        decoded += usize::from(decode_request(&bytes).is_ok());
        decoded += usize::from(decode_reply(&bytes).is_ok());
    }
    println!("examined {rounds} random frame(s); {decoded} decoded");
}

/// The named messages the committed frames hold, in the file's order.
#[expect(
    clippy::too_many_lines,
    reason = "one literal per committed frame, in the file's order"
)]
fn named_messages() -> Vec<(&'static str, u32, Result<Request, Reply>)> {
    let event = ReviewEvent {
        t_ms: 1_700_000_000_123,
        origin: [0x0102_0304_0506_0708, 0x1112_1314_1516_1718],
        seq: 9,
        card: 1_600_000_000_001,
        note: 1_600_000_000_002,
        home_deck: 1_600_000_000_003,
        preset: 1,
        grade: 3,
        kind: 1,
        duration_ms: 4_200,
        study_day: 20_000,
    };
    let checkpoint = CheckpointReply {
        status: Status::Ok,
        model_id: [11, 12, 13, 14],
        adapter: Adapter::OwnerOnly,
        bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };
    vec![
        (
            "request-hello",
            7,
            Ok(Request::Hello {
                major: 1,
                minor: 0,
                release: [0, 3, 0],
            }),
        ),
        (
            "request-fold",
            7,
            Ok(Request::Fold {
                events: vec![event, ReviewEvent { seq: 10, ..event }],
            }),
        ),
        (
            "request-preview",
            7,
            Ok(Request::Preview(Preview {
                event,
                target_ppm: 900_000,
            })),
        ),
        ("request-commit", 7, Ok(Request::Commit { event })),
        (
            "request-retract",
            7,
            Ok(Request::Retract {
                t_ms: event.t_ms,
                origin: event.origin,
                seq: event.seq,
            }),
        ),
        ("request-checkpoint", 7, Ok(Request::Checkpoint)),
        (
            "request-restore",
            7,
            Ok(Request::Restore(Restore::from(checkpoint.clone()))),
        ),
        ("request-purge", 7, Ok(Request::Purge)),
        ("request-state-hash", 7, Ok(Request::StateHash)),
        (
            "reply-hello",
            7,
            Err(Reply::Hello(HelloReply {
                status: Status::Ok,
                major: 1,
                minor: 0,
                build_id: [1, 2, 3, 4],
                model_id: [5, 6, 7, 8],
                model: Model::Trained,
                adapter: Adapter::Off,
            })),
        ),
        (
            "reply-ack",
            7,
            Err(Reply::Ack {
                status: Status::Ok,
                folded: 2,
                head_t_ms: event.t_ms,
            }),
        ),
        (
            "reply-due",
            7,
            Err(Reply::Due {
                status: Status::Ok,
                card: event.card,
                staged_t_ms: event.t_ms,
                interval_days: 3,
                due_day: 20_003,
                model_id: [5, 6, 7, 8],
            }),
        ),
        ("reply-checkpoint", 7, Err(Reply::Checkpoint(checkpoint))),
        (
            "reply-hash",
            7,
            Err(Reply::Hash {
                status: Status::Ok,
                words: [21, 22, 23, 24],
            }),
        ),
        (
            "reply-refused",
            7,
            Err(Reply::Refused {
                status: Status::Absent,
            }),
        ),
    ]
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0F)]));
    }
    out
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex"))
        .collect()
}

/// The committed frames, by name.
fn committed() -> BTreeMap<String, String> {
    FRAMES
        .lines()
        .map(|line| {
            let vector: Value = serde_json::from_str(line).expect("a JSON line");
            (
                vector["name"].as_str().expect("a name").to_owned(),
                vector["hex"].as_str().expect("a hex frame").to_owned(),
            )
        })
        .collect()
}

/// An encoder of a named message: the crate's, or a planted one.
type Encoder = dyn Fn(&str, u32, &Result<Request, Reply>) -> Vec<u8>;

/// Every named message whose encoding by `encoder` is not its committed frame.
fn encode_findings(encoder: &Encoder) -> Vec<String> {
    let frames = committed();
    let mut findings = Vec::new();
    for (name, epoch, message) in named_messages() {
        let Some(expected) = frames.get(name) else {
            findings.push(format!("{name}: no committed frame"));
            continue;
        };
        let ours = hex(&encoder(name, epoch, &message));
        if ours != *expected {
            findings.push(format!("{name}: encoded {ours} != committed {expected}"));
        }
    }
    findings
}

/// Every committed frame the crate's decoder does not read back as its named message.
fn decode_findings() -> Vec<String> {
    let frames = committed();
    let mut findings = Vec::new();
    for (name, epoch, message) in named_messages() {
        let Some(expected) = frames.get(name) else {
            continue;
        };
        let bytes = unhex(expected);
        let back = match &message {
            Ok(_) => decode_request(&bytes)
                .map(|frame| (frame.epoch, Ok(frame.message)))
                .ok(),
            Err(_) => decode_reply(&bytes)
                .map(|frame| (frame.epoch, Err(frame.message)))
                .ok(),
        };
        if back != Some((epoch, message.clone())) {
            findings.push(format!("{name}: the committed frame decodes to {back:?}"));
        }
    }
    findings
}

/// The crate's own encoder.
fn crate_encoder(_name: &str, epoch: u32, message: &Result<Request, Reply>) -> Vec<u8> {
    match message {
        Ok(request) => encode_request(epoch, request).expect("encodes"),
        Err(reply) => encode_reply(epoch, reply).expect("encodes"),
    }
}

/// A planted encoder that writes every frame as committed but a `Due` reply's interval and due
/// day in swapped order. It never calls the crate's encoder.
fn swapped_encoder(name: &str, _epoch: u32, _message: &Result<Request, Reply>) -> Vec<u8> {
    let mut frame = unhex(&committed()[name]);
    if name == "reply-due" {
        // Header 13, status 1, card 8, staged 8: the interval starts at 30 and the due day at 34.
        let interval: [u8; 4] = frame[30..34].try_into().expect("four bytes");
        let due: [u8; 4] = frame[34..38].try_into().expect("four bytes");
        frame[30..34].copy_from_slice(&due);
        frame[34..38].copy_from_slice(&interval);
    }
    frame
}

#[test]
fn every_frame_is_byte_equal_to_the_committed_vectors() {
    let frames = committed();
    let names: Vec<&str> = named_messages().iter().map(|(name, _, _)| *name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(
        frames.keys().map(String::as_str).collect::<Vec<_>>(),
        sorted,
        "the committed file holds exactly the named messages"
    );
    println!("examined {} committed frame(s)", names.len());
    assert_eq!(encode_findings(&crate_encoder), Vec::<String>::new());
    assert_eq!(decode_findings(), Vec::<String>::new());
}

#[test]
fn a_field_order_swap_in_the_encoder_breaks_the_vectors() {
    let findings = encode_findings(&swapped_encoder);
    println!(
        "examined the swapped encoder: {} finding(s)",
        findings.len()
    );
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].starts_with("reply-due: encoded "),
        "{findings:?}"
    );
}

/// The five malformed frames, written by hand from the layout, each with the refusal it must
/// give.
fn malformed() -> Vec<(&'static str, Vec<u8>, DecodeError)> {
    // A `Hello` request's body: major 1, minor 0, release 0.3.0.
    let hello: Vec<u8> = [1_u16, 0, 0, 3, 0]
        .iter()
        .flat_map(|part| part.to_le_bytes())
        .collect();

    let mut trailing_body = hello.clone();
    trailing_body.push(0);
    let trailing = raw_frame(HELLO, &trailing_body);

    let unknown_kind = raw_frame(0x7F, &[]);

    let count = u32::try_from(FOLD_MAX + 1).expect("small");
    let over_count = raw_frame(FOLD, &count.to_le_bytes());

    let mut len_mismatch = raw_frame(HELLO, &hello);
    let len = u32::try_from(len_mismatch.len() + 1).expect("small");
    len_mismatch[..4].copy_from_slice(&len.to_le_bytes());

    let over_cap = raw_frame(PURGE, &vec![0; FRAME_MAX + 1 - 13]);

    vec![
        ("trailing bytes", trailing, DecodeError::TrailingBytes),
        ("an unknown kind", unknown_kind, DecodeError::UnknownKind),
        (
            "a count over its bound",
            over_count,
            DecodeError::CountOverBound,
        ),
        ("a len mismatch", len_mismatch, DecodeError::LenMismatch),
        ("a frame over 1 MiB", over_cap, DecodeError::OverCap),
    ]
}

/// Every malformed frame `decoder` does not refuse with its named refusal.
fn malformed_findings(decoder: fn(&[u8]) -> Result<Frame<Request>, DecodeError>) -> Vec<String> {
    malformed()
        .into_iter()
        .filter_map(|(name, frame, refusal)| {
            let got = decoder(&frame).map(|decoded| decoded.message);
            (got != Err(refusal)).then(|| format!("{name}: {got:?}, not {refusal:?}"))
        })
        .collect()
}

/// A planted decoder that admits every frame.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a planted decoder keeps the decoder's signature"
)]
fn accept_all(_frame: &[u8]) -> Result<Frame<Request>, DecodeError> {
    Ok(Frame {
        major: MAJOR,
        minor: MINOR,
        epoch: 0,
        message: Request::Purge,
    })
}

#[test]
fn the_decoder_refuses_the_five_malformed_frames_by_name() {
    let cases = malformed();
    let findings = malformed_findings(decode_request);
    println!("examined {} of 5 malformed frame(s)", cases.len());
    assert_eq!(cases.len(), 5);
    assert_eq!(findings, Vec::<String>::new());
    // The reply decoder refuses the same header faults.
    for (name, frame, refusal) in &cases[3..] {
        assert_eq!(
            decode_reply(frame).map(|f| f.message),
            Err(*refusal),
            "{name}"
        );
    }
}

#[test]
fn an_accept_all_decoder_passes_none_of_the_malformed_frames() {
    let findings = malformed_findings(accept_all);
    println!(
        "examined the accept-all decoder: {} finding(s)",
        findings.len()
    );
    assert_eq!(findings.len(), 5, "{findings:?}");
}

#[test]
fn an_enum_value_outside_its_type_and_another_major_are_refused() {
    // A `Refused` reply whose status byte is 7, one past the last status.
    assert_eq!(
        decode_reply(&raw_frame(REFUSED, &[7])),
        Err(DecodeError::UnknownEnum)
    );
    let mut frame = raw_frame(REFUSED, &[0]);
    frame[4..6].copy_from_slice(&2_u16.to_le_bytes());
    assert_eq!(decode_reply(&frame), Err(DecodeError::Major));
    let short = raw_frame(REFUSED, &[0]);
    assert_eq!(decode_reply(&short[..12]), Err(DecodeError::Truncated));
}
