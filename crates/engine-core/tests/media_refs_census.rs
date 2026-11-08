//! The engine core names no engine media rewrite (SPEC-601 A9): a face's references are rewritten
//! by the media core's `rewrite`, so no source file of the core names the engine's
//! `replace_media_refs`.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::path::{Path, PathBuf};

/// The engine's media rewrite, which the core no longer calls.
const ENGINE_REWRITE: &str = "replace_media_refs";

/// Every Rust source file under `dir`, in path order.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("the folder reads")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// The line numbers of `text` that name the engine's media rewrite.
fn naming(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains(ENGINE_REWRITE))
        .map(|(index, _)| index + 1)
        .collect()
}

#[test]
fn the_engine_core_names_no_engine_media_rewrite() {
    let planted = "use anki::text::replace_media_refs;\n";
    assert_eq!(
        naming(planted),
        vec![1],
        "a planted line naming the engine rewrite is found"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    assert!(!files.is_empty(), "the core's sources are found");
    let mut hits = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("the source reads");
        let shown = file.strip_prefix(root).expect("under the crate").display();
        for line in naming(&text) {
            hits.push(format!("{shown}:{line}"));
        }
    }
    assert_eq!(
        hits,
        Vec::<String>::new(),
        "the core's lines naming the engine media rewrite"
    );
    println!(
        "examined {} source file(s) of the engine core: {} line(s) name the engine media rewrite",
        files.len(),
        hits.len()
    );
}
