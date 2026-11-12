//! One canonical byte encoding of an operation (SPEC-509 R1; ADR-527 D1).
//!
//! A frame is a version byte, the head (origin, sequence number, clock) and the body (the kind's
//! tag and the fields in declaration order). Integers are little-endian, text is UTF-8 behind a
//! 32-bit length, an option is a 0 or 1 byte before its value, and a list is a 16-bit count
//! before its items. One function encodes and one decodes, on the server and on both clients, so
//! two equal operations are equal bytes, and the server compares bytes to know a duplicate.
//! [`decode`] is total: any input is a frame or a named refusal, never a panic.

use std::fmt;

use crate::key::{Key, Origin};
use crate::op::{NotetypePart, Op, OpHead, OpKind};

/// The wire version this module writes and the only one it reads.
pub const VERSION: u8 = 1;

/// The bytes before the body: the version byte, the origin, the sequence number and the clock.
pub const HEAD_BYTES: usize = 1 + 16 + 8 + 8;

/// The most bytes one frame may hold: 256 KiB.
pub const MAX_OP_BYTES: usize = 256 * 1024;

/// The longest an answer may take, in milliseconds: one hour.
pub const MAX_DURATION_MS: u32 = 3_600_000;

/// Why an operation cannot be encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// The frame would hold more than [`MAX_OP_BYTES`].
    TooLarge {
        /// The frame's length.
        bytes: usize,
    },
    /// A value is outside the range its field admits.
    OutOfRange {
        /// The field.
        field: &'static str,
    },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { bytes } => {
                write!(f, "an operation of {bytes} bytes is over {MAX_OP_BYTES}")
            }
            Self::OutOfRange { field } => write!(f, "the field {field} is out of its range"),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Why bytes are not a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The input is longer than [`MAX_OP_BYTES`].
    TooLarge {
        /// The input's length.
        bytes: usize,
    },
    /// The input ends inside a field.
    Truncated,
    /// The version byte is not [`VERSION`].
    Version {
        /// The byte found.
        found: u8,
    },
    /// The body's tag names no kind.
    UnknownKind {
        /// The tag found.
        tag: u8,
    },
    /// A value is outside the range its field admits.
    OutOfRange {
        /// The field.
        field: &'static str,
    },
    /// A text field is not UTF-8.
    NotUtf8,
    /// Bytes follow the last field.
    Trailing {
        /// How many.
        extra: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { bytes } => write!(f, "{bytes} bytes is over {MAX_OP_BYTES}"),
            Self::Truncated => f.write_str("the frame ends inside a field"),
            Self::Version { found } => write!(f, "wire version {found} is not {VERSION}"),
            Self::UnknownKind { tag } => write!(f, "tag {tag} names no operation"),
            Self::OutOfRange { field } => write!(f, "the field {field} is out of its range"),
            Self::NotUtf8 => f.write_str("a text field is not UTF-8"),
            Self::Trailing { extra } => write!(f, "{extra} byte(s) follow the last field"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// The first field of `op` outside its range, if any: the rules the encoder and the decoder
/// share. Only an answer's ease, kind and duration have ranges narrower than their types.
fn out_of_range(op: &Op) -> Option<&'static str> {
    let Op::ReviewAnswered {
        ease,
        kind,
        duration_ms,
        ..
    } = op
    else {
        return None;
    };
    if !(1..=4).contains(ease) {
        Some("ease")
    } else if *kind > 3 {
        Some("kind")
    } else if *duration_ms > MAX_DURATION_MS {
        Some("duration_ms")
    } else {
        None
    }
}

/// The bytes being written.
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn put(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn flag(&mut self, on: bool) {
        self.put(&[u8::from(on)]);
    }

    fn text(&mut self, text: &str) -> Result<(), EncodeError> {
        let bytes = text.as_bytes();
        let len =
            u32::try_from(bytes.len()).map_err(|_| EncodeError::TooLarge { bytes: bytes.len() })?;
        self.put(&len.to_le_bytes());
        self.put(bytes);
        Ok(())
    }

    fn key(&mut self, key: &Key) {
        self.put(&key.t.to_le_bytes());
        self.put(&[key.rank]);
        self.put(&key.origin);
        self.put(&key.seq.to_le_bytes());
    }

    fn option<T>(&mut self, value: Option<&T>, put: impl FnOnce(&mut Self, &T)) {
        self.flag(value.is_some());
        if let Some(value) = value {
            put(self, value);
        }
    }

    fn ids(&mut self, ids: &[i64]) {
        for id in ids {
            self.put(&id.to_le_bytes());
        }
    }

    fn texts(&mut self, texts: &[String]) -> Result<(), EncodeError> {
        let count =
            u16::try_from(texts.len()).map_err(|_| EncodeError::OutOfRange { field: "fields" })?;
        self.put(&count.to_le_bytes());
        texts.iter().try_for_each(|text| self.text(text))
    }

    fn part(&mut self, part: NotetypePart) {
        match part {
            NotetypePart::Template(index) => {
                self.put(&[0]);
                self.put(&index.to_le_bytes());
            }
            NotetypePart::Css => self.put(&[1]),
            NotetypePart::FieldName(index) => {
                self.put(&[2]);
                self.put(&index.to_le_bytes());
            }
        }
    }

    /// The fields of `op`, in declaration order.
    fn fields(&mut self, op: &Op) -> Result<(), EncodeError> {
        match op {
            Op::ReviewAnswered {
                card,
                note,
                home_deck,
                preset,
                ease,
                kind,
                duration_ms,
                study_day,
                basis,
                due_hint,
                model_version,
            } => {
                self.ids(&[*card, *note, *home_deck, *preset]);
                self.put(&[*ease, *kind]);
                self.put(&duration_ms.to_le_bytes());
                self.put(&study_day.to_le_bytes());
                self.option(basis.as_ref(), Self::key);
                self.option(due_hint.as_ref(), |w, day| w.put(&day.to_le_bytes()));
                self.option(model_version.as_ref(), |w, version| w.put(version));
            }
            Op::ReviewUndone { target } => self.key(target),
            Op::NoteCreated {
                note,
                notetype,
                deck,
                guid,
                fields,
            } => {
                self.ids(&[*note, *notetype, *deck]);
                self.put(guid);
                self.texts(fields)?;
            }
            Op::NoteFieldSet {
                note,
                field,
                value,
                base_rev,
            } => {
                self.ids(&[*note]);
                self.put(&field.to_le_bytes());
                self.text(value)?;
                self.option(base_rev.as_ref(), Self::key);
            }
            Op::NoteDeleted { note } => self.ids(&[*note]),
            Op::CardMoved {
                card,
                deck,
                base_rev,
            } => {
                self.ids(&[*card, *deck]);
                self.option(base_rev.as_ref(), Self::key);
            }
            Op::CardFlag { card, flag } => {
                self.ids(&[*card]);
                self.put(&[*flag]);
            }
            Op::CardBury { card, day } => {
                self.ids(&[*card]);
                self.put(&day.to_le_bytes());
            }
            Op::CardSuspend { card, on } => {
                self.ids(&[*card]);
                self.flag(*on);
            }
            Op::DeckCreated { deck, parent, name } => {
                self.ids(&[*deck]);
                self.option(parent.as_ref(), |w, parent| w.ids(&[*parent]));
                self.text(name)?;
            }
            Op::DeckRenamed {
                deck,
                name,
                base_rev,
            } => {
                self.ids(&[*deck]);
                self.text(name)?;
                self.option(base_rev.as_ref(), Self::key);
            }
            Op::NotetypeEdited {
                notetype,
                part,
                value,
                base_rev,
            } => {
                self.ids(&[*notetype]);
                self.part(*part);
                self.text(value)?;
                self.option(base_rev.as_ref(), Self::key);
            }
            Op::MediaBound { name, sha256, size } => {
                self.text(name)?;
                self.put(sha256);
                self.put(&size.to_le_bytes());
            }
            Op::MediaUnbound { name } => self.text(name)?,
        }
        Ok(())
    }
}

/// The body of `op`: its kind's tag and its fields, in declaration order.
///
/// # Errors
///
/// [`EncodeError::OutOfRange`] for a value its field does not admit, and
/// [`EncodeError::TooLarge`] for a body over [`MAX_OP_BYTES`].
pub fn encode_body(op: &Op) -> Result<Vec<u8>, EncodeError> {
    if let Some(field) = out_of_range(op) {
        return Err(EncodeError::OutOfRange { field });
    }
    let mut w = Writer { bytes: Vec::new() };
    w.put(&[op.kind().tag()]);
    w.fields(op)?;
    let bytes = HEAD_BYTES + w.bytes.len();
    if bytes > MAX_OP_BYTES {
        return Err(EncodeError::TooLarge { bytes });
    }
    Ok(w.bytes)
}

/// The frame of `op` made under `head`: the version byte, the head and the body.
///
/// # Errors
///
/// As [`encode_body`].
pub fn encode(head: &OpHead, op: &Op) -> Result<Vec<u8>, EncodeError> {
    let body = encode_body(op)?;
    let mut frame = Vec::with_capacity(HEAD_BYTES + body.len());
    frame.push(VERSION);
    frame.extend_from_slice(&head.origin);
    frame.extend_from_slice(&head.seq.to_le_bytes());
    frame.extend_from_slice(&head.hlc.to_le_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

/// The bytes still to read.
struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if n > self.rest.len() {
            return Err(DecodeError::Truncated);
        }
        let (taken, rest) = self.rest.split_at(n);
        self.rest = rest;
        Ok(taken)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, DecodeError> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn flag(&mut self, field: &'static str) -> Result<bool, DecodeError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::OutOfRange { field }),
        }
    }

    fn text(&mut self) -> Result<String, DecodeError> {
        let len =
            usize::try_from(self.u32()?).map_err(|_| DecodeError::OutOfRange { field: "text" })?;
        // The Kani witness (ADR-527 D7): with its feature on, the length prefix is read without
        // the remaining-bytes check, so a long prefix panics and `kani_proofs::sync_codec` fails.
        #[expect(
            clippy::disallowed_macros,
            reason = "a Kani witness feature, off in every build but the formal check's witness run (ADR-527 D7)"
        )]
        let bytes = if cfg!(feature = "kani-witness-sync-codec-len") {
            let (taken, rest) = self.rest.split_at(len);
            self.rest = rest;
            taken
        } else {
            self.take(len)?
        };
        String::from_utf8(bytes.to_vec()).map_err(|_| DecodeError::NotUtf8)
    }

    fn key(&mut self) -> Result<Key, DecodeError> {
        Ok(Key {
            t: self.i64()?,
            rank: self.byte()?,
            origin: self.array()?,
            seq: self.u64()?,
        })
    }

    fn option<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Option<T>, DecodeError> {
        if self.flag("option")? {
            read(self).map(Some)
        } else {
            Ok(None)
        }
    }

    fn body(&mut self) -> Result<Op, DecodeError> {
        let tag = self.byte()?;
        let kind = OpKind::from_tag(tag).ok_or(DecodeError::UnknownKind { tag })?;
        Ok(match kind {
            OpKind::ReviewAnswered => Op::ReviewAnswered {
                card: self.i64()?,
                note: self.i64()?,
                home_deck: self.i64()?,
                preset: self.i64()?,
                ease: self.byte()?,
                kind: self.byte()?,
                duration_ms: self.u32()?,
                study_day: self.i32()?,
                basis: self.option(Self::key)?,
                due_hint: self.option(Self::i32)?,
                model_version: self.option(Self::array)?,
            },
            OpKind::ReviewUndone => Op::ReviewUndone {
                target: self.key()?,
            },
            OpKind::NoteCreated => {
                let note = self.i64()?;
                let notetype = self.i64()?;
                let deck = self.i64()?;
                let guid = self.array()?;
                let count = self.u16()?;
                let mut fields = Vec::new();
                for _ in 0..count {
                    fields.push(self.text()?);
                }
                Op::NoteCreated {
                    note,
                    notetype,
                    deck,
                    guid,
                    fields,
                }
            }
            OpKind::NoteFieldSet => Op::NoteFieldSet {
                note: self.i64()?,
                field: self.u16()?,
                value: self.text()?,
                base_rev: self.option(Self::key)?,
            },
            OpKind::NoteDeleted => Op::NoteDeleted { note: self.i64()? },
            OpKind::CardMoved => Op::CardMoved {
                card: self.i64()?,
                deck: self.i64()?,
                base_rev: self.option(Self::key)?,
            },
            OpKind::CardFlag => Op::CardFlag {
                card: self.i64()?,
                flag: self.byte()?,
            },
            OpKind::CardBury => Op::CardBury {
                card: self.i64()?,
                day: self.i32()?,
            },
            OpKind::CardSuspend => Op::CardSuspend {
                card: self.i64()?,
                on: self.flag("on")?,
            },
            OpKind::DeckCreated => Op::DeckCreated {
                deck: self.i64()?,
                parent: self.option(Self::i64)?,
                name: self.text()?,
            },
            OpKind::DeckRenamed => Op::DeckRenamed {
                deck: self.i64()?,
                name: self.text()?,
                base_rev: self.option(Self::key)?,
            },
            OpKind::NotetypeEdited => Op::NotetypeEdited {
                notetype: self.i64()?,
                part: match self.byte()? {
                    0 => NotetypePart::Template(self.u16()?),
                    1 => NotetypePart::Css,
                    2 => NotetypePart::FieldName(self.u16()?),
                    _ => return Err(DecodeError::OutOfRange { field: "part" }),
                },
                value: self.text()?,
                base_rev: self.option(Self::key)?,
            },
            OpKind::MediaBound => Op::MediaBound {
                name: self.text()?,
                sha256: self.array()?,
                size: self.u32()?,
            },
            OpKind::MediaUnbound => Op::MediaUnbound { name: self.text()? },
        })
    }
}

/// The head and the operation `frame` holds.
///
/// # Errors
///
/// A [`DecodeError`] naming the first thing that is not a frame: an input over
/// [`MAX_OP_BYTES`], another version, a truncation, an unknown kind, a value out of its range,
/// text that is not UTF-8, or bytes after the last field.
pub fn decode(frame: &[u8]) -> Result<(OpHead, Op), DecodeError> {
    if frame.len() > MAX_OP_BYTES {
        return Err(DecodeError::TooLarge { bytes: frame.len() });
    }
    let mut r = Reader { rest: frame };
    let found = r.byte()?;
    if found != VERSION {
        return Err(DecodeError::Version { found });
    }
    let origin: Origin = r.array()?;
    let head = OpHead {
        origin,
        seq: r.u64()?,
        hlc: r.i64()?,
    };
    let op = r.body()?;
    if !r.rest.is_empty() {
        return Err(DecodeError::Trailing {
            extra: r.rest.len(),
        });
    }
    if let Some(field) = out_of_range(&op) {
        return Err(DecodeError::OutOfRange { field });
    }
    Ok((head, op))
}
