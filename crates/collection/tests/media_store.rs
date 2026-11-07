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
