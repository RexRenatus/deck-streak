//! A write outside the readings folder, through `..` or a symbolic link, or over an existing note is
//! refused (SPEC-042 A8, R5, R6, R11), and the vault features refuse to start, creating nothing,
//! when the readings folder is missing (A11, R1).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Environment, SettingsError, StudyDay};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::{
    Rails, ReadingsTree, RealFs, StartRefusal, TopicKey, VaultError, VaultSettings,
};

const READINGS: &str = "12-Readings";
const ARCHIVE: &str = "Archive";
const DIGEST: &str = "5d41402abc4b2a76b9719d911017c592aaf1d7f2c3b4e5a69788796a5b4c3d2e";
const BODY: &str = "# Hearsay\n\nAn out-of-court statement offered for its truth.";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Every file and folder under `root`, each file with its bytes, keyed by its relative path.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("a readable folder") {
            let path = entry.expect("a readable entry").path();
            let relative = path
                .strip_prefix(root)
                .expect("under the root")
                .to_path_buf();
            if path.is_symlink() {
                found.insert(relative, Some(b"a symbolic link".to_vec()));
            } else if path.is_dir() {
                pending.push(path);
                found.insert(relative, None);
            } else {
                found.insert(relative, Some(fs::read(&path).expect("a readable file")));
            }
        }
    }
    examined("entries", found.into_iter().collect())
        .into_iter()
        .collect()
}

fn settings(root: &Path) -> VaultSettings {
    VaultSettings::from_env(&Environment::from_vars([
        (VAULT_ROOT, root.as_os_str().to_owned()),
        (READINGS_FOLDER, OsString::from(READINGS)),
        (ARCHIVE_FOLDER, OsString::from(ARCHIVE)),
    ]))
    .expect("the vault's settings")
}

fn rails() -> Rails {
    Rails::vendored().expect("the vendored rails")
}

#[test]
fn a_write_outside_the_readings_folder_or_over_a_note_is_refused() {
    let vault = tempfile::tempdir().expect("a temporary vault");
    let readings = vault.path().join(READINGS);
    fs::create_dir(&readings).expect("the readings folder");
    let outside = tempfile::tempdir().expect("a folder outside the vault");
    fs::write(
        outside.path().join("elsewhere.md"),
        "a file outside the vault\n",
    )
    .expect("a file outside");
    let outside_before = snapshot(outside.path());
    let tree = ReadingsTree::open(&settings(vault.path()), RealFs, rails()).expect("the vault");
    let topic = TopicKey::new("law/evidence").expect("a topic key");

    // Through `..`: a topic key that could climb out of its folder is refused before any path is
    // built, and so is every key that is not a lowercase slug.
    for key in [
        "../escape",
        "law/../../escape",
        "law/./evidence",
        "Law/Evidence",
        "law//evidence",
        "law/evidence/",
        ".hidden",
        "",
    ] {
        assert!(
            matches!(TopicKey::new(key), Err(VaultError::InvalidTopicKey)),
            "the topic key {key:?} was accepted"
        );
    }

    // Through a symbolic link: a day folder that links outside the readings folder.
    let linked = StudyDay::from_epoch_day(20_000);
    symlink(outside.path(), readings.join(linked.to_string())).expect("a planted link");
    let refused = tree
        .create(linked, &topic, DIGEST, BODY)
        .expect_err("a write through a link out of the readings folder");
    assert!(
        matches!(refused, VaultError::OutsideConfinement),
        "refused as {refused:?}"
    );
    // A note that is a link out of the readings folder is never read or written through.
    let noted = StudyDay::from_epoch_day(20_001);
    fs::create_dir(readings.join(noted.to_string())).expect("a day folder");
    symlink(
        outside.path().join("elsewhere.md"),
        readings.join(noted.to_string()).join(topic.file_name()),
    )
    .expect("a planted note link");
    let refused = tree
        .stamp_studied(noted, &topic)
        .expect_err("a stamp through a link");
    assert!(
        matches!(refused, VaultError::NotARegularFile),
        "refused as {refused:?}"
    );
    assert_eq!(
        snapshot(outside.path()),
        outside_before,
        "nothing outside the readings folder changed"
    );

    // Over an existing note: the same name, and the same name in another case.
    let day = StudyDay::from_epoch_day(20_002);
    let created = tree
        .create(day, &topic, DIGEST, BODY)
        .expect("the first note of the day");
    let refused = tree
        .create(day, &topic, DIGEST, "Another body.")
        .expect_err("a second note over the first");
    assert!(
        matches!(refused, VaultError::NoteExists),
        "refused as {refused:?}"
    );
    let owners_day = StudyDay::from_epoch_day(20_003);
    let owners_folder = readings.join(owners_day.to_string());
    fs::create_dir(&owners_folder).expect("a day folder");
    fs::write(owners_folder.join("Law-Evidence.md"), "the owner's note\n").expect("the owner's");
    let refused = tree
        .create(owners_day, &topic, DIGEST, BODY)
        .expect_err("a note over the owner's, in another case");
    assert!(
        matches!(refused, VaultError::NoteExists),
        "refused as {refused:?}"
    );
    assert_eq!(
        fs::read_to_string(vault.path().join(&created.path)).expect("the first note"),
        fs::read_to_string(vault.path().join(tree.note_path(day, &topic))).expect("the same note"),
    );
    assert!(
        fs::read_to_string(vault.path().join(&created.path))
            .expect("the first note")
            .contains(BODY),
        "the first note keeps its body"
    );
    assert_eq!(
        fs::read_to_string(owners_folder.join("Law-Evidence.md")).expect("the owner's note"),
        "the owner's note\n"
    );
    assert_eq!(
        fs::read_dir(&owners_folder).expect("the folder").count(),
        1,
        "no second note was written beside the owner's"
    );
}

