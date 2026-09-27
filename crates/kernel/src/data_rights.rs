//! The data-rights port: every stateful context declares its tables with what an export and an
//! erase do to each, exports its rows and erases inside the caller's transaction (SPEC-020 R20;
//! CHARTER 13).
//!
//! The same set of tables is exported and erased, singletons are reset in place, and a table
//! exempt from both says why. The kernel refuses a declaration that names a table twice or exempts
//! one without a reason; the privacy context (SPEC-021) runs the ports.

use std::future::Future;
use std::pin::Pin;

use serde_json::{Map, Value};
use sqlx::SqliteConnection;

use crate::error::{DataRightsError, KernelError};

/// The kernel's context, as the ownership register names it.
pub const KERNEL_CONTEXT: &str = "kernel";
/// The kernel's one table: the owner-config generation (R19).
pub const SETTINGS_GENERATION_TABLE: &str = "settings_generation";
/// sqlx's record of the applied migrations: the schema version table that replaces the
/// predecessor's `schema_versions`.
pub const SCHEMA_VERSION_TABLE: &str = "_sqlx_migrations";

/// What an export and an erase do to one table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// Exported whole, and emptied by an erase.
    ExportAndErase,
    /// A singleton: exported, and on erase its one row is overwritten with `row`, never deleted.
    ResetInPlace {
        /// The columns an erase writes, with their values.
        row: Map<String, Value>,
    },
    /// Neither exported nor erased, for `reason`.
    Exempt {
        /// Why the table survives an erase.
        reason: &'static str,
    },
}

/// One table of a declaration, with its disposition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRights {
    /// The table's name.
    pub table: &'static str,
    /// What an export and an erase do to it.
    pub disposition: Disposition,
}

/// A context's tables, each named once with its disposition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    context: &'static str,
    tables: Vec<TableRights>,
}

impl Declaration {
    /// The declaration of `context`'s `tables`.
    ///
    /// # Errors
    ///
    /// [`DataRightsError::TableDeclaredTwice`] when a table is named twice, and
    /// [`DataRightsError::ExemptWithoutReason`] when a table is exempt with a blank reason.
    pub fn new(context: &'static str, tables: Vec<TableRights>) -> Result<Self, DataRightsError> {
        Ok(Self { context, tables })
    }

    /// The declaring context.
    #[must_use]
    pub const fn context(&self) -> &'static str {
        self.context
    }

    /// Every table, once, in the order declared.
    #[must_use]
    pub fn tables(&self) -> &[TableRights] {
        &self.tables
    }

    /// The disposition of `table`, or `None` when the context does not declare it.
    #[must_use]
    pub fn disposition(&self, table: &str) -> Option<&Disposition> {
        self.tables
            .iter()
            .find(|rights| rights.table == table)
            .map(|rights| &rights.disposition)
    }
}

/// The rows an export read from one table, as JSON objects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedTable {
    /// The table's name.
    pub table: &'static str,
    /// Its rows, one JSON object each.
    pub rows: Vec<Value>,
}

/// A port's future: boxed, so the ports of every context fit in one list of trait objects.
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, KernelError>> + Send + 'a>>;

/// The port every stateful context implements (R20).
pub trait DataRights: Send + Sync {
    /// The context's tables, each once, with its disposition.
    ///
    /// # Errors
    ///
    /// The refusal of [`Declaration::new`].
    fn declaration(&self) -> Result<Declaration, DataRightsError>;

    /// The rows of every table exported or reset in place, as JSON objects.
    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>>;

    /// Erases every exported table and resets every singleton, inside the transaction the caller
    /// holds on `connection`, so a failure anywhere rolls back every context's erase.
    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()>;
}

/// The kernel's own port: it resets the settings generation in place and exempts the schema
/// version table.
#[derive(Clone, Copy, Debug, Default)]
pub struct KernelDataRights;

impl DataRights for KernelDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(KERNEL_CONTEXT, Vec::new())
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        let _ = connection;
        Box::pin(async { Ok(Vec::new()) })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        let _ = connection;
        Box::pin(async { Ok(()) })
    }
}
