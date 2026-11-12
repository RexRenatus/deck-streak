//! The renderer's dependency census (SPEC-603 A1, ADR-624 D1): its manifest has no
//! `[dependencies]` entry in any table, its dev-dependencies are the golden reader's two, it has no
//! features, and its library forbids unsafe code. A planted manifest with one dependency, built
//! in memory here, is refused by the same reader.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::Path;

/// True for a table header that holds dependencies: `[dependencies]`, `[build-dependencies]`, a
/// dotted `[dependencies.<name>]`, or a target's `[target.<cfg>.dependencies]`.
fn is_dependency_table(name: &str) -> bool {
    name.split('.')
        .any(|part| part == "dependencies" || part == "build-dependencies")
}

/// Every entry the manifest's dependency tables hold, one per key line or dotted table.
fn dependency_entries(manifest: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('[') {
            let name = header.trim_end_matches(']').trim();
            inside = is_dependency_table(name);
            if inside && name.split('.').count() > 1 && !name.starts_with("target") {
                entries.push(name.to_owned());
            }
            continue;
        }
        if inside && !line.is_empty() && !line.starts_with('#') {
            entries.push(line.to_owned());
        }
    }
    entries
}

/// The keys of the manifest's `[dev-dependencies]` table.
fn dev_dependency_keys(manifest: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('[') {
            inside = header.trim_end_matches(']').trim() == "dev-dependencies";
            continue;
        }
        if inside && !line.is_empty() && !line.starts_with('#') {
            let key = line.split(['.', '=']).next().unwrap_or_default().trim();
            keys.push(key.to_owned());
        }
    }
    keys
}

#[test]
fn crate_has_no_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("the manifest");
    let entries = dependency_entries(&manifest);
    println!(
        "examined {} manifest line(s): dependency entries {}",
        manifest.lines().count(),
        entries.len()
    );
    assert!(manifest.lines().count() > 0, "the manifest is empty");
    assert!(
        entries.is_empty(),
        "the renderer has no dependency: {entries:?}"
    );
    assert_eq!(
        dev_dependency_keys(&manifest),
        ["serde", "serde_json"],
        "the golden reader's two dev-dependencies, and no other"
    );
    assert!(
        !manifest.lines().any(|line| line.trim() == "[features]"),
        "the renderer has no features"
    );
    let lib = fs::read_to_string(root.join("src/lib.rs")).expect("the library root");
    assert!(
        lib.lines()
            .any(|line| line.trim() == "#![forbid(unsafe_code)]"),
        "the library forbids unsafe code"
    );
    let planted = format!("{manifest}\n[dependencies]\nsha2.workspace = true\n");
    assert_eq!(
        dependency_entries(&planted),
        ["sha2.workspace = true"],
        "a planted dependency is refused by the same reader"
    );
    let dotted = format!("{manifest}\n[target.'cfg(unix)'.dependencies.libc]\nversion = \"1\"\n");
    assert_eq!(
        dependency_entries(&dotted),
        ["version = \"1\""],
        "a planted target dependency is refused by the same reader"
    );
}
