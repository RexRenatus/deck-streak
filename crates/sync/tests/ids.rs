//! One id allocator beside the clock (SPEC-509 A07): two simulated devices on slots 1 and 2 make
//! disjoint ids over 10,000 random instants, each id in its slot's residue, above everything its
//! device had seen; a slot is admitted below 64 and refused from 64.

// An integration test is test code: its helpers panic on a malformed case, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_sync::{Hlc, IdError, SLOTS, Slot};

/// `SplitMix64`, a fixed-seed generator, so a failure reproduces.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A draw below `bound`.
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

#[test]
fn two_devices_on_slots_1_and_2_make_disjoint_ids_over_10000_random_instants() {
    let mut random = SplitMix64(0x0509_5EED);
    let slots = [Slot::new(1).expect("slot 1"), Slot::new(2).expect("slot 2")];
    let mut clocks = [Hlc::starting_at(0), Hlc::starting_at(0)];
    let mut made: [Vec<i64>; 2] = [Vec::new(), Vec::new()];
    let start: i64 = 1_761_955_200_000;
    let mut syncs = 0_u64;
    for step in 0..10_000_i64 {
        let device = usize::from(random.below(2) == 1);
        // A wall instant near the step's own, up to a minute behind or ahead: clocks skew.
        let skew = i64::try_from(random.below(120_001)).expect("a small skew") - 60_000;
        let pt = start + step * 250 + skew;
        let id = clocks[device]
            .next_id_at(pt, slots[device])
            .expect("an id in range");
        made[device].push(id);
        if random.below(5) == 0 {
            let (first, second) = (clocks[0].last(), clocks[1].last());
            clocks[0].recv(second);
            clocks[1].recv(first);
            syncs += 1;
        }
    }
    let mut every = BTreeSet::new();
    for (device, ids) in made.iter().enumerate() {
        let number = i64::from(slots[device].get());
        assert!(
            ids.windows(2).all(|pair| pair[0] < pair[1]),
            "device {device} rises"
        );
        for id in ids {
            assert_eq!(id.rem_euclid(SLOTS), number, "id {id} of device {device}");
            assert!(every.insert(*id), "id {id} was made twice");
        }
    }
    let examined = made[0].len() + made[1].len();
    println!(
        "examined {examined} instant(s): {} and {} id(s), {syncs} sync(s)",
        made[0].len(),
        made[1].len()
    );
    assert_eq!(examined, 10_000);
    assert!(
        made.iter().all(|ids| ids.len() > 4_000),
        "both devices make ids"
    );
}

#[test]
fn a_slot_is_admitted_below_64_and_refused_from_64() {
    assert_eq!(Slot::new(0), Some(Slot::SERVER));
    assert_eq!(Slot::new(63).map(Slot::get), Some(63));
    assert_eq!(Slot::new(64), None);
    assert_eq!(Slot::new(u8::MAX), None);
    let admitted = (0..=u8::MAX).filter_map(Slot::new).count();
    println!("examined 256 slot number(s), {admitted} admitted");
    assert_eq!(admitted, 64);
    assert_eq!(
        IdError::Exhausted.to_string(),
        "no id above the floor fits in 64 bits"
    );
}
