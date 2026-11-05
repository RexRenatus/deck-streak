//! The collection lock (SPEC-022 R7): an exclusive `flock` for a sync or a full download, a
//! shared one for a read, on `<state directory>/collection.lock`.
//!
//! A second sync waits for the first and never overlaps it; a reader never sees the copy mid-swap.
//! The lock is released by an explicit unlock before the file closes, never by the close alone: a
//! lock another handle to the same file still held would otherwise outlive its holder. A lock
//! handed to a task by [`Held::hold_to_end`] stays held until that task ends, and when its runtime
//! drops the task, until the process exits and the operating system closes the file.

use std::fs::{File, OpenOptions, TryLockError};
use std::future::Future;
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
        self.take(File::lock).await
    }

    /// Takes the lock exclusively without waiting: `None` when another holder has it, so a caller
    /// that finds it held starts nothing (SPEC-094 R8). The lock is released by an explicit unlock.
    ///
    /// # Errors
    ///
    /// The operating system's error when the file cannot be opened or locked for a reason other
    /// than being held.
    pub async fn try_exclusive(&self) -> io::Result<Option<Held>> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)?;
            match file.try_lock() {
                Ok(()) => Ok(Some(Held { file: Some(file) })),
                Err(TryLockError::WouldBlock) => Ok(None),
                Err(TryLockError::Error(error)) => Err(error),
            }
        })
        .await
        .map_err(io::Error::other)?
    }

    /// Takes the lock shared, waiting for an exclusive holder to release it.
    ///
    /// # Errors
    ///
    /// The operating system's error when the file cannot be opened or locked.
    pub async fn shared(&self) -> io::Result<Held> {
        self.take(File::lock_shared).await
    }

    /// Opens the lock file and takes `lock` on it on tokio's blocking pool, where a wait for
    /// another holder blocks a thread made for blocking rather than an async worker.
    async fn take(&self, lock: fn(&File) -> io::Result<()>) -> io::Result<Held> {
        let path = self.path.clone();
        let file = tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)?;
            lock(&file)?;
            Ok::<_, io::Error>(file)
        })
        .await
        .map_err(io::Error::other)??;
        Ok(Held { file: Some(file) })
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

    /// Runs `work` in a task of its own that holds this lock, and releases the lock when the
    /// work ends (SPEC-470 R5). A caller who stops waiting leaves the task to finish its work and
    /// release the lock. A panic in the work unlocks, then resumes in the caller. A task its
    /// runtime drops keeps the lock until the process exits.
    ///
    /// # Errors
    ///
    /// The task's join error, for any end of the task but a panic.
    ///
    /// # Panics
    ///
    /// The work's panic, resumed in the caller.
    pub async fn hold_to_end<T, F>(self, work: F) -> io::Result<(T, io::Result<()>)>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task = async move {
            let mut kept = ToEnd(Some(self));
            let value = work.await;
            (value, kept.release())
        };
        match tokio::spawn(task).await {
            Ok(ended) => Ok(ended),
            Err(error) => match error.try_into_panic() {
                Ok(panic) => std::panic::resume_unwind(panic),
                Err(error) => Err(io::Error::other(error)),
            },
        }
    }
}

/// A held lock a task keeps to the end of its work.
struct ToEnd(Option<Held>);

impl ToEnd {
    /// Releases the lock at the work's end.
    fn release(&mut self) -> io::Result<()> {
        self.0.take().map_or(Ok(()), Held::release)
    }
}

impl Drop for ToEnd {
    /// Unlocks while a panic unwinds, and otherwise keeps the lock until the process exits.
    fn drop(&mut self) {
        if let Some(held) = self.0.take() {
            if std::thread::panicking() {
                drop(held);
            } else {
                std::mem::forget(held);
            }
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}
