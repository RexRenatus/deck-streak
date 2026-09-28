//! The export: one JSON document holding every table a port exports or resets (SPEC-021 R2;
//! CHARTER 13; `docs/schematics/data-rights-export-and-erase.md`).
//!
//! Every port reads inside one read transaction, so the document is one snapshot of the database: a
//! sync that commits while the export runs is in all of it or in none. A port whose export differs
//! from its declaration is refused by its context and the table, so the owner is never handed a
//! partial copy.

use deck_streak_kernel::{DataRights, Db};
use serde_json::{Map, Value};

use crate::{PrivacyError, declarations, exported};

/// The export document's schema: the value of its [`SCHEMA_KEY`].
pub const EXPORT_SCHEMA: &str = "deckstreak.export.v1";
/// The document's one key that names no table.
pub const SCHEMA_KEY: &str = "schema";

/// The owner's export: one JSON object holding [`SCHEMA_KEY`] and one key per table, each holding
/// that table's rows as JSON objects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Export {
    document: Value,
}

impl Export {
    /// The document, as JSON.
    #[must_use]
    pub const fn as_json(&self) -> &Value {
        &self.document
    }

    /// The document as one line of compact JSON: what `deckstreakd data export` writes.
    #[must_use]
    pub fn to_line(&self) -> String {
        self.document.to_string()
    }
}

/// The export of every table `ports` declare exported or reset: each such table's rows, a
/// singleton's one row included, and no exempt table.
///
/// # Errors
///
/// The refusals of [`declarations`] before anything is read; [`PrivacyError::Port`] when a port's
/// export fails; [`PrivacyError::ExportMismatch`] when one returns other tables than it declares;
/// and [`PrivacyError::Database`] when the read transaction cannot begin.
pub async fn export(db: &Db, ports: &[&dyn DataRights]) -> Result<Export, PrivacyError> {
    let declarations = declarations(ports)?;
    let mut read = db
        .reader()
        .begin()
        .await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    let mut document = Map::new();
    document.insert(SCHEMA_KEY.to_owned(), Value::from(EXPORT_SCHEMA));
    for (port, declaration) in ports.iter().zip(&declarations) {
        let tables = port
            .export(&mut read)
            .await
            .map_err(|source| PrivacyError::Port {
                context: declaration.context(),
                source,
            })?;
        for (table, rows) in exported(declaration, tables)? {
            document.insert(table.to_owned(), Value::Array(rows));
        }
    }
    // The transaction only read: ending it releases the snapshot.
    read.rollback()
        .await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    Ok(Export {
        document: Value::Object(document),
    })
}
