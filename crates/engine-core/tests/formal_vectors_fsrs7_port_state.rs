//! SPEC-413 A7 (R11): the engine core's FSRS-7 port state gives every vector the Lean port writes.
//! For each fold line of `formal/vectors/fsrs7-port-state.jsonl`, written by
//! `formal/lean/Formal/Fsrs7PortStateVectors.lean` from the port in
//! `formal/lean/Formal/Fsrs7PortState.lean`, folding the line's events in turn into the default
//! model's new state gives each recorded answer, the state's checkpoint is the recorded bytes, and
//! restoring them gives the recorded answer and, for a state, the same checkpoint again; for each
//! restore line, restoring the bytes gives the recorded answer. The port is reached only through
//! `Fsrs7`'s public `SchedulerPort` methods. The vectors assume a 64-bit `usize`.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "an integration test panics on a malformed vector, and prints its examined count on \
              purpose"
)]

use deck_streak_engine_core::replay::Fsrs7;
use deck_streak_engine_core::scheduler::{
    EventKey, FoldError, LoadError, ReviewEvent, SchedulerPort,
};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/fsrs7-port-state.jsonl");

/// An integer the vector holds.
fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer, not {value}"))
}

/// An event as its eight signed fields.
fn event(value: &Value) -> ReviewEvent {
    let fields = value.as_array().expect("an event is an array");
    let [card, note, deck, preset, grade, answer_ms, key, study_day] = fields.as_slice() else {
        panic!("an event has eight fields: {value}");
    };
    ReviewEvent {
        card: int(card),
        note: int(note),
        deck: int(deck),
        preset: int(preset),
        grade: u8::try_from(int(grade)).expect("a grade fits a u8"),
        answer_ms: u32::try_from(int(answer_ms)).expect("an answer time fits a u32"),
        key: EventKey(int(key)),
        study_day: i32::try_from(int(study_day)).expect("a study day fits an i32"),
    }
}

/// Bytes the vector holds as lowercase hex.
fn bytes(value: &Value) -> Vec<u8> {
    let text = value.as_str().expect("bytes are a hex string");
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let digits = std::str::from_utf8(pair).expect("hex is ASCII");
            u8::from_str_radix(digits, 16).unwrap_or_else(|_| panic!("a hex byte, not {digits}"))
        })
        .collect()
}

/// A fold refusal's kind, as the writer names it.
fn fold_kind(error: FoldError) -> String {
    match error {
        FoldError::OtherModel => "other_model".to_owned(),
        FoldError::KeyNotAfter { .. } => "key_not_after".to_owned(),
        FoldError::GradeOutOfRange { .. } => "grade_out_of_range".to_owned(),
        FoldError::AnswerTooLong { .. } => "answer_too_long".to_owned(),
        other => format!("unexpected {other:?}"),
    }
}

/// A restore's answer as the writer names it, and the restored state's checkpoint.
fn restored(port: &Fsrs7, checkpoint: &[u8]) -> (String, Option<Vec<u8>>) {
    match port.restore(checkpoint) {
        Ok(state) => {
            let again = port.checkpoint(&state);
            (
                format!("ok:{}", again.len().saturating_sub(45) / 49),
                Some(again),
            )
        }
        Err(LoadError::Truncated) => ("truncated".to_owned(), None),
        Err(LoadError::NotACheckpoint) => ("not_a_checkpoint".to_owned(), None),
        Err(LoadError::OtherModel) => ("other_model".to_owned(), None),
        Err(LoadError::Refused(error)) => (format!("refused:{}", fold_kind(error)), None),
    }
}

/// One fold vector: `None` when the port gives every recorded answer and byte, else the first
/// difference.
fn fold_differs(port: &Fsrs7, vector: &Value) -> Option<String> {
    let events = vector["events"]
        .as_array()
        .expect("the events are an array");
    let answers = vector["answers"]
        .as_array()
        .expect("the answers are an array");
    if events.len() != answers.len() {
        return Some(format!(
            "{} events and {} answers",
            events.len(),
            answers.len()
        ));
    }
    let mut state = port.new_state();
    for (at, (folded, answer)) in events.iter().zip(answers).enumerate() {
        let given = match port.fold(&mut state, &event(folded)) {
            Ok(()) => "ok".to_owned(),
            Err(error) => fold_kind(error),
        };
        if answer.as_str() != Some(given.as_str()) {
            return Some(format!("fold {at} answers {given}"));
        }
    }
    let checkpoint = port.checkpoint(&state);
    if checkpoint != bytes(&vector["checkpoint"]) {
        return Some(format!("the checkpoint is {checkpoint:02x?}"));
    }
    let (answer, again) = restored(port, &checkpoint);
    if vector["restore"].as_str() != Some(answer.as_str()) {
        return Some(format!("its restore answers {answer}"));
    }
    again
        .is_some_and(|again| again != checkpoint)
        .then(|| "the restored state's checkpoint is other bytes".to_owned())
}

