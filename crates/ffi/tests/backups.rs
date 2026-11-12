//! The device's backups through the adapter (SPEC-516 R5, B5): back up now writes the open
//! collection into a new backup in the collection's folder through the core; the list names each
//! backup and server copy in that folder by its id, its kind and its age, newest first, after the
//! core's retention kept three of each kind; and only a listed backup's bytes are exported.
//!
//! Every backup the list reads is either made through the adapter or a file this test writes by a
//! name of the adapter's form, with a stamp it chose. The ids a backup holds are read back through
//! a fresh engine of the test's own, never through the adapter it judges.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[expect(
    dead_code,
    reason = "the backups tests open the synthetic collection by its path and read its ids only"
)]
mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use deck_streak_ffi::backups::{BackupRefusal, DeviceBackup, DeviceBackupKind};
use deck_streak_ffi::engine::Engine;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// A second, in nanoseconds: the unit the names' stamps count in.
const SECOND: u128 = 1_000_000_000;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn engine() -> Arc<Engine> {
    Engine::new(Vec::new()).expect("the engine starts from the default init message")
}

fn text(path: &Path) -> String {
    path.to_str().expect("a scratch path is UTF-8").to_owned()
}

/// The clock now, in nanoseconds since the epoch, as the adapter's names count it.
fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos()
}

/// A folder of the test's own, empty.
fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-backups")
        .join(format!("{test}-{}-{}", std::process::id(), now()));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

/// The card and note ids a fresh engine of the test's own reads from the collection file `path`.
fn held(path: &Path) -> [BTreeSet<i64>; 2] {
    let col = CollectionBuilder::new(path)
        .build()
        .expect("the test's engine opens the file");
    let db = col.storage.db();
    let ids = |sql: &str| -> BTreeSet<i64> {
        let mut statement = db.prepare(sql).expect("the test's read prepares");
        statement
            .query_map([], |row| row.get(0))
            .expect("the test's read runs")
            .map(|id| id.expect("an id reads"))
            .collect()
    };
    let sets = [ids("select id from cards"), ids("select id from notes")];
    col.close(None).expect("the test's engine closes the file");
    sets
}

