//! The 14 tables as `DeckStreak`'s own row types (SPEC-602 R5, R8; ADR-622 D2): the twelve
//! collection tables of the schema snapshot and the media database's two, every column kept.
//!
//! Each row type's fields are its table's columns in column order; a blob column reads as its
//! typed view beside its raw bytes.

use rusqlite::types::{Type, ValueRef};
use rusqlite::{Row, params};

use crate::blob::{
    Blob, BlobView, ConfigValue, DeckCommonView, DeckConfigView, DeckKindView, FieldConfigView,
    NotetypeConfigView, TagConfigView, TemplateConfigView,
};
use crate::opener::{Reader, Refusal};

/// A row of `cards`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardRow {
    /// `cards.id`.
    pub id: i64,
    /// `cards.nid`.
    pub nid: i64,
    /// `cards.did`.
    pub did: i64,
    /// `cards.ord`.
    pub ord: i64,
    /// `cards.mod`.
    pub r#mod: i64,
    /// `cards.usn`.
    pub usn: i64,
    /// `cards.type`.
    pub r#type: i64,
    /// `cards.queue`.
    pub queue: i64,
    /// `cards.due`.
    pub due: i64,
    /// `cards.ivl`.
    pub ivl: i64,
    /// `cards.factor`.
    pub factor: i64,
    /// `cards.reps`.
    pub reps: i64,
    /// `cards.lapses`.
    pub lapses: i64,
    /// `cards.left`.
    pub left: i64,
    /// `cards.odue`.
    pub odue: i64,
    /// `cards.odid`.
    pub odid: i64,
    /// `cards.flags`.
    pub flags: i64,
    /// `cards.data`.
    pub data: String,
}

/// A row of `col`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColRow {
    /// `col.id`.
    pub id: i64,
    /// `col.crt`.
    pub crt: i64,
    /// `col.mod`.
    pub r#mod: i64,
    /// `col.scm`.
    pub scm: i64,
    /// `col.ver`.
    pub ver: i64,
    /// `col.dty`.
    pub dty: i64,
    /// `col.usn`.
    pub usn: i64,
    /// `col.ls`.
    pub ls: i64,
    /// `col.conf`.
    pub conf: String,
    /// `col.models`.
    pub models: String,
    /// `col.decks`.
    pub decks: String,
    /// `col.dconf`.
    pub dconf: String,
    /// `col.tags`.
    pub tags: String,
}

/// A row of `config`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigRow {
    /// `config.KEY`.
    pub key: String,
    /// `config.usn`.
    pub usn: i64,
    /// `config.mtime_secs`.
    pub mtime_secs: i64,
    /// `config.val`.
    pub val: Blob<ConfigValue>,
}

/// A row of `deck_config`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckConfigRow {
    /// `deck_config.id`.
    pub id: i64,
    /// `deck_config.name`.
    pub name: String,
    /// `deck_config.mtime_secs`.
    pub mtime_secs: i64,
    /// `deck_config.usn`.
    pub usn: i64,
    /// `deck_config.config`.
    pub config: Blob<DeckConfigView>,
}

/// A row of `decks`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckRow {
    /// `decks.id`.
    pub id: i64,
    /// `decks.name`.
    pub name: String,
    /// `decks.mtime_secs`.
    pub mtime_secs: i64,
    /// `decks.usn`.
    pub usn: i64,
    /// `decks.common`.
    pub common: Blob<DeckCommonView>,
    /// `decks.kind`.
    pub kind: Blob<DeckKindView>,
}

/// A row of `fields`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRow {
    /// `fields.ntid`.
    pub ntid: i64,
    /// `fields.ord`.
    pub ord: i64,
    /// `fields.name`.
    pub name: String,
    /// `fields.config`.
    pub config: Blob<FieldConfigView>,
}

/// A row of `graves`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraveRow {
    /// `graves.oid`.
    pub oid: i64,
    /// `graves.type`.
    pub r#type: i64,
    /// `graves.usn`.
    pub usn: i64,
}

