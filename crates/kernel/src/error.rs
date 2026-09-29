//! The kernel's errors (SPEC-020 R10, R11, R20; SPEC-066 R1).
//!
//! A library returns its own `thiserror` types. No error here ever carries a setting's value or a
//! secret: a refusal names the setting, the shape it expects or the credential's id, and nothing a
//! log line could leak.

/// Why the kernel could not do what was asked.
#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// A credential refused start.
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// A data-rights declaration was refused.
    #[error(transparent)]
    DataRights(#[from] DataRightsError),
    /// The database refused an operation; the source says why.
    #[error("the database refused the operation")]
    Database(#[from] sqlx::Error),
    /// The migrations could not be applied; the source says which and why.
    #[error("the database's migrations could not be applied")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    /// A process installs its one log subscriber once.
    #[error("the process's log subscriber is already installed")]
    LoggingInstalled,
    /// The blocking work of an offloaded operation panicked or was cancelled; its payload is not
    /// kept, because it may hold the work's data.
    #[error("the offloaded operation {operation} did not complete")]
    Offload {
        /// The operation the caller named.
        operation: &'static str,
    },
}

/// Why a setting refuses start. It names the setting and never its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// A required setting is unset or blank.
    #[error("the setting {setting} is required and is not set")]
    Missing {
        /// The setting's name.
        setting: &'static str,
    },
    /// A setting does not have the shape it must have.
    #[error("the setting {setting} is malformed: it must be {expected}")]
    Malformed {
        /// The setting's name.
        setting: &'static str,
        /// The shape the setting expects.
        expected: &'static str,
    },
    /// An explicitly set digest hour is earlier than the rollover hour (CHARTER 7).
    #[error(
        "the setting {digest} is earlier than the setting {rollover}: a digest never fires before \
         the day it reports has closed"
    )]
    DigestBeforeRollover {
        /// The digest hour's setting.
        digest: &'static str,
        /// The rollover hour's setting.
        rollover: &'static str,
    },
}

/// Why a credential refuses start. It names the credential's id and never its value.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// The credentials directory holds no file of that id.
    #[error("the credential {id} is missing from the credentials directory")]
    Missing {
        /// The credential's id.
        id: &'static str,
    },
    /// The credential's file holds no value: no bytes, or only the one trailing newline the loader
    /// trims. It refuses start as a missing credential does (SPEC-066 R1; ADR-067).
    #[error("the credential {id} is empty in the credentials directory")]
    Empty {
        /// The credential's id.
        id: &'static str,
    },
    /// The credential's file exists and cannot be read.
    #[error("the credential {id} cannot be read")]
    Unreadable {
        /// The credential's id.
        id: &'static str,
        /// The operating system's reason.
        #[source]
        source: std::io::Error,
    },
    /// The credential's file is not UTF-8 text.
    #[error("the credential {id} is not UTF-8 text")]
    NotText {
        /// The credential's id.
        id: &'static str,
    },
    /// An id is a plain file name in the credentials directory: not empty, not `.` or `..`, and
    /// without a `/` or a NUL.
    #[error("{id:?} is not a credential id")]
    InvalidId {
        /// The id that was asked for.
        id: &'static str,
    },
}

/// Why a context's data-rights declaration is refused (CHARTER 13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DataRightsError {
    /// A table is declared more than once, so its export and its erase could disagree.
    #[error("the context {context} declares the table {table} more than once")]
    TableDeclaredTwice {
        /// The declaring context.
        context: &'static str,
        /// The table declared twice.
        table: &'static str,
    },
    /// A table is exempt from export and erase with no reason given.
    #[error("the context {context} exempts the table {table} without a reason")]
    ExemptWithoutReason {
        /// The declaring context.
        context: &'static str,
        /// The table exempted.
        table: &'static str,
    },
}

/// A text that is not an ISO calendar date of the proleptic Gregorian calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not an ISO calendar date (YYYY-MM-DD)")]
pub struct IsoDateError;
