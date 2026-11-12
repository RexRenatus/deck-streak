//! The card health collection (SPEC-507 R17): one card with a review record, in a deck `Health` of its
//! own, whose figures the adapter's card health test reads and whose sheet the app's tests open.
//!
//! One builder, written with the engine's own API and its storage before the adapter runs. The
//! `review-fixture` example includes this file alone to write the collection into `health/` beside the
//! review fixture, and the adapter's test includes it to build the same collection in a scratch
//! directory of its own. The card's id is fixed at [`CARD`], a millisecond instant on 2023-11-14 UTC,
//! so the date it was created on reads the same on every run. Its log holds [`OLDER_ROWS`] entries set
//! by hand, then seven rows whose answers' capped times are 8,000, 0, 60,000, 12,000, 20,001 and 29,998
//! ms: an hour's answer counts as the cap, a negative time as 0, a learning Again and a review Good are
//! no lapse, and an entry set by hand is a row but no answer. Its counters read 8 reviews and 1 lapse
//! against the log's 6 answers and 1 lapse, and its data stores an FSRS memory state (stability 12.34
//! days, difficulty 5.6) that no part of the record may carry. The card is a review card due on the
//! day the collection is made, so its deck's queue shows it. It returns every failure as an error
//! rather than panicking, because the example is not a test.

#![allow(
    dead_code,
    reason = "the example and the test that include this file read the part they need"
)]

use std::error::Error;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;

/// The deck that holds the card.
pub const DECK: &str = "Health";
/// The card's fixed id: 2023-11-14 21:56:40 UTC, in epoch milliseconds.
pub const CARD: i64 = 1_699_999_000_000;
/// The seven rows' first instant: 2023-11-14 22:13:20 UTC.
pub const BASE_MS: i64 = 1_700_000_000_000;
/// The entries set by hand that come before the seven rows, so the card has more rows than the
/// record carries.
pub const OLDER_ROWS: i64 = 200;
/// The card's front.
const FRONT: &str = "a card with a record";
/// The card's back.
const BACK: &str = "its back";
/// An FSRS memory state as the engine stores it in a card's data.
const STORED_STATE: &str = r#"{"s":12.34,"d":5.6,"dr":0.9}"#;

/// The card health collection built and closed, ready for the adapter to open.
pub struct Health {
    /// The directory that holds the collection and its media folder.
    pub dir: PathBuf,
    /// The collection file, `collection.anki2` in `dir`.
    pub collection: PathBuf,
    /// The card's id.
    pub card: i64,
}

/// One review-log row of the card: its instant, button, interval, time and type.
fn revlog(at: i64, ease: i64, interval: i64, time: i64, kind: i64) -> String {
    format!(
        "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
         values ({at}, {CARD}, -1, {ease}, {interval}, 0, 2500, {time}, {kind});"
    )
}

/// The card's rows, oldest first: the entries set by hand, then the seven.
fn rows() -> Vec<String> {
    let mut rows: Vec<String> = (0..OLDER_ROWS)
        .map(|index| revlog(BASE_MS - (OLDER_ROWS - index) * 1_000, 0, 0, 0, 4))
        .collect();
    rows.extend([
        revlog(BASE_MS + 1_000, 3, -600, 8_000, 0),
        // a learning Again is no lapse, and its negative time counts as 0
        revlog(BASE_MS + 2_000, 1, -60, -5_000, 0),
        // an hour's answer counts as the cap, 60,000 ms
        revlog(BASE_MS + 3_000, 3, 1, 3_600_000, 0),
        // a review Again: the one lapse
        revlog(BASE_MS + 4_000, 1, -600, 12_000, 1),
        // an entry set by hand is a row, not an answer
        revlog(BASE_MS + 5_000, 0, 0, 0, 4),
        revlog(BASE_MS + 6_000, 3, 1, 20_001, 2),
        // a review Good is no lapse
        revlog(BASE_MS + 7_000, 3, 12, 29_998, 1),
    ]);
    rows
}

/// Builds the card health collection in `dir`: `collection.anki2` and an empty `collection.media/`.
///
/// # Errors
///
/// Any step the engine or the file system refuses: the directories, the collection, the deck, the
/// stock Basic note type, the note, fixing the card's id, or writing its rows and fields.
pub fn build(dir: &Path) -> Result<Health, Box<dyn Error>> {
    std::fs::create_dir_all(dir.join("collection.media"))?;
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
    let db = col.storage.db();
    let fixed = db.execute("update cards set id = ? where nid = ?", [CARD, note.id.0])?;
    if fixed != 1 {
        return Err(format!("the note holds {fixed} card(s), not one").into());
    }
    // A review card due on the collection's first day, its counters apart from its log, and an
    // FSRS memory state in its data.
    let mut statements = rows();
    statements.push(format!(
        "update cards set type = 2, queue = 2, due = 0, ivl = 12, factor = 2500, reps = 8, \
         lapses = 1, data = '{STORED_STATE}' where id = {CARD};"
    ));
    db.execute_batch(&statements.concat())?;
    col.close(None)?;
    Ok(Health {
        dir: dir.to_path_buf(),
        collection,
        card: CARD,
    })
}