/// A row of `notes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRow {
    /// `notes.id`.
    pub id: i64,
    /// `notes.guid`.
    pub guid: String,
    /// `notes.mid`.
    pub mid: i64,
    /// `notes.mod`.
    pub r#mod: i64,
    /// `notes.usn`.
    pub usn: i64,
    /// `notes.tags`.
    pub tags: String,
    /// `notes.flds`.
    pub flds: String,
    /// `notes.sfld`: integer affinity, so a sort field of digits is stored as an integer and
    /// reads back as its digits.
    pub sfld: String,
    /// `notes.csum`.
    pub csum: i64,
    /// `notes.flags`.
    pub flags: i64,
    /// `notes.data`.
    pub data: String,
}

/// A row of `notetypes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotetypeRow {
    /// `notetypes.id`.
    pub id: i64,
    /// `notetypes.name`.
    pub name: String,
    /// `notetypes.mtime_secs`.
    pub mtime_secs: i64,
    /// `notetypes.usn`.
    pub usn: i64,
    /// `notetypes.config`.
    pub config: Blob<NotetypeConfigView>,
}

/// A row of `revlog`: one history entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevlogRow {
    /// `revlog.id`.
    pub id: i64,
    /// `revlog.cid`.
    pub cid: i64,
    /// `revlog.usn`.
    pub usn: i64,
    /// `revlog.ease`.
    pub ease: i64,
    /// `revlog.ivl`.
    pub ivl: i64,
    /// `revlog.lastIvl`.
    pub last_ivl: i64,
    /// `revlog.factor`.
    pub factor: i64,
    /// `revlog.time`.
    pub time: i64,
    /// `revlog.type`.
    pub r#type: i64,
}

/// A row of `tags`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagRow {
    /// `tags.tag`.
    pub tag: String,
    /// `tags.usn`.
    pub usn: i64,
    /// `tags.collapsed`.
    pub collapsed: bool,
    /// `tags.config`, which may be null.
    pub config: Option<Blob<TagConfigView>>,
}

/// A row of `templates`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateRow {
    /// `templates.ntid`.
    pub ntid: i64,
    /// `templates.ord`.
    pub ord: i64,
    /// `templates.name`.
    pub name: String,
    /// `templates.mtime_secs`.
    pub mtime_secs: i64,
    /// `templates.usn`.
    pub usn: i64,
    /// `templates.config`.
    pub config: Blob<TemplateConfigView>,
}

/// A row of the media database's `media`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaRow {
    /// `media.fname`.
    pub fname: String,
    /// `media.csum`, null for a deleted file.
    pub csum: Option<String>,
    /// `media.mtime`.
    pub mtime: i64,
    /// `media.dirty`.
    pub dirty: i64,
}

/// A row of the media database's `meta`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaRow {
    /// `meta.dirMod`.
    pub dir_mod: Option<i64>,
    /// `meta.lastUsn`.
    pub last_usn: Option<i64>,
}

/// Every row of a collection, by table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Collection {
    /// The rows of `cards`.
    pub cards: Vec<CardRow>,
    /// The rows of `col`.
    pub col: Vec<ColRow>,
    /// The rows of `config`.
    pub config: Vec<ConfigRow>,
    /// The rows of `deck_config`.
    pub deck_config: Vec<DeckConfigRow>,
    /// The rows of `decks`.
    pub decks: Vec<DeckRow>,
    /// The rows of `fields`.
    pub fields: Vec<FieldRow>,
    /// The rows of `graves`.
    pub graves: Vec<GraveRow>,
    /// The rows of `notes`.
    pub notes: Vec<NoteRow>,
    /// The rows of `notetypes`.
    pub notetypes: Vec<NotetypeRow>,
    /// The rows of `revlog`.
    pub revlog: Vec<RevlogRow>,
    /// The rows of `tags`.
    pub tags: Vec<TagRow>,
    /// The rows of `templates`.
    pub templates: Vec<TemplateRow>,
}

/// Every row of a media database, by table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Media {
    /// The rows of `media`.
    pub media: Vec<MediaRow>,
    /// The rows of `meta`.
    pub meta: Vec<MetaRow>,
}

