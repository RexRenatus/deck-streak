//! `DeckStreak`'s anti-corruption layer for Anki's file formats (SPEC-602, ADR-622).
//!
//! Every foreign schema-18 file opens through one defensive opener ([`opener`]), and every row of
//! a collection and of its media database is read, read-only, as `DeckStreak`'s own types
//! ([`read`]). The blob columns read as typed views beside their raw bytes ([`blob`]), a note's
//! fields split and join on the unit separator ([`fields`]), and a protobuf blob walks as wire
//! format with no schema ([`wire`]).

pub mod blob;
pub mod fields;
pub mod opener;
pub mod read;
pub mod wire;

pub use opener::{Kind, Limits, Reader, Refusal, open};
