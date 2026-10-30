//! The pending queue: a device's own operations the server has not judged yet (SPEC-509 R6;
//! ADR-527 D6).
//!
//! An operation enters the queue when the device makes it and leaves it only on its judgement:
//! the server's verdict on it, or its appearance in a pull. Sending it changes its stage and
//! nothing else, so an answer lost on the way, a crash or a refusal leaves it pending, and it is
//! sent again. A terminal rejection moves it, bytes and reason, to the rejections the owner sees.

use std::collections::VecDeque;

use crate::key::Origin;

/// Why the server refused an operation for good: it leaves the queue into the rejections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// The bytes do not decode, or a value is out of its range.
    Malformed,
    /// An id the operation creates already exists.
    IdTaken,
    /// The edit names a note that was deleted; its value is kept as a conflict.
    Gone,
    /// A delete of a note that has a review, which the never-list forbids.
    ReviewedNoteDelete,
    /// An undo of a review that is no longer the card's last.
    UndoStale,
    /// The device's sequence number was already judged with other bytes.
    SeqReuse,
}

/// Why the server could not judge an operation now: it stays pending and is sent again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// An earlier sequence number of the device has not been judged.
    Gap,
    /// The operation's clock is too far past the server's.
    ClockAhead,
    /// A media binding names a blob the server does not hold yet.
    BlobMissing,
    /// The client is below the minimum the server admits.
    BelowMinimumClient,
    /// The server's writer is busy.
    Busy,
}

/// The server's verdict on one pushed operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Judged and written at `server_seq`.
    Accepted {
        /// The operation's place in the server's log.
        server_seq: i64,
    },
    /// Already judged with the same bytes, at `server_seq`.
    Duplicate {
        /// The operation's place in the server's log.
        server_seq: i64,
    },
    /// Refused for good.
    Rejected(Rejection),
    /// Not judged now; send it again.
    Refused(Refusal),
}

/// Where a pending operation is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Made, and waiting to be sent.
    Queued,
    /// Sent, and waiting for its judgement.
    Sent,
}

/// One pending operation: its sequence number, its frame and its stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingOp {
    seq: u64,
    frame: Vec<u8>,
    stage: Stage,
}

impl PendingOp {
    /// The device's sequence number for the operation.
    #[must_use]
    pub const fn seq(&self) -> u64 {
        self.seq
    }

    /// The operation's encoded frame.
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.frame
    }

    /// Where the operation is in its life.
    #[must_use]
    pub const fn stage(&self) -> Stage {
        self.stage
    }
}

/// An operation the server refused for good, kept with its bytes for the owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejected {
    /// The device's sequence number for the operation.
    pub seq: u64,
    /// Why it was refused.
    pub reason: Rejection,
    /// Its frame, byte for byte.
    pub frame: Vec<u8>,
}

/// How an operation left the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Settled {
    /// The server wrote it at `server_seq`: it is part of the base now.
    Acknowledged {
        /// The device's sequence number for the operation.
        seq: u64,
        /// The operation's place in the server's log.
        server_seq: i64,
    },
    /// The server refused it for good; it is among the rejections.
    Rejected(Rejected),
}

/// A device's pending queue, in its own sequence order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingQueue {
    origin: Origin,
    ops: VecDeque<PendingOp>,
    rejected: Vec<Rejected>,
}

impl PendingQueue {
    /// An empty queue for the device `origin`.
    #[must_use]
    pub const fn new(origin: Origin) -> Self {
        Self {
            origin,
            ops: VecDeque::new(),
            rejected: Vec::new(),
        }
    }

    /// The pending operations, oldest first.
    pub fn pending(&self) -> impl Iterator<Item = &PendingOp> {
        self.ops.iter()
    }

    /// The operations the server refused for good, oldest first.
    #[must_use]
    pub fn rejected(&self) -> &[Rejected] {
        &self.rejected
    }

    /// Adds an operation the device just made, with its sequence number and frame.
    pub fn enqueue(&mut self, seq: u64, frame: Vec<u8>) {
        self.ops.push_back(PendingOp {
            seq,
            frame,
            stage: Stage::Queued,
        });
    }

    /// The operations to push now, oldest first and at most `limit`, as sequence numbers and
    /// frames. Each is marked sent and stays in the queue until it is judged.
    pub fn send(&mut self, limit: usize) -> Vec<(u64, Vec<u8>)> {
        let count = limit.min(self.ops.len());
        self.ops
            .drain(..count)
            .map(|op| (op.seq, op.frame))
            .collect()
    }

    /// Applies the server's verdict on the operation `seq`. An accepted, duplicate or rejected
    /// operation leaves the queue; a refused one goes back to queued. `None` when no pending
    /// operation has that number, or when it stays.
    pub fn on_verdict(&mut self, seq: u64, verdict: Verdict) -> Option<Settled> {
        let at = self.ops.iter().position(|op| op.seq == seq)?;
        match verdict {
            Verdict::Accepted { server_seq } | Verdict::Duplicate { server_seq } => {
                self.ops.remove(at);
                Some(Settled::Acknowledged { seq, server_seq })
            }
            Verdict::Rejected(reason) => {
                let op = self.ops.remove(at)?;
                let rejected = Rejected {
                    seq,
                    reason,
                    frame: op.frame,
                };
                self.rejected.push(rejected.clone());
                Some(Settled::Rejected(rejected))
            }
            Verdict::Refused(_) => {
                if let Some(op) = self.ops.get_mut(at) {
                    op.stage = Stage::Queued;
                }
                None
            }
        }
    }

    /// Takes an operation the device pulled from the server's log at `server_seq`: one of the
    /// device's own pending operations leaves the queue, acknowledged. `None` for another
    /// device's operation or one no longer pending.
    pub fn on_pulled(&mut self, origin: &Origin, seq: u64, server_seq: i64) -> Option<Settled> {
        if *origin != self.origin {
            return None;
        }
        let at = self.ops.iter().position(|op| op.seq == seq)?;
        self.ops.remove(at);
        Some(Settled::Acknowledged { seq, server_seq })
    }
}
