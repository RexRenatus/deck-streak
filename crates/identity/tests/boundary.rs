//! The launch data's hash is compared only through `Mac::verify_slice`, which compares in constant
//! time (SPEC-024 A6, R1; web-security's `ws.tg-init-data-constant-time`, applied by hand).
//!
//! The census reads every Rust file under `crates/identity/src/` with its comments and literals
//! blanked, splits the code into clauses at `;`, `{` and `}`, and refuses each clause that names a
//! hash (an identifier holding `hash`, in any case) and compares: `==`, `!=`, `.eq(`, `.ne(`,
//! `.cmp(`, `.partial_cmp(`, `ct_eq(`, `starts_with(` or `ends_with(`. It asserts the positive
//! artifacts too: `verify_slice` is called exactly once, in `init_data.rs`, and an HMAC is
//! finalized exactly once, to derive the key, so no MAC's bytes leave it to be compared by hand
//! under another name. The refusal is proved on a planted file in `tests/fixtures/`.
//!
//! The sealing key is the second HMAC finalized to derive a key, in `sync_seal.rs` (SPEC-363,
//! ADR-374): it leaves the crate only as the key the API releases and is never compared, so the
//! finalize list names exactly those two sites, and a third still fails.

// An integration test is test code: its helpers panic on a failed fixture, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// The comparisons a clause naming a hash may not make.
const COMPARISONS: [&str; 9] = [
    "==",
    "!=",
    ".eq(",
    ".ne(",
    ".cmp(",
    ".partial_cmp(",
    "ct_eq(",
    "starts_with(",
    "ends_with(",
];

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Every `.rs` file under `directory`, in path order.
fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// `text` with every comment and every string or character literal blanked, its line breaks kept,
/// so a line of the code is the text's.
fn code_of(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        let after_ident = at > 0 && (chars[at - 1].is_alphanumeric() || chars[at - 1] == '_');
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                out.push(' ');
                at += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while at < chars.len() {
                if chars[at] == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    at += 2;
                } else if chars[at] == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[at]));
                    at += 1;
                }
            }
        } else if c == 'r' && !after_ident && matches!(next, Some('"' | '#')) {
            // A raw string: r"..." or r#"..."#, with as many hashes closing it as opened it.
            let mut hashes = 0;
            let mut cursor = at + 1;
            while chars.get(cursor) == Some(&'#') {
                hashes += 1;
                cursor += 1;
            }
            if chars.get(cursor) == Some(&'"') {
                out.push('r');
                out.extend(std::iter::repeat_n(' ', cursor - at));
                at = cursor + 1;
                while at < chars.len() {
                    let closes = chars[at] == '"'
                        && (1..=hashes).all(|offset| chars.get(at + offset) == Some(&'#'));
                    if closes {
                        out.extend(std::iter::repeat_n(' ', hashes + 1));
                        at += hashes + 1;
                        break;
                    }
                    out.push(blank(chars[at]));
                    at += 1;
                }
            } else {
                out.push(c);
                at += 1;
            }
        } else if c == '"' {
            out.push(' ');
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                let step = if chars[at] == '\\' { 2 } else { 1 };
                for skipped in chars.iter().skip(at).take(step) {
                    out.push(blank(*skipped));
                }
                at += step;
            }
            out.push(' ');
            at += 1;
        } else if c == '\'' && (next == Some('\\') || chars.get(at + 2) == Some(&'\'')) {
            // A character literal; a lifetime (`'a` with no closing quote) is code.
            out.push(' ');
            at += 1;
            while at < chars.len() && chars[at] != '\'' {
                let step = if chars[at] == '\\' { 2 } else { 1 };
                out.extend(std::iter::repeat_n(' ', step));
                at += step;
            }
            out.push(' ');
            at += 1;
        } else {
            out.push(c);
            at += 1;
        }
    }
    out
}

/// The clauses of `code`, split at `;`, `{` and `}`, each with the line it starts on.
fn clauses(code: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut line = 1;
    let mut start_line = 1;
    let mut clause = String::new();
    for c in code.chars() {
        if matches!(c, ';' | '{' | '}') {
            if !clause.trim().is_empty() {
                found.push((
                    start_line,
                    clause.split_whitespace().collect::<Vec<_>>().join(" "),
                ));
            }
            clause.clear();
            start_line = line;
        } else {
            if clause.trim().is_empty() && !c.is_whitespace() {
                start_line = line;
            }
            clause.push(c);
        }
        if c == '\n' {
            line += 1;
        }
    }
    if !clause.trim().is_empty() {
        found.push((
            start_line,
            clause.split_whitespace().collect::<Vec<_>>().join(" "),
        ));
    }
    found
}

/// Whether `clause` names a hash: an identifier holding `hash`, in any case.
fn names_a_hash(clause: &str) -> bool {
    clause
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| word.to_ascii_lowercase().contains("hash"))
}

/// Each clause of `text` that names a hash and compares it, as `line: clause`.
fn hash_comparisons(text: &str) -> Vec<String> {
    clauses(&code_of(text))
        .into_iter()
        .filter(|(_, clause)| {
            names_a_hash(clause) && COMPARISONS.iter().any(|compare| clause.contains(compare))
        })
        .map(|(line, clause)| format!("{line}: {clause}"))
        .collect()
}

/// How many times `call` is made in the code of `text`, comments and literals aside.
fn calls(text: &str, call: &str) -> usize {
    code_of(text).matches(call).count()
}

#[test]
fn the_hash_is_compared_only_through_verify_slice() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // The census refuses the planted comparisons, each on its own line, and nothing that only a
    // comment or a string literal says.
    let planted = fs::read_to_string(root.join("tests/fixtures/planted_compare.rs.fixture"))
        .expect("the planted fixture");
    let refused: Vec<usize> = hash_comparisons(&planted)
        .iter()
        .map(|finding| {
            finding
                .split_once(':')
                .and_then(|(line, _)| line.parse().ok())
                .expect("a finding names its line")
        })
        .collect();
    assert_eq!(refused, vec![16, 21], "{:#?}", hash_comparisons(&planted));

    let sources: Vec<(PathBuf, String)> = examined("source file(s)", rust_files(&root.join("src")))
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("a readable source");
            (path, text)
        })
        .collect();
    let name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    };

    // The one comparison of the hash: verify_slice, called once, in the validator.
    let verified: Vec<String> = sources
        .iter()
        .flat_map(|(path, text)| std::iter::repeat_n(name(path), calls(text, ".verify_slice(")))
        .collect();
    assert_eq!(verified, vec!["init_data.rs".to_owned()]);

    // An HMAC is finalized once, to derive the key; every other is consumed by verify_slice.
    // The sealing key is the second, derived in sync_seal.rs (SPEC-363, ADR-374).
    let finalized: Vec<String> = sources
        .iter()
        .flat_map(|(path, text)| std::iter::repeat_n(name(path), calls(text, ".finalize")))
        .collect();
    assert_eq!(
        finalized,
        vec!["init_data.rs".to_owned(), "sync_seal.rs".to_owned()]
    );

    // And no clause of the crate names a hash and compares it.
    let findings: Vec<String> = sources
        .iter()
        .flat_map(|(path, text)| {
            hash_comparisons(text)
                .into_iter()
                .map(move |finding| format!("{}:{finding}", name(path)))
        })
        .collect();
    assert_eq!(findings, Vec::<String>::new());
}
