//! The 8 blob columns as typed views beside their raw bytes (SPEC-602 R7, ADR-622 D2).
//!
//! A view models only the fields it names; every other field rides in the raw bytes. A view that
//! is unchanged keeps its raw bytes byte for byte; a changed view is encoded onto them.

use crate::wire::{self, WireField, WireValue};

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
        if V::decode(&self.raw)? == self.view {
            Ok(self.raw.clone())
        } else {
            self.view.encode_onto(&self.raw)
        }
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

/// The fields of one message, walked as wire format.
struct Message<'a> {
    column: &'static str,
    fields: Vec<WireField<'a>>,
}

impl<'a> Message<'a> {
    fn walk(column: &'static str, raw: &'a [u8]) -> Result<Self, BlobError> {
        let fields = wire::walk(raw).map_err(|_| BlobError { column })?;
        Ok(Self { column, fields })
    }

    /// The last value of field `number`, the protobuf rule for a scalar.
    fn last(&self, number: u128) -> Option<&WireValue<'a>> {
        self.fields
            .iter()
            .rev()
            .find(|field| field.number == number)
            .map(|field| &field.value)
    }

    fn present(&self, number: u128) -> bool {
        self.last(number).is_some()
    }

    fn varint(&self, number: u128) -> Result<Option<u128>, BlobError> {
        match self.last(number) {
            None => Ok(None),
            Some(WireValue::Varint(value)) => Ok(Some(*value)),
            Some(_) => Err(BlobError {
                column: self.column,
            }),
        }
    }

    fn uint32(&self, number: u128) -> Result<Option<u32>, BlobError> {
        self.varint(number)?
            .map(|value| {
                u32::try_from(value).map_err(|_| BlobError {
                    column: self.column,
                })
            })
            .transpose()
    }

    fn int64(&self, number: u128) -> Result<i64, BlobError> {
        let Some(value) = self.varint(number)? else {
            return Ok(0);
        };
        let value = u64::try_from(value).map_err(|_| BlobError {
            column: self.column,
        })?;
        Ok(i64::from_ne_bytes(value.to_ne_bytes()))
    }

    fn flag(&self, number: u128) -> Result<bool, BlobError> {
        Ok(self.varint(number)?.is_some_and(|value| value != 0))
    }

    fn bytes(&self, number: u128) -> Result<Option<&'a [u8]>, BlobError> {
        match self.last(number) {
            None => Ok(None),
            Some(WireValue::Length(bytes)) => Ok(Some(bytes)),
            Some(_) => Err(BlobError {
                column: self.column,
            }),
        }
    }

    fn text(&self, number: u128) -> Result<String, BlobError> {
        self.bytes(number)?.map_or_else(
            || Ok(String::new()),
            |bytes| {
                String::from_utf8(bytes.to_vec()).map_err(|_| BlobError {
                    column: self.column,
                })
            },
        )
    }

    /// The message with every field in `modelled` replaced by `fields`, in field-number order.
    fn merged(&self, modelled: &[u128], fields: Vec<(u128, Vec<u8>)>) -> Vec<u8> {
        let mut kept: Vec<(u128, Vec<u8>)> = self
            .fields
            .iter()
            .filter(|field| !modelled.contains(&field.number))
            .map(|field| (field.number, encoded(field)))
            .collect();
        kept.extend(fields);
        kept.sort_by_key(|(number, _)| *number);
        kept.into_iter().flat_map(|(_, bytes)| bytes).collect()
    }
}

/// The most seven-bit groups a `u128` varint takes: 19 groups hold 133 bits.
const VARINT_GROUPS: u32 = 19;

/// `value` as a varint, in at most the groups a `u128` can take.
fn put_varint(out: &mut Vec<u8>, value: u128) {
    let mut rest = value;
    for _ in 0..VARINT_GROUPS {
        let low = u8::try_from(rest & 0x7f).unwrap_or(0);
        rest >>= 7;
        let more = rest != 0;
        out.push(if more { low | 0x80 } else { low });
        if !more {
            return;
        }
    }
}

fn put_key(out: &mut Vec<u8>, number: u128, wire_type: u8) {
    put_varint(out, (number << 3) | u128::from(wire_type));
}

/// One field as wire bytes: its key, then its value.
fn encoded(field: &WireField<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    put_key(&mut out, field.number, field.value.wire_type());
    match &field.value {
        WireValue::Varint(value) => put_varint(&mut out, *value),
        WireValue::Length(bytes) => {
            put_varint(&mut out, bytes.len() as u128);
            out.extend_from_slice(bytes);
        }
        WireValue::Fixed64(bytes) | WireValue::Fixed32(bytes) => out.extend_from_slice(bytes),
    }
    out
}

/// A varint field, omitted at zero.
fn varint_field(number: u128, value: u128) -> Option<(u128, Vec<u8>)> {
    (value != 0).then(|| present_varint(number, value))
}

/// A varint field, written even at zero: an `optional` field that is set.
fn present_varint(number: u128, value: u128) -> (u128, Vec<u8>) {
    let mut out = Vec::new();
    put_key(&mut out, number, 0);
    put_varint(&mut out, value);
    (number, out)
}

/// A length-delimited field, omitted when empty unless `always`.
fn length_field(number: u128, bytes: &[u8], always: bool) -> Option<(u128, Vec<u8>)> {
    (always || !bytes.is_empty()).then(|| {
        let mut out = Vec::new();
        put_key(&mut out, number, 2);
        put_varint(&mut out, bytes.len() as u128);
        out.extend_from_slice(bytes);
        (number, out)
    })
}

