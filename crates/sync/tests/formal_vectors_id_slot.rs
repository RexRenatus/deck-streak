//! The id allocator answers every vector the Lean port writes, byte for byte (SPEC-509 A07): for
//! each floor, instant and slot in `formal/vectors/id-slot.jsonl`, written by
//! `formal/lean/Formal/IdSlot.lean`'s writer, `next_id` gives the port's id, or none where the id
//! would pass 64 bits, and the line rendered from the crate's answer equals the file's line. The
//! inputs reach both ends of the 64-bit range, so the refusals are answered too.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_sync::{Slot, next_id};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/id-slot.jsonl");

fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

#[test]
fn next_id_answers_every_lean_id_slot_vector_byte_equal() {
    let mut lines = VECTORS.lines();
    let header = lines.next().expect("a header line");
    let (mut examined, mut refused) = (0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        let (floor, now, slot_number) = (
            number(&vector, "floor"),
            number(&vector, "now"),
            number(&vector, "slot"),
        );
        let slot = u8::try_from(slot_number)
            .ok()
            .and_then(Slot::new)
            .expect("a slot below 64");
        let id = next_id(floor, now, slot).map_or_else(
            |_| {
                refused += 1;
                "null".to_owned()
            },
            |id| id.to_string(),
        );
        assert_eq!(
            format!("{{\"floor\":{floor},\"now\":{now},\"slot\":{slot_number},\"id\":{id}}}"),
            line,
            "the crate's line for {vector}"
        );
        examined += 1;
    }
    println!("examined {examined} id vector(s), {refused} of them refused");
    assert_eq!(examined, 280, "every input the writer prints");
    assert!(refused > 0, "the inputs reach the top of the range");
    let header: Value = serde_json::from_str(header).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "IdSlot", "{header}");
    assert_eq!(header["covers"], "crates/sync/src/ids.rs", "{header}");
    assert_eq!(header["anchor"], "next_id", "{header}");
    assert_eq!(header["vectors"].as_u64(), Some(examined), "{header}");
}