/// Every row `sql` selects, each read by `map`.
fn rows<T>(
    reader: &Reader,
    sql: &str,
    map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, Refusal> {
    let mut statement = reader.connection().prepare(sql)?;
    let read = statement
        .query_map(params![], map)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(read)
}

/// The blob in column `index`, read as its view.
fn blob<V: BlobView>(row: &Row<'_>, index: usize) -> rusqlite::Result<Blob<V>> {
    let raw: Vec<u8> = row.get(index)?;
    Blob::read(&raw)
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(index, Type::Blob, error.into()))
}

/// The text of column `index` whatever its storage class: an integer or a real as its digits.
fn text_of(row: &Row<'_>, index: usize) -> rusqlite::Result<String> {
    match row.get_ref(index)? {
        ValueRef::Integer(value) => Ok(value.to_string()),
        ValueRef::Real(value) => Ok(value.to_string()),
        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => {
            String::from_utf8(bytes.to_vec()).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(index, Type::Text, error.into())
            })
        }
        ValueRef::Null => Err(rusqlite::Error::InvalidColumnType(
            index,
            "sfld".to_owned(),
            Type::Null,
        )),
    }
}

/// Every row of the collection `reader` opened.
///
/// # Errors
///
/// A [`Refusal`] when a row does not read as its type.
pub fn collection(reader: &Reader) -> Result<Collection, Refusal> {
    Ok(Collection {
        cards: rows(
            reader,
            "SELECT id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps, lapses, \
             left, odue, odid, flags, data FROM cards",
            |row| {
                Ok(CardRow {
                    id: row.get(0)?,
                    nid: row.get(1)?,
                    did: row.get(2)?,
                    ord: row.get(3)?,
                    r#mod: row.get(4)?,
                    usn: row.get(5)?,
                    r#type: row.get(6)?,
                    queue: row.get(7)?,
                    due: row.get(8)?,
                    ivl: row.get(9)?,
                    factor: row.get(10)?,
                    reps: row.get(11)?,
                    lapses: row.get(12)?,
                    left: row.get(13)?,
                    odue: row.get(14)?,
                    odid: row.get(15)?,
                    flags: row.get(16)?,
                    data: row.get(17)?,
                })
            },
        )?,
        col: rows(
            reader,
            "SELECT id, crt, mod, scm, ver, dty, usn, ls, conf, models, decks, dconf, tags \
             FROM col",
            |row| {
                Ok(ColRow {
                    id: row.get(0)?,
                    crt: row.get(1)?,
                    r#mod: row.get(2)?,
                    scm: row.get(3)?,
                    ver: row.get(4)?,
                    dty: row.get(5)?,
                    usn: row.get(6)?,
                    ls: row.get(7)?,
                    conf: row.get(8)?,
                    models: row.get(9)?,
                    decks: row.get(10)?,
                    dconf: row.get(11)?,
                    tags: row.get(12)?,
                })
            },
        )?,
        config: rows(
            reader,
            "SELECT KEY, usn, mtime_secs, val FROM config",
            |row| {
                Ok(ConfigRow {
                    key: row.get(0)?,
                    usn: row.get(1)?,
                    mtime_secs: row.get(2)?,
                    val: blob(row, 3)?,
                })
            },
        )?,
        deck_config: rows(
            reader,
            "SELECT id, name, mtime_secs, usn, config FROM deck_config",
            |row| {
                Ok(DeckConfigRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    mtime_secs: row.get(2)?,
                    usn: row.get(3)?,
                    config: blob(row, 4)?,
                })
            },
        )?,
        decks: rows(
            reader,
            "SELECT id, name, mtime_secs, usn, common, kind FROM decks",
            |row| {
                Ok(DeckRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    mtime_secs: row.get(2)?,
                    usn: row.get(3)?,
                    common: blob(row, 4)?,
                    kind: blob(row, 5)?,
                })
            },
        )?,
        ..note_tables(reader)?
    })
}

