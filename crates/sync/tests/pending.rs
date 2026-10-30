//! A pending operation leaves the queue only on its judgement (SPEC-509 A04; the design's T2-4):
//! the server's verdict on it, or its appearance in a pull. Sending it, sending it again and a
//! retryable refusal each leave it pending; an acceptance, a duplicate and a pull acknowledge it;
//! a terminal rejection moves it, bytes and reason, to the rejections the owner sees.

// An integration test is test code: it prints the examined count on purpose.
#![allow(clippy::print_stdout)]

use deck_streak_sync::{PendingQueue, Refusal, Rejected, Rejection, Settled, Stage, Verdict};

const ME: [u8; 16] = [1; 16];
const OTHER: [u8; 16] = [2; 16];

fn frame(seq: u64) -> Vec<u8> {
    format!("op {seq}").into_bytes()
}

/// The queue's pending operations as sequence numbers and stages.
fn pending(queue: &PendingQueue) -> Vec<(u64, Stage)> {
    queue.pending().map(|op| (op.seq(), op.stage())).collect()
}

#[test]
fn a_pending_operation_leaves_the_queue_only_on_its_verdict_or_its_appearance_in_a_pull() {
    let mut queue = PendingQueue::new(ME);
    for seq in 1..=5 {
        queue.enqueue(seq, frame(seq));
    }
    let mut transitions = 0_u64;

    // Sending marks the oldest operations sent and removes none.
    let sent = queue.send(3);
    assert_eq!(sent, vec![(1, frame(1)), (2, frame(2)), (3, frame(3))]);
    assert_eq!(
        pending(&queue),
        vec![
            (1, Stage::Sent),
            (2, Stage::Sent),
            (3, Stage::Sent),
            (4, Stage::Queued),
            (5, Stage::Queued),
        ]
    );
    transitions += 1;

    // A retryable refusal keeps the operation, queued to be sent again.
    for refusal in [
        Refusal::Gap,
        Refusal::ClockAhead,
        Refusal::BlobMissing,
        Refusal::BelowMinimumClient,
        Refusal::Busy,
    ] {
        assert_eq!(queue.on_verdict(1, Verdict::Refused(refusal)), None);
        assert_eq!(pending(&queue)[0], (1, Stage::Queued), "{refusal:?}");
        transitions += 1;
    }

    // An acceptance and a duplicate acknowledge the operation, which leaves.
    assert_eq!(
        queue.on_verdict(2, Verdict::Accepted { server_seq: 40 }),
        Some(Settled::Acknowledged {
            seq: 2,
            server_seq: 40
        })
    );
    assert_eq!(
        queue.on_verdict(3, Verdict::Duplicate { server_seq: 41 }),
        Some(Settled::Acknowledged {
            seq: 3,
            server_seq: 41
        })
    );
    assert_eq!(
        pending(&queue),
        vec![(1, Stage::Queued), (4, Stage::Queued), (5, Stage::Queued)]
    );
    transitions += 2;

    // Another device's operation in a pull changes nothing; the device's own leaves, sent or not.
    assert_eq!(queue.on_pulled(&OTHER, 4, 42), None);
    assert_eq!(pending(&queue).len(), 3);
    assert_eq!(
        queue.on_pulled(&ME, 4, 43),
        Some(Settled::Acknowledged {
            seq: 4,
            server_seq: 43
        })
    );
    assert_eq!(
        pending(&queue),
        vec![(1, Stage::Queued), (5, Stage::Queued)]
    );
    transitions += 2;

    // A terminal rejection moves the operation, bytes and reason, to the rejections.
    assert_eq!(queue.send(10), vec![(1, frame(1)), (5, frame(5))]);
    let rejected = Rejected {
        seq: 5,
        reason: Rejection::Malformed,
        frame: frame(5),
    };
    assert_eq!(
        queue.on_verdict(5, Verdict::Rejected(Rejection::Malformed)),
        Some(Settled::Rejected(rejected.clone()))
    );
    assert_eq!(queue.rejected(), [rejected].as_slice());
    assert_eq!(pending(&queue), vec![(1, Stage::Sent)]);
    transitions += 2;

    // A verdict or a pull for an operation no longer pending changes nothing.
    assert_eq!(
        queue.on_verdict(2, Verdict::Accepted { server_seq: 40 }),
        None
    );
    assert_eq!(queue.on_pulled(&ME, 5, 44), None);
    assert_eq!(queue.rejected().len(), 1);
    transitions += 2;

    // Sent again after a lost answer, the operation is still pending until it is judged.
    assert_eq!(queue.send(1), vec![(1, frame(1))]);
    assert_eq!(queue.send(1), vec![(1, frame(1))]);
    assert_eq!(pending(&queue), vec![(1, Stage::Sent)]);
    assert_eq!(
        queue.on_verdict(1, Verdict::Accepted { server_seq: 45 }),
        Some(Settled::Acknowledged {
            seq: 1,
            server_seq: 45
        })
    );
    assert_eq!(pending(&queue), Vec::new());
    assert_eq!(queue.pending().next().map(|op| op.frame().len()), None);
    transitions += 2;

    println!("examined {transitions} transition(s) of a pending operation");
}

#[test]
fn every_terminal_reason_keeps_the_bytes_and_every_pending_operation_keeps_its_frame() {
    let reasons = [
        Rejection::Malformed,
        Rejection::IdTaken,
        Rejection::Gone,
        Rejection::ReviewedNoteDelete,
        Rejection::UndoStale,
        Rejection::SeqReuse,
    ];
    let mut queue = PendingQueue::new(ME);
    for (seq, _) in (1..).zip(reasons) {
        queue.enqueue(seq, frame(seq));
    }
    let frames: Vec<Vec<u8>> = queue.pending().map(|op| op.frame().to_vec()).collect();
    assert_eq!(frames, (1..=6).map(frame).collect::<Vec<_>>());
    for (seq, reason) in (1..).zip(reasons) {
        assert!(matches!(
            queue.on_verdict(seq, Verdict::Rejected(reason)),
            Some(Settled::Rejected(Rejected { seq: s, reason: r, .. })) if s == seq && r == reason
        ));
    }
    let kept: Vec<(u64, Rejection, Vec<u8>)> = queue
        .rejected()
        .iter()
        .map(|rejected| (rejected.seq, rejected.reason, rejected.frame.clone()))
        .collect();
    assert_eq!(
        kept,
        (1..)
            .zip(reasons)
            .map(|(seq, reason)| (seq, reason, frame(seq)))
            .collect::<Vec<_>>()
    );
    assert_eq!(pending(&queue), Vec::new());
    println!("examined {} terminal reason(s)", reasons.len());
}
