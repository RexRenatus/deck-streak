//! The key every operation carries, and the one order every party folds by (SPEC-509 R3;
//! ADR-527 D3).
//!
//! A card's reviews are folded in key order, never in the order the server's log delivered them,
//! so two devices that hold the same reviews fold them alike. The order is lexicographic over
//! `(t, rank, origin, seq)`: two operations of one device differ in `seq` and two devices differ
//! in `origin`, so two distinct operations never compare equal.

use std::cmp::Ordering;

/// A device's origin: the 16 random bytes it draws once and stamps on everything it makes.
pub type Origin = [u8; 16];

/// An operation's key, compared field by field in declaration order by [`compare`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    /// The operation's hybrid clock, in milliseconds (for a review, its id).
    pub t: i64,
    /// The rank among operations at one instant; a review made on a device is rank 0.
    pub rank: u8,
    /// The device that made the operation.
    pub origin: Origin,
    /// The device's sequence number for the operation.
    pub seq: u64,
}

/// The key order: by clock, then rank, then the origin's bytes, then the sequence number.
#[must_use]
#[expect(
    clippy::disallowed_macros,
    reason = "a Kani witness feature, off in every build but the formal check's witness run (ADR-527 D7)"
)]
pub fn compare(a: &Key, b: &Key) -> Ordering {
    let before_seq =
        a.t.cmp(&b.t)
            .then(a.rank.cmp(&b.rank))
            .then(a.origin.cmp(&b.origin));
    // The Kani witness (ADR-527 D7): with its feature on, the order ignores the sequence number,
    // so two distinct operations of one device compare equal and `kani_proofs::key_order` fails.
    if cfg!(feature = "kani-witness-key-order-noseq") {
        before_seq
    } else {
        before_seq.then(a.seq.cmp(&b.seq))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(self, other)
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Puts `keys` in key order, whatever order they arrived in: the order a card's reviews are
/// folded in.
pub fn sort_keys(keys: &mut [Key]) {
    keys.sort_by(compare);
}
