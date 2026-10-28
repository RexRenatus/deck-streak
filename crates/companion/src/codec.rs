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

/// Encodes `request` as one frame of session `epoch`: this stub writes the header alone.
///
/// # Errors
///
/// None in this stub.
pub fn encode_request(epoch: u32, request: &Request) -> Result<Vec<u8>, EncodeError> {
    Ok(header(request_kind_of(request), epoch))
}

/// Encodes `reply` as one frame of session `epoch`: this stub writes the header alone.
///
/// # Errors
///
/// None in this stub.
pub fn encode_reply(epoch: u32, reply: &Reply) -> Result<Vec<u8>, EncodeError> {
    Ok(header(reply_kind_of(reply), epoch))
}

/// Decodes one request frame: this stub admits every frame as `Purge`.
///
/// # Errors
///
/// None in this stub.
pub fn decode_request(_bytes: &[u8]) -> Result<Frame<Request>, DecodeError> {
    Ok(Frame {
        major: MAJOR,
        minor: MINOR,
        epoch: 0,
        message: Request::Purge,
    })
}

/// Decodes one reply frame: this stub admits every frame as an absent refusal.
///
/// # Errors
///
/// None in this stub.
pub fn decode_reply(_bytes: &[u8]) -> Result<Frame<Reply>, DecodeError> {
    Ok(Frame {
        major: MAJOR,
        minor: MINOR,
        epoch: 0,
        message: Reply::Refused {
            status: Status::Absent,
        },
    })
}

/// A frame's header, with its `len`.
fn header(kind: u8, epoch: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN);
    out.extend_from_slice(&13_u32.to_le_bytes());
    out.extend_from_slice(&MAJOR.to_le_bytes());
    out.extend_from_slice(&MINOR.to_le_bytes());
    out.push(kind);
    out.extend_from_slice(&epoch.to_le_bytes());
    out
}
