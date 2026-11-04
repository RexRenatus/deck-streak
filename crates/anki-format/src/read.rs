//! The 14 tables as `DeckStreak`'s own row types (SPEC-602 R5, R8; ADR-622 D2): the twelve
//! collection tables of the schema snapshot and the media database's two, every column kept.
//!
//! Red-first stand-in: the rows carry only the columns the tests name, and the readers answer
//! no row.

use crate::blob::{Blob, DeckKindView, NotetypeConfigView, TemplateConfigView};
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
    /// `cards.odue`.
    pub odue: i64,
    /// `cards.odid`.
    pub odid: i64,
}

/// A row of `col`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColRow {
    /// `col.ver`.
    pub ver: i64,
}

/// A row of `config`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigRow {}

/// A row of `deck_config`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckConfigRow {}

/// A row of `decks`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckRow {
    /// `decks.id`.
    pub id: i64,
    /// `decks.name`.
    pub name: String,
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
}

/// A row of `graves`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraveRow {}

/// A row of `notes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRow {
    /// `notes.id`.
    pub id: i64,
    /// `notes.guid`.
    pub guid: String,
    /// `notes.mid`.
    pub mid: i64,
    /// `notes.tags`.
    pub tags: String,
    /// `notes.flds`.
    pub flds: String,
    /// `notes.sfld`.
    pub sfld: String,
    /// `notes.csum`.
    pub csum: i64,
}

/// A row of `notetypes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotetypeRow {
    /// `notetypes.id`.
    pub id: i64,
    /// `notetypes.name`.
    pub name: String,
    /// `notetypes.config`.
    pub config: Blob<NotetypeConfigView>,
}

/// A row of `revlog`.
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
pub struct TagRow {}

/// A row of `templates`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateRow {
    /// `templates.ntid`.
    pub ntid: i64,
    /// `templates.ord`.
    pub ord: i64,
    /// `templates.name`.
    pub name: String,
    /// `templates.config`.
    pub config: Blob<TemplateConfigView>,
}

/// A row of the media database's `media`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaRow {}

/// A row of the media database's `meta`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaRow {}

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

/// Every row of the collection `reader` opened.
///
/// # Errors
///
/// A [`Refusal`] when a row does not read as its type.
pub fn collection(reader: &Reader) -> Result<Collection, Refusal> {
    let _ = reader;
    Ok(Collection::default())
}

/// Every row of the media database `reader` opened.
///
/// # Errors
///
/// A [`Refusal`] when a row does not read as its type.
pub fn media(reader: &Reader) -> Result<Media, Refusal> {
    let _ = reader;
    Ok(Media::default())
}
