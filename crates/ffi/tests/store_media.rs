//! SPEC-601 A6: the iOS face reader over the media store, built switched off. A put file reads
//! back by name; an unbound name reads nothing; a name whose file is quarantined is reported
//! wanted; and the reader inlines fonts and shows video, as `MediaFolder` does.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

use deck_streak_collection::{MediaStore, Origin, Role, Store};
use deck_streak_engine_core::media::Reader;
use deck_streak_ffi::store_media::StoreMedia;

const NOW_MS: i64 = 1_700_000_000_000;
const LIMIT: u64 = 1 << 20;
const BYTES: &[u8] = b"a picture's bytes";

/// A fresh folder for `test` under the build's own temporary folder.
fn folder(test: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("store_media-{test}"));
    if folder.exists() {
        std::fs::remove_dir_all(&folder).expect("the old folder goes");
    }
    std::fs::create_dir_all(&folder).expect("the folder is made");
    folder
}

/// A reader over a new media store in `folder`.
fn reader(folder: &Path) -> StoreMedia {
    let role = Role::Server {
        origin: Origin::new([7; 16]),
    };
    let store = Store::open(&folder.join("collection.db"), role).expect("the store opens");
    StoreMedia::new(MediaStore::open(store, &folder.join("media")).expect("the media store opens"))
}

#[test]
fn a_put_file_reads_back_by_name() {
    let reader = reader(&folder("put"));
    reader
        .media()
        .put("picture.png", BYTES, NOW_MS)
        .expect("the file is put");
    assert_eq!(reader.read("picture.png", LIMIT), Some(BYTES.to_vec()));
    assert_eq!(reader.read("picture.png", 4), Some(BYTES[..4].to_vec()));
}

#[test]
fn an_unbound_name_reads_nothing() {
    let reader = reader(&folder("unbound"));
    reader
        .media()
        .put("picture.png", BYTES, NOW_MS)
        .expect("the file is put");
    assert_eq!(reader.read("picture.png", LIMIT), Some(BYTES.to_vec()));
    assert_eq!(reader.read("never-bound.png", LIMIT), None);
}

#[test]
fn a_quarantined_name_is_wanted() {
    let reader = reader(&folder("wanted"));
    let hash = reader
        .media()
        .put("picture.png", BYTES, NOW_MS)
        .expect("the file is put");
    let placed = reader.media().blobs().path_of(&hash);
    assert!(
        placed.exists(),
        "the put places the file at its hash's path"
    );
    std::fs::write(&placed, b"other, longer bytes than the row says").expect("the file is planted");
    assert_eq!(reader.read("picture.png", LIMIT), None);
    assert_eq!(reader.media().wanted(), vec![hash]);
    assert_eq!(reader.read("picture.png", LIMIT), None);
}

#[test]
fn the_reader_inlines_fonts_and_shows_video() {
    let reader = reader(&folder("flags"));
    assert!(reader.inlines_fonts());
    assert!(reader.shows_video());
}

/// The crate root, as the compiler reads it.
const CRATE_ROOT: &str = include_str!("../src/lib.rs");

/// A source-text guard: the crate root declares `store_media` among the adapter's modules, as
/// written. The row of SPEC-601 A12 on that line mutates it, a change no behaviour test can see.
#[test]
fn the_crate_root_declares_the_store_media_module_as_written() {
    assert!(
        CRATE_ROOT.contains("pub mod review_undo;\npub mod store_media;\npub mod voices;\n"),
        "the crate root declares store_media between review_undo and voices, with no attribute"
    );
}
