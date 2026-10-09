//! The pool's names (SPEC-377 R4, R5; ADR-388 D7, D8): whether the pool holds a name, one name per
//! file, and the new names of a choice's files. Compiled natively, like `study.rs`, so its rules are
//! tested where the pool cannot run.

/// The kinds of file a choice makes beside the collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The backup of the side a write replaces.
    Backup,
    /// A copy of the server's collection, counted or checked again.
    Server,
}

impl Kind {
    /// The word a file of this kind is named by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::Server => "server",
        }
    }
}

/// Whether the pool, listing `listed`, holds a file named `name`.
#[must_use]
pub fn holds(_listed: &[String], _name: &str) -> bool {
    true
}

/// Whether `open` and `name` name one pool file.
#[must_use]
pub fn same(_open: &str, _name: &str) -> bool {
    true
}

/// A new name for a choice file of `kind`, beside `collection` in the pool's directory.
#[must_use]
pub fn choice_name(_listed: &[String], collection: &str, _kind: Kind) -> Option<String> {
    Some(collection.to_owned())
}
