//! The frames schema v1 travels in, and their codec.
//!
//! A frame is `len u32 | major u16 | minor u16 | kind u8 | epoch u32 | body`, little-endian, in
//! a fixed field order with no padding. `len` counts the whole frame, header included, and no
//! frame is longer than [`FRAME_MAX`]. A `Vec` is its count, `u32`, then its items. A body's
//! fields are its type's fields in their declared order ([`crate::wire`]); an enum is its `u8`
//! value. `epoch` is the app's counter for the current companion session: it rises at every
//! `Hello`, so a reply from an earlier session is stale.
//!
//! The decoder refuses, by name, a frame over the cap, a frame shorter than its header or its
//! body, a `len` other than the frame's length, another major, an unknown kind, a count over its
//! bound, an enum value outside its type, and bytes after the body. It never panics: every read
//! is bounds-checked.

use alloc::vec::Vec;

use crate::wire::{
    Adapter, CheckpointReply, FOLD_MAX, FRAME_MAX, HEADER_LEN, HelloReply, MAJOR, MINOR, Model,
    Preview, Reply, Request, Restore, ReviewEvent, Status,
};

/// The kind byte of each request.
pub mod request_kind {
    /// `Hello`.
    pub const HELLO: u8 = 1;
    /// `Fold`.
    pub const FOLD: u8 = 2;
    /// `Preview`.
    pub const PREVIEW: u8 = 3;
    /// `Commit`.
    pub const COMMIT: u8 = 4;
    /// `Retract`.
    pub const RETRACT: u8 = 5;
    /// `Checkpoint`.
    pub const CHECKPOINT: u8 = 6;
    /// `Restore`.
    pub const RESTORE: u8 = 7;
    /// `Purge`.
    pub const PURGE: u8 = 8;
    /// `StateHash`.
    pub const STATE_HASH: u8 = 9;
}

/// The kind byte of each reply.
pub mod reply_kind {
    /// `Hello`.
    pub const HELLO: u8 = 0x81;
    /// `Ack`.
    pub const ACK: u8 = 0x82;
    /// `Due`.
    pub const DUE: u8 = 0x83;
    /// `Checkpoint`.
    pub const CHECKPOINT: u8 = 0x84;
    /// `Hash`.
    pub const HASH: u8 = 0x85;
    /// `Refused`.
    pub const REFUSED: u8 = 0x86;
}

/// A decoded frame: its header's version and epoch, and its message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame<M> {
    /// The schema major the frame was written in; always [`MAJOR`] once decoded.
    pub major: u16,
    /// The schema minor the frame was written in.
    pub minor: u16,
    /// The session the frame belongs to.
    pub epoch: u32,
    /// The request or the reply.
    pub message: M,
}

/// Why a frame was not decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodeError {
    /// The frame is longer than [`FRAME_MAX`].
    OverCap,
    /// The frame ends before its header or its body does.
    Truncated,
    /// The header's `len` is not the frame's length.
    LenMismatch,
    /// The header names another major.
    Major,
    /// The kind byte names no request or reply.
    UnknownKind,
    /// A count is over its bound.
    CountOverBound,
    /// An enum field holds a value outside its type.
    UnknownEnum,
    /// Bytes follow the body.
    TrailingBytes,
}

/// Why a message was not encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncodeError {
    /// A `Fold` with more than [`FOLD_MAX`] events.
    CountOverBound,
    /// The frame would be longer than [`FRAME_MAX`].
    OverCap,
}

/// The kind byte a request travels under.
#[must_use]
pub fn request_kind_of(request: &Request) -> u8 {
    match request {
        Request::Hello { .. } => request_kind::HELLO,
        Request::Fold { .. } => request_kind::FOLD,
        Request::Preview(_) => request_kind::PREVIEW,
        Request::Commit { .. } => request_kind::COMMIT,
        Request::Retract { .. } => request_kind::RETRACT,
        Request::Checkpoint => request_kind::CHECKPOINT,
        Request::Restore(_) => request_kind::RESTORE,
        Request::Purge => request_kind::PURGE,
        Request::StateHash => request_kind::STATE_HASH,
    }
}

