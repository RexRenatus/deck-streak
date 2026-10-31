//! # deck-streak-companion
//!
//! What this crate owns: the scheduling companion's documented interface (SPEC-501, ADR-519).
//! A scheduling companion is a scheduler that lives beside the app, outside its public code,
//! and answers due dates. This crate is everything the two sides share:
//!
//! - [`wire`]: schema v1, every type that crosses, made of integers and closed enums only, with
//!   one byte field, the companion's own checkpoint, which the app hands back verbatim;
//! - [`codec`]: the frames those types travel in, little-endian with a fixed field order, and
//!   the decoder that refuses every malformed frame;
//! - [`port`]: the [`port::Companion`] trait a body implements, and its default,
//!   [`port::Absent`], which refuses every request;
//! - [`decide`]: the handshake ([`decide::negotiate`]) and the one decision whose `Ok` lets a
//!   due date be written ([`decide::decide`]); everything else is a named
//!   [`decide::Refusal`], and a refusal means nothing is written and no other scheduler is asked.
//!
//! What it does not own: any body. The public slot, `deck-streak-companion-slot`, opens
//! [`port::Absent`]. No source here reads a clock, a file, the environment, the network, a
//! process or a thread, and the crate depends on nothing, so the server and both clients link one
//! interface.

#![no_std]

extern crate alloc;

pub mod codec;
pub mod decide;
pub mod port;
pub mod wire;
