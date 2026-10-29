//! The store's migrations (SPEC-512 R2; ADR-530 D4): embedded from `crates/collection/migrations/`,
//! each run in one `BEGIN IMMEDIATE` transaction that ends with `PRAGMA user_version = n`.

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
pub const MIGRATIONS: &[Migration] = &[];

/// The newest migration's version: a store at head reads it as its `user_version`.
pub const HEAD: i64 = 0;
