//! The store's migrations (SPEC-512 R2; ADR-530 D4): embedded from `crates/collection/migrations/`,
//! each run in one `BEGIN IMMEDIATE` transaction that ends with `PRAGMA user_version = n`.

use rusqlite::{Connection, TransactionBehavior};

use crate::error::StoreError;
use crate::sql;

/// One embedded migration, named as its file is: `<version, six digits>_<description>.sql`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Migration {
    /// The `user_version` the migration leaves.
    pub version: i64,
    /// The file name's description part.
    pub description: &'static str,
    /// The migration's statements.
    pub sql: &'static str,
}

/// Every migration, oldest first.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "collection_base",
    sql: include_str!("../migrations/000001_collection_base.sql"),
}];

/// The newest migration's version: a store at head reads it as its `user_version`.
pub const HEAD: i64 = 1;

/// Runs every migration above the file's `user_version`, each in its own `BEGIN IMMEDIATE`
/// transaction. The version is read again inside each transaction, so two openers racing on one
/// new file apply each migration once.
///
/// # Errors
///
/// [`StoreError::Sqlite`] when a migration fails; its transaction rolls back whole.
pub(crate) fn apply(conn: &mut Connection) -> Result<(), StoreError> {
    for migration in MIGRATIONS {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let at: i64 = tx.query_row(sql::READ_USER_VERSION, [], |row| row.get(0))?;
        if at >= migration.version {
            tx.commit()?;
            continue;
        }
        tx.execute_batch(migration.sql)?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
    }
    Ok(())
}
