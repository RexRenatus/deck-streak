//! Every write lands through a temporary name the sync bridge ignores, in the target's own
//! directory, then a file sync, the rename and a directory sync, in that order; a step that fails
//! leaves the target as it was and removes the temporary file (SPEC-042 A1, R2).

// An integration test is test code: its helpers panic on an unreadable file.
#![allow(clippy::expect_used)]

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use deck_streak_vault::{DirEntry, EntryKind, RealFs, VaultFile, VaultFs, atomic};

/// One step a write took, as the recording file system saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    CreateNew(PathBuf),
    Write(PathBuf),
    SyncFile(PathBuf),
    Rename(PathBuf, PathBuf),
    SyncDir(PathBuf),
    RemoveFile(PathBuf),
}

/// The step the recording file system makes fail, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fail {
    Nothing,
    Write,
    SyncFile,
    Rename,
}

type Log = Arc<Mutex<Vec<Step>>>;

/// A file system that records every write step, does it on the real file system, and fails the
/// step it was told to.
struct Recording {
    log: Log,
    fail: Fail,
}

impl Recording {
    fn new(fail: Fail) -> Self {
        Self {
            log: Arc::default(),
            fail,
        }
    }

    fn steps(&self) -> Vec<Step> {
        self.log.lock().expect("the log").clone()
    }
}

fn record(log: &Log, step: Step) {
    log.lock().expect("the log").push(step);
}

fn planted() -> io::Error {
    io::Error::other("a planted failure")
}

struct RecordingFile {
    inner: Box<dyn VaultFile>,
    path: PathBuf,
    log: Log,
    fail: Fail,
}

impl VaultFile for RecordingFile {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        record(&self.log, Step::Write(self.path.clone()));
        if self.fail == Fail::Write {
            return Err(planted());
        }
        self.inner.write_all(bytes)
    }

    fn sync(&mut self) -> io::Result<()> {
        record(&self.log, Step::SyncFile(self.path.clone()));
        if self.fail == Fail::SyncFile {
            return Err(planted());
        }
        self.inner.sync()
    }
}

impl VaultFs for Recording {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        record(&self.log, Step::CreateNew(path.to_path_buf()));
        Ok(Box::new(RecordingFile {
            inner: RealFs.create_new(path)?,
            path: path.to_path_buf(),
            log: Arc::clone(&self.log),
            fail: self.fail,
        }))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        record(
            &self.log,
            Step::Rename(from.to_path_buf(), to.to_path_buf()),
        );
        if self.fail == Fail::Rename {
            return Err(planted());
        }
        RealFs.rename(from, to)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        record(&self.log, Step::SyncDir(dir.to_path_buf()));
        RealFs.sync_dir(dir)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        record(&self.log, Step::RemoveFile(path.to_path_buf()));
        RealFs.remove_file(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        RealFs.read(path)
    }

    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        RealFs.kind(path)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        RealFs.list(dir)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        RealFs.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        RealFs.remove_dir(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        RealFs.canonicalize(path)
    }
}

/// The sync bridge's ignore pattern, `\.tmp\.\d+\.|\.tmp$|\.crswap$|^~|\.crdownload$`, matched by
/// hand: a name it matches is never replicated to the owner's devices.
#[allow(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the bridge's pattern is case-sensitive"
)]
fn bridge_ignores(name: &str) -> bool {
    let tmp_then_digits_then_dot = name.match_indices(".tmp.").any(|(at, _)| {
        let rest = &name[at + ".tmp.".len()..];
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        digits > 0 && rest[digits..].starts_with('.')
    });
    tmp_then_digits_then_dot
        || name.ends_with(".tmp")
        || name.ends_with(".crswap")
        || name.starts_with('~')
        || name.ends_with(".crdownload")
}

#[test]
fn a_write_lands_through_an_ignored_temp_name_then_fsync_rename_and_dir_fsync() {
    let vault = tempfile::tempdir().expect("a temporary vault");
    let target = vault.path().join("law-evidence.md");
    std::fs::write(&target, "the note before\n").expect("the note before the write");
    let fs = Recording::new(Fail::Nothing);

    atomic::write(&fs, &target, b"the note after\n").expect("the write lands");

    let temp = vault
        .path()
        .join(format!(".law-evidence.md.{}.tmp", std::process::id()));
    assert_eq!(
        fs.steps(),
        vec![
            Step::CreateNew(temp.clone()),
            Step::Write(temp.clone()),
            Step::SyncFile(temp.clone()),
            Step::Rename(temp.clone(), target.clone()),
            Step::SyncDir(vault.path().to_path_buf()),
        ],
        "a temp file in the target's own directory, then its fsync, the rename and the dir fsync"
    );
    let name = temp
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a UTF-8 name");
    assert!(
        bridge_ignores(name),
        "{name} is not ignored by the sync bridge"
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("the target"),
        "the note after\n"
    );
    assert!(!temp.exists(), "the temporary file was left behind");
}

#[test]
fn a_failed_step_leaves_the_target_as_it_was_and_removes_the_temp_file() {
    for fail in [Fail::Write, Fail::SyncFile, Fail::Rename] {
        let vault = tempfile::tempdir().expect("a temporary vault");
        let target = vault.path().join("law-evidence.md");
        std::fs::write(&target, "the note before\n").expect("the note before the write");
        let fs = Recording::new(fail);

        let refused = atomic::write(&fs, &target, b"the note after\n");

        assert!(refused.is_err(), "a failed {fail:?} failed the write");
        assert_eq!(
            std::fs::read_to_string(&target).expect("the target"),
            "the note before\n",
            "a failed {fail:?} left the target as it was"
        );
        let temp = vault
            .path()
            .join(format!(".law-evidence.md.{}.tmp", std::process::id()));
        assert_eq!(
            fs.steps().last(),
            Some(&Step::RemoveFile(temp.clone())),
            "a failed {fail:?} removes the temporary file"
        );
        assert!(!temp.exists(), "a failed {fail:?} left the temporary file");
    }
}
