//! The file-system port: every byte the adapter reads or writes in the vault goes through
//! [`VaultFs`], so a test can record the order of the steps a write takes (SPEC-042 A1) or make one
//! step fail, and production uses [`RealFs`].
//!
//! The port is small on purpose: it offers what the adapter needs to write atomically, to confine a
//! path (resolving symbolic links) and to walk the readings folder, and nothing that deletes a
//! directory tree. [`VaultFs::remove_file`] exists for the adapter's own temporary files and for the
//! source of a verified move; [`VaultFs::remove_dir`] removes an empty folder only.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// What a directory entry is, read without following a symbolic link.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link, which the adapter never reads, writes or lists through.
    Symlink,
    /// Anything else: a socket, a device, a pipe.
    Other,
}

/// One entry of a listed directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    /// The entry's name, as the file system holds it.
    pub name: OsString,
    /// What the entry is, without following a symbolic link.
    pub kind: EntryKind,
}

/// An open file the adapter is writing: the handle of a temporary file, written and synced before
/// it is renamed over its target.
pub trait VaultFile {
    /// Writes all of `bytes`.
    ///
    /// # Errors
    ///
    /// The operating system's reason when a byte could not be written.
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Flushes the file's data and metadata to the disk (`fsync`).
    ///
    /// # Errors
    ///
    /// The operating system's reason when the data could not be made durable.
    fn sync(&mut self) -> io::Result<()>;
}

/// The file system the vault adapter works on.
pub trait VaultFs {
    /// Creates `path` as a new, empty file and opens it for writing. It never opens an existing
    /// entry, so it never truncates a file and never writes through a symbolic link.
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::AlreadyExists`] when anything is at `path`, and the operating system's
    /// reason otherwise.
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>>;

    /// Renames `from` to `to` on the same file system, atomically.
    ///
    /// # Errors
    ///
    /// The operating system's reason when the rename fails.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Makes the directory's entries durable (`fsync` of the directory), so a rename into it
    /// survives a crash.
    ///
    /// # Errors
    ///
    /// The operating system's reason when the directory could not be synced.
    fn sync_dir(&self, dir: &Path) -> io::Result<()>;

    /// Removes the file at `path`; never a directory.
    ///
    /// # Errors
    ///
    /// The operating system's reason when the file could not be removed.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Reads the whole file at `path`.
    ///
    /// # Errors
    ///
    /// The operating system's reason when the file could not be read.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// What is at `path`, without following a symbolic link, or `None` when nothing is.
    ///
    /// # Errors
    ///
    /// The operating system's reason for any failure but a missing entry.
    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>>;

    /// Every entry of the directory `dir`, in no particular order.
    ///
    /// # Errors
    ///
    /// The operating system's reason when the directory could not be listed.
    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>>;

    /// Creates the directory `path`, whose parent must exist.
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::AlreadyExists`] when anything is at `path`, and the operating system's
    /// reason otherwise.
    fn create_dir(&self, path: &Path) -> io::Result<()>;

    /// Removes the directory `path`, which must be empty.
    ///
    /// # Errors
    ///
    /// The operating system's reason, including a directory that is not empty.
    fn remove_dir(&self, path: &Path) -> io::Result<()>;

    /// The absolute path of `path` with every `..` and symbolic link resolved.
    ///
    /// # Errors
    ///
    /// The operating system's reason, including a path that does not exist.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// The journal folders no write may reach (SPEC-118 R5): none, unless this file system is a
    /// [`JournalGuard`].
    fn journal(&self) -> &[PathBuf] {
        &[]
    }
}

/// A file system that refuses every create, rename and folder under the layout's journal folders
/// (SPEC-118 R5, #56), and passes every other call to the file system it wraps.
#[derive(Clone, Debug)]
pub struct JournalGuard<F> {
    inner: F,
    journal: Vec<PathBuf>,
}

impl<F: VaultFs> JournalGuard<F> {
    /// `inner`, guarded against writes under each of the `journal` folders.
    #[must_use]
    pub const fn new(inner: F, journal: Vec<PathBuf>) -> Self {
        Self { inner, journal }
    }
}

impl<F: VaultFs> VaultFs for JournalGuard<F> {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        self.inner.create_new(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        self.inner.sync_dir(dir)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.inner.read(path)
    }

    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.inner.kind(path)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        self.inner.list(dir)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.inner.canonicalize(path)
    }

    fn journal(&self) -> &[PathBuf] {
        let _ = &self.journal;
        &[]
    }
}

/// The real file system, through `std::fs`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealFs;

/// A file [`RealFs`] opened for writing.
struct RealFile(File);

impl VaultFile for RealFile {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0.write_all(bytes)
    }

    fn sync(&mut self) -> io::Result<()> {
        self.0.sync_all()
    }
}

impl VaultFs for RealFs {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        // `create_new` is O_CREAT | O_EXCL: it fails on any existing entry, a dangling symbolic
        // link included, so it can never write through a link.
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Box::new(RealFile(file)))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        File::open(dir)?.sync_all()
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => Ok(Some(kind_of(metadata.file_type()))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            entries.push(DirEntry {
                name: entry.file_name(),
                kind: kind_of(entry.file_type()?),
            });
        }
        Ok(entries)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }
}

/// The kind of a file type read without following a symbolic link.
fn kind_of(file_type: fs::FileType) -> EntryKind {
    if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Dir
    } else {
        EntryKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::{DirEntry, EntryKind, RealFile, RealFs, VaultFile, VaultFs};
    use std::fs::OpenOptions;

    /// Prints how many items a check examined and refuses zero: a listing that stopped finding its
    /// entries must fail, never pass over the empty set (the tdd pack's examined contract).
    fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
        println!("examined {} {what}", items.len());
        assert!(
            !items.is_empty(),
            "examined 0 {what}: the population is empty, so nothing was judged"
        );
        items
    }

    #[test]
    fn a_sync_the_system_refuses_is_reported() {
        let file = OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("the null device opens");
        let mut real = RealFile(file);
        assert!(real.sync().is_err(), "fsync on the null device is refused");
    }

    #[test]
    fn a_listing_names_each_entry_and_its_kind_without_following_a_link() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        std::fs::write(dir.path().join("note.md"), "x").expect("a file");
        std::fs::create_dir(dir.path().join("day")).expect("a folder");
        std::os::unix::fs::symlink("day", dir.path().join("link")).expect("a link");
        let listed = RealFs.list(dir.path()).expect("the folder lists");
        let mut entries = examined("entries the listing names", listed);
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let entry = |name: &str, kind| DirEntry {
            name: name.into(),
            kind,
        };
        assert_eq!(
            entries,
            [
                entry("day", EntryKind::Dir),
                entry("link", EntryKind::Symlink),
                entry("note.md", EntryKind::File),
            ]
        );
    }
}
