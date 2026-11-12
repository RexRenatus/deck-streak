//! The memory-state parse equals the predecessor's (SPEC-077 A1).

// An integration test is test code: it panics on a malformed golden and prints the examined count.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::memory_state::{MemoryState, parse};

/// Whether two floats agree within the golden's tolerance.
fn near(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9
}

/// The state a golden's output object holds, or none for JSON null.
fn expected(output: &serde_json::Value) -> Option<MemoryState> {
    if output.is_null() {
        return None;
    }
    Some(MemoryState {
        stability: output["stability"].as_f64().expect("stability"),
        difficulty: output["difficulty"].as_f64().expect("difficulty"),
        decay: output["decay"].as_f64().expect("decay"),
        desired_retention: output["desired_retention"].as_f64(),
        last_review_sec: output["last_review_sec"].as_i64(),
    })
}

#[test]
fn the_memory_state_parse_matches_the_predecessors_golden() {
    let examined = golden::each_case("memory_state", |case| {
        let data = case.input["data"].as_str();
        let ours = parse(data);
        let theirs = expected(&case.output);
        match (ours, theirs) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                assert!(
                    near(a.stability, b.stability)
                        && near(a.difficulty, b.difficulty)
                        && near(a.decay, b.decay)
                        && a.last_review_sec == b.last_review_sec
                        && a.desired_retention.map(|v| (v * 1e9).round())
                            == b.desired_retention.map(|v| (v * 1e9).round()),
                    "the state of {data:?}: ours {a:?}, theirs {b:?}"
                );
            }
            (a, b) => panic!("the state of {data:?}: ours {a:?}, theirs {b:?}"),
        }
    });
    assert!(examined.count > 0);
}
