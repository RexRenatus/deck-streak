//! The port a scheduling companion's body implements, and its default.

use crate::wire::{Reply, Request, Status};

/// A scheduling companion: one call per request, one reply per call, no I/O of its own.
///
/// `epoch` is the session the request belongs to; a body echoes it in its reply's frame, so a
/// reply from an earlier session reads as stale ([`crate::decide::decide`]).
pub trait Companion {
    /// Answers one request.
    fn call(&mut self, epoch: u32, request: &Request) -> Reply;
}

/// No companion: every request is refused with [`Status::Absent`]. It is the public slot's body,
/// so a build that carries no companion writes no due date from one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Absent;

impl Companion for Absent {
    fn call(&mut self, _epoch: u32, _request: &Request) -> Reply {
        Reply::Refused {
            status: Status::Absent,
        }
    }
}
