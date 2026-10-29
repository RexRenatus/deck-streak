//! Every statement of the store (SPEC-512 R5; ADR-530 D7): each is a constant here or in
//! [`crate::data_rights`], and [`STATEMENTS`] lists every one of them, so the statement census can
//! prepare each against a store at head.

/// One registered statement: its constant's name and its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statement {
    /// The constant's name, as the source writes it.
    pub name: &'static str,
    /// The statement's text.
    pub sql: &'static str,
}

/// The file's `application_id`.
pub const READ_APPLICATION_ID: &str = "PRAGMA application_id";
/// The file's `user_version`.
pub const READ_USER_VERSION: &str = "PRAGMA user_version";
/// The connection's journal mode.
pub const READ_JOURNAL_MODE: &str = "PRAGMA journal_mode";
/// The connection's `synchronous` level.
pub const READ_SYNCHRONOUS: &str = "PRAGMA synchronous";
/// Whether the connection enforces foreign keys.
pub const READ_FOREIGN_KEYS: &str = "PRAGMA foreign_keys";
/// Whether the connection overwrites a deleted row's bytes.
pub const READ_SECURE_DELETE: &str = "PRAGMA secure_delete";
/// Whether the connection refuses every write.
pub const READ_QUERY_ONLY: &str = "PRAGMA query_only";
/// The connection's busy timeout.
pub const READ_BUSY_TIMEOUT: &str = "PRAGMA busy_timeout";
/// The file's integrity check.
pub const INTEGRITY_CHECK: &str = "PRAGMA integrity_check";
/// The file's foreign key check.
pub const FOREIGN_KEY_CHECK: &str = "PRAGMA foreign_key_check";

/// Every statement of the store.
pub const STATEMENTS: &[Statement] = &[];
