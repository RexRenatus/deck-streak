//! `plan_sweep` keeps live, pinned, in-grace and imported files (SPEC-513 A6): each file stays for
//! the first reason that holds, every other file is deleted, and every name written that no
//! stored file has is listed missing.

use std::collections::BTreeSet;

use deck_streak_media::{Facts, Fate, Keep, StoredFile, SweepPlan, SweepPolicy, fate, plan_sweep};

/// The sweep's instant: 2026-10-30T00:00:00Z, in milliseconds.
const NOW: i64 = 1_793_318_400_000;

/// A day, in milliseconds: the grace window of these tests.
const DAY: u64 = 86_400_000;

fn stored(name: &str, added_ms: i64, imported: bool) -> StoredFile {
    StoredFile {
        name: name.to_owned(),
        added_ms,
        imported,
    }
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

const POLICY: SweepPolicy = SweepPolicy {
    now_ms: NOW,
    grace_ms: DAY,
};

#[test]
fn plan_sweep_keeps_live_pinned_in_grace_and_imported_files() {
    let old = NOW - 30 * 86_400_000;
    let files = [
        stored("live.png", old, false),
        stored("_pinned.woff2", old, false),
        stored("fresh.png", NOW - 1, false),
        stored("future.png", NOW + 5, false),
        stored("deck.png", old, true),
        stored("_live_and_pinned.png", old, true),
    ];
    let plan = plan_sweep(
        &files,
        &names(&["live.png", "_live_and_pinned.png"]),
        POLICY,
    );
    assert_eq!(
        plan,
        SweepPlan {
            delete: vec![],
            kept: vec![
                ("_live_and_pinned.png".to_owned(), Keep::Live),
                ("_pinned.woff2".to_owned(), Keep::Pinned),
                ("deck.png".to_owned(), Keep::Imported),
                ("fresh.png".to_owned(), Keep::InGrace),
                ("future.png".to_owned(), Keep::InGrace),
                ("live.png".to_owned(), Keep::Live),
            ],
            missing: vec![],
        },
        "every kept file stays for its first reason, in name order"
    );
}

#[test]
fn every_other_file_is_deleted_and_every_missing_name_is_listed() {
    let old = NOW - 30 * 86_400_000;
    let files = [
        stored("unused.png", old, false),
        stored("used.png", old, false),
        stored("edge.png", NOW - i64::try_from(DAY).expect("a day"), false),
    ];
    let plan = plan_sweep(
        &files,
        &names(&["used.png", "gone.mp3", "also gone.png"]),
        POLICY,
    );
    assert_eq!(plan.delete, vec!["edge.png", "unused.png"]);
    assert_eq!(plan.kept, vec![("used.png".to_owned(), Keep::Live)]);
    assert_eq!(plan.missing, vec!["also gone.png", "gone.mp3"]);
}

#[test]
fn a_file_is_in_grace_until_the_window_has_passed() {
    let facts = |added_ms| Facts {
        referenced: false,
        pinned: false,
        imported: false,
        added_ms,
    };
    let window = i64::try_from(DAY).expect("a day");
    assert_eq!(
        fate(facts(NOW - window + 1), POLICY),
        Fate::Keep(Keep::InGrace)
    );
    assert_eq!(fate(facts(NOW - window), POLICY), Fate::Delete);
    assert_eq!(fate(facts(NOW), POLICY), Fate::Keep(Keep::InGrace));
    assert_eq!(fate(facts(i64::MAX), POLICY), Fate::Keep(Keep::InGrace));
    assert_eq!(fate(facts(i64::MIN), POLICY), Fate::Delete);
    let widest = SweepPolicy {
        now_ms: i64::MAX,
        grace_ms: u64::MAX,
    };
    assert_eq!(
        fate(facts(i64::MIN), widest),
        Fate::Delete,
        "the widest age is past the widest window, and nothing overflows"
    );
    assert_eq!(fate(facts(0), widest), Fate::Keep(Keep::InGrace));
}

#[test]
fn each_reason_is_asked_in_order_and_none_leaves_the_file_deleted() {
    let old = NOW - 30 * 86_400_000;
    let mut examined = 0;
    for referenced in [false, true] {
        for pinned in [false, true] {
            for imported in [false, true] {
                for added_ms in [old, NOW] {
                    let facts = Facts {
                        referenced,
                        pinned,
                        imported,
                        added_ms,
                    };
                    let expected = if referenced {
                        Fate::Keep(Keep::Live)
                    } else if pinned {
                        Fate::Keep(Keep::Pinned)
                    } else if added_ms == NOW {
                        Fate::Keep(Keep::InGrace)
                    } else if imported {
                        Fate::Keep(Keep::Imported)
                    } else {
                        Fate::Delete
                    };
                    assert_eq!(fate(facts, POLICY), expected, "{facts:?}");
                    examined += 1;
                }
            }
        }
    }
    println!("examined {examined} fact set(s)");
    assert_eq!(examined, 16);
}
