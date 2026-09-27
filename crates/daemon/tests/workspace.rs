//! The workspace's shape, held as a test: one crate per context, each named for its directory
//! and each inheriting the workspace lint table (SPEC-002 A1).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// Prints how many items a guard examined and refuses zero: a glob that stopped matching must
/// fail, never pass over the empty set (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn member_manifests() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for entry in fs::read_dir(crates_dir()).expect("crates/ is readable") {
        let dir = entry.expect("a readable directory entry").path();
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            let name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a UTF-8 directory name")
                .to_owned();
            let text = fs::read_to_string(&manifest).expect("a readable Cargo.toml");
            found.push((name, text));
        }
    }
    found.sort();
    found
}

#[test]
fn every_crate_is_named_for_its_context_directory() {
    for (dir, text) in examined("crate manifests", member_manifests()) {
        let expected = format!("name = \"deck-streak-{dir}\"");
        assert!(
            text.contains(&expected),
            "crates/{dir}/Cargo.toml must declare {expected}"
        );
    }
}

#[test]
fn every_crate_inherits_the_workspace_lints() {
    for (dir, text) in examined("crate manifests", member_manifests()) {
        assert!(
            text.contains("[lints]\nworkspace = true"),
            "crates/{dir}/Cargo.toml must inherit [workspace.lints]"
        );
    }
}
