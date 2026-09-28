//! # deck-streak-privacy
//!
//! What this context owns: Data rights: export and erase over every context's data-rights port,
//! proved symmetric, and the retention purge.
//!
//! What it does not own: Any table: each context owns its tables and answers for them through
//! the port.
//!
//! [`export`] reads every port's exported and reset tables in one read transaction and returns one
//! JSON document; [`erase`] runs every port's erase in one `BEGIN IMMEDIATE` transaction with
//! `secure_delete` on, checks that each port left exactly what its declaration says before it
//! commits, then compacts the file and truncates its write-ahead log (SPEC-021 R2, R3;
//! `docs/schematics/data-rights-export-and-erase.md`). Which ports they run is the caller's:
//! coordination's registry holds every context's (R1).
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod erase;
pub mod export;

use std::collections::BTreeMap;
use std::fmt;

use deck_streak_kernel::{DataRights, DataRightsError, Declaration, KernelError};

pub use erase::{Erasure, erase};
pub use export::{EXPORT_SCHEMA, Export, SCHEMA_KEY, export};

/// Why an export or an erase did not complete. No refusal carries a row's value: it names the
/// context and the table, and the database's cause is its source.
#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    /// A port's declaration was refused.
    #[error(transparent)]
    Declaration(#[from] DataRightsError),
    /// Two ports declare one table: their export keys would collide and their erases could
    /// disagree about it.
    #[error("the table {table} is declared by both the context {first} and the context {second}")]
    DeclaredTwice {
        /// The table.
        table: &'static str,
        /// The context that declared it first, in the ports' order.
        first: &'static str,
        /// The context that declared it again.
        second: &'static str,
    },
    /// A table takes the export's own schema key, so the document could not hold both.
    #[error("the context {context} declares a table named {table}, the export's schema key")]
    ReservedName {
        /// The declaring context.
        context: &'static str,
        /// The table.
        table: &'static str,
    },
    /// A port's export does not return exactly the tables it declares exported or reset, once
    /// each, so the owner would be handed a partial or a doubled copy.
    #[error("the context {context}'s export {problem}: {table}")]
    ExportMismatch {
        /// The port's context.
        context: &'static str,
        /// The table.
        table: &'static str,
        /// How the export differs from the declaration.
        problem: ExportProblem,
    },
    /// A port's erase left a table its declaration clears or resets, so the erase rolled back.
    #[error(
        "the context {context}'s erase left the table {table} as its declaration forbids, so \
         nothing was erased"
    )]
    EraseIncomplete {
        /// The port's context.
        context: &'static str,
        /// The table.
        table: &'static str,
    },
    /// A port failed; in an erase the transaction rolled back, so nothing was erased.
    #[error("the context {context}'s port failed")]
    Port {
        /// The port's context.
        context: &'static str,
        /// Why.
        #[source]
        source: KernelError,
    },
    /// SQLite did not turn `secure_delete` on, so the erase stopped before it deleted anything.
    #[error("the database did not turn secure_delete on, so nothing was erased")]
    SecureDeleteOff,
    /// The database refused one of the engine's own statements before an erase committed (its
    /// begin, its pragma, its commit), or an export's read transaction; nothing was erased.
    #[error("the database refused the operation")]
    Database(#[source] KernelError),
    /// The erase committed and the compaction after it failed: what it erased may remain in the
    /// file's free pages or its write-ahead log until the next compaction.
    #[error("the erase committed, and the database's compaction after it failed")]
    Compaction(#[source] KernelError),
}

/// How a port's export differs from its declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportProblem {
    /// It returned a table it does not declare exported or reset.
    Undeclared,
    /// It returned one table twice.
    Twice,
    /// It left out a table it declares exported or reset.
    Omitted,
}

impl fmt::Display for ExportProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Undeclared => "returned a table it does not declare exported",
            Self::Twice => "returned a table twice",
            Self::Omitted => "left out a table it declares exported",
        })
    }
}

/// Every port's declaration, in the ports' order: what an export and an erase run over.
///
/// # Errors
///
/// [`PrivacyError::Declaration`] when the kernel refuses a declaration,
/// [`PrivacyError::DeclaredTwice`] when two ports declare one table, and
/// [`PrivacyError::ReservedName`] when a table is named for the export's schema key.
pub fn declarations(ports: &[&dyn DataRights]) -> Result<Vec<Declaration>, PrivacyError> {
    let mut owners: BTreeMap<&'static str, &'static str> = BTreeMap::new();
    let mut found = Vec::with_capacity(ports.len());
    for port in ports {
        let declaration = port.declaration()?;
        let context = declaration.context();
        for rights in declaration.tables() {
            if rights.table == SCHEMA_KEY {
                return Err(PrivacyError::ReservedName {
                    context,
                    table: rights.table,
                });
            }
            if let Some(first) = owners.insert(rights.table, context) {
                return Err(PrivacyError::DeclaredTwice {
                    table: rights.table,
                    first,
                    second: context,
                });
            }
        }
        found.push(declaration);
    }
    Ok(found)
}
