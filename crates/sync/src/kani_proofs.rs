//! Kani proof harnesses for the sync core (SPEC-509 R9; ADR-527 D7).
//!
//! The module is compiled only under `cfg(kani)`, so a normal build and `cargo test` never see
//! it. The formal check's Kani kind reads the header lines below as the entry `kani/sync`, runs
//! each harness through `cargo kani`, and runs it again with its witness feature on, where it must
//! fail. No other line of this file may carry the header token: the registry reads every line that
//! does as a header.

// @phx cites #15
// @phx covers crates/sync/src/codec.rs anchor=decode digest=sha256:300225e3068b01ba3fa8586d06368caf84bddcaf388ddd291365087ea27c33bc
// @phx covers crates/sync/src/codec.rs anchor=encode digest=sha256:17f2afd09a4a0cff7d85210b6642d295b2d06ba2fcfba3a4e61712563e24c726
// @phx covers crates/sync/src/key.rs anchor=compare digest=sha256:471a3c7435f96360232781932b72560b51c34f5ae5cbf7f9cfff937bd69457d3
// @phx covers crates/sync/src/clock.rs anchor=tick digest=sha256:5f1a758b212cfeec0d493044bd9e0352999ed213f047dbc934414c2ff2ee793e
// @phx covers crates/sync/src/ids.rs anchor=next_id digest=sha256:5456766c92ee96b5f5f61995a0ab0eea6456e8d63f894f1242d44e4e1454621c
// @phx harness sync_codec ramp=report unwind=50
// @phx witness feature=kani-witness-sync-codec-len kills=sync_codec
// @phx harness key_order ramp=report unwind=17
// @phx witness feature=kani-witness-key-order-noseq kills=key_order
// @phx harness hlc_tick_no_overflow ramp=report unwind=2
// @phx witness feature=kani-witness-hlc-plain-add kills=hlc_tick_no_overflow
// @phx harness next_id_holds_its_residue_and_bounds ramp=report unwind=2
// @phx witness feature=kani-witness-next-id-residue kills=next_id_holds_its_residue_and_bounds

use std::cmp::Ordering;

use crate::clock::{ClockError, tick};
use crate::codec::{decode, encode};
use crate::ids::{SLOTS, Slot, next_id};
use crate::key::{Key, compare};
use crate::op::{Op, OpHead};

/// A key of every field symbolic, its origin's 16 bytes included.
fn any_key() -> Key {
    Key {
        t: kani::any(),
        rank: kani::any(),
        origin: kani::any(),
        seq: kani::any(),
    }
}

/// K-sync-codec: decoding never panics on any input of up to 48 bytes, and a note of two fields
/// of 4 bytes each decodes from its own frame to itself.
#[kani::proof]
#[kani::unwind(50)]
fn sync_codec() {
    let bytes: [u8; 48] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= bytes.len());
    let _ = decode(&bytes[..len]);

    let first: [u8; 4] = kani::any();
    let second: [u8; 4] = kani::any();
    kani::assume(first.iter().chain(second.iter()).all(u8::is_ascii));
    let (Ok(first), Ok(second)) = (
        String::from_utf8(first.to_vec()),
        String::from_utf8(second.to_vec()),
    ) else {
        panic!("ASCII bytes are UTF-8");
    };
    let head = OpHead {
        origin: kani::any(),
        seq: kani::any(),
        hlc: kani::any(),
    };
    let op = Op::NoteCreated {
        note: kani::any(),
        notetype: kani::any(),
        deck: kani::any(),
        guid: kani::any(),
        fields: vec![first, second],
    };
    let frame = encode(&head, &op);
    assert!(frame.is_ok());
    if let Ok(frame) = frame {
        assert!(decode(&frame) == Ok((head, op)));
    }
}

/// K-key-order: the key order is a strict total order over keys with symbolic 16-byte origins:
/// equal exactly when the keys are, antisymmetric and transitive.
#[kani::proof]
#[kani::unwind(17)]
fn key_order() {
    let (a, b, c) = (any_key(), any_key(), any_key());
    assert!((compare(&a, &b) == Ordering::Equal) == (a == b));
    assert!(compare(&a, &b) == compare(&b, &a).reverse());
    if compare(&a, &b) == Ordering::Less && compare(&b, &c) == Ordering::Less {
        assert!(compare(&a, &c) == Ordering::Less);
    }
}

/// K-hlc: the clock never overflows. Its next reading is past the device's last clock and every
/// clock it saw, and at least the wall, or it is refused exactly when no reading is left.
#[kani::proof]
#[kani::unwind(2)]
fn hlc_tick_no_overflow() {
    let (last, seen, pt): (i64, i64, i64) = (kani::any(), kani::any(), kani::any());
    match tick(last, seen, pt) {
        Ok(next) => assert!(next > last && next > seen && next >= pt),
        Err(ClockError::Exhausted) => assert!(last.max(seen) == i64::MAX),
    }
}

/// K-next-id: the allocator never panics for any floor and instant, and every id it makes holds
/// its slot's residue, is above the floor and is at or above the instant.
#[kani::proof]
#[kani::unwind(2)]
fn next_id_holds_its_residue_and_bounds() {
    let (floor, now, number): (i64, i64, u8) = (kani::any(), kani::any(), kani::any());
    let Some(slot) = Slot::new(number) else {
        return;
    };
    if let Ok(id) = next_id(floor, now, slot) {
        assert!(id.rem_euclid(SLOTS) == i64::from(number));
        assert!(id > floor);
        assert!(id >= now);
    }
}
