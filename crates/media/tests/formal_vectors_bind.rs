//! The write-once machine answers every vector its Lean port writes (SPEC-513 A3): for each line
//! of `formal/vectors/media-bind.jsonl`, written by `formal/lean/Formal/MediaBind.lean`'s port,
//! `step` or `release_step` gives the same slot after and the same outcome. A hash `n` of the
//! port is the hash whose 32 bytes are all `n`.

#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_media::{
    BindRefusal, BindState, Binding, Bound, MediaHash, Release, ReleaseRefusal, release_step, step,
};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/media-bind.jsonl");

fn hash(value: &Value) -> MediaHash {
    let byte = u8::try_from(value.as_u64().expect("a hash number")).expect("a small hash");
    MediaHash::from_bytes([byte; 32])
}

fn slot(value: &Value) -> Option<Binding> {
    if value.is_null() {
        return None;
    }
    let state = match value["state"].as_str().expect("a state") {
        "live" => BindState::Live,
        "released" => BindState::Released,
        other => panic!("a state name: {other}"),
    };
    Some(Binding {
        hash: hash(&value["hash"]),
        state,
    })
}

fn bind_name(outcome: Result<Bound, BindRefusal>) -> &'static str {
    match outcome {
        Ok(Bound::New) => "new",
        Ok(Bound::Unchanged) => "unchanged",
        Ok(Bound::Revived) => "revived",
        Err(BindRefusal::Mismatch) => "mismatch",
        Err(BindRefusal::Conflict) => "conflict",
    }
}

fn release_name(outcome: Result<Release, ReleaseRefusal>) -> &'static str {
    match outcome {
        Ok(Release::Released) => "released",
        Ok(Release::AlreadyReleased) => "already_released",
        Err(ReleaseRefusal::Unbound) => "unbound",
    }
}

#[test]
fn step_and_release_step_answer_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("the header is JSON");
    assert_eq!(header["schema"], "phx.formal.vectors.v1");
    assert_eq!(header["entry"], "MediaBind");
    assert_eq!(header["covers"], "crates/media/src/bind.rs");
    assert_eq!(header["anchor"], "step");
    let rows: Vec<Value> = lines
        .map(|line| serde_json::from_str(line).expect("a vector is JSON"))
        .collect();
    assert_eq!(
        usize::try_from(header["vectors"].as_u64().expect("a count")).expect("a count"),
        rows.len(),
        "the header counts the lines that follow it"
    );
    let (mut binds, mut releases) = (0, 0);
    for row in &rows {
        let before = slot(&row["slot"]);
        let expected_after = slot(&row["after"]);
        let expected = row["outcome"].as_str().expect("an outcome");
        match row["op"].as_str().expect("an op") {
            "bind" => {
                let claimed = (!row["claimed"].is_null()).then(|| hash(&row["claimed"]));
                let (after, outcome) = step(before, claimed, hash(&row["hash"]));
                assert_eq!(after, expected_after, "{row}");
                assert_eq!(bind_name(outcome), expected, "{row}");
                binds += 1;
            }
            "release" => {
                let (after, outcome) = release_step(before);
                assert_eq!(after, expected_after, "{row}");
                assert_eq!(release_name(outcome), expected, "{row}");
                releases += 1;
            }
            other => panic!("an op: {other}"),
        }
    }
    println!("examined {binds} bind vector(s) and {releases} release vector(s)");
    assert_eq!((binds, releases), (30, 5));
}
