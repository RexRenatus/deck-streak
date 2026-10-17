//! SPEC-377 R4, R5, A5 (ADR-388 D7, D8): the pool's names. The pool holds only the names it lists
//! and knows each file by its one name, and every choice file is a new name: never the collection's
//! and never one the pool lists.

use deck_streak_web_engine::files::{
    Backup, Kind, backup_of, backups, choice_name, exported, holds, record, same, unrecorded,
};

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

#[test]
fn only_a_listed_backup_is_exported() {
    let pool = listed(&[
        COLLECTION,
        "/deck-streak/backup-1.anki2",
        "/deck-streak/backup-1.anki2@1000",
        "/deck-streak/server-2.anki2",
    ]);
    let asked = [
        ("backup-1", Some("/deck-streak/backup-1.anki2")),
        ("server-2", Some("/deck-streak/server-2.anki2")),
        ("backup-2", None),
        ("server-1", None),
        ("collection", None),
        ("backup-01", None),
        ("../deck-streak/backup-1", None),
        ("backup-1.anki2@1000", None),
        ("", None),
    ];
    let answered: Vec<(&str, Option<String>)> = asked
        .iter()
        .map(|(id, _)| (*id, exported(&pool, COLLECTION, id)))
        .collect();
    let expected: Vec<(&str, Option<String>)> = asked
        .iter()
        .map(|(id, name)| (*id, name.map(str::to_owned)))
        .collect();
    assert_eq!(answered, expected);
    // The collection is listed, and still no id exports it.
    assert!(holds(&pool, COLLECTION), "the pool lists the collection");
    assert_eq!(exported(&pool, COLLECTION, "collection"), None);
}

#[test]
fn the_backups_are_listed_newest_first_by_kind() {
    // Recorded at 300, 100 and 200; backup-4 and server-1 share 200, so the larger number is the
    // newer; backup-5 has no record yet, so it is not listed.
    let pool = listed(&[
        "/deck-streak/server-1.anki2@200",
        COLLECTION,
        "/deck-streak/backup-2.anki2",
        "/deck-streak/backup-4.anki2@200",
        "/deck-streak/server-1.anki2",
        "/deck-streak/backup-2.anki2@300",
        "/deck-streak/backup-4.anki2",
        "/deck-streak/server-3.anki2",
        "/deck-streak/server-3.anki2@100",
        "/deck-streak/backup-5.anki2",
    ]);
    let backup = |id: &str, kind, number, made, name: &str| Backup {
        id: id.to_owned(),
        kind,
        number,
        made,
        name: format!("/deck-streak/{name}"),
        record: format!("/deck-streak/{name}@{made}"),
    };
    assert_eq!(
        backups(&pool),
        vec![
            backup("backup-2", Kind::Backup, 2, 300, "backup-2.anki2"),
            backup("backup-4", Kind::Backup, 4, 200, "backup-4.anki2"),
            backup("server-1", Kind::Server, 1, 200, "server-1.anki2"),
            backup("server-3", Kind::Server, 3, 100, "server-3.anki2"),
        ]
    );
    assert_eq!(backups(&listed(&[COLLECTION])), Vec::<Backup>::new());
}

#[test]
fn a_backup_is_named_by_its_kind_and_a_number_from_one() {
    let asked = [
        ("/deck-streak/backup-1.anki2", Some((Kind::Backup, 1))),
        ("/deck-streak/server-12.anki2", Some((Kind::Server, 12))),
        ("server-7.anki2", Some((Kind::Server, 7))),
        ("/deck-streak/backup-0.anki2", None),
        ("/deck-streak/backup-01.anki2", None),
        ("/deck-streak/backup-+1.anki2", None),
        ("/deck-streak/backup-1.anki2@5", None),
        ("/deck-streak/backup-1.sqlite", None),
        ("/deck-streak/copy-1.anki2", None),
        ("/deck-streak/backup1.anki2", None),
        (COLLECTION, None),
    ];
    let answered: Vec<(&str, Option<(Kind, u32)>)> = asked
        .iter()
        .map(|(name, _)| (*name, backup_of(name)))
        .collect();
    assert_eq!(answered, asked);
}

#[test]
fn each_backup_without_a_record_is_recorded_once() {
    assert_eq!(
        record("/deck-streak/backup-1.anki2", 1_736_911_810_000),
        "/deck-streak/backup-1.anki2@1736911810000"
    );
    let pool = listed(&[
        COLLECTION,
        "/deck-streak/backup-1.anki2",
        "/deck-streak/backup-1.anki2@10",
        "/deck-streak/server-2.anki2",
        "/deck-streak/backup-3.anki2",
        "/deck-streak/backup-3.anki2@+7",
        "/deck-streak/server-12.anki2@1",
    ]);
    // backup-1 has its record; server-2 has none, backup-3's record is malformed, and server-12's
    // record names a file the pool does not list.
    assert_eq!(
        unrecorded(&pool),
        vec![
            "/deck-streak/server-2.anki2".to_owned(),
            "/deck-streak/backup-3.anki2".to_owned(),
        ]
    );
    assert_eq!(unrecorded(&listed(&[COLLECTION])), Vec::<String>::new());
}
