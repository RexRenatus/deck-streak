//! The table censuses' shared reader answers every vector the Lean port writes (SPEC-431 R21, A13;
//! #878): for each tree in `formal/vectors/table-census-reach.jsonl`, written by
//! `formal/lean/Formal/TableCensusReach.lean`'s port, the reader in
//! `tools/table-census/table_census.rs` refuses exactly the files the vector's `reach` lists, over
//! the same tree planted as source.
//!
//! The trees are SPEC-331 A1's population for each census's name, in the writer's order: at each
//! position of the name, the piece before it and the piece after it are lone literals in two files,
//! and its character is a lone literal in a third, in five lone shapes, a letter in both cases.
//! Each vector names its tree by a label, and this test plants the tree of the same label as the
//! three census tests plant it. Every tree is one the merge-base's reader refused, as the vector's
//! `dev` records. A positive control per name joins its pieces in one file, which the same
//! planting and reader must refuse, so a planting the reader cannot see fails here too.

// An integration test is test code: its helpers panic on an unreadable tree or a malformed vector,
// and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use serde_json::Value;

// The shared reader of every crate's literals (SPEC-324 R1 to R5), included by path (ADR-029).
#[path = "../../../tools/table-census/table_census.rs"]
mod table_census;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/table-census-reach.jsonl");

/// The three censuses' names, in the writer's order, each with the crate that owns its table.
const TABLES: [(&str, &str); 3] = [
    ("xp_ledger", "progression"),
    ("xp_settlement", "progression"),
    ("coin_ledger", "economy"),
];

