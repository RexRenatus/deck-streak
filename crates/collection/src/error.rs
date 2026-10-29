//! Why the store refused or failed (SPEC-512 R1, R2, R8).

/// Why a store, a reader or a foreign file could not be opened, read or written.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The file belongs to another application: its `application_id` is not the store's, and it is
    /// not empty. Nothing in it was changed.
    #[error("the file belongs to another application (application id {0})")]
    ForeignFile(i32),
    /// A foreign reader was handed a collection store, which only the store's own opener opens.
    #[error("the file is a collection store, which a foreign reader never opens")]
    StoreFile,
    /// The store was written by a newer app: its schema is above the newest migration this app
    /// carries, and it is never downgraded.
    #[error("the store is at schema {found}, newer than this app's {newest}")]
    NewerSchema {
        /// The store's `user_version`.
        found: i64,
        /// The newest migration this app carries.
        newest: i64,
    },
    /// A reader was handed a store below head, which only a writer migrates.
    #[error("the store is at schema {found}, and a reader needs {newest}")]
    NotAtHead {
        /// The store's `user_version`.
        found: i64,
        /// The newest migration this app carries.
        newest: i64,
    },
    /// The store was made for another role, writer id or slot.
    #[error("the store was made for another role")]
    RoleMismatch,
    /// The store was opened to read, and refuses every write.
    #[error("the store was opened to read, and refuses every write")]
    ReadOnly,
    /// A device's erase could not remove the store's file.
    #[error("the store's file could not be removed")]
    Remove(#[source] std::io::Error),
    /// `SQLite` refused or failed a statement.
    #[error("the store's database failed a statement")]
    Sqlite(#[from] rusqlite::Error),
}
