//! The review fixture's second collection (SPEC-380 R10): a deck `Occlusion` holding one image
//! occlusion card, new on the default preset, beside a media folder that holds its image.
//!
//! One builder, written with the engine's own API before the adapter runs: the engine adds its
//! stock image occlusion note type, and the note is that type's own, with its occlusion, image and
//! header fields filled from the same fixed inputs the engine core's occlusion test builds its note
//! from: the image's bytes, one rectangle and a header that speaks. The `review-fixture` example
//! includes this file alone to write the collection in `occlusion/` beside the first, and the
//! adapter's tests include it to build the same collection in a scratch directory of their own. It
//! returns every failure as an error rather than panicking, because the example is not a test.

#![allow(
    dead_code,
    reason = "the example and each test that includes this file read the part they need"
)]

use std::error::Error;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;

/// The directory, beside the first collection, that holds the second.
pub const DIRECTORY: &str = "occlusion";
/// The deck that holds the image occlusion card.
pub const DECK: &str = "Occlusion";
/// The engine's stock image occlusion note type, by the name it gives it.
const NOTETYPE: &str = "Image Occlusion";
/// The occluded image's file name in the media folder, and its bytes: a PNG signature.
pub const IMAGE: &str = "occluded.png";
const IMAGE_BYTES: &[u8] = b"\x89PNG\r\n\x1a\n";
/// One rectangle over the image, as the engine's occlusion field spells a shape.
const SHAPES: &str = "{{c1::image-occlusion:rect:left=.2:top=.3:width=.4:height=.1}}";
/// A header that speaks, so the face would carry a clip were it shown.
const HEADER: &str = "[anki:tts lang=en_US]the parts of a cell[/anki:tts]";

/// The second collection, built and closed, ready for the adapter to open.
pub struct Occlusion {
    /// The directory that holds the collection and its media folder.
    pub dir: PathBuf,
    /// The collection file, `collection.anki2` in `dir`.
    pub collection: PathBuf,
    /// The image occlusion note's one card.
    pub card: i64,
}

/// Builds the second collection in `dir`: `collection.anki2` and `collection.media/` with the
/// image.
///
/// # Errors
///
/// Any step the engine or the file system refuses: the directories, the image, the collection,
/// the deck, the stock image occlusion note type, the note, or reading its card.
pub fn build(dir: &Path) -> Result<Occlusion, Box<dyn Error>> {
    let media = dir.join("collection.media");
    std::fs::create_dir_all(&media)?;
    std::fs::write(media.join(IMAGE), IMAGE_BYTES)?;
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection).build()?;
    let deck = col.get_or_create_normal_deck(DECK)?.id;
    col.add_image_occlusion_notetype()?;
    let notetype = col
        .get_notetype_by_name(NOTETYPE)?
        .ok_or("the engine created no stock image occlusion note type")?;
    let mut note = notetype.new_note();
    // The stock type's fields, in the order the engine adds them: occlusion, image, header.
    note.set_field(0, SHAPES)?;
    note.set_field(1, format!(r#"<img src="{IMAGE}">"#))?;
    note.set_field(2, HEADER)?;
    col.add_note(&mut note, deck)?;
    let card =
        col.storage
            .db()
            .query_row("select id from cards where nid = ?", [note.id.0], |row| {
                row.get(0)
            })?;
    col.close(None)?;
    Ok(Occlusion {
        dir: dir.to_path_buf(),
        collection,
        card,
    })
}
