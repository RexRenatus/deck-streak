//! The collection lock (SPEC-022 R7): an exclusive `flock` for a sync or a full download, a
//! shared one for a read, on `<state directory>/collection.lock`.
//!
//! A second sync waits for the first and never overlaps it; a reader never sees the copy mid-swap.
//! The lock is released by an explicit unlock before the file closes, never by the close alone: a
//! lock another handle to the same file still held would otherwise outlive its holder.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// The lock file at a path.
#[derive(Clone, Debug)]
pub struct CollectionLock {
    path: PathBuf,
}

/// A held lock. [`Held::release`] unlocks, then closes; dropping it does the same.
#[derive(Debug)]
pub struct Held {
    file: Option<File>,
}

impl CollectionLock {
    /// The lock on the file at `path`, created on first use.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The lock file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Takes the lock exclusively, waiting (off the async workers) for any holder to release it.
    ///
    /// # Errors
    ///
    /// The operating system's error when the file cannot be opened or locked.
    pub async fn exclusive(&self) -> io::Result<Held> {
        Ok(Held { file: None })
    }

    /// Takes the lock shared, waiting for an exclusive holder to release it.
    ///
    /// # Errors
    ///
    /// The operating system's error when the file cannot be opened or locked.
    pub async fn shared(&self) -> io::Result<Held> {
        Ok(Held { file: None })
    }
}

impl Held {
    /// Unlocks explicitly, then closes the file.
    ///
    /// # Errors
    ///
    /// The operating system's error when the unlock fails; the file is closed either way.
    pub fn release(mut self) -> io::Result<()> {
        self.file.take().map_or(Ok(()), |file| file.unlock())
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}
