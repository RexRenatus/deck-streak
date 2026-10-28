//! # deck-streak-companion-slot
//!
//! What this crate owns: the one substitutable body behind the scheduling companion's interface
//! (SPEC-501, ADR-519). [`open`] is the only door: it takes the bytes of a model and gives the
//! companion every client composes. This public body opens [`Absent`], which refuses every
//! request, so a build that carries this body writes no due date from a companion.
//!
//! A build that carries another body replaces this crate whole, with the same name and the same
//! signature; nothing else in the workspace changes. The crate depends on the interface crate
//! alone, reads nothing and writes nothing.

#![no_std]

extern crate alloc;

use alloc::boxed::Box;

use deck_streak_companion::port::{Absent, Companion};

/// Opens the companion this build carries, over a model's bytes. The public body ignores them
/// and opens [`Absent`].
#[must_use]
pub fn open(_weights: &[u8]) -> Box<dyn Companion + Send> {
    Box::new(Absent)
}
