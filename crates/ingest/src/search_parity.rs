//! The read-only search parity tool (SPEC-514 R9 and A6): the cards the engine's search selects
//! in a collection against the cards the search crate's own evaluator selects in the same rows, counted
//! and never listed.
//!
//! The evaluator's side opens the collection only through the kernel's
//! [`Db::open_foreign_read_only`] (`mode=ro` and `query_only`, as [`crate::reader`] does), so a
//! write through it is refused by `SQLite` itself. The engine opens collections writable and, in
//! client mode, rewrites a configured offset, so its side runs only on a scratch copy the tool
//! makes. A scratch path that names the source's own file (the same device and inode, so a hard
//! link, a symbolic link or another spelling of the path) is refused before anything is opened,
//! and the copy creates its scratch path (`O_CREAT | O_EXCL`), so it never truncates an existing
//! file. The answer is three counts (match, engine-only, evaluator-only): no card id, deck name,
//! field text or search text leaves the tool.
//!
//! A known gap: the read takes every note type's first field as its sort field (the note type's
//! configured sort field is in its protobuf configuration, which the read does not decode), so a
//! text search over a note type sorted by another field may count a note on one side only.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_kernel::{Db, ForeignDb};
use deck_streak_search::{
    Card, Collection, Deck, EngineDay, Note, Notetype, Profile, Refusal, Review, eval,
};

use crate::engine::{EngineError, RslibEngine};

/// The separator between a stored deck name's levels and a stored note's fields.
const SEPARATOR: char = '\u{1f}';

/// What the tool answers: how many cards both sides select, and how many only one side does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParityCounts {
    /// Cards both the engine and the evaluator select.
    pub matched: usize,
    /// Cards only the engine selects.
    pub engine_only: usize,
    /// Cards only the search crate's evaluator selects.
    pub ds_only: usize,
}

impl fmt::Display for ParityCounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "match={} engine-only={} ds-only={}",
            self.matched, self.engine_only, self.ds_only
        )
    }
}

/// Why the tool gives no counts. No variant names a path, a search or a row.
#[derive(Debug, thiserror::Error)]
pub enum ParityError {
    /// The scratch path the engine would write names the source's own file (its device and
    /// inode): refused before anything opens.
    #[error("the engine's scratch copy would be the source collection")]
    WritableOpen,
    /// The search is not one expression of the operator profile.
    #[error("the search is refused: {0}")]
    Search(Refusal),
    /// The read-only open, or one of its reads, failed.
    #[error("the read-only read of the collection failed")]
    Read,
    /// Copying the source to the scratch path failed, an existing scratch path among the causes:
    /// the copy creates the path and never opens an existing one.
    #[error("the scratch copy could not be made")]
    Copy,
    /// The engine's day or search failed on the scratch copy.
    #[error("the engine failed on the scratch copy: {0}")]
    Engine(EngineError),
}

/// Counts the cards `search` selects in the collection at `source` on each side: the search crate's
/// evaluator over the rows read through the read-only open, and the engine's search on a copy
/// at `scratch`, on the engine's own day.
///
/// # Errors
///
/// [`ParityError::Search`] for a search the operator profile refuses,
/// [`ParityError::WritableOpen`] when `scratch` names the source's own file,
/// [`ParityError::Read`] when the read-only read fails, [`ParityError::Copy`] when the copy fails
/// (an existing `scratch` path among the causes), and [`ParityError::Engine`] when the engine
/// fails on the copy.
pub async fn search_parity(
    source: &Path,
    scratch: &Path,
    search: &str,
) -> Result<ParityCounts, ParityError> {
    let query = Profile::OPERATOR
        .parse(search)
        .map_err(ParityError::Search)?;
    if names_the_source(source, scratch) {
        return Err(ParityError::WritableOpen);
    }
    let foreign = Db::open_foreign_read_only(source)
        .await
        .map_err(|_| ParityError::Read)?;
    let rows = read_rows(&foreign).await;
    foreign.close().await;
    let rows = rows?;
    copy_to_a_new_file(source, scratch).map_err(|_| ParityError::Copy)?;
    let (day, selected) = RslibEngine
        .scratch_search(scratch, search)
        .map_err(ParityError::Engine)?;
    let engine_ids: BTreeSet<i64> = selected.into_iter().map(|card| card.id).collect();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        });
    let ds_ids: BTreeSet<i64> = eval(
        &query,
        &rows,
        EngineDay {
            days_elapsed: day.days_elapsed,
            next_day_at: day.next_day_at,
            now,
        },
    )
    .into_iter()
    .map(|index| rows.cards[index].id)
    .collect();
    Ok(ParityCounts {
        matched: engine_ids.intersection(&ds_ids).count(),
        engine_only: engine_ids.difference(&ds_ids).count(),
        ds_only: ds_ids.difference(&engine_ids).count(),
    })
}

/// Whether `scratch` names the file `source` names: the same device and inode, so a hard link, a
/// symbolic link or another spelling of the source's path is the source. A path that does not
/// exist names no file yet; the copy then creates it.
fn names_the_source(source: &Path, scratch: &Path) -> bool {
    match (std::fs::metadata(source), std::fs::metadata(scratch)) {
        (Ok(source), Ok(scratch)) => {
            (source.dev(), source.ino()) == (scratch.dev(), scratch.ino())
        }
        _ => false,
    }
}

