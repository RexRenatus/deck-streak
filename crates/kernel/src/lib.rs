//! # deck-streak-kernel
//!
//! What this context owns: The shared kernel: identifiers, the study day and its 04:00 rollover,
//! the track (language or law), the verdict type, the gate class (the closed name of a check an
//! output gate runs, which the agent's gate reports and the readings rank), the error type, typed
//! configuration and credentials, the clock, the redacting log setup, the bounded offload for
//! blocking work, and the
//! `SQLite` repository base (WAL, foreign keys, busy timeout, `BEGIN IMMEDIATE`) with the
//! data-rights port every context implements (SPEC-020).
//!
//! What it does not own: No domain rule of any context. A type enters here only when two
//! contexts that may not depend on each other both need it.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod clock;
pub mod conventions;
pub mod courses;
pub mod credentials;
pub mod data_rights;
pub mod db;
pub mod error;
pub mod gate_class;
pub mod ids;
pub mod logging;
pub mod offload;
pub mod redact;
pub mod settings;
pub mod study_day;
pub mod track;
pub mod verdict;

pub use clock::{Clock, ManualClock, SystemClock, UtcMillis};
pub use conventions::{Conventions, ConventionsError, Direction};
pub use courses::{CourseCode, Courses, CoursesError};
pub use credentials::{CredentialLoader, Secret};
pub use data_rights::{
    DataRights, Declaration, Disposition, ExportedTable, KernelDataRights, PortFuture, TableRights,
};
pub use db::{Db, ForeignDb, MIGRATOR};
pub use error::{CredentialError, DataRightsError, IsoDateError, KernelError, SettingsError};
pub use gate_class::{GateClass, UnknownGateClass};
pub use ids::TelegramUserId;
pub use offload::Offload;
pub use redact::Redactor;
pub use settings::{CredentialsDirectory, Environment, KernelSettings, OffloadWorkers, Setting};
pub use study_day::{Hour, StudyDay, StudyDayRule, UtcOffset};
pub use track::Track;
pub use verdict::Verdict;
