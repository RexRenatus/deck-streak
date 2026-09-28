//! The real file system keeps the port's contract (SPEC-042 R2): a folder sync and a lookup report
//! the operating system's failure, a lookup reads a missing entry as nothing, and a folder is
//! removed only when it is empty.

use std::fs;
use std::io;

use deck_streak_vault::{EntryKind, RealFs, VaultFs};

#[test]
fn syncing_a_missing_folder_reports_it() {
    let dir = tempfile::tempdir().expect("a temporary folder");

    RealFs.sync_dir(dir.path()).expect("a present folder syncs");
    let missing = RealFs
        .sync_dir(&dir.path().join("absent"))
        .expect_err("a missing folder cannot be synced");

    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
}

#[test]
fn a_lookup_reads_a_missing_entry_as_nothing_and_a_path_through_a_file_as_a_failure() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let note = dir.path().join("note.md");
    fs::write(&note, "a note\n").expect("a note");

    assert_eq!(RealFs.kind(&note).expect("a lookup"), Some(EntryKind::File));
    assert_eq!(
        RealFs.kind(&dir.path().join("absent")).expect("a lookup"),
        None
    );
    let through = RealFs
        .kind(&note.join("child"))
        .expect_err("a path through a file is a failure, not a missing entry");
    assert_eq!(through.kind(), io::ErrorKind::NotADirectory);
}

#[test]
fn a_folder_is_removed_only_when_it_is_empty() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let empty = dir.path().join("empty");
    let full = dir.path().join("full");
    fs::create_dir(&empty).expect("an empty folder");
    fs::create_dir(&full).expect("a folder");
    fs::write(full.join("note.md"), "a note\n").expect("a note");

    RealFs
        .remove_dir(&empty)
        .expect("an empty folder is removed");
    let refused = RealFs
        .remove_dir(&full)
        .expect_err("a folder that holds a note is kept");

    assert!(!empty.exists(), "the empty folder is gone");
    assert_eq!(refused.kind(), io::ErrorKind::DirectoryNotEmpty);
    assert!(full.join("note.md").exists(), "the note is kept");
}
