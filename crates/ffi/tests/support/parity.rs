//! The parity collection (SPEC-393 R15): one note of a two-template note type, in the deck
//! `Parity`, beside a media folder that holds a font and a video.
//!
//! The note type's CSS names the font; its first template speaks the front through a TTS tag that
//! asks for two voices, and its second shows two video tags, one spelt in capitals, and a custom
//! element whose name starts with `video`. One builder, written with the engine's own API before
//! the adapter runs: the `review-fixture` example includes it to write `parity/` beside the review
//! collection, and the adapter's tests build the same collection in a scratch directory of their
//! own. Both files are built here from fixed bytes, never committed as binaries, and the two cards'
//! ids are fixed. It returns every failure as an error rather than panicking, because the example
//! is not a test.

#![allow(
    dead_code,
    reason = "the example and each test that includes this file read the part they need"
)]

use std::error::Error;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;
use anki::notetype::{Notetype, NotetypeId};

/// The deck that holds the note's two cards.
pub const DECK: &str = "Parity";
/// The font's file name in the media folder, as the CSS names it.
pub const FONT: &str = "_parity.ttf";
/// The font's bytes.
pub const FONT_BYTES: &[u8] = b"parity-font";
/// The video's file name in the media folder.
pub const VIDEO: &str = "parity.mp4";
/// The video's bytes.
pub const VIDEO_BYTES: &[u8] = b"parity-video";
/// The note's front, which the first template speaks.
pub const FRONT: &str = "a parity card";
/// The note's back, which the second template shows above its videos.
pub const BACK: &str = "the parity back";
/// The voices the first template's TTS tag asks for, in its order.
pub const VOICES: [&str; 2] = ["Absent_Voice", "Desk_Parity_Voice"];
/// The custom element the second template shows, whose name starts with `video`.
pub const VIDEO_NOTE: &str = "<video-note>";

/// The note type's CSS: a font face over the font file, and the card's family.
const CSS: &str = concat!(
    "@font-face { font-family: parity; src: url(\"_parity.ttf\"); }\n",
    ".card { font-family: parity; }\n",
);
/// The first template's front: the front, spoken by the first of two voices that is installed.
const SPEAKS: &str = "{{tts en_US voices=Absent_Voice,Desk_Parity_Voice:Front}}";
/// The second template's front: the back, a video tag, the same tag in capitals, and the custom
/// element.
const SHOWS: &str = concat!(
    "{{Back}}",
    r#"<video src="parity.mp4" controls></video>"#,
    r#"<VIDEO controls src="parity.mp4"></VIDEO>"#,
    "<video-note>a note beside the video</video-note>",
);
/// The two cards' ids, by template: the speaking card, then the showing card.
const CARD_IDS: [i64; 2] = [2_000_001, 2_000_002];

/// The parity note's two cards' fixed ids, by template.
pub struct Cards {
    /// The first template's card, which speaks the front.
    pub speaks: i64,
    /// The second template's card, which shows the videos.
    pub shows: i64,
}

/// A parity collection built and closed, ready for the adapter to open.
pub struct Parity {
    /// The directory that holds the collection and its media folder.
    pub dir: PathBuf,
    /// The collection file, `collection.anki2` in `dir`.
    pub collection: PathBuf,
    /// The two cards' ids.
    pub cards: Cards,
}

/// Builds the parity collection in `dir`: `collection.anki2` and `collection.media/` with the
/// font and the video.
///
/// # Errors
///
/// Any step the engine or the file system refuses: the directories, the two files, the
/// collection, the deck, the note type, the note, or fixing a card's id.
pub fn build(dir: &Path) -> Result<Parity, Box<dyn Error>> {
    let media = dir.join("collection.media");
    std::fs::create_dir_all(&media)?;
    std::fs::write(media.join(FONT), FONT_BYTES)?;
    std::fs::write(media.join(VIDEO), VIDEO_BYTES)?;
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection).build()?;
    let deck = col.get_or_create_normal_deck(DECK)?.id;
    let reversed = col
        .get_notetype_by_name("Basic (and reversed card)")?
        .ok_or("the engine created no stock two-template note type")?;
    let mut parity = Notetype::clone(&reversed);
    parity.id = NotetypeId(0);
    "Parity".clone_into(&mut parity.name);
    CSS.clone_into(&mut parity.config.css);
    for (index, front) in [SPEAKS, SHOWS].into_iter().enumerate() {
        let template = parity
            .templates
            .get_mut(index)
            .ok_or("the stock note type has fewer than two templates")?;
        front.clone_into(&mut template.config.q_format);
    }
    col.add_notetype(&mut parity, false)?;
    let mut note = parity.new_note();
    note.set_field(0, FRONT)?;
    note.set_field(1, BACK)?;
    col.add_note(&mut note, deck)?;
    for (ord, id) in (0_i64..).zip(CARD_IDS) {
        let fixed = col.storage.db().execute(
            "update cards set id = ? where nid = ? and ord = ?",
            [id, note.id.0, ord],
        )?;
        if fixed != 1 {
            return Err(
                format!("the note holds {fixed} card(s) of template {ord}, not one").into(),
            );
        }
    }
    col.close(None)?;
    let [speaks, shows] = CARD_IDS;
    Ok(Parity {
        dir: dir.to_path_buf(),
        collection,
        cards: Cards { speaks, shows },
    })
}
