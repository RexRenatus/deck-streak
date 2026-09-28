//! The export: one JSON document holding every table a port exports or resets (SPEC-021 R2;
//! CHARTER 13; `docs/schematics/data-rights-export-and-erase.md`).

use deck_streak_kernel::{DataRights, Db, Disposition};
use serde_json::{Map, Value};

use crate::PrivacyError;

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

/// The export of every table `ports` declare exported or reset.
///
/// # Errors
///
/// [`PrivacyError::Database`] when no connection can be had, and [`PrivacyError::Port`] when a
/// port's export fails.
pub async fn export(db: &Db, ports: &[&dyn DataRights]) -> Result<Export, PrivacyError> {
    let mut connection = db
        .reader()
        .acquire()
        .await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    let mut document = Map::new();
    document.insert(SCHEMA_KEY.to_owned(), Value::from(EXPORT_SCHEMA));
    for port in ports {
        let declaration = port.declaration()?;
        let tables = port
            .export(&mut connection)
            .await
            .map_err(|source| PrivacyError::Port {
                context: declaration.context(),
                source,
            })?;
        for table in tables {
            // This first export keeps the tables a port empties and leaves out the singletons it
            // resets.
            if declaration.disposition(table.table) == Some(&Disposition::ExportAndErase) {
                document.insert(table.table.to_owned(), Value::Array(table.rows));
            }
        }
    }
    Ok(Export {
        document: Value::Object(document),
    })
}