/// The kind byte a reply travels under.
#[must_use]
pub fn reply_kind_of(reply: &Reply) -> u8 {
    match reply {
        Reply::Hello(_) => reply_kind::HELLO,
        Reply::Ack { .. } => reply_kind::ACK,
        Reply::Due { .. } => reply_kind::DUE,
        Reply::Checkpoint(_) => reply_kind::CHECKPOINT,
        Reply::Hash { .. } => reply_kind::HASH,
        Reply::Refused { .. } => reply_kind::REFUSED,
    }
}

/// Encodes `request` as one frame of session `epoch`, at [`MAJOR`] and [`MINOR`].
///
/// # Errors
///
/// [`EncodeError::CountOverBound`] for a `Fold` of more than [`FOLD_MAX`] events, and
/// [`EncodeError::OverCap`] for a frame longer than [`FRAME_MAX`].
pub fn encode_request(epoch: u32, request: &Request) -> Result<Vec<u8>, EncodeError> {
    let mut out = header(request_kind_of(request), epoch);
    match request {
        Request::Hello {
            major,
            minor,
            release,
        } => {
            put_u16(&mut out, *major);
            put_u16(&mut out, *minor);
            for part in release {
                put_u16(&mut out, *part);
            }
        }
        Request::Fold { events } => {
            if events.len() > FOLD_MAX {
                return Err(EncodeError::CountOverBound);
            }
            put_count(&mut out, events.len())?;
            for event in events {
                put_event(&mut out, event);
            }
        }
        Request::Preview(preview) => {
            put_event(&mut out, &preview.event);
            put_u32(&mut out, preview.target_ppm);
        }
        Request::Commit { event } => put_event(&mut out, event),
        Request::Retract { t_ms, origin, seq } => {
            put_i64(&mut out, *t_ms);
            put_u64s(&mut out, origin);
            put_u64(&mut out, *seq);
        }
        Request::Restore(restore) => put_bytes(&mut out, restore.bytes())?,
        Request::Checkpoint | Request::Purge | Request::StateHash => {}
    }
    seal(out)
}

/// Encodes `reply` as one frame of session `epoch`, at [`MAJOR`] and [`MINOR`].
///
/// # Errors
///
/// [`EncodeError::OverCap`] for a frame longer than [`FRAME_MAX`].
pub fn encode_reply(epoch: u32, reply: &Reply) -> Result<Vec<u8>, EncodeError> {
    let mut out = header(reply_kind_of(reply), epoch);
    match reply {
        Reply::Hello(hello) => {
            out.push(hello.status as u8);
            put_u16(&mut out, hello.major);
            put_u16(&mut out, hello.minor);
            put_u64s(&mut out, &hello.build_id);
            put_u64s(&mut out, &hello.model_id);
            out.push(hello.model as u8);
            out.push(hello.adapter as u8);
        }
        Reply::Ack {
            status,
            folded,
            head_t_ms,
        } => {
            out.push(*status as u8);
            put_u64(&mut out, *folded);
            put_i64(&mut out, *head_t_ms);
        }
        Reply::Due {
            status,
            card,
            staged_t_ms,
            interval_days,
            due_day,
            model_id,
        } => {
            out.push(*status as u8);
            put_i64(&mut out, *card);
            put_i64(&mut out, *staged_t_ms);
            put_u32(&mut out, *interval_days);
            put_i32(&mut out, *due_day);
            put_u64s(&mut out, model_id);
        }
        Reply::Checkpoint(checkpoint) => {
            out.push(checkpoint.status as u8);
            put_u64s(&mut out, &checkpoint.model_id);
            out.push(checkpoint.adapter as u8);
            put_bytes(&mut out, &checkpoint.bytes)?;
        }
        Reply::Hash { status, words } => {
            out.push(*status as u8);
            put_u64s(&mut out, words);
        }
        Reply::Refused { status } => out.push(*status as u8),
    }
    seal(out)
}

