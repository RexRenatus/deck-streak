//! Every grant key the drill post-back forms is one the grant grammar accepts (SPEC-110 R10; the
//! class rule of the round-1 review). The population is generated, not listed: an id of every
//! length from 1 to the grammar's own limit plus eight, over the id grammar's whole alphabet.

// An integration test is test code: its helpers panic, and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_progression::grant::{GrantSource, SOURCE_MAX_LEN};
use deck_streak_vault::drills::{GRANT_SOURCE_MAX, KEY_ID_MAX, KEY_PREFIX, drill_key};

/// The characters an id body may hold (`^[a-z0-9][a-z0-9:._-]*$`, after its first character).
const BODY: &str = "abcdefghijklmnopqrstuvwxyz0123456789:._-";
/// The characters an id may begin with.
const FIRST: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

/// The id of `len` characters that begins `first` and cycles the body alphabet from `offset`.
fn id_of(first: char, len: usize, offset: usize) -> String {
    let body: Vec<char> = BODY.chars().collect();
    let mut id = String::from(first);
    while id.len() < len {
        id.push(body[(offset + id.len()) % body.len()]);
    }
    id
}

/// Every member of the population: each first character at each length, the ids that begin `h.`
/// where the grammar allows them, and one all-punctuation body per length.
fn population() -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for len in 1..=SOURCE_MAX_LEN + 8 {
        for (at, first) in FIRST.chars().enumerate() {
            ids.insert(id_of(first, len, at));
        }
        if len >= 2 {
            let mut id = String::from("h.");
            id.push_str(&id_of('a', len, 3)[2..]);
            ids.insert(id);
        }
        let mut punctuation = String::from("a");
        while punctuation.len() < len {
            punctuation.push(":._-".chars().nth(punctuation.len() % 4).expect("a char"));
        }
        ids.insert(punctuation);
    }
    ids
}

#[test]
fn the_kept_id_bound_is_the_grammars_own_limit_less_the_prefix() {
    assert_eq!(GRANT_SOURCE_MAX, SOURCE_MAX_LEN, "one limit, not two");
    assert_eq!(KEY_ID_MAX, SOURCE_MAX_LEN - KEY_PREFIX.len());
}

#[test]
fn every_key_the_post_back_forms_is_a_source_the_grammar_accepts() {
    let ids = population();
    let mut keys = BTreeSet::new();
    let mut kept = 0_usize;
    let mut hashed = 0_usize;
    for id in &ids {
        let key = drill_key(id);
        assert!(
            GrantSource::new(&key).is_ok(),
            "the grammar refuses the key of an id of {} characters (begins {:?})",
            id.len(),
            &id[..id.len().min(3)]
        );
        if id.len() <= KEY_ID_MAX && !id.starts_with("h.") {
            assert_eq!(
                key,
                format!("{KEY_PREFIX}{id}"),
                "an id of {} is kept",
                id.len()
            );
            kept += 1;
        } else {
            assert!(
                key.starts_with("drill:h.") && key.len() == "drill:h.".len() + 32,
                "an id of {} characters is hashed: {key}",
                id.len()
            );
            hashed += 1;
        }
        assert!(keys.insert(key), "two ids share a key");
    }
    assert!(kept > 0 && hashed > 0, "both arms are examined");
    assert_eq!(keys.len(), ids.len(), "no two members' keys are equal");
    println!("examined {} ids ({kept} kept, {hashed} hashed)", ids.len());
}