/// Copies `source` to `scratch`, which the copy creates (`O_CREAT | O_EXCL`): any existing entry
/// at `scratch`, a dangling symbolic link included, fails the open, so no existing file is ever
/// truncated.
fn copy_to_a_new_file(source: &Path, scratch: &Path) -> io::Result<u64> {
    let mut from = File::open(source)?;
    let mut to = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(scratch)?;
    io::copy(&mut from, &mut to)
}

/// A card row as the read takes it.
type CardRow = (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);

/// The tables the evaluator needs, as the read takes them.
struct Tables {
    decks: Vec<(i64, String)>,
    types: Vec<(i64, String)>,
    fields: Vec<(i64, i64, String)>,
    templates: Vec<(i64, i64, String)>,
    notes: Vec<(i64, i64, String, String)>,
    cards: Vec<CardRow>,
    revlog: Vec<(i64, i64, i64)>,
}

/// Reads the rows the evaluator needs through the read-only handle.
async fn read_rows(foreign: &ForeignDb) -> Result<Collection, ParityError> {
    let pool = foreign.reader();
    let decks: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM decks")
        .fetch_all(pool)
        .await
        .map_err(|_| ParityError::Read)?;
    let types: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM notetypes")
        .fetch_all(pool)
        .await
        .map_err(|_| ParityError::Read)?;
    let fields: Vec<(i64, i64, String)> =
        sqlx::query_as("SELECT ntid, ord, name FROM fields ORDER BY ntid, ord")
            .fetch_all(pool)
            .await
            .map_err(|_| ParityError::Read)?;
    let templates: Vec<(i64, i64, String)> =
        sqlx::query_as("SELECT ntid, ord, name FROM templates ORDER BY ntid, ord")
            .fetch_all(pool)
            .await
            .map_err(|_| ParityError::Read)?;
    let notes: Vec<(i64, i64, String, String)> =
        sqlx::query_as("SELECT id, mid, flds, tags FROM notes")
            .fetch_all(pool)
            .await
            .map_err(|_| ParityError::Read)?;
    let cards: Vec<CardRow> = sqlx::query_as(
        "SELECT id, nid, did, odid, ord, type, queue, due, odue, ivl, reps, lapses FROM cards",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ParityError::Read)?;
    let revlog: Vec<(i64, i64, i64)> = sqlx::query_as("SELECT id, cid, ease FROM revlog")
        .fetch_all(pool)
        .await
        .map_err(|_| ParityError::Read)?;
    Ok(assemble(Tables {
        decks,
        types,
        fields,
        templates,
        notes,
        cards,
        revlog,
    }))
}

/// The evaluator's rows from the read tables: a note of an unknown note type, a card of an
/// unknown note and a review of an unknown card are left out, as the engine's joins leave them.
fn assemble(tables: Tables) -> Collection {
    let Tables {
        decks,
        types,
        fields,
        templates,
        notes,
        cards,
        revlog,
    } = tables;

    let type_index: BTreeMap<i64, usize> = types
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();
    let mut notetypes: Vec<Notetype> = types
        .into_iter()
        .map(|(id, name)| Notetype {
            id,
            name,
            fields: Vec::new(),
            templates: Vec::new(),
            sort_field: 0,
        })
        .collect();
    for (ntid, _, name) in fields {
        if let Some(&i) = type_index.get(&ntid) {
            notetypes[i].fields.push(name);
        }
    }
    for (ntid, _, name) in templates {
        if let Some(&i) = type_index.get(&ntid) {
            notetypes[i].templates.push(name);
        }
    }
    let mut note_index = BTreeMap::new();
    let mut read_notes = Vec::with_capacity(notes.len());
    for (id, mid, flds, tags) in notes {
        let Some(&notetype) = type_index.get(&mid) else {
            continue;
        };
        note_index.insert(id, read_notes.len());
        read_notes.push(Note {
            id,
            notetype,
            fields: flds.split(SEPARATOR).map(str::to_owned).collect(),
            tags: tags.split_whitespace().map(str::to_owned).collect(),
        });
    }
    let mut card_index = BTreeMap::new();
    let mut read_cards = Vec::with_capacity(cards.len());
    for (id, nid, did, odid, ord, kind, queue, due, odue, ivl, reps, lapses) in cards {
        let Some(&note) = note_index.get(&nid) else {
            continue;
        };
        card_index.insert(id, read_cards.len());
        read_cards.push(Card {
            id,
            note,
            deck_id: did,
            original_deck_id: odid,
            ord: u16::try_from(ord).unwrap_or(u16::MAX),
            kind,
            queue,
            due,
            original_due: odue,
            interval: ivl,
            reps,
            lapses,
        });
    }
    let reviews = revlog
        .into_iter()
        .filter_map(|(id, cid, ease)| card_index.get(&cid).map(|&card| Review { id, card, ease }))
        .collect();
    Collection {
        decks: decks
            .into_iter()
            .map(|(id, name)| Deck {
                id,
                name: name.replace(SEPARATOR, "::"),
            })
            .collect(),
        notetypes,
        notes: read_notes,
        cards: read_cards,
        revlog: reviews,
    }
}
