//! A name is bound to one hash, once (SPEC-513 A3): binding it again to the same hash changes
//! nothing, to another hash is refused, a released name comes back only for its own hash, and a
//! certified name is refused for any hash but its own.

use deck_streak_media::{
    BindRefusal, BindState, Binding, Bindings, Bound, MediaHash, MediaName, Release,
    ReleaseRefusal, release_step, step,
};

fn hash(byte: u8) -> MediaHash {
    MediaHash::from_bytes([byte; 32])
}

fn imported(text: &str) -> MediaName {
    MediaName::Imported(text.to_owned())
}

/// A slot bound live to the hash of `byte`.
#[allow(
    clippy::unnecessary_wraps,
    reason = "a slot is an Option, and the helper writes one"
)]
fn live(byte: u8) -> Option<Binding> {
    Some(Binding {
        hash: hash(byte),
        state: BindState::Live,
    })
}

/// A slot bound, released, to the hash of `byte`.
#[allow(
    clippy::unnecessary_wraps,
    reason = "a slot is an Option, and the helper writes one"
)]
fn released(byte: u8) -> Option<Binding> {
    Some(Binding {
        hash: hash(byte),
        state: BindState::Released,
    })
}

#[test]
fn bind_is_write_once() {
    let mut bindings = Bindings::new();
    let name = imported("ds_harbor.png");
    assert_eq!(bindings.bind(&name, hash(1)), Ok(Bound::New));
    assert_eq!(bindings.get(&name), live(1));
    assert_eq!(
        bindings.bind(&name, hash(2)),
        Err(BindRefusal::Conflict),
        "a bound name is never bound to another hash"
    );
    assert_eq!(bindings.get(&name), live(1), "the refusal changed nothing");
    assert_eq!(bindings.len(), 1);
}

#[test]
fn binding_the_same_hash_again_changes_nothing() {
    let mut bindings = Bindings::new();
    let name = imported("ds_amber.png");
    assert_eq!(bindings.bind(&name, hash(3)), Ok(Bound::New));
    let before = bindings.clone();
    assert_eq!(bindings.bind(&name, hash(3)), Ok(Bound::Unchanged));
    assert_eq!(bindings, before, "the repeat changed nothing");
}

#[test]
fn a_released_name_comes_back_only_for_its_own_hash() {
    let mut bindings = Bindings::new();
    let name = imported("ds_basil.png");
    assert_eq!(
        bindings.release(&name),
        Err(ReleaseRefusal::Unbound),
        "an unbound name has nothing to release"
    );
    assert_eq!(bindings.bind(&name, hash(4)), Ok(Bound::New));
    assert_eq!(bindings.release(&name), Ok(Release::Released));
    assert_eq!(
        bindings.get(&name),
        released(4),
        "a released name keeps its hash"
    );
    assert_eq!(bindings.release(&name), Ok(Release::AlreadyReleased));
    assert_eq!(
        bindings.bind(&name, hash(5)),
        Err(BindRefusal::Conflict),
        "another hash cannot revive it"
    );
    assert_eq!(bindings.get(&name), released(4));
    assert_eq!(bindings.bind(&name, hash(4)), Ok(Bound::Revived));
    assert_eq!(bindings.get(&name), live(4));
}

#[test]
fn a_bind_under_a_mismatched_certified_name_is_refused() {
    let mut bindings = Bindings::new();
    let name = MediaName::for_bytes(b"cobalt", Some("png")).expect("a certified name");
    assert_eq!(
        bindings.bind(&name, MediaHash::of(b"dune")),
        Err(BindRefusal::Mismatch)
    );
    assert!(bindings.is_empty(), "a refused bind binds nothing");
    assert_eq!(
        bindings.bind(&name, MediaHash::of(b"cobalt")),
        Ok(Bound::New)
    );
}

/// Every slot, claim and hash over two hashes: what each bind and each release gives, written out
/// by rule, and the slot `Bindings` keeps agrees with the machine.
#[test]
fn the_machine_answers_every_slot_claim_and_hash_by_its_rules() {
    let slots = [None, live(0), released(0), live(1), released(1)];
    let claims = [None, Some(hash(0)), Some(hash(1))];
    let mut examined = 0;
    for slot in slots {
        for claimed in claims {
            for value in [hash(0), hash(1)] {
                let expected = if claimed.is_some_and(|claimed| claimed != value) {
                    (slot, Err(BindRefusal::Mismatch))
                } else {
                    match slot {
                        None => (
                            Some(Binding {
                                hash: value,
                                state: BindState::Live,
                            }),
                            Ok(Bound::New),
                        ),
                        Some(bound) if bound.hash != value => (slot, Err(BindRefusal::Conflict)),
                        Some(bound) if bound.state == BindState::Live => {
                            (slot, Ok(Bound::Unchanged))
                        }
                        Some(_) => (
                            Some(Binding {
                                hash: value,
                                state: BindState::Live,
                            }),
                            Ok(Bound::Revived),
                        ),
                    }
                };
                assert_eq!(
                    step(slot, claimed, value),
                    expected,
                    "{slot:?} {claimed:?} {value:?}"
                );
                examined += 1;
            }
        }
        let expected_release = match slot {
            None => (None, Err(ReleaseRefusal::Unbound)),
            Some(bound) if bound.state == BindState::Live => (
                Some(Binding {
                    hash: bound.hash,
                    state: BindState::Released,
                }),
                Ok(Release::Released),
            ),
            Some(_) => (slot, Ok(Release::AlreadyReleased)),
        };
        assert_eq!(release_step(slot), expected_release, "{slot:?}");
    }
    println!("examined {examined} bind(s) and {} release(s)", slots.len());
    assert_eq!(examined, 30);
}
