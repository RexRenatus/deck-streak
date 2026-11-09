//! SPEC-601 A1 to A5 and A8: the media store over the collection store and the user's blob folder.
//! A store at head reads user version 2; a file placed without its row is deleted at the next
//! open; a claimed hash with other bytes is refused; `get` caps at its limit; a size mismatch
//! quarantines the file and reports it wanted; an unbound or tombstoned name reads nothing; a file
//! lies at its blob path; and the `cfg(kani)` harness file is held as text.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_collection::media_store::FILE_CAP;
use deck_streak_collection::{BlobError, BlobStore, MediaError, MediaStore, Store};
use deck_streak_media::MediaHash;

use support::{SERVER, raw};

const NOW_MS: i64 = 1_700_000_000_000;
const LIMIT: u64 = 1 << 20;
const BYTES: &[u8] = b"the bytes of one media file";

/// A temporary folder holding a collection store and a blob folder.
struct Folder {
    _dir: tempfile::TempDir,
    db: PathBuf,
    root: PathBuf,
}

fn folder() -> Folder {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let db = dir.path().join("collection.db");
    let root = dir.path().join("media");
    Folder {
        _dir: dir,
        db,
        root,
    }
}

/// The media store over the collection store at `db` and the blob folder at `root`.
fn open(db: &Path, root: &Path) -> MediaStore {
    let store = Store::open(db, SERVER).expect("the collection store opens");
    MediaStore::open(store, root).expect("the media store opens")
}

/// How many `media_file` rows name `hash`.
fn rows(db: &Path, hash: &MediaHash) -> i64 {
    raw(db)
        .query_row(
            "SELECT count(*) FROM media_file WHERE hash = ?1",
            [hash.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .expect("the rows are counted")
}

/// The state of the `media_file` row of `hash`.
fn state(db: &Path, hash: &MediaHash) -> i64 {
    raw(db)
        .query_row(
            "SELECT state FROM media_file WHERE hash = ?1",
            [hash.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .expect("the row is read")
}

/// The entries of the staging folder, none when it is absent.
fn staged(root: &Path) -> usize {
    fs::read_dir(root.join("tmp")).map_or(0, Iterator::count)
}

#[test]
fn a_store_at_head_reads_user_version_two() {
    let folder = folder();
    drop(open(&folder.db, &folder.root));
    let version: i64 = raw(&folder.db)
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("the version is read");
    assert_eq!(version, 2);
}

#[test]
fn a_file_without_its_row_is_deleted_at_the_next_open() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let kept = media
        .put("kept.png", BYTES, NOW_MS)
        .expect("the file is put");
    let kept_path = media.blobs().path_of(&kept);
    assert!(kept_path.exists(), "a put places its file");
    // The crash: the file is placed and its row's transaction never commits.
    let orphan = media
        .blobs()
        .put(b"a file whose row never committed", FILE_CAP)
        .expect("the file is placed");
    let orphan_path = media.blobs().path_of(&orphan);
    assert!(orphan_path.exists(), "the file is placed before its row");
    fs::write(folder.root.join("tmp").join("staged"), b"half a put").expect("a staged file");
    drop(media);
    let media = open(&folder.db, &folder.root);
    assert!(!orphan_path.exists(), "the next open deletes the file");
    assert_eq!(rows(&folder.db, &orphan), 0, "no row was adopted");
    assert_eq!(
        staged(&folder.root),
        0,
        "the open empties the staging folder"
    );
    assert!(kept_path.exists(), "a file with its present row is kept");
    assert_eq!(media.reader().read("kept.png", LIMIT), Some(BYTES.to_vec()));
}

#[test]
fn a_claimed_hash_with_other_bytes_is_refused() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let claimed = MediaHash::of(b"the bytes the peer named");
    let other = b"other bytes than the peer named";
    let outcome = media.put_claimed(&claimed, other, NOW_MS);
    assert!(
        matches!(outcome, Err(MediaError::Blob(BlobError::Mismatch))),
        "{outcome:?}"
    );
    assert!(!media.blobs().path_of(&claimed).exists());
    assert!(!media.blobs().path_of(&MediaHash::of(other)).exists());
    assert_eq!(staged(&folder.root), 0);
    assert_eq!(rows(&folder.db, &claimed), 0);
    let matching = b"the bytes the peer named";
    media
        .put_claimed(&claimed, matching, NOW_MS)
        .expect("matching bytes are admitted");
    assert_eq!(rows(&folder.db, &claimed), 1);
}

#[test]
fn get_returns_at_most_the_limit() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let hash = media
        .put("clip.mp3", BYTES, NOW_MS)
        .expect("the file is put");
    assert_eq!(media.blobs().get(&hash, 4), Some(BYTES[..4].to_vec()));
    assert_eq!(media.blobs().get(&hash, u64::MAX), Some(BYTES.to_vec()));
    assert_eq!(media.blobs().get(&MediaHash::of(b"never put"), 4), None);
}

#[test]
fn a_size_mismatch_quarantines_the_file_and_reports_it_wanted() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let hash = media
        .put("picture.png", BYTES, NOW_MS)
        .expect("the file is put");
    let placed = media.blobs().path_of(&hash);
    assert!(placed.exists(), "a put places its file");
    fs::write(&placed, b"longer bytes than the row's size says").expect("the file is planted");
    assert_eq!(media.reader().read("picture.png", LIMIT), None);
    assert!(!placed.exists(), "the file leaves its hash's path");
    assert!(media.blobs().quarantined_path_of(&hash).exists());
    assert_eq!(media.wanted(), vec![hash]);
    assert_eq!(state(&folder.db, &hash), 1);
}