#[test]
fn a_missing_readings_folder_refuses_and_creates_nothing() {
    let vault = tempfile::tempdir().expect("a temporary vault");
    fs::write(vault.path().join("Inbox note.md"), "the owner's note\n").expect("a note");
    let before = snapshot(vault.path());

    let refused = ReadingsTree::open(&settings(vault.path()), RealFs, rails()).map(|_| ());

    assert!(
        matches!(
            refused,
            Err(VaultError::Start(StartRefusal::ReadingsFolderMissing))
        ),
        "a missing readings folder started as {refused:?}"
    );
    assert_eq!(
        snapshot(vault.path()),
        before,
        "the start check created nothing"
    );

    // A root that is not a directory refuses too.
    let refused = ReadingsTree::open(
        &settings(&vault.path().join("Inbox note.md")),
        RealFs,
        rails(),
    )
    .map(|_| ());
    assert!(
        matches!(
            refused,
            Err(VaultError::Start(StartRefusal::RootNotADirectory))
        ),
        "a file as the vault root started as {refused:?}"
    );

    // A present readings folder opens, and the start check leaves nothing behind in it.
    fs::create_dir(vault.path().join(READINGS)).expect("the readings folder");
    ReadingsTree::open(&settings(vault.path()), RealFs, rails()).expect("the vault opens");
    assert_eq!(
        fs::read_dir(vault.path().join(READINGS))
            .expect("the readings folder")
            .count(),
        0,
        "the start check's probe file was removed"
    );
}

#[test]
fn the_vault_settings_refuse_by_name_and_never_by_value() {
    let unset = VaultSettings::from_env(&Environment::from_vars([
        (READINGS_FOLDER, READINGS),
        (ARCHIVE_FOLDER, ARCHIVE),
    ]));
    assert_eq!(
        unset,
        Err(SettingsError::Missing {
            setting: VAULT_ROOT
        })
    );
    let relative = VaultSettings::from_env(&Environment::from_vars([
        (VAULT_ROOT, "relative/vault"),
        (READINGS_FOLDER, READINGS),
        (ARCHIVE_FOLDER, ARCHIVE),
    ]));
    assert_eq!(
        relative,
        Err(SettingsError::Malformed {
            setting: VAULT_ROOT,
            expected: "an absolute directory path",
        })
    );
    for folder in ["12-Readings/Archive", "..", ".obsidian", "a\tb"] {
        let refused = VaultSettings::from_env(&Environment::from_vars([
            (VAULT_ROOT, "/vault"),
            (READINGS_FOLDER, folder),
            (ARCHIVE_FOLDER, ARCHIVE),
        ]));
        assert!(
            matches!(
                refused,
                Err(SettingsError::Malformed {
                    setting: READINGS_FOLDER,
                    ..
                })
            ),
            "the folder name {folder:?} was accepted"
        );
    }
    let settings = settings(Path::new("/vault"));
    assert_eq!(
        format!("{:?}", settings.root),
        "VaultRoot(..)",
        "the root's value is private configuration"
    );
}

#[test]
fn a_readings_folder_the_service_cannot_write_refuses_start() {
    let vault = tempfile::tempdir().expect("a temporary vault");
    let readings = vault.path().join(READINGS);
    fs::create_dir(&readings).expect("the readings folder");
    fs::set_permissions(&readings, fs::Permissions::from_mode(0o555)).expect("read-only");
    // The precondition: this user cannot create a file there (a superuser could, and would make
    // this test prove nothing).
    let probe = fs::File::create(readings.join("probe"));
    assert!(
        probe.is_err(),
        "the test runs as a user who can write a read-only folder, so it proves nothing"
    );

    let refused = ReadingsTree::open(&settings(vault.path()), RealFs, rails())
        .expect_err("an unwritable readings folder refuses start");

    fs::set_permissions(&readings, fs::Permissions::from_mode(0o755)).expect("writable again");
    assert!(
        matches!(
            refused,
            VaultError::Start(StartRefusal::ReadingsFolderNotWritable)
        ),
        "refused as {refused:?}"
    );
}
