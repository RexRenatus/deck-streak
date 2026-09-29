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

/// The longest a varint may run, in bits of shift: the predecessor refuses one at or past it.
const VARINT_LIMIT: u32 = 70;

/// One varint at `at`, and the position after it.
fn varint(data: &[u8], mut at: usize) -> Result<(u128, usize), WireError> {
    let mut result: u128 = 0;
    let mut shift: u32 = 0;
    loop {
        let Some(&byte) = data.get(at) else {
            return Err(WireError("truncated varint".to_owned()));
        };
        at += 1;
        result |= u128::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, at));
        }
        shift += 7;
        if shift >= VARINT_LIMIT {
            return Err(WireError("varint too long".to_owned()));
        }
    }
}

/// The `width` bytes at `at`, or the truncation `what` names.
fn fixed<'a>(data: &'a [u8], at: usize, width: usize, what: &str) -> Result<&'a [u8], WireError> {
    at.checked_add(width)
        .and_then(|end| data.get(at..end))
        .ok_or_else(|| WireError(format!("truncated {what} field")))
}

/// Walks `data` as protobuf wire format: every top-level field, in order.
///
/// # Errors
///
/// [`WireError`] on a truncation or an unsupported wire type; never a partial result.
pub fn walk(data: &[u8]) -> Result<Vec<WireField<'_>>, WireError> {
    let mut at = 0;
    let mut fields = Vec::new();
    while at < data.len() {
        let (key, next) = varint(data, at)?;
        at = next;
        let number = key >> 3;
        let value = match key & 0x7 {
            0 => {
                let (value, next) = varint(data, at)?;
                at = next;
                WireValue::Varint(value)
            }
            1 => {
                let bytes = fixed(data, at, 8, "64-bit")?;
                at += 8;
                WireValue::Fixed64(bytes)
            }
            2 => {
                let (length, next) = varint(data, at)?;
                let length = usize::try_from(length).unwrap_or(usize::MAX);
                let bytes = fixed(data, next, length, "length-delimited")?;
                at = next + length;
                WireValue::Length(bytes)
            }
            5 => {
                let bytes = fixed(data, at, 4, "32-bit")?;
                at += 4;
                WireValue::Fixed32(bytes)
            }
            other => return Err(WireError(format!("unsupported wire type {other}"))),
        };
        fields.push(WireField { number, value });
    }
    Ok(fields)
}
