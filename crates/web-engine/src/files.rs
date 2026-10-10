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

/// Whether the pool, listing `listed`, holds a file named `name`. The pool knows a file only by
/// the name it lists, so no other spelling is a file it holds.
#[must_use]
pub fn holds(listed: &[String], name: &str) -> bool {
    listed.iter().any(|file| file == name)
}

/// Whether `open` and `name` name one pool file. The pool has no directories to walk and no links
/// to follow, so one file has one name (ADR-388 D8).
#[must_use]
pub fn same(open: &str, name: &str) -> bool {
    open == name
}

/// A new name for a choice file of `kind`, beside `collection` in the pool's directory: never the
/// collection's and never one the pool lists. Of `listed.len() + 2` candidates at least two are
/// neither, so a name is always found.
#[must_use]
pub fn choice_name(listed: &[String], collection: &str, kind: Kind) -> Option<String> {
    let directory = collection
        .rsplit_once('/')
        .map_or("", |(directory, _)| directory);
    (1..=listed.len() + 2)
        .map(|n| format!("{directory}/{}-{n}.anki2", kind.word()))
        .find(|name| !(name == collection || holds(listed, name)))
}
