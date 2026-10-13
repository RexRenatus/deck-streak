//! # deck-streak-agent
//!
//! What this context owns: The AI agent: duty runs through the headless Claude Code runner
//! behind the subscription proxy, persona instantiation from public templates and the private
//! roster, prompt composition with untrusted text fenced as data, the output gate that runs the
//! packs' blocking checks, per-run turn and time caps, and fail-closed degradation.
//!
//! What it does not own: Any credential: the device key reaches the runner's environment at
//! launch and never this code's.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
//!
//! SPEC-044 builds the persona engine (ADR-044): the public templates and their instantiation
//! ([`persona`]), the private roster that binds each topic to a template and fills its slots
//! ([`roster`]), the reader that reads one subject's memory and records every read ([`memory`]),
//! and the frontmatter the engine writes on every output from what it did ([`output`]).
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod compose;
pub mod data_rights;
pub mod deck_gate;
pub mod duty;
pub mod fence;
pub mod gate;
pub mod memory;
pub mod output;
pub mod persona;
pub mod roster;
pub mod route;
pub mod runner;
pub mod runs;
pub mod verdict;

pub use deck_gate::{
    CardDecks, DECK_SENSITIVE, DECK_UNREADABLE, DeckFuture, DeckGate, DeckScope, DeckVerdict,
};
pub use memory::{
    LiveBand, MemoryError, MemoryPort, MemoryPorts, MemoryRead, MemoryReader, MemorySource, Recall,
    resolve_band,
};
pub use output::{Frontmatter, OUTPUT_SCHEMA};
pub use persona::{
    CefrBand, Duty, Persona, PersonaError, Slot, Slots, Subject, SubjectKind, TEMPLATE_SCHEMA,
    Template, TemplateId, TemplateSet,
};
pub use roster::{ROSTER, ROSTER_SCHEMA, Roster, RosterPath, TopicKey};
