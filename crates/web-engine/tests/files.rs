//! SPEC-377 R4, R5, A5 (ADR-388 D7, D8): the pool's names. The pool holds only the names it lists
//! and knows each file by its one name, and every choice file is a new name: never the collection's
//! and never one the pool lists.

use deck_streak_web_engine::files::{Kind, choice_name, holds, same};

/// The collection's path inside the pool, as `src/wasm.rs` names it.
const COLLECTION: &str = "/deck-streak/collection.anki2";

fn listed(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn the_pool_holds_only_the_names_it_lists() {
    let pool = listed(&[COLLECTION, "/deck-streak/backup-1.anki2"]);
    let asked = [
        (COLLECTION, true),
        ("/deck-streak/backup-1.anki2", true),
        ("/deck-streak/backup-2.anki2", false),
        ("/deck-streak/Collection.anki2", false),
        ("/deck-streak/collection.anki2-journal", false),
        ("", false),
    ];
    let answered: Vec<(&str, bool)> = asked
        .iter()
        .map(|(name, _)| (*name, holds(&pool, name)))
        .collect();
    assert_eq!(answered, asked);
    assert!(!holds(&[], COLLECTION), "an empty pool holds no name");
}

#[test]
fn the_pool_finds_the_collection_by_its_one_name() {
    let asked = [
        (COLLECTION, true),
        ("/deck-streak/./collection.anki2", false),
        ("deck-streak/collection.anki2", false),
        ("/deck-streak/backup-1.anki2", false),
    ];
    let answered: Vec<(&str, bool)> = asked
        .iter()
        .map(|(name, _)| (*name, same(COLLECTION, name)))
        .collect();
    assert_eq!(answered, asked);
}

#[test]
fn a_choice_file_is_never_a_name_the_pool_holds() {
    for (kind, word) in [(Kind::Backup, "backup"), (Kind::Server, "server")] {
        let mut pool = listed(&[COLLECTION, "/deck-streak/collection.anki2-journal"]);
        let mut minted = Vec::new();
        for _ in 0..4 {
            let name = choice_name(&pool, COLLECTION, kind).expect("a free name is minted");
            assert!(
                !pool.contains(&name),
                "{name} is a name the pool lists: {pool:?}"
            );
            minted.push(name.clone());
            pool.push(name);
        }
        let wanted: Vec<String> = (1..=4)
            .map(|n| format!("/deck-streak/{word}-{n}.anki2"))
            .collect();
        assert_eq!(minted, wanted);
    }
}

#[test]
fn a_choice_file_is_never_the_collection() {
    let first = "/deck-streak/backup-1.anki2";
    assert_eq!(
        [
            choice_name(&[], first, Kind::Backup),
            choice_name(&listed(&[first]), first, Kind::Backup),
            choice_name(&[], COLLECTION, Kind::Server),
        ],
        [
            Some("/deck-streak/backup-2.anki2".to_owned()),
            Some("/deck-streak/backup-2.anki2".to_owned()),
            Some("/deck-streak/server-1.anki2".to_owned()),
        ]
    );
}