/// The tables from `fields` to `notetypes`; the rest come from [`history_tables`], and the first
/// five are left for [`collection`] to fill.
fn note_tables(reader: &Reader) -> Result<Collection, Refusal> {
    Ok(Collection {
        fields: rows(
            reader,
            "SELECT ntid, ord, name, config FROM fields",
            |row| {
                Ok(FieldRow {
                    ntid: row.get(0)?,
                    ord: row.get(1)?,
                    name: row.get(2)?,
                    config: blob(row, 3)?,
                })
            },
        )?,
        graves: rows(reader, "SELECT oid, type, usn FROM graves", |row| {
            Ok(GraveRow {
                oid: row.get(0)?,
                r#type: row.get(1)?,
                usn: row.get(2)?,
            })
        })?,
        notes: rows(
            reader,
            "SELECT id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data FROM notes",
            |row| {
                Ok(NoteRow {
                    id: row.get(0)?,
                    guid: row.get(1)?,
                    mid: row.get(2)?,
                    r#mod: row.get(3)?,
                    usn: row.get(4)?,
                    tags: row.get(5)?,
                    flds: row.get(6)?,
                    sfld: text_of(row, 7)?,
                    csum: row.get(8)?,
                    flags: row.get(9)?,
                    data: row.get(10)?,
                })
            },
        )?,
        notetypes: rows(
            reader,
            "SELECT id, name, mtime_secs, usn, config FROM notetypes",
            |row| {
                Ok(NotetypeRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    mtime_secs: row.get(2)?,
                    usn: row.get(3)?,
                    config: blob(row, 4)?,
                })
            },
        )?,
        ..history_tables(reader)?
    })
}

/// The tables from `revlog` on; every other table is left empty for the callers to fill.
fn history_tables(reader: &Reader) -> Result<Collection, Refusal> {
    Ok(Collection {
        revlog: rows(
            reader,
            "SELECT id, cid, usn, ease, ivl, lastIvl, factor, time, type FROM revlog",
            |row| {
                Ok(RevlogRow {
                    id: row.get(0)?,
                    cid: row.get(1)?,
                    usn: row.get(2)?,
                    ease: row.get(3)?,
                    ivl: row.get(4)?,
                    last_ivl: row.get(5)?,
                    factor: row.get(6)?,
                    time: row.get(7)?,
                    r#type: row.get(8)?,
                })
            },
        )?,
        tags: rows(
            reader,
            "SELECT tag, usn, collapsed, config FROM tags",
            |row| {
                let raw: Option<Vec<u8>> = row.get(3)?;
                Ok(TagRow {
                    tag: row.get(0)?,
                    usn: row.get(1)?,
                    collapsed: row.get(2)?,
                    config: raw
                        .map(|raw| {
                            Blob::read(&raw).map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    3,
                                    Type::Blob,
                                    error.into(),
                                )
                            })
                        })
                        .transpose()?,
                })
            },
        )?,
        templates: rows(
            reader,
            "SELECT ntid, ord, name, mtime_secs, usn, config FROM templates",
            |row| {
                Ok(TemplateRow {
                    ntid: row.get(0)?,
                    ord: row.get(1)?,
                    name: row.get(2)?,
                    mtime_secs: row.get(3)?,
                    usn: row.get(4)?,
                    config: blob(row, 5)?,
                })
            },
        )?,
        ..Collection::default()
    })
}

/// Every row of the media database `reader` opened.
///
/// # Errors
///
/// A [`Refusal`] when a row does not read as its type.
pub fn media(reader: &Reader) -> Result<Media, Refusal> {
    Ok(Media {
        media: rows(
            reader,
            "SELECT fname, csum, mtime, dirty FROM media",
            |row| {
                Ok(MediaRow {
                    fname: row.get(0)?,
                    csum: row.get(1)?,
                    mtime: row.get(2)?,
                    dirty: row.get(3)?,
                })
            },
        )?,
        meta: rows(reader, "SELECT dirMod, lastUsn FROM meta", |row| {
            Ok(MetaRow {
                dir_mod: row.get(0)?,
                last_usn: row.get(1)?,
            })
        })?,
    })
}
