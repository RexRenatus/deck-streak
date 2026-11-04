//! The 8 blob columns as typed views beside their raw bytes (SPEC-602 R7, ADR-622 D2).
//!
//! A view models only the fields it names; every other field rides in the raw bytes. A view that
//! is unchanged keeps its raw bytes byte for byte; a changed view is encoded onto them.
//!
//! Red-first stand-in: the codec answers a fixed wrong value.

/// Why a blob does not read as its view.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the blob of {column} does not read as its view")]
pub struct BlobError {
    /// The blob column, such as `decks.kind`.
    pub column: &'static str,
}

/// A typed view of one blob column.
pub trait BlobView: Sized + PartialEq {
    /// The column, such as `notetypes.config`.
    const COLUMN: &'static str;

    /// The view of `raw`.
    ///
    /// # Errors
    ///
    /// [`BlobError`] when `raw` does not read as the column's message.
    fn decode(raw: &[u8]) -> Result<Self, BlobError>;

    /// `base` with this view's fields replaced or inserted in field-number order, defaults omitted.
    ///
    /// # Errors
    ///
    /// [`BlobError`] when `base` does not read as the column's message.
    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError>;
}

/// A blob read from a row: its view and its raw bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blob<V> {
    /// The typed view.
    pub view: V,
    /// The bytes the row held.
    pub raw: Vec<u8>,
}

impl<V: BlobView> Blob<V> {
    /// The blob of `raw`.
    ///
    /// # Errors
    ///
    /// [`BlobError`] when `raw` does not read as the view's column.
    pub fn read(raw: &[u8]) -> Result<Self, BlobError> {
        Ok(Self {
            view: V::decode(raw)?,
            raw: raw.to_vec(),
        })
    }

    /// The bytes to store: the raw bytes when the view is unchanged, else the view on them.
    ///
    /// # Errors
    ///
    /// [`BlobError`] when the raw bytes do not read as the view's column.
    pub fn bytes(&self) -> Result<Vec<u8>, BlobError> {
        Ok(Vec::new())
    }
}

/// `config.val`: a JSON text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigValue {
    /// The JSON text, as stored.
    pub json: String,
}

/// `deck_config.config`: the daily limits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeckConfigView {
    /// Field 9: new cards a day.
    pub new_per_day: u32,
    /// Field 10: reviews a day.
    pub reviews_per_day: u32,
}

/// `decks.common`: the collapse flags.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeckCommonView {
    /// Field 1.
    pub study_collapsed: bool,
    /// Field 2.
    pub browser_collapsed: bool,
}

/// `decks.kind`: a normal deck's options and limits, or a filtered deck.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeckKindView {
    /// Field 2 is present: a filtered deck.
    pub filtered: bool,
    /// Field 1, its field 1: the deck options id of a normal deck.
    pub config_id: i64,
    /// Field 1, its field 6: the deck's own review limit.
    pub review_limit: Option<u32>,
    /// Field 1, its field 7: the deck's own new-card limit.
    pub new_limit: Option<u32>,
}

/// `fields.config`: the editor font.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldConfigView {
    /// Field 3.
    pub font_name: String,
    /// Field 4.
    pub font_size: u32,
}

/// `notetypes.config`: the kind, the sort field and the style sheet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NotetypeConfigView {
    /// Field 1: 0 standard, 1 cloze.
    pub kind: u32,
    /// Field 2: the sort field's ordinal.
    pub sort_field: u32,
    /// Field 3.
    pub css: String,
}

/// `tags.config`: no field is modelled.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TagConfigView {}

/// `templates.config`: the two sides and the browser font size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateConfigView {
    /// Field 1: the front.
    pub q_format: String,
    /// Field 2: the back.
    pub a_format: String,
    /// Field 7.
    pub browser_font_size: u32,
}

macro_rules! inert_view {
    ($view:ty, $column:literal) => {
        impl BlobView for $view {
            const COLUMN: &'static str = $column;

            fn decode(_raw: &[u8]) -> Result<Self, BlobError> {
                Ok(Self::default())
            }

            fn encode_onto(&self, _base: &[u8]) -> Result<Vec<u8>, BlobError> {
                Ok(vec![0xff])
            }
        }
    };
}

inert_view!(ConfigValue, "config.val");
inert_view!(DeckConfigView, "deck_config.config");
inert_view!(DeckCommonView, "decks.common");
inert_view!(DeckKindView, "decks.kind");
inert_view!(FieldConfigView, "fields.config");
inert_view!(NotetypeConfigView, "notetypes.config");
inert_view!(TagConfigView, "tags.config");
inert_view!(TemplateConfigView, "templates.config");
