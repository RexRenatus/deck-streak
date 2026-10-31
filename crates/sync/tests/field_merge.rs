//! Concurrent edits of one register end in one value and one loser set whatever order they arrive
//! in (SPEC-509 A05). Three edits made on one base arrive in each of their 6
//! orders; every order ends at the edit with the greatest key, with the other two recorded as
//! conflicts. An edit made on the current value moves the register forward with no conflict.

// An integration test is test code: its helpers panic on a malformed case, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_sync::{Conflict, Edit, Key, Register};

fn key(t: i64, origin: u8, seq: u64) -> Key {
    Key {
        t,
        rank: 0,
        origin: [origin; 16],
        seq,
    }
}

fn edit(key: Key, value: &str) -> Edit<String> {
    Edit {
        key,
        value: value.to_owned(),
    }
}

/// Every order of three items, by index.
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

#[test]
fn all_6_orders_of_3_concurrent_field_edits_end_in_one_value_and_one_loser_set() {
    let base = key(10, 9, 1);
    let concurrent = [
        edit(key(20, 1, 4), "from the first device"),
        edit(key(20, 2, 2), "from the second device"),
        edit(key(15, 3, 8), "from the third device, older"),
    ];
    let mut outcomes = BTreeSet::new();
    let mut examined = 0_u64;
    for order in ORDERS {
        let mut register = Register::new();
        register.receive(edit(base, "the base"), None);
        assert_eq!(register.conflicts(), Vec::<Conflict<String>>::new());
        for index in order {
            register.receive(concurrent[index].clone(), Some(base));
        }
        let winner = register.current().expect("a written register").clone();
        let losers: BTreeSet<String> = register
            .conflicts()
            .iter()
            .map(|conflict| conflict.loser.value.clone())
            .collect();
        assert_eq!(register.conflicts().len(), 2, "{order:?}");
        let last = register.conflicts().last().expect("a conflict");
        assert_eq!(last.winner, winner.key, "{order:?}");
        outcomes.insert((winner.value, losers));
        examined += 1;
    }
    println!("examined {examined} of 6 orders");
    assert_eq!(examined, 6);
    assert_eq!(
        outcomes,
        BTreeSet::from([(
            "from the second device".to_owned(),
            BTreeSet::from([
                "from the first device".to_owned(),
                "from the third device, older".to_owned(),
            ])
        )]),
        "one value and one loser set for every order"
    );
}

#[test]
fn an_edit_made_on_the_current_value_moves_the_register_forward_with_no_conflict() {
    let mut register = Register::new();
    register.receive(edit(key(10, 1, 1), "first"), None);
    register.receive(edit(key(11, 2, 1), "second"), Some(key(10, 1, 1)));
    register.receive(edit(key(12, 1, 2), "third"), Some(key(11, 2, 1)));
    assert_eq!(register.current(), Some(&edit(key(12, 1, 2), "third")));
    assert_eq!(register.conflicts(), Vec::<Conflict<String>>::new());

    // A stale edit made offline on the first value loses to the newer one, and is kept.
    register.receive(edit(key(5, 3, 9), "stale"), Some(key(10, 1, 1)));
    assert_eq!(register.current(), Some(&edit(key(12, 1, 2), "third")));
    assert_eq!(register.conflicts().len(), 1);
    assert_eq!(register.conflicts()[0].loser, edit(key(5, 3, 9), "stale"));
    assert_eq!(register.conflicts()[0].winner, key(12, 1, 2));

    // A register never written takes an edit whatever base it names.
    let mut fresh = Register::new();
    fresh.receive(edit(key(1, 1, 1), "only"), Some(key(9, 9, 9)));
    assert_eq!(fresh.current(), Some(&edit(key(1, 1, 1), "only")));
    assert_eq!(fresh.conflicts(), Vec::<Conflict<String>>::new());
    assert_eq!(Register::<String>::default(), Register::new());
}
