//! DeckStreak's collection store (SPEC-512, ADR-530): one `SQLite` file per user, the same schema
//! on the server and on each device, opened only by [`Store::open`], [`Store::open_reader`] and
//! [`ForeignFile::open_read_only`].
//!
//! The crate reads no clock and holds no runtime: every instant is a parameter, and a server caller
//! runs it on its own blocking offload. Every statement is a registered constant of [`sql`] or
//! [`data_rights`], prepared by the statement census. The review log is append-only: its rows, its
//! retractions, its client record and the notes' field history never change, and leave only while
//! the erase gate is open, which [`data_rights`] alone opens. [`CollectionRead`] is the read port
//! the server's contexts read through; the store answers it through [`Snapshot`], and ingest's
//! schema-18 adapter answers it over Anki's file. Nothing live reads or writes the store yet.

pub mod data_rights;
pub mod error;
pub mod migrate;
pub mod read;
pub mod sql;
pub mod store;

pub use error::StoreError;
pub use read::{
    CardRead, CollectionRead, FieldRead, LogCounts, NoteRead, ReviewRead, TemplateRead, scope_json,
};
pub use store::{
    APPLICATION_ID, BUSY_TIMEOUT_MS, ForeignFile, Health, OpenSettings, Origin, Role, Slot,
    Snapshot, Store, Tx,
};
