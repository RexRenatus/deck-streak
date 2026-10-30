// @phx cites #822
// @phx covers crates/media/src/hash.rs anchor=blob_path digest=sha256:1c54b6a38d8fae6e9f86a3d4afea36b85b13f42b3745dab676f7cfc973c0b8dc
// @phx covers crates/media/src/bind.rs anchor=step digest=sha256:1682f90fcead53523bf03966167fb9a8ac71a3bdc2a22fbe184432eb9df96cd6
// @phx covers crates/media/src/sweep.rs anchor=fate digest=sha256:7e8a4c2017b74dd5315f0abb5beff9df9bd4c76eb3c950afb69161c790e4c531
// @phx covers crates/media/src/sweep.rs anchor=in_grace digest=sha256:51406200d7d6364f17f5659ff25f0264940a7d53643a6a77ebfab97b079133ef
// @phx covers crates/media/src/scan.rs anchor=scan digest=sha256:1cf2ea966a4b5e4d5c1effd17563c8f74b6df71cf1b3596f8bdf32a1a0ba4c13
// @phx covers crates/media/src/scan.rs anchor=sound_at digest=sha256:121337b5ba557a48a710c09a6d4fbb5aa431ca2275d587b0566a69bdc4cea735
// @phx harness k_blob_path ramp=report unwind=70
// @phx witness feature=kani-witness-blob-path kills=k_blob_path
// @phx harness k_bind ramp=report unwind=2
// @phx witness feature=kani-witness-bind kills=k_bind
// @phx harness k_sweep ramp=report unwind=2
// @phx witness feature=kani-witness-sweep kills=k_sweep
// @phx harness k_scan ramp=report unwind=12
// @phx witness feature=kani-witness-scan kills=k_scan

// The media core's Kani harnesses (SPEC-513 section 8; ADR-531 D7). Each checks the shipped
// function itself over every input of its bound, and each is killed by its witness, a mutant
// behind a feature of this crate that no build turns on. The ledger names them K-blob-path,
// K-bind, K-sweep and K-scan.

use crate::bind::{BindRefusal, BindState, Binding, Bound, step};
use crate::hash::MediaHash;
use crate::scan::scan;
use crate::sweep::{Facts, Fate, Keep, SweepPolicy, fate};

/// K-blob-path: every hash's blob path is two hex digits, a separator and the hash's hex form,
/// and reads back to that hash alone.
#[kani::proof]
#[kani::unwind(70)]
fn k_blob_path() {
    let hash = MediaHash::from_bytes(kani::any());
    let path = hash.blob_path();
    assert_eq!(path[2], b'/');
    assert_eq!(path[..2], hash.hex()[..2]);
    assert_eq!(MediaHash::from_blob_path(&path), Some(hash));
}

/// A slot of any state: unbound, or bound live or released to any hash.
fn any_slot() -> Option<Binding> {
    if kani::any() {
        let state = if kani::any() {
            BindState::Live
        } else {
            BindState::Released
        };
        Some(Binding {
            hash: MediaHash::from_bytes(kani::any()),
            state,
        })
    } else {
        None
    }
}

/// K-bind: a bind is write-once. Over any slot, any claim and any hash: an admitted bind leaves the
/// slot live at that hash, and a bound slot's hash never changes; a refusal leaves the slot as it
/// was; a released slot comes back only for its own hash; a claim of another hash is refused.
#[kani::proof]
#[kani::unwind(2)]
fn k_bind() {
    let slot = any_slot();
    let claimed = if kani::any() {
        Some(MediaHash::from_bytes(kani::any()))
    } else {
        None
    };
    let hash = MediaHash::from_bytes(kani::any());
    let (after, outcome) = step(slot, claimed, hash);
    match outcome {
        Ok(bound) => {
            assert_eq!(
                after,
                Some(Binding {
                    hash,
                    state: BindState::Live
                })
            );
            if let Some(before) = slot {
                assert_eq!(before.hash, hash);
                if before.state == BindState::Released {
                    assert_eq!(bound, Bound::Revived);
                }
            }
        }
        Err(_) => assert_eq!(after, slot),
    }
    if claimed.is_some_and(|claimed| claimed != hash) {
        assert_eq!(outcome, Err(BindRefusal::Mismatch));
    }
}

/// K-sweep: over any facts and any instant and window, a live, pinned, in-grace or imported file
/// is kept for the first reason that holds, and every other file is deleted.
#[kani::proof]
#[kani::unwind(2)]
fn k_sweep() {
    let facts = Facts {
        referenced: kani::any(),
        pinned: kani::any(),
        imported: kani::any(),
        added_ms: kani::any(),
    };
    let policy = SweepPolicy {
        now_ms: kani::any(),
        grace_ms: kani::any(),
    };
    let young = facts.added_ms >= policy.now_ms
        || i128::from(policy.now_ms) - i128::from(facts.added_ms) < i128::from(policy.grace_ms);
    let expected = if facts.referenced {
        Fate::Keep(Keep::Live)
    } else if facts.pinned {
        Fate::Keep(Keep::Pinned)
    } else if young {
        Fate::Keep(Keep::InGrace)
    } else if facts.imported {
        Fate::Keep(Keep::Imported)
    } else {
        Fate::Delete
    };
    assert_eq!(fate(facts, policy), expected);
}

/// K-scan: over any four ASCII bytes the grammar ends without a panic and names nothing empty;
/// and a sound tag around any one character other than `]` names exactly that character.
#[kani::proof]
#[kani::unwind(12)]
fn k_scan() {
    let bytes: [u8; 4] = kani::any();
    kani::assume(bytes.iter().all(u8::is_ascii));
    if let Ok(text) = core::str::from_utf8(&bytes) {
        for name in scan(text) {
            assert!(!name.is_empty());
        }
    }
    let inner: u8 = kani::any();
    kani::assume(inner.is_ascii_graphic() && inner != b']');
    let tag = [b'[', b's', b'o', b'u', b'n', b'd', b':', inner, b']'];
    if let Ok(text) = core::str::from_utf8(&tag) {
        assert_eq!(scan(text), vec![String::from(char::from(inner))]);
    }
}
