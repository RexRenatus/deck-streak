//! The key order sorts every vector the Lean port writes, byte for byte (SPEC-509 A03; the
//! design's T2-3): for each arrival order in `formal/vectors/review-key.jsonl`, written by
//! `formal/lean/Formal/ReviewKey.lean`'s writer, `sort_keys` gives the port's sorted reviews, and
//! the line rendered from the crate's answer equals the file's line.
//!
//! The port's key is `(hlc, origin, seq)` with the origin a number. A review made on a device has
//! rank 0, and the origin number is written big-endian in the key's 16 bytes, so byte order is
//! number order.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_sync::{Key, sort_keys};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/review-key.jsonl");

/// A port's key `[hlc, origin, seq]` as a review key.
fn key(triple: &Value) -> Key {
    let parts = triple.as_array().expect("a key is an array");
    let natural = |at: usize| {
        parts[at]
            .as_u64()
            .unwrap_or_else(|| panic!("a natural number at {at} in {triple}"))
    };
    Key {
        t: i64::try_from(natural(0)).expect("a clock in range"),
        rank: 0,
        origin: u128::from(natural(1)).to_be_bytes(),
        seq: natural(2),
    }
}

/// Keys as the port writes them.
fn render(keys: &[Key]) -> String {
    let parts: Vec<String> = keys
        .iter()
        .map(|key| {
            format!(
                "[{},{},{}]",
                key.t,
                u128::from_be_bytes(key.origin),
                key.seq
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

#[test]
fn sort_keys_orders_every_lean_review_key_vector_byte_equal() {
    let mut lines = VECTORS.lines();
    let header = lines.next().expect("a header line");
    let mut examined = 0_u64;
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        let mut keys: Vec<Key> = vector["arrived"]
            .as_array()
            .expect("an arrival order")
            .iter()
            .map(key)
            .collect();
        let arrived = render(&keys);
        sort_keys(&mut keys);
        assert_eq!(
            format!("{{\"arrived\":{arrived},\"sorted\":{}}}", render(&keys)),
            line,
            "the crate's line for {vector}"
        );
        examined += 1;
    }
    println!("examined {examined} review key vector(s)");
    assert_eq!(examined, 7, "every input the writer prints");
    let header: Value = serde_json::from_str(header).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "ReviewKey", "{header}");
    assert_eq!(header["covers"], "crates/sync/src/key.rs", "{header}");
    assert_eq!(header["anchor"], "sort_keys", "{header}");
    assert_eq!(header["vectors"].as_u64(), Some(examined), "{header}");
}
