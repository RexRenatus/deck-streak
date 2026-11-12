//! A6 (SPEC-602 R6): the opener registers `unicase`, and names order under it as the engine's
//! collation golden orders them; a connection without the registration has no such collation.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::path::{Path, PathBuf};

use deck_streak_anki_format::Kind;
use rusqlite::{Connection, OpenFlags, params};
use serde_json::Value;
use support::{
    engine_collection, examined, golden_cases, list, open_file, plain, text, with_engine,
};

/// The ordering query: one case's names, by the collation, ties in their input order.
const ORDER: &str = "SELECT name FROM fields WHERE ntid >= ?1 AND ntid < ?2 \
                     ORDER BY name COLLATE unicase, ntid";

/// A collection whose `fields` rows hold each case's names, case `c`'s name `i` at ntid c*1000+i.
fn names_collection(dir: &Path, cases: &[Value]) -> PathBuf {
    let path = engine_collection(dir);
    with_engine(&path, |db| {
        db.execute("DELETE FROM fields", [])
            .expect("the stock fields are cleared");
        for (c, case) in cases.iter().enumerate() {
            for (i, name) in list(&case["input"]["names"]).iter().enumerate() {
                let ntid = i64::try_from(c * 1000 + i).expect("an id");
                db.execute(
                    "INSERT INTO fields VALUES (?1, 0, ?2, x'')",
                    params![ntid, text(name)],
                )
                .expect("a name");
            }
        }
    });
    path
}

fn ordered(connection: &Connection, case: usize) -> Result<Vec<String>, rusqlite::Error> {
    let from = i64::try_from(case * 1000).expect("an id");
    let mut statement = connection.prepare(ORDER)?;
    let names = statement.query_map(params![from, from + 1000], |row| row.get(0))?;
    names.collect()
}

#[test]
fn the_unicase_order_equals_the_collation_golden() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cases = examined("collation case(s)", golden_cases("anki.collation.order"));
    let path = names_collection(dir.path(), &cases);
    let reader = open_file(&path, Kind::Collection).expect("the collection opens");
    let mut findings = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let wanted: Vec<String> = list(&case["output"]["order"]).iter().map(text).collect();
        match ordered(reader.connection(), index) {
            Ok(found) if found == wanted => {}
            Ok(found) => findings.push(format!("case {index}: {found:?}, not {wanted:?}")),
            Err(error) => findings.push(format!("case {index}: {error}")),
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the collation order");
}

#[test]
fn a_connection_without_the_registration_has_no_unicase() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cases = golden_cases("anki.collation.order");
    let path = names_collection(dir.path(), &cases[..1]);
    let unregistered = plain(&path, OpenFlags::SQLITE_OPEN_READ_ONLY);
    let refused = ordered(&unregistered, 0)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    drop(unregistered);
    assert!(
        refused.contains("no such collation sequence"),
        "the test's own connection has no unicase: {refused:?}"
    );
    let reader = open_file(&path, Kind::Collection).expect("the collection opens");
    let found = ordered(reader.connection(), 0).map_err(|error| error.to_string());
    let wanted: Vec<String> = list(&cases[0]["output"]["order"])
        .iter()
        .map(text)
        .collect();
    assert_eq!(
        found,
        Ok(wanted),
        "the crate's connection orders by the registration"
    );
}