/// Decodes one request frame.
///
/// # Errors
///
/// The first [`DecodeError`] the frame shows, in the order the module names them.
pub fn decode_request(bytes: &[u8]) -> Result<Frame<Request>, DecodeError> {
    let (head, mut body) = open(bytes)?;
    let message = match head.kind {
        request_kind::HELLO => Request::Hello {
            major: body.u16()?,
            minor: body.u16()?,
            release: [body.u16()?, body.u16()?, body.u16()?],
        },
        request_kind::FOLD => {
            let count = body.count()?;
            if count > FOLD_MAX {
                return Err(DecodeError::CountOverBound);
            }
            let mut events = Vec::new();
            for _ in 0..count {
                events.push(body.event()?);
            }
            Request::Fold { events }
        }
        request_kind::PREVIEW => Request::Preview(Preview {
            event: body.event()?,
            target_ppm: body.u32()?,
        }),
        request_kind::COMMIT => Request::Commit {
            event: body.event()?,
        },
        request_kind::RETRACT => Request::Retract {
            t_ms: body.i64()?,
            origin: [body.u64()?, body.u64()?],
            seq: body.u64()?,
        },
        request_kind::CHECKPOINT => Request::Checkpoint,
        request_kind::RESTORE => Request::Restore(Restore::decoded(body.bytes()?)),
        request_kind::PURGE => Request::Purge,
        request_kind::STATE_HASH => Request::StateHash,
        _ => return Err(DecodeError::UnknownKind),
    };
    body.end()?;
    Ok(head.with(message))
}

/// Decodes one reply frame.
///
/// # Errors
///
/// The first [`DecodeError`] the frame shows, in the order the module names them.
pub fn decode_reply(bytes: &[u8]) -> Result<Frame<Reply>, DecodeError> {
    let (head, mut body) = open(bytes)?;
    let message = match head.kind {
        reply_kind::HELLO => Reply::Hello(HelloReply {
            status: body.status()?,
            major: body.u16()?,
            minor: body.u16()?,
            build_id: body.u64x4()?,
            model_id: body.u64x4()?,
            model: body.model()?,
            adapter: body.adapter()?,
        }),
        reply_kind::ACK => Reply::Ack {
            status: body.status()?,
            folded: body.u64()?,
            head_t_ms: body.i64()?,
        },
        reply_kind::DUE => Reply::Due {
            status: body.status()?,
            card: body.i64()?,
            staged_t_ms: body.i64()?,
            interval_days: body.u32()?,
            due_day: body.i32()?,
            model_id: body.u64x4()?,
        },
        reply_kind::CHECKPOINT => Reply::Checkpoint(CheckpointReply {
            status: body.status()?,
            model_id: body.u64x4()?,
            adapter: body.adapter()?,
            bytes: body.bytes()?,
        }),
        reply_kind::HASH => Reply::Hash {
            status: body.status()?,
            words: body.u64x4()?,
        },
        reply_kind::REFUSED => Reply::Refused {
            status: body.status()?,
        },
        _ => return Err(DecodeError::UnknownKind),
    };
    body.end()?;
    Ok(head.with(message))
}

/// A frame's header, read.
struct Head {
    minor: u16,
    kind: u8,
    epoch: u32,
}

impl Head {
    fn with<M>(self, message: M) -> Frame<M> {
        Frame {
            major: MAJOR,
            minor: self.minor,
            epoch: self.epoch,
            message,
        }
    }
}

/// Reads and checks a frame's header, and gives its body's reader.
fn open(bytes: &[u8]) -> Result<(Head, Reader<'_>), DecodeError> {
    if bytes.len() > FRAME_MAX {
        return Err(DecodeError::OverCap);
    }
    let mut reader = Reader { bytes, at: 0 };
    let len = reader.u32()?;
    let major = reader.u16()?;
    let minor = reader.u16()?;
    let kind = reader.u8()?;
    let epoch = reader.u32()?;
    if usize::try_from(len).ok() != Some(bytes.len()) {
        return Err(DecodeError::LenMismatch);
    }
    if major != MAJOR {
        return Err(DecodeError::Major);
    }
    Ok((Head { minor, kind, epoch }, reader))
}