/// One of SPEC-331 A1's lone shapes: its label, and the file it writes around a character.
type LoneShape = (&'static str, fn(char) -> String);

/// One planted tree: its label, and each file's path and text.
type Tree = (String, Vec<(&'static str, String)>);

/// The five lone shapes, in the writer's order, each written as the census tests write it.
fn shapes() -> [LoneShape; 5] {
    [
        ("method argument", |c| {
            format!("pub fn c(date: &str) -> Option<&str> {{\n    date.strip_prefix({c:?})\n}}\n")
        }),
        ("matches! arm", |c| {
            format!("pub fn c(x: char) -> bool {{\n    matches!(x, {c:?})\n}}\n")
        }),
        ("format template", |c| {
            format!("pub fn c(n: u32) -> String {{\n    format!(\"{c}{{n}}\")\n}}\n")
        }),
        ("named const used alone", |c| {
            format!(
                "const MARK: char = {c:?};\npub fn c(t: &str) -> Option<&str> {{\n    \
                 t.strip_prefix(MARK)\n}}\n"
            )
        }),
        ("split set", |c| {
            format!("pub fn c(t: &str) -> usize {{\n    t.split([{c:?}, '.']).count()\n}}\n")
        }),
    ]
}

/// A1's trees for `table`, in the writer's order: each position, each case, each shape.
fn trees(table: &str) -> Vec<Tree> {
    let mut trees = Vec::new();
    for (at, character) in table.char_indices() {
        let before = &table[..at];
        let after = &table[at + 1..];
        let mut cases = vec![character];
        if character.is_ascii_alphabetic() {
            cases.push(character.to_ascii_uppercase());
        }
        for case in cases {
            for (shape, text) in shapes() {
                let mut files = Vec::new();
                if !before.is_empty() {
                    files.push((
                        "crates/quests/src/a1_before.rs",
                        format!(
                            "pub fn a(t: &str) -> bool {{\n    t.starts_with({before:?})\n}}\n"
                        ),
                    ));
                }
                if !after.is_empty() {
                    files.push((
                        "crates/quests/src/a1_after.rs",
                        format!("pub fn b(t: &str) -> bool {{\n    t.ends_with({after:?})\n}}\n"),
                    ));
                }
                files.push(("crates/streaks/src/a1_char.rs", text(case)));
                trees.push((format!("{table}@{at} {case:?} {shape}"), files));
            }
        }
    }
    trees
}

/// The files the shared reader refuses for `table`, owned by `owner`, in a tree that holds only
/// `files`. No planted file names the table as written.
fn refused(table: &str, owner: &str, files: &[(&str, String)]) -> BTreeSet<String> {
    let planted = tempfile::tempdir().expect("a temporary directory");
    for (path, text) in files {
        assert!(!text.contains(table), "{path} names {table} as written");
        let file = planted.path().join(path);
        fs::create_dir_all(file.parent().expect("a planted file's folder")).expect("its folder");
        fs::write(&file, text).expect("the planted file");
    }
    table_census::refusals(planted.path(), table, owner, &BTreeSet::new())
        .into_iter()
        .map(|refusal| {
            refusal
                .split_once(' ')
                .map(|(file, _)| file.to_owned())
                .expect("a refusal begins with its file")
        })
        .collect()
}

/// A vector's list of refused files, as a set.
fn files_of(vector: &Value, field: &str) -> BTreeSet<String> {
    vector[field]
        .as_array()
        .unwrap_or_else(|| panic!("the refused files {field} in {vector}"))
        .iter()
        .map(|file| file.as_str().expect("a refused file").to_owned())
        .collect()
}

/// The positive control for `table`: its pieces joined in one file, refused by that file.
fn control(table: &str, owner: &str) {
    let middle = table.len() / 2;
    let joined = format!(
        "pub fn joined() -> String {{\n    format!(\"{{}}{{}}{{}}\", {:?}, {:?}, {:?})\n}}\n",
        &table[..middle],
        table[middle..]
            .chars()
            .next()
            .expect("the middle character")
            .to_ascii_uppercase(),
        &table[middle + 1..]
    );
    let file = "crates/quests/src/a1_joined.rs";
    assert_eq!(
        refused(table, owner, &[(file, joined)]),
        BTreeSet::from([file.to_owned()]),
        "the control for {table}"
    );
}

#[test]
fn the_table_census_reach_answers_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "TableCensusReach", "{header}");
    assert_eq!(
        header["covers"], "tools/table-census/table_census.rs",
        "{header}"
    );
    assert_eq!(header["anchor"], "covering", "{header}");
    let vectors: Vec<Value> = lines
        .map(|line| serde_json::from_str(line).expect("a JSON vector"))
        .collect();
    let planted: Vec<(&str, &str, Tree)> = TABLES
        .iter()
        .flat_map(|&(table, owner)| {
            trees(table)
                .into_iter()
                .map(move |tree| (table, owner, tree))
        })
        .collect();
    assert_eq!(vectors.len(), planted.len(), "one vector per planted tree");
    let mut examined = BTreeMap::<&str, u64>::new();
    let (mut dev_refused, mut reach_refused) = (0_u64, 0_u64);
    for (vector, (table, owner, (label, files))) in vectors.iter().zip(&planted) {
        assert_eq!(vector["table"], *table, "{vector}");
        assert_eq!(vector["tree"], label.as_str(), "{vector}");
        let theirs = files_of(vector, "reach");
        let dev = files_of(vector, "dev");
        assert!(
            theirs.is_subset(&dev),
            "the reach refuses within dev's: {vector}"
        );
        assert_eq!(refused(table, owner, files), theirs, "{vector}");
        dev_refused += u64::from(!dev.is_empty());
        reach_refused += u64::from(!theirs.is_empty());
        *examined.entry(*table).or_default() += 1;
    }
    for (table, owner) in TABLES {
        control(table, owner);
    }
    let trees: u64 = examined.values().sum();
    println!("examined {trees} tree(s) by table: {examined:?}");
    println!("dev's reader refused {dev_refused}, the reach refuses {reach_refused}");
    assert_eq!(
        examined.into_iter().collect::<Vec<_>>(),
        vec![
            ("coin_ledger", 105),
            ("xp_ledger", 85),
            ("xp_settlement", 125)
        ],
        "every tree the writer prints"
    );
    assert_eq!(
        (dev_refused, reach_refused),
        (trees, 0),
        "every tree is one dev's reader refused, and the reach refuses none"
    );
    assert_eq!(header["vectors"].as_u64(), Some(trees), "{header}");
}