#[test]
fn an_unbound_or_tombstoned_name_reads_nothing() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    media
        .put("live.png", b"a live file", NOW_MS)
        .expect("the file is put");
    assert_eq!(
        media.reader().read("live.png", LIMIT),
        Some(b"a live file".to_vec())
    );
    media
        .put("unbound.png", b"an unbound file", NOW_MS)
        .expect("the file is put");
    raw(&folder.db)
        .execute(
            "UPDATE media_name SET unbound = 1 WHERE name = 'unbound.png'",
            [],
        )
        .expect("the name is unbound");
    assert_eq!(media.reader().read("unbound.png", LIMIT), None);
    let tombstoned = media
        .put("tombstoned.png", b"a tombstoned file", NOW_MS)
        .expect("the file is put");
    raw(&folder.db)
        .execute(
            "UPDATE media_file SET state = 2 WHERE hash = ?1",
            [tombstoned.as_bytes().as_slice()],
        )
        .expect("the file is tombstoned");
    assert_eq!(media.reader().read("tombstoned.png", LIMIT), None);
    assert_eq!(media.reader().read("never-bound.png", LIMIT), None);
    assert_eq!(media.wanted(), vec![tombstoned]);
}

#[test]
fn a_file_lies_at_its_blob_path() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let hash = media
        .put("layout.png", BYTES, NOW_MS)
        .expect("the file is put");
    let blob_path = hash.blob_path();
    let expected = folder
        .root
        .join("sha256")
        .join(std::str::from_utf8(&blob_path).expect("a blob path is ASCII"));
    assert_eq!(fs::read(&expected).ok(), Some(BYTES.to_vec()));
    assert_eq!(media.blobs().path_of(&hash), expected);
}

/// Pins `FILE_CAP`, `100 * 1024 * 1024`: 100 MiB, the cap `put` gives the blob folder, so a file
/// of two MiB is taken.
#[test]
fn the_store_takes_a_file_up_to_one_hundred_mebibytes() {
    assert_eq!(FILE_CAP, 104_857_600, "100 MiB");
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let large = vec![0x5a_u8; 2 * 1024 * 1024];
    let hash = media
        .put("large.bin", &large, NOW_MS)
        .expect("a file of two MiB is taken");
    assert_eq!(hash, MediaHash::of(&large));
    assert_eq!(
        media
            .reader()
            .read("large.bin", u64::MAX)
            .map(|read| read.len()),
        Some(large.len())
    );
}

/// The refusal of a put of `bytes` named `name`, which leaves no file and no row.
fn refused_name(media: &MediaStore, db: &Path, name: &str, bytes: &[u8]) -> Option<MediaError> {
    let refused = media.put(name, bytes, NOW_MS).err();
    let hash = MediaHash::of(bytes);
    assert!(
        !media.blobs().path_of(&hash).exists(),
        "{name:?} placed its file"
    );
    assert_eq!(rows(db, &hash), 0, "{name:?} committed its row");
    refused
}

/// Pins `put`'s name rule, `name.is_empty() || name.len() > NAME_MAX`: an empty name and a name
/// of 256 bytes are refused before any file is placed, and a name of 255 bytes is taken.
#[test]
fn a_name_is_one_to_two_hundred_fifty_five_bytes() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let empty = refused_name(&media, &folder.db, "", b"the bytes of an empty name");
    assert!(matches!(empty, Some(MediaError::Name)), "{empty:?}");
    let long = "n".repeat(256);
    let refused = refused_name(&media, &folder.db, &long, b"the bytes of a long name");
    assert!(matches!(refused, Some(MediaError::Name)), "{refused:?}");
    let longest = "n".repeat(255);
    let hash = media
        .put(&longest, b"the bytes of the longest name", NOW_MS)
        .expect("a name of 255 bytes is taken");
    assert_eq!(rows(&folder.db, &hash), 1);
    assert_eq!(
        media.reader().read(&longest, LIMIT),
        Some(b"the bytes of the longest name".to_vec())
    );
}

