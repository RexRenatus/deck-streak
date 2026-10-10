//! The browser's backups kept (SPEC-377 R17; ADR-388 D15): the one rule the web engine applies
//! before each backup list, held here so the core's tests hold it.
//!
//! A file is removed once [`KEEP`] newer files of its kind exist; the newest of a kind is never
//! removed, and the kinds are counted apart. The rule reads only each file's kind and when it was
//! made, so it names no path and opens nothing.

/// The files of each kind kept.
pub const KEEP: usize = 3;

/// One file the rule judges: its kind, and when it was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held<K, T> {
    /// The file's kind; the files of one kind are counted together.
    pub kind: K,
    /// When the file was made; a larger value is newer.
    pub made: T,
}

/// The indices into `held` of the files to remove, ascending, when [`KEEP`] of each kind are kept.
#[must_use]
pub fn removals<K: PartialEq + Copy, T: Ord>(held: &[Held<K, T>]) -> Vec<usize> {
    removed(held, KEEP)
}

/// The indices into `held` of the files to remove, ascending, when `keep` of each kind are kept.
/// The newest of a kind is never removed, whatever `keep` is.
#[must_use]
pub fn removed<K: PartialEq + Copy, T: Ord>(held: &[Held<K, T>], keep: usize) -> Vec<usize> {
    // the grouping key: the files of one kind are counted together, and only together
    let kind = |file: &Held<K, T>| file.kind;
    held.iter()
        .enumerate()
        .filter(|(_, file)| {
            let newer = held
                .iter()
                .filter(|other| kind(other) == kind(file) && other.made > file.made)
                .count();
            // the newest of a kind is exempt; any other file goes once `keep` newer ones exist
            !(newer == 0 || newer < keep)
        })
        .map(|(index, _)| index)
        .collect()
}
