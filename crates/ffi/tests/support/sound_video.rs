//! The sound video collection (SPEC-410 R8): one note of the stock Basic note type, in the deck
//! `Sound video`, beside a media folder that holds three videos and a sound.
//!
//! The note's front names a video, a sound and a second video through sound tags, and its back
//! names a third video whose extension is spelt in capitals. One builder, written with the engine's
//! own API before the adapter runs: the `review-fixture` example includes it to write
//! `sound-video/` beside the review collection, and the adapter's tests build the same collection
//! in a scratch directory of their own. Every file is built here from fixed bytes, never committed
//! as a binary, and the card's id is fixed. The four base names differ, because the simulator's
//! file system ignores case. It returns every failure as an error rather than panicking, because
//! the example is not a test.

#![allow(
    dead_code,
    reason = "the example and each test that includes this file read the part they need"
)]

use std::error::Error;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;

/// The deck that holds the note's card.
pub const DECK: &str = "Sound video";
/// The note's front: a video, a sound and a second video, each named by a sound tag.
pub const FRONT: &str = "[sound:sign.mp4][sound:beep.mp3][sound:wave.m4v]";
/// The note's back: a third video, its extension spelt in capitals.
pub const BACK: &str = "[sound:turn.MOV]";
/// The first video's file name in the media folder.
pub const SIGN: &str = "sign.mp4";
/// The first video's bytes.
pub const SIGN_BYTES: &[u8] = b"sign-video";
/// The sound's file name in the media folder.
pub const BEEP: &str = "beep.mp3";
/// The sound's bytes.
pub const BEEP_BYTES: &[u8] = b"beep-sound";
/// The second video's file name in the media folder.
pub const WAVE: &str = "wave.m4v";
/// The second video's bytes.
pub const WAVE_BYTES: &[u8] = b"wave-video";
/// The back's video's file name in the media folder, its extension in capitals.
pub const TURN: &str = "turn.MOV";
/// The back's video's bytes.
pub const TURN_BYTES: &[u8] = b"turn-video";
/// The card's id.
const CARD_ID: i64 = 3_000_001;

/// A sound video collection built and closed, ready for the adapter to open.
pub struct SoundVideo {
    /// The directory that holds the collection and its media folder.
    pub dir: PathBuf,
    /// The collection file, `collection.anki2` in `dir`.
    pub collection: PathBuf,
    /// The note's one card's id.
    pub card: i64,
}

/// Builds the sound video collection in `dir`: `collection.anki2` and `collection.media/` with
/// the three videos and the sound.
///
/// # Errors
///
/// Any step the engine or the file system refuses: the directories, the four files, the
/// collection, the deck, the note type, the note, or fixing the card's id.
pub fn build(dir: &Path) -> Result<SoundVideo, Box<dyn Error>> {
    let media = dir.join("collection.media");
    std::fs::create_dir_all(&media)?;
    for (name, bytes) in [
        (SIGN, SIGN_BYTES),
        (BEEP, BEEP_BYTES),
        (WAVE, WAVE_BYTES),
        (TURN, TURN_BYTES),
    ] {
        std::fs::write(media.join(name), bytes)?;
    }
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection).build()?;
    let deck = col.get_or_create_normal_deck(DECK)?.id;
    let basic = col
        .get_notetype_by_name("Basic")?
        .ok_or("the engine created no stock Basic note type")?;
    let mut note = basic.new_note();
    note.set_field(0, FRONT)?;
    note.set_field(1, BACK)?;
    col.add_note(&mut note, deck)?;
    let fixed = col.storage.db().execute(
        "update cards set id = ? where nid = ? and ord = 0",
        [CARD_ID, note.id.0],
    )?;
    if fixed != 1 {
        return Err(format!("the note holds {fixed} card(s) of its template, not one").into());
    }
    col.close(None)?;
    Ok(SoundVideo {
        dir: dir.to_path_buf(),
        collection,
        card: CARD_ID,
    })
}
