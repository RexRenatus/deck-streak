//! Archive paths equal the predecessor's, a year-boundary ISO week included (SPEC-042 A6, R8), and a
//! superseded note whose name is taken in its archive folder takes the next free numeric suffix, so
//! nothing is ever overwritten.

// An integration test is test code: its helpers panic on an unreadable file, and the golden reader
// prints the examined count on purpose. clippy.toml's in-test allowances cover `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use deck_streak_kernel::{Environment, StudyDay};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::readings_tree::archive_folder;
use deck_streak_vault::{
    FolderName, Rails, ReadingsTree, RealFs, TopicKey, VaultError, VaultSettings,
};
use tempfile::TempDir;

const READINGS: &str = "12-Readings";
const ARCHIVE: &str = "Archive";
const DIGEST: &str = "5d41402abc4b2a76b9719d911017c592aaf1d7f2c3b4e5a69788796a5b4c3d2e";

/// A temporary vault with an empty readings folder, and the date tree over it.
fn vault() -> (TempDir, ReadingsTree<RealFs>) {
    let dir = tempfile::tempdir().expect("a temporary vault");
    fs::create_dir(dir.path().join(READINGS)).expect("the readings folder");
    let settings = VaultSettings::from_env(&Environment::from_vars([
        (VAULT_ROOT, dir.path().as_os_str().to_owned()),
        (READINGS_FOLDER, OsString::from(READINGS)),
        (ARCHIVE_FOLDER, OsString::from(ARCHIVE)),
    ]))
    .expect("the vault's settings");
    let rails = Rails::vendored().expect("the vendored rails");
    let tree = ReadingsTree::open(&settings, RealFs, rails).expect("the vault opens");
    (dir, tree)
}

#[test]
fn archive_paths_match_the_parity_golden() {
    // The predecessor's folder names, which DeckStreak reads from configuration (R1).
    let readings = FolderName::new(READINGS).expect("a folder name");
    let archive = FolderName::new(ARCHIVE).expect("a folder name");
    let mut december_week_one = 0;
    let mut january_last_week = 0;
    golden::each_case("archive_dir", |case| {
        let day = StudyDay::from_epoch_day(case.input["day"].as_i64().expect("an epoch day"));
        let folders: Vec<&str> = case.output["folders"]
            .as_array()
            .expect("the folders below the root")
            .iter()
            .map(|folder| folder.as_str().expect("a folder name"))
            .collect();
        let named =
            StudyDay::from_epoch_day(case.output["day"].as_i64().expect("the day it names"));
        let mut expected: PathBuf = folders.iter().collect();
        expected.push(named.to_string());
        assert_eq!(
            archive_folder(&readings, &archive, day),
            expected,
            "the archive folder of {}",
            case.input
        );
        match (folders[3], folders[4]) {
            ("12", "W01") => december_week_one += 1,
            ("01", "W52" | "W53") => january_last_week += 1,
            _ => {}
        }
    });
    assert!(
        december_week_one > 0 && january_last_week > 0,
        "the golden holds days whose ISO week belongs to the other calendar year, on both sides: \
         {december_week_one} in December, {january_last_week} in January"
    );
}

/// How long one archive may run before a test calls it a hang. An archive ends in milliseconds, so
/// one still running after this has stopped finding a free name: the test fails then, rather than
/// walking every numeric suffix until the mutation tool's own timeout.
const HANG: Duration = Duration::from_secs(5);

/// `tree`'s archive of `topic`'s note of `day`, from a run on its own thread that must end within
/// [`HANG`]; the tree comes back beside the archive path.
fn archive_within(
    tree: ReadingsTree<RealFs>,
    day: StudyDay,
    topic: &TopicKey,
) -> (ReadingsTree<RealFs>, Result<PathBuf, VaultError>) {
    let topic = topic.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let archived = tree.archive(day, &topic);
        // A send fails only once the test has stopped waiting, having failed on a hang.
        let _ = sender.send((tree, archived));
    });
    match receiver.recv_timeout(HANG) {
        Ok(done) => done,
        Err(RecvTimeoutError::Timeout) => panic!("the archive ran past {HANG:?}"),
        Err(RecvTimeoutError::Disconnected) => panic!("the archive panicked"),
    }
}

#[test]
fn a_taken_archive_name_gets_a_numeric_suffix_and_nothing_is_overwritten() {
    let (dir, mut tree) = vault();
    let day = StudyDay::from_epoch_day(20_000);
    let topic = TopicKey::new("law/evidence").expect("a topic key");
    let readings = FolderName::new(READINGS).expect("a folder name");
    let archive = FolderName::new(ARCHIVE).expect("a folder name");
    let folder = dir.path().join(archive_folder(&readings, &archive, day));
    let bodies = [
        "The first reading.",
        "The second reading.",
        "The third reading.",
        "The fourth reading.",
    ];
    let mut archived = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        if index == 3 {
            // The owner's own note already holds the next name, in another case.
            fs::write(folder.join("LAW-EVIDENCE-4.md"), "the owner's own note\n")
                .expect("the owner's note");
        }
        tree.create(day, &topic, DIGEST, body)
            .expect("a reading of the day");
        let (returned, path) = archive_within(tree, day, &topic);
        tree = returned;
        archived.push(path.expect("the superseded note is archived"));
    }

    let names: Vec<String> = archived
        .iter()
        .map(|path| {
            path.file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        names,
        [
            "law-evidence.md",
            "law-evidence-2.md",
            "law-evidence-3.md",
            "law-evidence-5.md",
        ]
    );
    for (path, body) in archived.iter().zip(bodies) {
        let text = fs::read_to_string(dir.path().join(path)).expect("an archived note");
        assert!(text.contains(body), "{} holds {body}", path.display());
    }
    assert_eq!(
        fs::read_to_string(folder.join("LAW-EVIDENCE-4.md")).expect("the owner's note"),
        "the owner's own note\n",
        "the owner's note is untouched"
    );
}
