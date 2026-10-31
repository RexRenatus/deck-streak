//! The web load check and the frames the web engine builds for a stored checkpoint (SPEC-511 C14,
//! C15, R9; ADR-529 D3): `load_check` keeps one valid case and refuses each of its eight discards
//! by name, a check that skips the head clause is refused by the same table, and the `Restore`
//! frame carries the stored checkpoint's bytes verbatim while an unreadable checkpoint builds none.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_companion::codec::{decode_request, encode_reply};
use deck_streak_companion::wire::{
    Adapter, CheckpointReply, HelloReply, Model, Reply, Request, Status,
};
use deck_streak_web_engine::companion::{Verdict, load_check, purge, restore};

/// The negotiated model.
const MODEL: [u64; 4] = [5, 6, 7, 8];
/// The reviews the companion folded, and the newest one's id.
const FOLDED: u64 = 12;
const HEAD: i64 = 1_736_911_810_000;
/// The companion's own state, opaque to the app: a zero byte and a byte above 0x7f among them.
const STATE: [u8; 6] = [0, 1, 0x7f, 0x80, 0xfe, 0xff];

/// A load check's inputs: the hello, the stored checkpoint and the companion's reply to `Restore`
/// as frames, whether the reply's head names a review, and the fold index held, if any.
#[derive(Clone)]
struct Case {
    hello: Vec<u8>,
    checkpoint: Vec<u8>,
    ack: Vec<u8>,
    head_named: bool,
    index: (Option<u64>, Option<i64>),
}

/// The signature of a load check: the web engine's, or a planted one.
type Check = fn(&[u8], &[u8], &[u8], bool, Option<u64>, Option<i64>) -> Verdict;

fn hello(status: Status, model: Model, adapter: Adapter) -> Vec<u8> {
    encode_reply(
        0,
        &Reply::Hello(HelloReply {
            status,
            major: 1,
            minor: 0,
            build_id: [9; 4],
            model_id: MODEL,
            model,
            adapter,
        }),
    )
    .expect("a hello frame")
}

fn checkpoint(status: Status, model_id: [u64; 4], adapter: Adapter) -> Vec<u8> {
    encode_reply(
        3,
        &Reply::Checkpoint(CheckpointReply {
            status,
            model_id,
            adapter,
            bytes: STATE.to_vec(),
        }),
    )
    .expect("a checkpoint frame")
}

fn ack(status: Status, folded: u64, head_t_ms: i64) -> Vec<u8> {
    encode_reply(
        4,
        &Reply::Ack {
            status,
            folded,
            head_t_ms,
        },
    )
    .expect("an ack frame")
}

/// The one valid case: every condition holds.
fn valid() -> Case {
    Case {
        hello: hello(Status::Ok, Model::Trained, Adapter::OwnerOnly),
        checkpoint: checkpoint(Status::Ok, MODEL, Adapter::OwnerOnly),
        ack: ack(Status::Ok, FOLDED, HEAD),
        head_named: true,
        index: (Some(FOLDED), Some(HEAD)),
    }
}

/// The table: each case, the verdict it must earn, and that verdict's code, written out here.
fn table() -> Vec<(&'static str, Case, Verdict, u8)> {
    let with = |change: &dyn Fn(&mut Case)| {
        let mut case = valid();
        change(&mut case);
        case
    };
    let cut = valid().checkpoint[..valid().checkpoint.len() - 1].to_vec();
    vec![
        ("every condition holds", valid(), Verdict::Keep, 0),
        (
            "nothing folded keeps with no index and no head",
            with(&|case| {
                case.ack = ack(Status::Ok, 0, 0);
                case.head_named = false;
                case.index = (None, None);
            }),
            Verdict::Keep,
            0,
        ),
        (
            "a fixture's hello does not negotiate",
            with(&|case| case.hello = hello(Status::Ok, Model::Fixture, Adapter::OwnerOnly)),
            Verdict::NotNegotiated,
            1,
        ),
        (
            "an absent hello does not negotiate",
            with(&|case| case.hello = hello(Status::Absent, Model::Trained, Adapter::OwnerOnly)),
            Verdict::NotNegotiated,
            1,
        ),
        (
            "a torn checkpoint is unreadable",
            with(&|case| case.checkpoint.clone_from(&cut)),
            Verdict::Unreadable,
            2,
        ),
        (
            "a checkpoint the companion did not write is unreadable",
            with(&|case| case.checkpoint = checkpoint(Status::Error, MODEL, Adapter::OwnerOnly)),
            Verdict::Unreadable,
            2,
        ),
        (
            "an ack stored as the checkpoint is unreadable",
            with(&|case| case.checkpoint = ack(Status::Ok, FOLDED, HEAD)),
            Verdict::Unreadable,
            2,
        ),
        (
            "another model's checkpoint",
            with(&|case| {
                case.checkpoint = checkpoint(Status::Ok, [5, 6, 7, 9], Adapter::OwnerOnly);
            }),
            Verdict::ModelChanged,
            3,
        ),
        (
            "a checkpoint folded under another adapter mode",
            with(&|case| case.checkpoint = checkpoint(Status::Ok, MODEL, Adapter::Off)),
            Verdict::AdapterChanged,
            4,
        ),
        (
            "the companion refused the restore",
            with(&|case| case.ack = ack(Status::Error, FOLDED, HEAD)),
            Verdict::LoadError,
            5,
        ),
        (
            "the companion answered no ack",
            with(&|case| case.ack = Vec::new()),
            Verdict::LoadError,
            5,
        ),
        (
            "the head names no review",
            with(&|case| case.head_named = false),
            Verdict::HeadNamesNoReview,
            6,
        ),
        (
            "no fold index is held",
            with(&|case| case.index = (None, None)),
            Verdict::IndexMissing,
            7,
        ),
        (
            "the fold index's count is not the ack's",
            with(&|case| case.index = (Some(FOLDED - 1), Some(HEAD))),
            Verdict::IndexMismatch,
            8,
        ),
        (
            "the fold index's head is not the ack's",
            with(&|case| case.index = (Some(FOLDED), Some(HEAD + 1))),
            Verdict::IndexMismatch,
            8,
        ),
    ]
}

