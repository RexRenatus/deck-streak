//! The rule that keeps, sends and drops the sync key (SPEC-363 R3; A2 to A6).
//!
//! Each test judges one function of `deck_streak_engine_core::credential` over generations written
//! here as literals, and every expected answer is a literal written from the SPEC's rule, never
//! read from the module. A4 encodes each of the engine's error kinds with the engine's own schema.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture fails its test, and an enumerating test prints what it examined"
)]

mod support;

use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use deck_streak_engine_core::credential::{
    Generation, Kept, Outcome, classify, may_send, on_obtained, on_outcome, on_removed,
};
use prost::Message;

/// The generation `n`.
fn generation(n: u64) -> Generation {
    Generation::from(n)
}

/// The engine's encoded error of `kind`, with a message no rule may read.
fn engine_error(kind: Kind) -> Vec<u8> {
    BackendError {
        message: "a synthetic answer".to_owned(),
        kind: kind.into(),
        ..BackendError::default()
    }
    .encode_to_vec()
}

/// A2: a login is kept only at the generation it started at, and names the next one; a login that
/// started before a removal or another kept login is discarded, and none is kept at the maximum.
#[test]
fn a_login_is_kept_only_at_the_generation_it_started() {
    assert_eq!(
        on_obtained(generation(3), generation(4)),
        Kept::Discard,
        "a login started at 3 lands after a removal or another kept login raised the store to 4"
    );
    assert_eq!(
        on_obtained(generation(2), generation(5)),
        Kept::Discard,
        "a login started at 2 lands after three later writes"
    );
    assert_eq!(
        on_obtained(generation(3), generation(3)),
        Kept::Store { at: generation(4) },
        "a login started at 3 lands with the store still at 3, and is kept at 4"
    );
    assert_eq!(
        on_obtained(generation(0), generation(0)),
        Kept::Store { at: generation(1) },
        "the first login of a store that never kept a key is kept at 1"
    );
    assert_eq!(
        on_obtained(generation(u64::MAX), generation(u64::MAX)),
        Kept::Discard,
        "at the maximum no next generation exists, so nothing is kept"
    );
}

/// A3: a send is admitted only when a sealed record is stored at the generation the sender holds.
#[test]
fn a_send_needs_the_held_generation_to_be_current() {
    assert!(
        !may_send(generation(1), generation(2), true),
        "a sender holding 1 after the store moved to 2 sends nothing"
    );
    assert!(
        !may_send(generation(3), generation(2), true),
        "a sender holding a generation the store never reached sends nothing"
    );
    assert!(
        !may_send(generation(2), generation(2), false),
        "with no sealed record stored nothing is sent, whatever the sender holds"
    );
    assert!(
        may_send(generation(2), generation(2), true),
        "a sender holding the stored generation, with a sealed record stored, sends"
    );
    assert!(
        may_send(generation(0), generation(0), true),
        "the same holds at generation 0"
    );
}

/// A4: only the sync server's refusal is a refusal. Success is accepted; every other kind the
/// engine names, a kind it does not name, an empty answer and one that does not decode are
/// failures.
#[test]
fn only_the_sync_servers_refusal_is_a_refusal() {
    assert_eq!(
        classify(Some(&engine_error(Kind::NetworkError))),
        Outcome::Failed,
        "a network failure keeps the key"
    );
    assert_eq!(
        classify(Some(&engine_error(Kind::SyncOtherError))),
        Outcome::Failed,
        "another sync error keeps the key"
    );
    assert_eq!(
        classify(Some(&engine_error(Kind::SyncServerMessage))),
        Outcome::Failed,
        "a server message keeps the key"
    );
    assert_eq!(
        classify(Some(&engine_error(Kind::SyncAuthError))),
        Outcome::Refused,
        "the sync server's refusal is a refusal"
    );
    assert_eq!(classify(None), Outcome::Accepted, "success is accepted");

    let kinds = support::examined(
        "error kinds of the engine",
        (0..=24)
            .filter_map(|value| Kind::try_from(value).ok())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        kinds.len(),
        25,
        "the engine names kinds 0 to 24, every one of them"
    );
    for kind in kinds {
        let expected = if kind == Kind::SyncAuthError {
            Outcome::Refused
        } else {
            Outcome::Failed
        };
        assert_eq!(
            classify(Some(&engine_error(kind))),
            expected,
            "{kind:?} classifies as {expected:?}"
        );
    }

    let unnamed = BackendError {
        kind: 99,
        ..BackendError::default()
    }
    .encode_to_vec();
    assert_eq!(
        classify(Some(&unnamed)),
        Outcome::Failed,
        "a kind the engine does not name keeps the key"
    );
    assert_eq!(
        classify(Some(&[])),
        Outcome::Failed,
        "an empty answer decodes as the default kind, which is no refusal"
    );
    let undecodable = [0x0a, 0x05, b'a'];
    assert!(
        BackendError::decode(undecodable.as_slice()).is_err(),
        "the fixture is a truncated field, which the engine's schema does not decode"
    );
    assert_eq!(
        classify(Some(&undecodable)),
        Outcome::Failed,
        "an answer that does not decode keeps the key"
    );
}

/// A5: a refusal drops the key only when it refused the current generation; an accepted or failed
/// send never drops it.
#[test]
fn a_refusal_drops_only_the_generation_it_refused() {
    assert!(
        !on_outcome(generation(2), generation(3), Outcome::Refused),
        "a refusal of 2 after another Worker kept 3 leaves the newer key alone"
    );
    assert!(
        on_outcome(generation(3), generation(3), Outcome::Refused),
        "a refusal of the current generation drops the key"
    );
    assert!(
        !on_outcome(generation(3), generation(3), Outcome::Failed),
        "a failure of the current generation keeps the key"
    );
    assert!(
        !on_outcome(generation(3), generation(3), Outcome::Accepted),
        "an accepted send keeps the key"
    );
    assert!(
        !on_outcome(generation(2), generation(3), Outcome::Failed),
        "a failure of an older generation keeps the key"
    );
}

/// A6: a removal names the next generation; the generation never falls, and at its maximum it
/// stops rather than wraps.
#[test]
fn a_removal_raises_the_generation_and_it_never_wraps() {
    assert_eq!(
        on_removed(generation(0)),
        Some(generation(1)),
        "a removal at 0 stores 1"
    );
    assert_eq!(
        on_removed(generation(7)),
        Some(generation(8)),
        "a removal at 7 stores 8"
    );
    assert_eq!(
        on_removed(generation(u64::MAX)),
        None,
        "a removal at the maximum names no generation"
    );
    assert_eq!(
        generation(u64::MAX).next(),
        None,
        "the maximum has no next generation: it never wraps to 0"
    );
    assert_eq!(
        generation(u64::MAX - 1).next(),
        Some(generation(u64::MAX)),
        "one below the maximum still rises"
    );
    assert_eq!(Generation::ZERO, generation(0), "a fresh store is at 0");
    assert_eq!(
        u64::from(generation(9)),
        9,
        "a generation reads back as its count"
    );
    assert!(
        generation(8) > generation(7),
        "a later generation orders after an earlier one"
    );
}
