//! The field merge answers every vector the Lean port writes, byte for byte (SPEC-509 A05): for
//! each list of edits in `formal/vectors/field-merge.jsonl`, written by
//! `formal/lean/Formal/FieldMerge.lean`'s writer, `apply` gives the port's register, and the line
//! rendered from the crate's answer equals the file's line.
//!
//! The port's edit is `(hlc, origin, seq, val)` with the origin and the value numbers. An edit's
//! key has rank 0, and the origin number is written big-endian in the key's 16 bytes, so byte
//! order is number order.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_sync::{Edit, Key, apply};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/field-merge.jsonl");

/// A port's edit `[hlc, origin, seq, val]` as an edit of a numeric register.
fn edit(quad: &Value) -> Edit<u64> {
    let parts = quad.as_array().expect("an edit is an array");
    let natural = |at: usize| {
        parts[at]
            .as_u64()
            .unwrap_or_else(|| panic!("a natural number at {at} in {quad}"))
    };
    Edit {
        key: Key {
            t: i64::try_from(natural(0)).expect("a clock in range"),
            rank: 0,
            origin: u128::from(natural(1)).to_be_bytes(),
            seq: natural(2),
        },
        value: natural(3),
    }
}

/// An edit as the port writes it.
fn render(edit: &Edit<u64>) -> String {
    format!(
        "[{},{},{},{}]",
        edit.key.t,
        u128::from_be_bytes(edit.key.origin),
        edit.key.seq,
        edit.value
    )
}

#[test]
fn apply_merges_every_lean_field_merge_vector_byte_equal() {
    let mut lines = VECTORS.lines();
    let header = lines.next().expect("a header line");
    let mut examined = 0_u64;
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        let edits: Vec<Edit<u64>> = vector["edits"]
            .as_array()
            .expect("a list of edits")
            .iter()
            .map(edit)
            .collect();
        let written: Vec<String> = edits.iter().map(render).collect();
        let register = apply(edits).map_or_else(|| "null".to_owned(), |edit| render(&edit));
        assert_eq!(
            format!(
                "{{\"edits\":[{}],\"register\":{register}}}",
                written.join(",")
            ),
            line,
            "the crate's line for {vector}"
        );
        examined += 1;
    }
    println!("examined {examined} field merge vector(s)");
    assert_eq!(examined, 6, "every input the writer prints");
    let header: Value = serde_json::from_str(header).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "FieldMerge", "{header}");
    assert_eq!(header["covers"], "crates/sync/src/merge.rs", "{header}");
    assert_eq!(header["anchor"], "merge", "{header}");
    assert_eq!(header["vectors"].as_u64(), Some(examined), "{header}");
}
