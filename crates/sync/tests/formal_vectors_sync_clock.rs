//! The hybrid clock answers every vector the Lean port writes, byte for byte (SPEC-509 A02; the
//! design's T2-2): for each input in `formal/vectors/sync-clock.jsonl`, written by
//! `formal/lean/Formal/SyncClock.lean`'s writer, `tick` and a device's clock that `recv`s the
//! seen clock and then ticks give the port's clock, and the line rendered from the crate's answer
//! equals the file's line.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_sync::{Hlc, tick};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/sync-clock.jsonl");

fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

#[test]
fn tick_and_recv_answer_every_lean_clock_vector_byte_equal() {
    let mut lines = VECTORS.lines();
    let header = lines.next().expect("a header line");
    let mut examined = 0_u64;
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        let (last, seen, pt) = (
            number(&vector, "last"),
            number(&vector, "seen"),
            number(&vector, "pt"),
        );
        let pure = tick(last, seen, pt).expect("a small clock ticks");
        let mut clock = Hlc::starting_at(last);
        clock.recv(seen);
        let made = clock.tick_at(pt).expect("a small clock ticks");
        assert_eq!(clock.last(), made, "the clock keeps the tick it made");
        for answer in [pure, made] {
            assert_eq!(
                format!("{{\"last\":{last},\"seen\":{seen},\"pt\":{pt},\"tick\":{answer}}}"),
                line,
                "the crate's line for {vector}"
            );
        }
        examined += 1;
    }
    println!("examined {examined} clock vector(s)");
    assert_eq!(examined, 64, "every input the writer prints");
    let header: Value = serde_json::from_str(header).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "SyncClock", "{header}");
    assert_eq!(header["covers"], "crates/sync/src/clock.rs", "{header}");
    assert_eq!(header["anchor"], "tick", "{header}");
    assert_eq!(header["vectors"].as_u64(), Some(examined), "{header}");
}