/// One restore vector: `None` when the port gives the recorded answer, else the answer.
fn restore_differs(port: &Fsrs7, vector: &Value) -> Option<String> {
    let checkpoint = bytes(&vector["bytes"]);
    let (answer, again) = restored(port, &checkpoint);
    if vector["restore"].as_str() != Some(answer.as_str()) {
        return Some(format!("the restore answers {answer}"));
    }
    again
        .is_some_and(|again| again != checkpoint)
        .then(|| "the restored state's checkpoint is other bytes".to_owned())
}

#[test]
fn the_engine_cores_port_state_gives_every_lean_vectors_output() {
    let port = Fsrs7::new(&[]).expect("the pinned revision's defaults make a model");
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("the header parses");
    assert_eq!(header["schema"], "phx.formal.vectors.v1");
    assert_eq!(header["entry"], "Fsrs7PortState");
    assert_eq!(
        header["covers"], "crates/engine-core/src/replay.rs",
        "the vectors were written from the entry that covers the port's fold"
    );
    assert_eq!(header["anchor"], "fold");
    let declared = header["vectors"].as_u64().expect("a declared count");

    let mut examined = 0_u64;
    let mut folds = 0_u64;
    let mut restores = 0_u64;
    let mut differing = Vec::new();
    for (at, line) in lines.enumerate() {
        let vector: Value = serde_json::from_str(line).expect("a vector parses");
        let differs = if vector.get("events").is_some() {
            folds += 1;
            fold_differs(&port, &vector)
        } else {
            restores += 1;
            restore_differs(&port, &vector)
        };
        if let Some(why) = differs {
            differing.push(format!("line {}: {line}: {why}", at + 2));
        }
        examined += 1;
    }
    println!(
        "examined {examined} vector(s): {folds} fold sequence(s), {restores} restore(s), {} \
         differing",
        differing.len()
    );
    assert!(examined > 0, "examined 0 vectors: nothing was judged");
    assert!(
        folds > 0 && restores > 0,
        "both halves are judged: {folds} fold sequence(s), {restores} restore(s)"
    );
    assert_eq!(examined, declared, "the header declares {declared} vectors");
    assert!(
        differing.is_empty(),
        "{} of {examined} vectors differ; the first: {}",
        differing.len(),
        differing.first().map_or("", String::as_str)
    );
}

/// The recorded counterexamples, as literal cases: a key of -1 then 0 folds both, as the signed
/// order reads them (an unsigned read of the bit patterns refuses the second); the same key again
/// is refused and leaves the state's checkpoint as it was; that checkpoint restores to a state
/// with the same checkpoint; and the new state's checkpoint with its format byte changed is not a
/// checkpoint.
#[test]
fn the_recorded_counterexamples_hold_as_literal_cases() {
    let port = Fsrs7::new(&[]).expect("the pinned revision's defaults make a model");
    let at = |key| ReviewEvent {
        card: 1,
        note: 2,
        deck: 3,
        preset: 4,
        grade: 3,
        answer_ms: 1000,
        key: EventKey(key),
        study_day: 0,
    };
    let mut state = port.new_state();
    assert_eq!(port.fold(&mut state, &at(-1)), Ok(()));
    assert_eq!(port.fold(&mut state, &at(0)), Ok(()));
    let before = port.checkpoint(&state);
    assert_eq!(before.len(), 45 + 2 * 49);
    assert!(matches!(
        port.fold(&mut state, &at(0)),
        Err(FoldError::KeyNotAfter { .. })
    ));
    assert_eq!(port.checkpoint(&state), before);
    let again = port.restore(&before).expect("the checkpoint restores");
    assert_eq!(port.checkpoint(&again), before);
    let mut other_format = port.checkpoint(&port.new_state());
    assert_eq!(other_format.len(), 45);
    if let Some(format) = other_format.get_mut(4) {
        *format = 2;
    }
    assert_eq!(
        port.restore(&other_format).err(),
        Some(LoadError::NotACheckpoint)
    );
}
