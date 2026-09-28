//! The one atomic write (SPEC-042 R2): every file the adapter writes lands through a temporary file
//! in the target's own directory, named `.<name>.<pid>.tmp`, which is written, synced, renamed over
//! the target, and followed by a sync of the directory.
//!
//! The temporary name matches the sync bridge's ignore pattern
//! (`\.tmp\.\d+\.|\.tmp$|\.crswap$|^~|\.crdownload$`), so the bridge never replicates a half-written
//! note as a document of its own. A failed step before the rename leaves the target as it was and
//! removes the temporary file. A failed directory sync after the rename is reported as an error too,
//! because the write is then not known to be durable; the target already holds the new bytes.

use std::path::{Path, PathBuf};

use crate::VaultError;
use crate::fs::VaultFs;

/// The temporary file a write to `target` goes through: `.<name>.<pid>.tmp`, in the target's own
/// directory, so the rename never crosses a file system. `None` when `target` names no file.
#[must_use]
pub fn temp_path(target: &Path, pid: u32) -> Option<PathBuf> {
    let name = target.file_name()?.to_str()?;
    Some(target.with_file_name(format!(".{name}.{pid}.tmp")))
}

/// Writes `bytes` to `target` atomically, replacing what is there. The caller has already decided
/// that the target may be written (the rails, the confinement and the no-overwrite rule).
///
/// The temporary file is created new, so it is never a file or a link that was there before; when
/// a step up to the rename fails, it is removed and the target is as it was.
///
/// # Errors
///
/// [`VaultError::NotARegularFile`] when `target` names no file, and [`VaultError::Io`] naming the
/// step that failed.
pub fn write<F: VaultFs + ?Sized>(fs: &F, target: &Path, bytes: &[u8]) -> Result<(), VaultError> {
    let temp = temp_path(target, std::process::id()).ok_or(VaultError::NotARegularFile)?;
    let directory = target.parent().ok_or(VaultError::NotARegularFile)?;
    let mut file = fs
        .create_new(&temp)
        .map_err(VaultError::io("create a temporary file beside the target"))?;
    let written = file
        .write_all(bytes)
        .map_err(VaultError::io("write the temporary file"))
        .and_then(|()| {
            file.sync()
                .map_err(VaultError::io("sync the temporary file"))
        });
    drop(file);
    let renamed = written.and_then(|()| {
        fs.rename(&temp, target)
            .map_err(VaultError::io("rename the temporary file over the target"))
    });
    if let Err(error) = renamed {
        // The temporary file is this write's own; removing it leaves the target as it was.
        let _removed = fs.remove_file(&temp);
        return Err(error);
    }
    fs.sync_dir(directory)
        .map_err(VaultError::io("sync the target's directory"))
}