/// Pins `entries`' guard, `error.kind() == std::io::ErrorKind::NotFound`: only a missing folder
/// reads as no entries, so a staging path that is a file refuses the open.
#[test]
fn a_staging_path_that_is_a_file_refuses_the_open() {
    let folder = folder();
    fs::create_dir_all(&folder.root).expect("the blob folder");
    fs::write(folder.root.join("tmp"), b"not a folder").expect("a file at the staging path");
    let store = Store::open(&folder.db, SERVER).expect("the collection store opens");
    let refused = MediaStore::open(store, &folder.root).err();
    match refused {
        Some(MediaError::Blob(BlobError::Io(error)))
            if error.kind() == std::io::ErrorKind::NotADirectory => {}
        other => panic!("an open over a staging file answered {other:?}"),
    }
}

/// Pins `put_flow`'s cap, `bytes.len() > cap`: bytes as long as the cap are placed, and one byte
/// over it is refused with nothing placed and nothing left staged.
#[test]
fn a_put_takes_bytes_up_to_its_cap() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let length = u64::try_from(BYTES.len()).expect("a short length");
    let hash = media
        .blobs()
        .put(BYTES, length)
        .expect("bytes as long as the cap are placed");
    assert_eq!(hash, MediaHash::of(BYTES));
    assert_eq!(media.blobs().get(&hash, u64::MAX), Some(BYTES.to_vec()));
    let over = b"one byte longer than its cap";
    let cap = u64::try_from(over.len() - 1).expect("a short length");
    let refused = media.blobs().put(over, cap);
    assert!(matches!(refused, Err(BlobError::TooLarge)), "{refused:?}");
    assert!(!media.blobs().path_of(&MediaHash::of(over)).exists());
    assert_eq!(
        staged(&folder.root),
        0,
        "the refused put left its staging file"
    );
}

/// How many staging names a test plants, past every name this test binary's puts reach.
const PLANTED: usize = 4096;

/// Pins `create`'s guard, `error.kind() == ErrorKind::AlreadyExists`: a staging name another file
/// holds is passed over for the next one, and the put gives up with its own `AlreadyExists` only
/// after every try, not with the operating system's refusal of the first name.
#[test]
fn a_held_staging_name_is_passed_over_until_the_tries_run_out() {
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let staging = folder.root.join("tmp");
    fs::create_dir_all(&staging).expect("the staging folder");
    for number in 0..PLANTED {
        fs::write(staging.join(format!("put-{number}")), b"").expect("a planted staging name");
    }
    let refused = media.blobs().put(BYTES, LIMIT);
    match refused {
        Err(BlobError::Io(error))
            if error.kind() == std::io::ErrorKind::AlreadyExists
                && error.raw_os_error().is_none() => {}
        other => panic!("a put over a staging folder whose names are all held answered {other:?}"),
    }
    assert_eq!(
        staged(&folder.root),
        PLANTED,
        "only the planted names are staged"
    );
    assert!(!media.blobs().path_of(&MediaHash::of(BYTES)).exists());
}

/// Pins `create`'s guard the other way: a staging folder that refuses the file is refused at the
/// first name, with the operating system's own refusal.
#[test]
#[cfg(unix)]
fn a_staging_folder_that_refuses_the_file_refuses_the_put() {
    use std::os::unix::fs::PermissionsExt;
    let folder = folder();
    let media = open(&folder.db, &folder.root);
    let staging = folder.root.join("tmp");
    fs::create_dir_all(&staging).expect("the staging folder");
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o555)).expect("read-only");
    let admitted = fs::write(staging.join("probe"), b"").is_ok();
    let refused = media.blobs().put(BYTES, LIMIT);
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).expect("writable again");
    assert!(
        !admitted,
        "the read-only staging folder admitted a write, so no put in it can be refused"
    );
    match refused {
        Err(BlobError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied => {}
        other => panic!("a put into a read-only staging folder answered {other:?}"),
    }
    assert!(!media.blobs().path_of(&MediaHash::of(BYTES)).exists());
}

/// The harness file, as the Kani verifier reads it.
const HARNESSES: &str = include_str!("../src/kani_proofs.rs");

