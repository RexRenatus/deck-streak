//! Support for the engine core's integration tests (SPEC-345): the examined count every
//! enumerating test reports, a synthetic collection the engine itself builds, and the engine's own
//! sync server on a loopback port (SPEC-364).

#![allow(
    dead_code,
    reason = "each test target includes this module and calls the part it needs"
)]

pub mod sync_server;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use anki::decks::DeckId;
use anki_proto::collection::OpenCollectionRequest;
use prost::Message;

/// Prints how many `what` a test examined and refuses none: a population that came back empty
/// judged nothing, and every assertion over it would pass.
pub fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The workspace's root directory, two levels above this crate.
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels below the workspace root")
        .to_path_buf()
}

/// A directory of its own for one test under the target's scratch space, made empty.
pub fn scratch(area: &str, test: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(area)
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A collection built for one test, closed, and ready for a dispatcher to open.
pub struct Synthetic {
    /// The test's own directory.
    pub dir: PathBuf,
    /// The collection file.
    pub collection: PathBuf,
    /// The two cards, in the order their notes were added.
    pub cards: [i64; 2],
}

/// Builds two Basic notes in the default deck, so two cards, with the engine's own API.
pub fn synthetic(test: &str) -> Synthetic {
    let dir = scratch("engine-core-dispatch", test);
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let mut cards = [0_i64; 2];
    for (index, card) in cards.iter_mut().enumerate() {
        let mut note = basic.new_note();
        note.set_field(0, format!("synthetic front {index}"))
            .expect("the front is set");
        note.set_field(1, format!("synthetic back {index}"))
            .expect("the back is set");
        col.add_note(&mut note, DeckId(1))
            .expect("the engine adds the note to the default deck");
        *card = col
            .storage
            .db()
            .query_row("select id from cards where nid = ?", [note.id.0], |row| {
                row.get(0)
            })
            .expect("the note has one card");
    }
    col.close(None).expect("the engine closes the collection");
    Synthetic {
        dir,
        collection,
        cards,
    }
}

fn text(path: &Path) -> String {
    path.to_str().expect("a scratch path is UTF-8").to_owned()
}

/// The encoded `OpenCollectionRequest` for a synthetic collection.
pub fn open_request(synthetic: &Synthetic) -> Vec<u8> {
    OpenCollectionRequest {
        collection_path: text(&synthetic.collection),
        media_folder_path: text(&synthetic.dir.join("collection.media")),
        media_db_path: text(&synthetic.dir.join("collection.media.db")),
    }
    .encode_to_vec()
}
