//! The synthetic collection the adapter's tests open and the harness bundles (SPEC-336 A1 to A6,
//! SPEC-339 R11).
//!
//! One builder, written with the engine's own API before the adapter runs: one Basic note, front
//! `synthetic front` and back `synthetic back`, in the default deck, and a second, empty deck. The
//! round-trip tests build it in a scratch directory of their own, and the `harness-fixture` example
//! includes this file alone to write the same collection where the harness job reads it. It returns
//! every failure as an error rather than panicking, because the example is not a test.

use std::error::Error;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;
use anki::decks::DeckId;

/// The second deck the synthetic collection holds, beside the engine's own default deck.
pub const SECOND_DECK: &str = "Synthetic";
/// The synthetic note's front, which a rendered question shows.
pub const FRONT: &str = "synthetic front";
/// The synthetic note's back.
pub const BACK: &str = "synthetic back";

/// A collection built and closed, ready for the adapter to open.
pub struct Synthetic {
    /// The directory that holds the collection and its media folder.
    pub dir: PathBuf,
    /// The collection file, `collection.anki2` in `dir`.
    pub collection: PathBuf,
    /// The synthetic note's id.
    pub note_id: i64,
    /// The id of the note's one card.
    pub card_id: i64,
}

/// Builds the synthetic collection in `dir`: `collection.anki2` and an empty `collection.media/`.
///
/// # Errors
///
/// Any step the engine or the file system refuses: the directory, the collection, the second
/// deck, the stock Basic note type, the note, or the card's id.
pub fn build(dir: &Path) -> Result<Synthetic, Box<dyn Error>> {
    std::fs::create_dir_all(dir.join("collection.media"))?;
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection).build()?;
    col.get_or_create_normal_deck(SECOND_DECK)?;
    let basic = col
        .get_notetype_by_name("Basic")?
        .ok_or("the engine created no stock Basic note type")?;
    let mut note = basic.new_note();
    note.set_field(0, FRONT)?;
    note.set_field(1, BACK)?;
    col.add_note(&mut note, DeckId(1))?;
    let note_id = note.id.0;
    let card_id: i64 =
        col.storage
            .db()
            .query_row("select id from cards where nid = ?", [note_id], |row| {
                row.get(0)
            })?;
    col.close(None)?;
    Ok(Synthetic {
        dir: dir.to_path_buf(),
        collection,
        note_id,
        card_id,
    })
}
