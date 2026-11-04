//! The protobuf wire walk: a blob read as wire format only, with no schema.
//!
//! Red-first stand-in: the walk answers an empty list. The walk itself moves here from ingest.

/// One field's value, by wire type.
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

/// Why a blob is not walkable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct WireError(pub String);

/// The refusal text of a pass that did not move the walk forward.
pub const NO_PROGRESS: &str = "wire walk made no progress";

/// Walks `data` as protobuf wire format: every top-level field, in order.
///
/// # Errors
///
/// [`WireError`] on a truncation or an unsupported wire type.
pub fn walk(data: &[u8]) -> Result<Vec<WireField<'_>>, WireError> {
    let _ = data;
    Ok(Vec::new())
}

/// The walk with its per-field reader injected.
///
/// # Errors
///
/// The reader's own [`WireError`], or [`NO_PROGRESS`] when a pass stays or steps back.
pub fn walk_with<'a, F>(data: &'a [u8], step: F) -> Result<Vec<WireField<'a>>, WireError>
where
    F: FnMut(&'a [u8], usize) -> Result<(WireField<'a>, usize), WireError>,
{
    let _ = (data, step);
    Ok(Vec::new())
}
