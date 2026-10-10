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

/// One backup or server copy the pool lists, with when it was first listed (SPEC-377 R15; ADR-388
/// D17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    /// The pool name's stem, `backup-2`: the one id the page names it by.
    pub id: String,
    /// Its kind.
    pub kind: Kind,
    /// The number its name was minted with.
    pub number: u32,
    /// When the web engine first listed it, in milliseconds since the epoch.
    pub made: i64,
    /// Its pool name.
    pub name: String,
    /// The pool name of its record.
    pub record: String,
}

/// The kind and number of the backup or server copy the pool names `name`: a last segment
/// `{kind}-{n}.anki2`, with `n` from 1 spelt as the number it is. Any other name is none.
#[must_use]
pub fn backup_of(name: &str) -> Option<(Kind, u32)> {
    let file = name.rsplit_once('/').map_or(name, |(_, file)| file);
    let (word, number) = file.strip_suffix(".anki2")?.split_once('-')?;
    let kind = [Kind::Backup, Kind::Server]
        .into_iter()
        .find(|kind| kind.word() == word)?;
    let minted = number
        .parse::<u32>()
        .ok()
        .filter(|minted| *minted > 0 && minted.to_string() == number)?;
    Some((kind, minted))
}

/// The pool name of the record that says `name` was first listed at `made`, in milliseconds since
/// the epoch: an empty pool entry beside it, since the pool keeps no file times (ADR-388 D17).
#[must_use]
pub fn record(name: &str, made: i64) -> String {
    format!("{name}@{made}")
}

/// When the record `listed` holds for `name` says it was first listed, with that record's name.
fn recorded<'a>(listed: &'a [String], name: &str) -> Option<(i64, &'a String)> {
    listed.iter().find_map(|entry| {
        let made = entry.strip_prefix(name)?.strip_prefix('@')?;
        let at = made
            .parse::<i64>()
            .ok()
            .filter(|at| at.to_string() == made)?;
        Some((at, entry))
    })
}

/// The backups and server copies `listed` holds with no record yet, in the pool's order: the ones
/// the web engine records when it lists them.
#[must_use]
pub fn unrecorded(listed: &[String]) -> Vec<String> {
    listed
        .iter()
        .filter(|name| backup_of(name).is_some() && recorded(listed, name).is_none())
        .cloned()
        .collect()
}

/// The backups and server copies `listed` holds with their records, newest first: by the record,
/// then by the number minted (ADR-388 D17). One with no record is not listed yet.
#[must_use]
pub fn backups(listed: &[String]) -> Vec<Backup> {
    let found: Vec<Backup> = listed
        .iter()
        .filter_map(|name| {
            let (kind, number) = backup_of(name)?;
            let (made, record) = recorded(listed, name)?;
            Some(Backup {
                id: format!("{}-{number}", kind.word()),
                kind,
                number,
                made,
                name: name.clone(),
                record: record.clone(),
            })
        })
        .collect();
    // The red stub: the pool's order, unsorted.
    found
}

/// The pool name of the backup or server copy `id` names beside `collection`, when the pool lists
/// it as one; any other id, the collection's among them, names nothing to export (SPEC-377 R16).
#[must_use]
pub fn exported(listed: &[String], collection: &str, id: &str) -> Option<String> {
    let directory = collection
        .rsplit_once('/')
        .map_or("", |(directory, _)| directory);
    let name = format!("{directory}/{id}.anki2");
    // The red stub: any id is exported.
    let _ = listed;
    Some(name)
}
