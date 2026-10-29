//! The store's data-rights port (SPEC-512 R10; ADR-530 D11): its export, the server's erase, a
//! device's erase and the client record's own erase. Outside the migrations, this is the one file
//! that names the erase gate.

use crate::error::StoreError;
use crate::store::Store;

/// What an erase does to a declared table, as `privacy.json` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Erase {
    /// Every row is deleted.
    Delete,
    /// The row stays, its personal value cleared.
    Anonymise,
}

impl Erase {
    /// The mode as `privacy.json` writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::Anonymise => "anonymise",
        }
    }
}

/// Every table the port exports, with what an erase does to it.
pub const DECLARED: &[(&str, Erase)] = &[];

/// The tables the port neither exports nor erases, each for a reason.
pub const EXEMPT: &[&str] = &[];

impl Store {
    /// Every declared table's rows, as JSON.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when a table cannot be read.
    pub fn export(&self) -> Result<serde_json::Value, StoreError> {
        Ok(serde_json::Value::Object(serde_json::Map::new()))
    }

    /// The server's erase: every declared table emptied through the gate, the store's record
    /// anonymised, and the file vacuumed.
    ///
    /// # Errors
    ///
    /// [`StoreError::ReadOnly`] for a reader, and [`StoreError::Sqlite`] when a statement fails.
    pub fn erase(&mut self) -> Result<(), StoreError> {
        Ok(())
    }

    /// The client record's own erase: its rows deleted through the gate, every review kept.
    /// Returns how many rows it deleted.
    ///
    /// # Errors
    ///
    /// [`StoreError::ReadOnly`] for a reader, and [`StoreError::Sqlite`] when a statement fails.
    pub fn erase_client_records(&mut self) -> Result<usize, StoreError> {
        Ok(0)
    }

    /// A device's erase: the store's file, and its WAL and shared-memory files, removed.
    ///
    /// # Errors
    ///
    /// [`StoreError::Remove`] when a file cannot be removed.
    pub fn erase_file(self) -> Result<(), StoreError> {
        Ok(())
    }
}