/// Each harness function of the file, from its `fn` line to its closing brace, as written.
const WRITTEN: [(&str, &str); 1] = [(
    "k_put",
    r"fn k_put() {
    let bytes: [u8; 2] = kani::any();
    let cap: u64 = kani::any();
    let hash = MediaHash::from_bytes(kani::any());
    let claimed = if kani::any() {
        Some(MediaHash::from_bytes(kani::any()))
    } else {
        None
    };
    let mut staging = Steps::new(kani::any(), kani::any(), kani::any());
    let outcome = put_flow(&mut staging, |_| hash, &bytes, cap, claimed.as_ref());
    match outcome {
        Ok(placed) => {
            assert_eq!(placed, hash);
            assert!(cap >= 2);
            assert!(claimed.is_none_or(|claimed| claimed == hash));
            assert!(staging.synced);
            assert_eq!(staging.placed, Some(hash));
        }
        Err(_) => assert_eq!(staging.placed, None),
    }
}",
)];

/// The function `name` of `text`: from its `fn` line to the first line after it that is `}`.
fn function<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let start = text.find(&format!("\nfn {name}("))? + 1;
    let length = text[start..].find("\n}\n")? + 2;
    Some(&text[start..start + length])
}

#[test]
fn each_kani_harness_reads_as_it_was_written() {
    for (name, written) in WRITTEN {
        assert_eq!(function(HARNESSES, name), Some(written), "{name}");
    }
    let planted = HARNESSES.replacen("assert!(cap >= 2);", "assert!(cap > 2);", 1);
    assert_ne!(
        function(&planted, "k_put"),
        function(HARNESSES, "k_put"),
        "a planted change to a harness reads as a change"
    );
    println!(
        "examined {} of {} harness function(s)",
        WRITTEN.len(),
        WRITTEN.len()
    );
}

/// `refused` and the `Staging` impl of `Steps` in the harness file, line by line as written: the
/// refusal a failing step answers; each step's guard, `stage` over its own flag and `sync` and
/// `place` over the flag of the step before and their own, each refusal through `refused`; what
/// each step records; and the discard that forgets the staged file. No test build compiles them.
const STEPS: [&str; 33] = [
    "fn refused() -> BlobError {",
    "    BlobError::Io(std::io::Error::from(std::io::ErrorKind::Other))",
    "}",
    "",
    "impl Staging for Steps {",
    "    fn stage(&mut self, _bytes: &[u8]) -> Result<(), BlobError> {",
    "        if !self.stages {",
    "            return Err(refused());",
    "        }",
    "        self.staged = true;",
    "        Ok(())",
    "    }",
    "",
    "    fn sync(&mut self) -> Result<(), BlobError> {",
    "        if !self.staged || !self.syncs {",
    "            return Err(refused());",
    "        }",
    "        self.synced = true;",
    "        Ok(())",
    "    }",
    "",
    "    fn place(&mut self, hash: &MediaHash) -> Result<(), BlobError> {",
    "        if !self.synced || !self.places {",
    "            return Err(refused());",
    "        }",
    "        self.placed = Some(*hash);",
    "        Ok(())",
    "    }",
    "",
    "    fn discard(&mut self) {",
    "        self.staged = false;",
    "    }",
    "}",
];

/// The blob store's source, whose staging sync no test process can observe.
const BLOB: &str = include_str!("../src/blob.rs");

/// `FsStaging::sync` line by line as written: it syncs the staged file to its disk before the
/// rename, which only a crash between the two would show, so the hold reads the call itself.
const SYNC: [&str; 4] = [
    "    fn sync(&mut self) -> Result<(), BlobError> {",
    "        let (_, file) = self.staged()?;",
    "        file.sync_all().map_err(BlobError::Io)",
    "    }",
];

/// The line numbers at which `lines` start in `text` as consecutive whole lines.
fn starts_of(text: &str, lines: &[&str]) -> Vec<usize> {
    let read: Vec<&str> = text.lines().collect();
    read.windows(lines.len())
        .enumerate()
        .filter(|(_, window)| *window == lines)
        .map(|(index, _)| index + 1)
        .collect()
}

#[test]
fn the_steps_staging_and_the_staging_sync_read_line_by_line_as_written() {
    assert_eq!(
        starts_of(HARNESSES, &STEPS),
        vec![38],
        "refused and the Steps staging, once, from line 38"
    );
    assert_eq!(
        starts_of(BLOB, &SYNC),
        vec![229],
        "the staging sync, once, from line 229"
    );
    let planted = HARNESSES.replacen(
        "if !self.staged || !self.syncs {",
        "if !self.staged && !self.syncs {",
        1,
    );
    assert_eq!(
        starts_of(&planted, &STEPS),
        Vec::<usize>::new(),
        "a planted change to a held line reads as a change"
    );
    let planted = BLOB.replacen("file.sync_all().map_err(BlobError::Io)", "Ok(())", 1);
    assert_eq!(
        starts_of(&planted, &SYNC),
        Vec::<usize>::new(),
        "a planted change to the sync reads as a change"
    );
    println!(
        "examined {} held line(s) of the harness file and {} of the blob store",
        STEPS.len(),
        SYNC.len()
    );
}
