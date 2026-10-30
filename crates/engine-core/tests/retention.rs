//! SPEC-377 R17, B4 (ADR-388 D15): the browser's backups kept. Three of each kind are kept, the
//! newest of a kind is never removed, and the kinds are counted apart. Every expected removal is
//! written out by hand from the fixture, never computed by the rule.

use deck_streak_engine_core::retention::{Held, KEEP, removals, removed};

/// The two kinds the web engine keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Backup,
    Server,
}

/// Files of `kind`, one per `made`, in the order given.
fn files(kind: Kind, made: &[i64]) -> Vec<Held<Kind, i64>> {
    made.iter().map(|made| Held { kind, made: *made }).collect()
}

#[test]
fn three_of_each_kind_are_kept() {
    assert_eq!(KEEP, 3, "the SPEC's three, written as a number");
    // Five backups, listed out of order: the two oldest (made 10 and 20, at indices 3 and 1) go.
    let five = files(Kind::Backup, &[30, 20, 50, 10, 40]);
    assert_eq!(removals(&five), vec![1, 3]);
    // Four: the oldest alone goes.
    let four = files(Kind::Backup, &[40, 10, 30, 20]);
    assert_eq!(removals(&four), vec![1]);
    // Three are all kept.
    let three = files(Kind::Backup, &[10, 30, 20]);
    assert_eq!(removals(&three), Vec::<usize>::new());
}

#[test]
fn the_newest_of_a_kind_is_never_removed() {
    // With none to keep, every file goes but the newest of each kind (made 30 and 25).
    let mut held = files(Kind::Backup, &[10, 30, 20]);
    held.extend(files(Kind::Server, &[25, 5]));
    assert_eq!(removed(&held, 0), vec![0, 2, 4]);
    // A kind's one file is its newest, and it stays.
    assert_eq!(removals(&files(Kind::Server, &[7])), Vec::<usize>::new());
    // Two files made at one time are both the newest, and both stay.
    assert_eq!(
        removed(&files(Kind::Backup, &[9, 9, 3]), 0),
        vec![2],
        "a tie for the newest keeps both"
    );
}

#[test]
fn each_kind_is_kept_apart() {
    // Four backups and two older server copies: the server copies are under their own three, so
    // only the oldest backup goes, though both server copies are older than it.
    let mut held = files(Kind::Server, &[1, 2]);
    held.extend(files(Kind::Backup, &[10, 20, 30, 40]));
    assert_eq!(removals(&held), vec![2]);
    // Four of each, interleaved: each kind loses its own oldest.
    let mixed = vec![
        Held {
            kind: Kind::Server,
            made: 4,
        },
        Held {
            kind: Kind::Backup,
            made: 5,
        },
        Held {
            kind: Kind::Server,
            made: 1,
        },
        Held {
            kind: Kind::Backup,
            made: 2,
        },
        Held {
            kind: Kind::Server,
            made: 3,
        },
        Held {
            kind: Kind::Backup,
            made: 7,
        },
        Held {
            kind: Kind::Server,
            made: 8,
        },
        Held {
            kind: Kind::Backup,
            made: 6,
        },
    ];
    assert_eq!(removals(&mixed), vec![2, 3]);
}
