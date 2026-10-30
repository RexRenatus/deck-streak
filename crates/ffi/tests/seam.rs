//! The test seam (SPEC-348 R7, A10).
//!
//! UI tests launch the app with `-DSCollectionDirectory <path>` to open a seeded collection. The
//! adapter alone reads the argument: the default with no argument, the value when it is an
//! absolute path to an existing directory, and every other value refused by name, each by its own
//! test. Paths are made under the target's scratch space, or are the test's own crate directory.

#![allow(clippy::expect_used, reason = "a failed fixture should fail its test")]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_ffi::engine::{CollectionDirectoryRefusal, collection_directory};

const DEFAULT: &str = "/default/collection";
const ARGUMENT: &str = "-DSCollectionDirectory";

/// A fresh directory of this test's own.
fn scratch(test: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-seam")
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn text(path: &Path) -> String {
    path.to_str().expect("a scratch path is UTF-8").to_owned()
}

fn arguments(values: &[&str]) -> Vec<String> {
    let mut arguments = vec!["DeckStreak".to_owned()];
    arguments.extend(values.iter().map(|value| (*value).to_owned()));
    arguments
}

#[test]
fn no_argument_gives_the_default() {
    assert_eq!(
        collection_directory(DEFAULT.to_owned(), arguments(&["-AppleLanguages", "(en)"])),
        Ok(DEFAULT.to_owned())
    );
}

#[test]
fn an_absolute_existing_directory_is_returned() {
    let dir = text(&scratch("existing"));
    assert_eq!(
        collection_directory(DEFAULT.to_owned(), arguments(&[ARGUMENT, &dir])),
        Ok(dir)
    );
}

#[test]
fn a_flag_without_a_value_is_refused() {
    assert_eq!(
        collection_directory(DEFAULT.to_owned(), arguments(&[ARGUMENT])),
        Err(CollectionDirectoryRefusal::NoValue)
    );
}

#[test]
fn a_relative_directory_is_refused() {
    // `tests` exists under the crate directory a test runs in, so only its being relative refuses
    // it.
    assert!(Path::new("tests").is_dir(), "the relative directory exists");
    assert_eq!(
        collection_directory(DEFAULT.to_owned(), arguments(&[ARGUMENT, "tests"])),
        Err(CollectionDirectoryRefusal::NotAbsolute)
    );
}

#[test]
fn a_missing_directory_is_refused() {
    let missing = text(&scratch("missing").join("absent"));
    assert_eq!(
        collection_directory(DEFAULT.to_owned(), arguments(&[ARGUMENT, &missing])),
        Err(CollectionDirectoryRefusal::Missing)
    );
}

#[test]
fn a_file_is_refused_as_not_a_directory() {
    let file = scratch("file").join("collection.anki2");
    std::fs::write(&file, b"").expect("a file is written");
    assert_eq!(
        collection_directory(DEFAULT.to_owned(), arguments(&[ARGUMENT, &text(&file)])),
        Err(CollectionDirectoryRefusal::NotADirectory)
    );
}