/// The names `folder` holds, sorted.
fn names(folder: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(folder)
        .expect("the folder reads")
        .map(|entry| {
            entry
                .expect("an entry reads")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    found.sort();
    found
}

/// A backup file of the adapter's form, `{kind}-{stamp}-{process}.anki2`, written by the test with
/// `bytes`, and its id.
fn planted(folder: &Path, kind: &str, stamp: u128, bytes: &[u8]) -> String {
    let id = format!("{kind}-{stamp}-7");
    std::fs::write(folder.join(format!("{id}.anki2")), bytes).expect("the test writes a backup");
    id
}

/// A listing's ids and kinds, newest first, without the ages.
fn shown(listed: &[DeviceBackup]) -> Vec<(String, DeviceBackupKind)> {
    listed
        .iter()
        .map(|backup| (backup.id.clone(), backup.kind))
        .collect()
}

#[test]
fn back_up_now_makes_a_backup_of_the_open_collection_in_its_folder() {
    let synthetic = support::synthetic("backups-back-up-now");
    let device = held(&synthetic.collection);
    let engine = engine();
    engine
        .run(
            OPEN_COLLECTION.0,
            OPEN_COLLECTION.1,
            support::open_request(&synthetic),
        )
        .expect("the adapter opens the collection");
    let folder = text(&synthetic.dir);
    let before = names(&synthetic.dir);

    let id = engine
        .back_up_now(folder.clone())
        .expect("back up now makes a backup");

    let file = synthetic.dir.join(format!("{id}.anki2"));
    assert!(
        file.is_file(),
        "the backup is a file in the collection's folder: {id}"
    );
    assert!(
        id.starts_with("backup-") && !before.contains(&format!("{id}.anki2")),
        "the backup is a new file of the backup kind: {id}"
    );
    let [cards, notes] = &device;
    examined("card(s) of the device", cards.iter().collect());
    examined("note(s) of the device", notes.iter().collect());
    let listed = engine
        .device_backups(folder)
        .expect("the folder's backups are listed");
    assert_eq!(shown(&listed), vec![(id, DeviceBackupKind::Backup)]);
    drop(engine);
    assert_eq!(
        held(&file),
        device,
        "the backup, opened by a fresh engine, holds every card and note of the device"
    );
}

#[test]
fn back_up_now_without_an_open_collection_makes_nothing() {
    let dir = folder("back-up-now-unopened");
    let refused = engine().back_up_now(text(&dir));
    assert!(
        matches!(refused, Err(BackupRefusal::Engine { .. })),
        "{refused:?}"
    );
    assert_eq!(names(&dir), Vec::<String>::new(), "no file was left");
}

#[test]
fn the_backups_are_listed_newest_first_and_three_of_each_kind_are_kept() {
    let dir = folder("listed-and-kept");
    let at = now();
    // five backups and four server copies, a minute apart, oldest first: the backups 600 to 360
    // seconds old, the copies 590 to 410
    let backups: Vec<String> = (0..5)
        .map(|n| planted(&dir, "backup", at - (600 - n * 60) * SECOND, b"b"))
        .collect();
    let copies: Vec<String> = (0..4)
        .map(|n| planted(&dir, "server-copy", at - (590 - n * 60) * SECOND, b"s"))
        .collect();
    // files the list never names: the collection, a journal, another form, another stamp form
    let journal = format!("{}.anki2-journal", backups[4]);
    let foreign = [
        "collection.anki2",
        "collection.anki2-wal",
        journal.as_str(),
        "backup-12.anki2",
        "backup-0012-7.anki2",
        "backup-x-7.anki2",
        "server-1-7.anki2.tmp",
        "notes.txt",
    ];
    for name in foreign {
        std::fs::write(dir.join(name), b"f").expect("the test writes a foreign file");
    }
    examined("backup(s) planted", backups.clone());
    examined("server copy(ies) planted", copies.clone());

    let listed = engine()
        .device_backups(text(&dir))
        .expect("the folder's backups are listed");

    assert_eq!(
        shown(&listed),
        vec![
            (backups[4].clone(), DeviceBackupKind::Backup),
            (copies[3].clone(), DeviceBackupKind::ServerCopy),
            (backups[3].clone(), DeviceBackupKind::Backup),
            (copies[2].clone(), DeviceBackupKind::ServerCopy),
            (backups[2].clone(), DeviceBackupKind::Backup),
            (copies[1].clone(), DeviceBackupKind::ServerCopy),
        ],
        "newest first, three of each kind"
    );
    // each age is the whole seconds since its stamp, read a moment after the stamps were taken
    let ages: Vec<u64> = listed.iter().map(|backup| backup.age_seconds).collect();
    let expected = [360_u64, 410, 420, 470, 480, 530];
    assert_eq!(ages.len(), expected.len(), "{ages:?}");
    for (age, least) in ages.iter().zip(expected) {
        assert!(
            (least..least + 60).contains(age),
            "an age of {age} seconds, not {least}: {ages:?}"
        );
    }
    // the removed files are gone, and every foreign file is kept
    let mut kept: Vec<String> = foreign.iter().map(|name| (*name).to_owned()).collect();
    kept.extend(listed.iter().map(|backup| format!("{}.anki2", backup.id)));
    kept.sort();
    assert_eq!(names(&dir), kept);
}

#[test]
fn only_a_listed_backup_is_exported() {
    let dir = folder("only-listed");
    let at = now();
    let backup = planted(&dir, "backup", at - 30 * SECOND, b"the backup's bytes");
    let copy = planted(&dir, "server-copy", at - 20 * SECOND, b"the copy's bytes");
    std::fs::write(dir.join("collection.anki2"), b"the collection").expect("a collection file");
    let engine = engine();
    let folder = text(&dir);
    assert_eq!(
        engine.backup_bytes(folder.clone(), backup.clone()),
        Ok(b"the backup's bytes".to_vec())
    );
    assert_eq!(
        engine.backup_bytes(folder.clone(), copy),
        Ok(b"the copy's bytes".to_vec())
    );
    let unlisted = examined(
        "id(s) that name no listed backup",
        vec![
            "collection".to_owned(),
            "../collection".to_owned(),
            format!("{backup}.anki2"),
            format!(
                "../{}/{backup}",
                dir.file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
            ),
            format!("backup-{}-7", at - 40 * SECOND),
            "backup-0030-7".to_owned(),
            String::new(),
        ],
    );
    for id in unlisted {
        assert_eq!(
            engine.backup_bytes(folder.clone(), id.clone()),
            Err(BackupRefusal::NotListed),
            "{id:?} names no listed backup"
        );
    }
}