fn signed(value: i64) -> u128 {
    u128::from(u64::from_ne_bytes(value.to_ne_bytes()))
}

impl BlobView for ConfigValue {
    const COLUMN: &'static str = "config.val";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        String::from_utf8(raw.to_vec())
            .map(|json| Self { json })
            .map_err(|_| BlobError {
                column: Self::COLUMN,
            })
    }

    fn encode_onto(&self, _base: &[u8]) -> Result<Vec<u8>, BlobError> {
        Ok(self.json.as_bytes().to_vec())
    }
}

impl BlobView for DeckConfigView {
    const COLUMN: &'static str = "deck_config.config";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        let message = Message::walk(Self::COLUMN, raw)?;
        Ok(Self {
            new_per_day: message.uint32(9)?.unwrap_or(0),
            reviews_per_day: message.uint32(10)?.unwrap_or(0),
        })
    }

    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError> {
        let message = Message::walk(Self::COLUMN, base)?;
        let fields = [
            varint_field(9, self.new_per_day.into()),
            varint_field(10, self.reviews_per_day.into()),
        ];
        Ok(message.merged(&[9, 10], fields.into_iter().flatten().collect()))
    }
}

impl BlobView for DeckCommonView {
    const COLUMN: &'static str = "decks.common";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        let message = Message::walk(Self::COLUMN, raw)?;
        Ok(Self {
            study_collapsed: message.flag(1)?,
            browser_collapsed: message.flag(2)?,
        })
    }

    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError> {
        let message = Message::walk(Self::COLUMN, base)?;
        let fields = [
            varint_field(1, self.study_collapsed.into()),
            varint_field(2, self.browser_collapsed.into()),
        ];
        Ok(message.merged(&[1, 2], fields.into_iter().flatten().collect()))
    }
}

impl BlobView for DeckKindView {
    const COLUMN: &'static str = "decks.kind";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        let message = Message::walk(Self::COLUMN, raw)?;
        let normal = Message::walk(Self::COLUMN, message.bytes(1)?.unwrap_or_default())?;
        Ok(Self {
            filtered: message.present(2),
            config_id: normal.int64(1)?,
            review_limit: normal.uint32(6)?,
            new_limit: normal.uint32(7)?,
        })
    }

    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError> {
        let message = Message::walk(Self::COLUMN, base)?;
        if self.filtered {
            let filtered = message.bytes(2)?.unwrap_or_default();
            let fields = length_field(2, filtered, true).into_iter().collect();
            return Ok(message.merged(&[1, 2], fields));
        }
        let normal = Message::walk(Self::COLUMN, message.bytes(1)?.unwrap_or_default())?;
        let normal_fields = [
            varint_field(1, signed(self.config_id)),
            self.review_limit
                .map(|limit| present_varint(6, limit.into())),
            self.new_limit.map(|limit| present_varint(7, limit.into())),
        ];
        let normal = normal.merged(&[1, 6, 7], normal_fields.into_iter().flatten().collect());
        let fields = length_field(1, &normal, true).into_iter().collect();
        Ok(message.merged(&[1, 2], fields))
    }
}

impl BlobView for FieldConfigView {
    const COLUMN: &'static str = "fields.config";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        let message = Message::walk(Self::COLUMN, raw)?;
        Ok(Self {
            font_name: message.text(3)?,
            font_size: message.uint32(4)?.unwrap_or(0),
        })
    }

    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError> {
        let message = Message::walk(Self::COLUMN, base)?;
        let fields = [
            length_field(3, self.font_name.as_bytes(), false),
            varint_field(4, self.font_size.into()),
        ];
        Ok(message.merged(&[3, 4], fields.into_iter().flatten().collect()))
    }
}

impl BlobView for NotetypeConfigView {
    const COLUMN: &'static str = "notetypes.config";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        let message = Message::walk(Self::COLUMN, raw)?;
        Ok(Self {
            kind: message.uint32(1)?.unwrap_or(0),
            sort_field: message.uint32(2)?.unwrap_or(0),
            css: message.text(3)?,
        })
    }

    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError> {
        let message = Message::walk(Self::COLUMN, base)?;
        let fields = [
            varint_field(1, self.kind.into()),
            varint_field(2, self.sort_field.into()),
            length_field(3, self.css.as_bytes(), false),
        ];
        Ok(message.merged(&[1, 2, 3], fields.into_iter().flatten().collect()))
    }
}

impl BlobView for TagConfigView {
    const COLUMN: &'static str = "tags.config";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        Message::walk(Self::COLUMN, raw)?;
        Ok(Self {})
    }

    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError> {
        let message = Message::walk(Self::COLUMN, base)?;
        Ok(message.merged(&[], Vec::new()))
    }
}

impl BlobView for TemplateConfigView {
    const COLUMN: &'static str = "templates.config";

    fn decode(raw: &[u8]) -> Result<Self, BlobError> {
        let message = Message::walk(Self::COLUMN, raw)?;
        Ok(Self {
            q_format: message.text(1)?,
            a_format: message.text(2)?,
            browser_font_size: message.uint32(7)?.unwrap_or(0),
        })
    }

    fn encode_onto(&self, base: &[u8]) -> Result<Vec<u8>, BlobError> {
        let message = Message::walk(Self::COLUMN, base)?;
        let fields = [
            length_field(1, self.q_format.as_bytes(), false),
            length_field(2, self.a_format.as_bytes(), false),
            varint_field(7, self.browser_font_size.into()),
        ];
        Ok(message.merged(&[1, 2, 7], fields.into_iter().flatten().collect()))
    }
}