/// Each row of the table `check` answers otherwise, named.
fn findings(check: Check) -> Vec<String> {
    table()
        .into_iter()
        .filter_map(|(name, case, wanted, _)| {
            let answered = check(
                &case.hello,
                &case.checkpoint,
                &case.ack,
                case.head_named,
                case.index.0,
                case.index.1,
            );
            (answered != wanted).then(|| format!("{name}: {answered:?}, not {wanted:?}"))
        })
        .collect()
}

#[test]
fn the_load_check_keeps_one_case_and_refuses_each_discard_by_name() {
    let rows = examined("load check case(s)", table());
    let verdicts: std::collections::BTreeSet<u8> =
        rows.iter().map(|(_, _, _, code)| *code).collect();
    assert_eq!(verdicts.len(), 9, "one keep and each of the eight discards");
    for (name, _, verdict, code) in &rows {
        assert_eq!(verdict.code(), *code, "{name}: {verdict:?}'s code");
    }
    assert_eq!(findings(load_check), Vec::<String>::new());
}

/// The control: a check that never asks whether the head names a review.
fn skips_the_head_clause(
    hello: &[u8],
    checkpoint: &[u8],
    ack: &[u8],
    _head_named: bool,
    index_count: Option<u64>,
    index_head: Option<i64>,
) -> Verdict {
    load_check(hello, checkpoint, ack, true, index_count, index_head)
}

#[test]
fn a_load_check_that_skips_the_head_clause_is_refused_by_the_table() {
    let refused = findings(skips_the_head_clause);
    assert_eq!(
        refused,
        vec!["the head names no review: Keep, not HeadNamesNoReview".to_owned()],
        "the table refuses the planted check by the row it breaks"
    );
}

#[test]
fn the_restore_frame_carries_the_stored_checkpoint_verbatim() {
    let stored = checkpoint(Status::Ok, MODEL, Adapter::OwnerOnly);
    let frame = restore(9, &stored);
    let decoded = decode_request(&frame).expect("a restore frame");
    assert_eq!(decoded.epoch, 9, "at the session's epoch");
    match decoded.message {
        Request::Restore(restored) => {
            assert_eq!(
                restored.bytes(),
                STATE,
                "the stored state's bytes, verbatim"
            );
        }
        other => panic!("a restore request, not {other:?}"),
    }
    let cut = stored[..stored.len() - 1].to_vec();
    let unreadable = examined(
        "unreadable checkpoint(s)",
        vec![
            ("no bytes", Vec::new()),
            ("a torn frame", cut),
            (
                "a checkpoint the companion did not write",
                checkpoint(Status::Busy, MODEL, Adapter::Off),
            ),
            ("an ack", ack(Status::Ok, FOLDED, HEAD)),
            ("a request", frame.clone()),
        ],
    );
    for (what, bytes) in unreadable {
        assert_eq!(
            restore(9, &bytes),
            Vec::<u8>::new(),
            "{what} builds no frame"
        );
    }
    let purged = decode_request(&purge(11)).expect("a purge frame");
    assert_eq!((purged.epoch, purged.message), (11, Request::Purge));
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}
