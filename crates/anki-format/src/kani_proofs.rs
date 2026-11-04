// The format crate's Kani harnesses (SPEC-602 R11; ADR-622 D7). Each checks the shipped function
// itself over every input of its bound. The ledger names them K-wire-walk and K-field-split; they
// stay planned until the verifier runs them.

use crate::{fields, wire};

/// K-wire-walk: the shipped walk over any 16 bytes ends, and every field it keeps carries a
/// supported wire type.
#[kani::proof]
#[kani::unwind(18)]
fn k_wire_walk() {
    let data: [u8; 16] = kani::any();
    if let Ok(found) = wire::walk(&data) {
        assert!(found.len() <= data.len());
        for field in &found {
            assert!(matches!(field.value.wire_type(), 0 | 1 | 2 | 5));
        }
    }
}

/// K-field-split: over any text of at most 8 bytes, joining its split gives the text back, and a
/// split has one more field than the text has separators.
#[kani::proof]
#[kani::unwind(10)]
fn k_field_split() {
    let bytes: [u8; 8] = kani::any();
    let length: usize = kani::any();
    kani::assume(length <= bytes.len());
    if let Ok(text) = std::str::from_utf8(&bytes[..length]) {
        let split = fields::split(text);
        assert_eq!(fields::join(&split), text);
        let separators = text
            .chars()
            .filter(|c| *c == fields::FIELD_SEPARATOR)
            .count();
        assert_eq!(split.len(), separators + 1);
    }
}