/// A bounds-checked little-endian reader over one frame.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let end = self.at.checked_add(N).ok_or(DecodeError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(DecodeError::Truncated)?;
        let mut taken = [0_u8; N];
        taken.copy_from_slice(slice);
        self.at = end;
        Ok(taken)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        self.take().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        self.take().map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Result<u64, DecodeError> {
        self.take().map(u64::from_le_bytes)
    }

    fn i32(&mut self) -> Result<i32, DecodeError> {
        self.take().map(i32::from_le_bytes)
    }

    fn i64(&mut self) -> Result<i64, DecodeError> {
        self.take().map(i64::from_le_bytes)
    }

    fn u64x4(&mut self) -> Result<[u64; 4], DecodeError> {
        Ok([self.u64()?, self.u64()?, self.u64()?, self.u64()?])
    }

    fn count(&mut self) -> Result<usize, DecodeError> {
        usize::try_from(self.u32()?).map_err(|_| DecodeError::CountOverBound)
    }

    fn bytes(&mut self) -> Result<Vec<u8>, DecodeError> {
        let count = self.count()?;
        let end = self.at.checked_add(count).ok_or(DecodeError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(DecodeError::Truncated)?;
        self.at = end;
        Ok(slice.to_vec())
    }

    fn status(&mut self) -> Result<Status, DecodeError> {
        match self.u8()? {
            0 => Ok(Status::Ok),
            1 => Ok(Status::Absent),
            2 => Ok(Status::FixtureRefuses),
            3 => Ok(Status::Busy),
            4 => Ok(Status::Error),
            5 => Ok(Status::Conflict),
            6 => Ok(Status::Unsupported),
            _ => Err(DecodeError::UnknownEnum),
        }
    }

    fn model(&mut self) -> Result<Model, DecodeError> {
        match self.u8()? {
            0 => Ok(Model::Trained),
            1 => Ok(Model::Fixture),
            _ => Err(DecodeError::UnknownEnum),
        }
    }

    fn adapter(&mut self) -> Result<Adapter, DecodeError> {
        match self.u8()? {
            0 => Ok(Adapter::Off),
            1 => Ok(Adapter::OwnerOnly),
            _ => Err(DecodeError::UnknownEnum),
        }
    }

    fn event(&mut self) -> Result<ReviewEvent, DecodeError> {
        Ok(ReviewEvent {
            t_ms: self.i64()?,
            origin: [self.u64()?, self.u64()?],
            seq: self.u64()?,
            card: self.i64()?,
            note: self.i64()?,
            home_deck: self.i64()?,
            preset: self.i64()?,
            grade: self.u8()?,
            kind: self.u8()?,
            duration_ms: self.u32()?,
            study_day: self.i32()?,
        })
    }

    fn end(&self) -> Result<(), DecodeError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes)
        }
    }
}

/// A frame's header with its `len` still zero; [`seal`] writes it.
fn header(kind: u8, epoch: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN);
    put_u32(&mut out, 0);
    put_u16(&mut out, MAJOR);
    put_u16(&mut out, MINOR);
    out.push(kind);
    put_u32(&mut out, epoch);
    out
}

/// Writes the frame's length into its header, refusing a frame over the cap.
fn seal(mut out: Vec<u8>) -> Result<Vec<u8>, EncodeError> {
    if out.len() > FRAME_MAX {
        return Err(EncodeError::OverCap);
    }
    let len = u32::try_from(out.len()).map_err(|_| EncodeError::OverCap)?;
    out[..4].copy_from_slice(&len.to_le_bytes());
    Ok(out)
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64s(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        put_u64(out, *value);
    }
}

fn put_count(out: &mut Vec<u8>, count: usize) -> Result<(), EncodeError> {
    let count = u32::try_from(count).map_err(|_| EncodeError::OverCap)?;
    put_u32(out, count);
    Ok(())
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), EncodeError> {
    if bytes.len() > FRAME_MAX {
        return Err(EncodeError::OverCap);
    }
    put_count(out, bytes.len())?;
    out.extend_from_slice(bytes);
    Ok(())
}

fn put_event(out: &mut Vec<u8>, event: &ReviewEvent) {
    put_i64(out, event.t_ms);
    put_u64s(out, &event.origin);
    put_u64(out, event.seq);
    put_i64(out, event.card);
    put_i64(out, event.note);
    put_i64(out, event.home_deck);
    put_i64(out, event.preset);
    out.push(event.grade);
    out.push(event.kind);
    put_u32(out, event.duration_ms);
    put_i32(out, event.study_day);
}
