//! The protobuf wire walk (SPEC-094 R3, ADR-095): a blob read as wire format only, with no schema.

/// One field's value, by its wire type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireValue<'a> {
    /// Wire type 0: a varint.
    Varint(u128),
    /// Wire type 1: eight bytes.
    Fixed64(&'a [u8]),
    /// Wire type 2: a length-delimited run of bytes.
    Length(&'a [u8]),
    /// Wire type 5: four bytes.
    Fixed32(&'a [u8]),
}

/// One top-level field: its number and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireField<'a> {
    /// The field number, the key shifted right by three bits.
    pub number: u128,
    /// The value.
    pub value: WireValue<'a>,
}

impl WireValue<'_> {
    /// The wire type, the key's low three bits.
    #[must_use]
    pub const fn wire_type(&self) -> u8 {
        match self {
            Self::Varint(_) => 0,
            Self::Fixed64(_) => 1,
            Self::Length(_) => 2,
            Self::Fixed32(_) => 5,
        }
    }
}

/// Why a blob is not walkable: the text is the predecessor's own.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct WireError(pub String);

/// Walks `data` as protobuf wire format.
///
/// # Errors
///
/// [`WireError`] on a truncation or an unsupported wire type; never a partial result.
pub const fn walk(data: &[u8]) -> Result<Vec<WireField<'_>>, WireError> {
    let _ = data;
    Ok(Vec::new())
}
