//! The protobuf wire format, as far as the adapter's tests use it: varints and length-delimited
//! fields out, and every wire type the engine's messages use in.

/// Appends a base-128 varint.
pub fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push(u8::try_from(value & 0x7f).expect("seven bits") | 0x80);
        value >>= 7;
    }
    out.push(u8::try_from(value).expect("seven bits"));
}

/// Appends a varint field.
pub fn put_varint_field(out: &mut Vec<u8>, field: u64, value: u64) {
    put_varint(out, field << 3);
    put_varint(out, value);
}

/// Appends a length-delimited field: a string, bytes or an embedded message.
pub fn put_bytes(out: &mut Vec<u8>, field: u64, bytes: &[u8]) {
    put_varint(out, (field << 3) | 2);
    put_varint(
        out,
        u64::try_from(bytes.len()).expect("a length fits 64 bits"),
    );
    out.extend_from_slice(bytes);
}

#[derive(Debug, Clone)]
enum Value {
    Varint(u64),
    Bytes(Vec<u8>),
    Fixed,
}

fn read_varint(bytes: &[u8], at: &mut usize) -> u64 {
    let mut value = 0_u64;
    let mut shift = 0;
    loop {
        let byte = bytes[*at];
        *at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return value;
        }
        shift += 7;
    }
}

fn fields(bytes: &[u8]) -> Vec<(u64, Value)> {
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        let key = read_varint(bytes, &mut at);
        let value = match key & 7 {
            0 => Value::Varint(read_varint(bytes, &mut at)),
            1 => {
                at += 8;
                Value::Fixed
            }
            2 => {
                let length = usize::try_from(read_varint(bytes, &mut at))
                    .expect("a length fits the address space");
                let value = bytes[at..at + length].to_vec();
                at += length;
                Value::Bytes(value)
            }
            5 => {
                at += 4;
                Value::Fixed
            }
            wire => panic!("wire type {wire} is not one the engine's messages use"),
        };
        out.push((key >> 3, value));
    }
    out
}

/// A varint field's value, or zero when the message omits it (proto3's default).
pub fn varint(bytes: &[u8], field: u64) -> u64 {
    fields(bytes)
        .into_iter()
        .filter_map(|(number, value)| match value {
            Value::Varint(value) if number == field => Some(value),
            _ => None,
        })
        .next_back()
        .unwrap_or_default()
}

/// An `int64` field's value: the varint read as two's complement.
pub fn signed(bytes: &[u8], field: u64) -> i64 {
    i64::from_ne_bytes(varint(bytes, field).to_ne_bytes())
}

/// A length-delimited field's bytes, or none when the message omits it.
pub fn bytes(bytes: &[u8], field: u64) -> Vec<u8> {
    repeated(bytes, field).pop().unwrap_or_default()
}

/// Every occurrence of a length-delimited field, in order.
pub fn repeated(bytes: &[u8], field: u64) -> Vec<Vec<u8>> {
    fields(bytes)
        .into_iter()
        .filter_map(|(number, value)| match value {
            Value::Bytes(value) if number == field => Some(value),
            _ => None,
        })
        .collect()
}
