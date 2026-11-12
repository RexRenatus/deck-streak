//! A1 (SPEC-602 R1): the crate depends on exactly its four admitted crates, and no source of it
//! names the engine.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use support::{examined, root};

const ADMITTED: [&str; 4] = ["rusqlite", "sha2", "thiserror", "unicase"];

/// The names of a manifest's `[dependencies]` entries.
fn dependencies(manifest: &str) -> BTreeSet<String> {
    let mut inside = false;
    let mut names = BTreeSet::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[dependencies]";
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name = line.split(['=', '.', ' ']).next().unwrap_or("");
        names.insert(name.to_owned());
    }
    names
}

fn refused(manifest: &str) -> Vec<String> {
    let admitted: BTreeSet<String> = ADMITTED.iter().map(|name| (*name).to_owned()).collect();
    let found = dependencies(manifest);
    let mut findings: Vec<String> = found
        .difference(&admitted)
        .map(|name| format!("{name} is not admitted"))
        .collect();
    findings.extend(
        admitted
            .difference(&found)
            .map(|name| format!("{name} is missing")),
    );
    findings
}

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("a source folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = fs::read_to_string(&path).expect("a readable source");
            out.push((path.display().to_string(), text));
        }
    }
}

fn naming_the_engine(files: &[(String, String)]) -> Vec<String> {
    files
        .iter()
        .filter(|(_, text)| text.contains("anki::"))
        .map(|(path, _)| path.clone())
        .collect()
}

#[test]
fn the_crate_depends_on_exactly_its_four_admitted_crates() {
    let manifest = fs::read_to_string(root().join("crates/anki-format/Cargo.toml"))
        .expect("the crate's manifest");
    examined(
        "dependency entr(ies)",
        dependencies(&manifest).into_iter().collect(),
    );
    assert_eq!(refused(&manifest), Vec::<String>::new(), "the dependencies");

    // A planted copy holding one more entry is refused.
    let planted = manifest.replace(
        "[dependencies]\n",
        "[dependencies]\nserde_json.workspace = true\n",
    );
    assert_eq!(
        refused(&planted),
        ["serde_json is not admitted"],
        "the planted manifest"
    );
}

#[test]
fn no_source_of_the_crate_names_the_engine() {
    let mut files = Vec::new();
    sources(&root().join("crates/anki-format/src"), &mut files);
    let files = examined("source file(s)", files);
    assert_eq!(
        naming_the_engine(&files),
        Vec::<String>::new(),
        "the engine"
    );

    // A planted source naming the engine is refused.
    let planted = vec![(
        "planted.rs".to_owned(),
        "use anki::collection::Collection;\n".to_owned(),
    )];
    assert_eq!(
        naming_the_engine(&planted),
        ["planted.rs"],
        "the planted source"
    );
}
