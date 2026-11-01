//! The checksum sqlx stores in `_sqlx_migrations` for each embedded migration is the one it stored
//! before the sha2 0.11 move (SPEC-565 A6; ADR-583). sqlx compares a database's stored checksums
//! with `MIGRATOR`'s when it migrates, so a changed checksum would refuse every database migrated
//! before the move. `tests/fixtures/migration-checksums.txt` holds each migration's version and
//! SHA-384 checksum as `MIGRATOR` held them at 4e600dbb.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use deck_streak_kernel::MIGRATOR;

/// Prints how many items the guard examined and refuses zero.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The golden file's (version, checksum) lines, in the file's order.
fn goldens() -> Vec<(i64, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/migration-checksums.txt");
    fs::read_to_string(&path)
        .expect("the migration checksum goldens")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let (version, checksum) = line
                .split_once(' ')
                .expect("a golden line holds a version and a checksum");
            (
                version
                    .parse()
                    .expect("a golden line's version is a number"),
                checksum.to_owned(),
            )
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("writing to a String");
        out
    })
}

#[test]
fn every_migration_checksum_is_the_one_stored_before_the_sha2_move() {
    let goldens = examined("golden migration checksum(s)", goldens());
    let embedded: Vec<(i64, String)> = MIGRATOR
        .iter()
        .map(|migration| (migration.version, hex(&migration.checksum)))
        .collect();
    assert_eq!(embedded, goldens);
}
