//! Every call in `crates/vault/src` that makes a folder, renames, or removes an entry, outside the
//! atomic writer and the port it writes through (`atomic.rs` and `fs.rs`), is listed here and
//! classed (SPEC-118 R5, #56; ADR-316). The population leaves out the same two files the tree's
//! own precedent does: `every_vault_file_write_is_the_atomic_writer` in `tests/atomic.rs` reads
//! `atomic.rs` and `fs.rs` as the writer and its port, and this census reads them the same way.
//! A call is made on the journal guard a staged run builds for its operation, or it is one of the
//! staged executor's borrowing adapter's delegations (a third class, held at four, so a fifth
//! delegation fails the census), or it is a named exclusion: the drill notes' writers, which wait
//! on a maintainer decision (#56). The census holds the count it examined equal to that listing,
//! so a new call anywhere in the crate fails it until it is classed, and a control that takes one
//! call off the guard is flagged.

// An integration test is test code: its helpers panic on an unreadable file, and the census prints
// its examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// The calls the census lists.
const METHODS: [&str; 5] = [
    "create_dir_all",
    "create_dir",
    "rename",
    "remove_file",
    "remove_dir",
];

/// The files outside the population: the atomic writer, and the port it writes through.
const WRITER: [&str; 2] = ["atomic.rs", "fs.rs"];

/// The staged executor's file, where every call is made on the run's journal guard.
const STAGED: &str = "staged.rs";

/// The receiver every guarded call is made on: the guard the staged executor builds per operation.
const GUARD: &str = "guard";

/// The borrowing adapter's receiver inside its own delegations.
const ADAPTER_SELF: &str = "self.0";

/// The borrowing adapter's constructor, which only the guard's builder may call.
const ADAPTER: &str = "Borrowed(";

/// The guard's builder in the staged executor.
const GUARD_BUILDER: &str = "guard";

/// The named exclusions, by file, enclosing function and call, each with how many times it occurs:
/// the dated readings tree's own folder, rename and removal calls, which the drill notes' writers
/// make and which wait on a maintainer decision (#56).
const EXCLUDED: [(&str, &str, &str, usize); 5] = [
    ("readings_tree.rs", "day_folder", "create_dir", 1),
    ("readings_tree.rs", "archive_dir", "create_dir", 1),
    ("readings_tree.rs", "archive_file", "rename", 1),
    ("readings_tree.rs", "roll_note", "remove_file", 2),
    ("readings_tree.rs", "remove_if_empty", "remove_dir", 1),
];

/// How many calls the population holds: two on the guard, the adapter's four delegations, and the
/// six named exclusions.
const LISTED: usize = 12;

/// How a census call is classed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// Made on the run's journal guard.
    Guarded,
    /// One of the borrowing adapter's delegations.
    Delegation,
    /// A named exclusion.
    Excluded,
    /// None of the above.
    Flagged,
}

/// One call the census found.
#[derive(Clone, Debug)]
struct Call {
    file: String,
    line: usize,
    function: String,
    method: &'static str,
    receiver: String,
    /// Where the receiver starts in the file's code.
    at: usize,
}

/// What the census found.
#[derive(Debug, Default)]
struct Census {
    calls: Vec<(Call, Class)>,
    /// The adapter's constructor calls, each with its enclosing function.
    adapters: Vec<String>,
}

impl Census {
    fn of(&self, class: Class) -> Vec<&Call> {
        self.calls
            .iter()
            .filter(|(_, found)| *found == class)
            .map(|(call, _)| call)
            .collect()
    }

    fn flagged(&self) -> Vec<String> {
        self.of(Class::Flagged)
            .into_iter()
            .map(|call| {
                format!(
                    "{}:{}: {}.{} in fn {}",
                    call.file, call.line, call.receiver, call.method, call.function
                )
            })
            .collect()
    }
}

/// Every `.rs` file under `dir`, sorted.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).expect("a source folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// The crate's sources outside the writer, by name, with their text.
fn population() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = examined("source file(s) under src", sources(&src));
    files
        .iter()
        .map(|path| {
            let name = path
                .strip_prefix(&src)
                .expect("a file under src")
                .to_string_lossy()
                .into_owned();
            let text = fs::read_to_string(path).expect("a source file");
            (name, text)
        })
        .filter(|(name, _)| !WRITER.contains(&name.as_str()))
        .collect()
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `text` with every comment, string literal and character literal blanked, newlines kept, so a
/// name in a comment or a string is not a call and line numbers still hold.
fn code_of(text: &str) -> Vec<char> {
    let chars: Vec<char> = text.chars().collect();
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let prev_ident = i > 0 && is_ident(chars[i - 1]);
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.extend([' ', ' ']);
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.extend([' ', ' ']);
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
        } else if c == 'r'
            && matches!(next, Some('"' | '#'))
            && (!prev_ident || (chars[i - 1] == 'b' && (i < 2 || !is_ident(chars[i - 2]))))
        {
            let mut j = i + 1;
            while chars.get(j) == Some(&'#') {
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                let hashes = j - i - 1;
                let mut k = j + 1;
                while k < chars.len() {
                    if chars[k] == '"' && (1..=hashes).all(|h| chars.get(k + h) == Some(&'#')) {
                        k += 1 + hashes;
                        break;
                    }
                    k += 1;
                }
                out.extend(chars[i..k.min(chars.len())].iter().map(|&c| blank(c)));
                i = k;
            } else {
                out.push(c);
                i += 1;
            }
        } else if c == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    out.push(' ');
                    if let Some(&escaped) = chars.get(i + 1) {
                        out.push(blank(escaped));
                    }
                    i += 2;
                } else if chars[i] == '"' {
                    out.push(' ');
                    i += 1;
                    break;
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
        } else if c == '\'' && next == Some('\\') {
            let mut k = i + 3;
            while k < chars.len() && chars[k] != '\'' {
                k += 1;
            }
            out.extend(chars[i..=k.min(chars.len() - 1)].iter().map(|_| ' '));
            i = k + 1;
        } else if c == '\'' && chars.get(i + 2) == Some(&'\'') {
            out.extend([' ', ' ', ' ']);
            i += 3;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `code` with every item under `#[cfg(test)]` blanked: the attribute, then through the first `;`
/// or the `}` that closes the first `{`.
fn without_test_items(mut code: Vec<char>) -> Vec<char> {
    let marker: Vec<char> = "#[cfg(test)]".chars().collect();
    let mut i = 0;
    while i + marker.len() <= code.len() {
        if code[i..i + marker.len()] != marker[..] {
            i += 1;
            continue;
        }
        let mut end = i + marker.len();
        let mut depth = 0usize;
        while end < code.len() {
            match code[end] {
                ';' if depth == 0 => break,
                '{' => depth += 1,
                '}' => {
                    if depth <= 1 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            end += 1;
        }
        let stop = end.min(code.len() - 1);
        for c in &mut code[i..=stop] {
            if *c != '\n' {
                *c = ' ';
            }
        }
        i = stop + 1;
    }
    code
}

/// Whether `code` holds `word` at `at`, with no identifier character on either side.
fn word_at(code: &[char], at: usize, word: &str) -> bool {
    let word: Vec<char> = word.chars().collect();
    code.get(at..at + word.len()) == Some(&word[..])
        && (at == 0 || !is_ident(code[at - 1]))
        && code.get(at + word.len()).is_none_or(|&c| !is_ident(c))
}

/// The name of the last `fn` declared before `at`, or an empty name at the top level.
fn enclosing_function(code: &[char], at: usize) -> String {
    let mut found = String::new();
    let mut i = 0;
    while i + 3 < at {
        if word_at(code, i, "fn") && code[i + 2] == ' ' {
            let name: String = code[i + 3..]
                .iter()
                .skip_while(|c| **c == ' ')
                .take_while(|c| is_ident(**c))
                .collect();
            found = name;
        }
        i += 1;
    }
    found
}

/// The receiver of a call whose `.` or `::` starts at `dot`: the path before it, read backwards
/// over whitespace, identifiers, field accesses and path separators; and where it starts.
fn receiver_before(code: &[char], dot: usize) -> (String, usize) {
    let mut end = dot;
    while end > 0 && code[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (is_ident(code[start - 1]) || matches!(code[start - 1], '.' | ':')) {
        start -= 1;
    }
    (code[start..end].iter().collect(), start)
}

/// Every census call in one file's code.
fn calls_in(file: &str, code: &[char]) -> Vec<Call> {
    let mut calls = Vec::new();
    for at in 0..code.len() {
        let separator = match code[at] {
            '.' => at,
            ':' if at > 0 && code[at - 1] == ':' => at - 1,
            _ => continue,
        };
        for method in METHODS {
            let name: Vec<char> = method.chars().collect();
            let after = at + 1;
            if code.get(after..after + name.len()) == Some(&name[..])
                && code.get(after + name.len()) == Some(&'(')
            {
                let (receiver, start) = receiver_before(code, separator);
                calls.push(Call {
                    file: file.to_owned(),
                    line: code[..at].iter().filter(|c| **c == '\n').count() + 1,
                    function: enclosing_function(code, at),
                    method,
                    receiver,
                    at: start,
                });
            }
        }
    }
    calls
}

/// The function each call of the adapter's constructor sits in.
fn adapters_in(code: &[char]) -> Vec<String> {
    let name: Vec<char> = ADAPTER.chars().collect();
    (0..code.len())
        .filter(|&at| {
            code.get(at..at + name.len()) == Some(&name[..]) && (at == 0 || !is_ident(code[at - 1]))
        })
        .filter(|&at| {
            let before: String = code[at.saturating_sub(7)..at].iter().collect();
            !before.trim_end().ends_with("struct")
        })
        .map(|at| enclosing_function(code, at))
        .collect()
}

/// Classes every call in `files`.
fn census(files: &[(String, String)]) -> Census {
    let mut found = Census::default();
    for (file, text) in files {
        let code = without_test_items(code_of(text));
        if file == STAGED {
            found.adapters.extend(adapters_in(&code));
        }
        for call in calls_in(file, &code) {
            let excluded = EXCLUDED.iter().any(|(name, function, method, _)| {
                name == file && *function == call.function && *method == call.method
            });
            let class = if file == STAGED && call.receiver == GUARD {
                Class::Guarded
            } else if file == STAGED
                && call.receiver == ADAPTER_SELF
                && call.function == call.method
            {
                Class::Delegation
            } else if excluded {
                Class::Excluded
            } else {
                Class::Flagged
            };
            found.calls.push((call, class));
        }
    }
    found
}

#[test]
fn every_folder_and_rename_call_of_the_vault_is_guarded_or_named() {
    let files = population();
    let found = census(&files);

    let flagged = found.flagged();
    assert!(
        flagged.is_empty(),
        "folder, rename and removal calls neither on the journal guard nor named: {flagged:#?}"
    );
    assert_eq!(
        found.of(Class::Guarded).len(),
        2,
        "the staged executor's folder and rename calls on the guard"
    );
    assert_eq!(
        found.of(Class::Delegation).len(),
        4,
        "the borrowing adapter's delegations"
    );
    for (name, function, method, count) in EXCLUDED {
        let named = found
            .of(Class::Excluded)
            .into_iter()
            .filter(|call| call.file == name && call.function == function && call.method == method)
            .count();
        assert_eq!(
            named, count,
            "the named exclusion {name}:{function}:{method} occurs as listed"
        );
    }
    assert_eq!(
        found.adapters,
        vec![GUARD_BUILDER.to_owned()],
        "the borrowing adapter is built once, by the guard's builder"
    );
    let calls = examined("folder, rename and removal call(s)", found.calls);
    assert_eq!(
        calls.len(),
        LISTED,
        "the census lists every call: {} examined in {} file(s)",
        calls.len(),
        files.len()
    );
}

#[test]
fn an_unguarded_staged_call_is_flagged() {
    let files = population();
    let found = census(&files);
    let guarded = found.of(Class::Guarded);
    assert!(
        !guarded.is_empty(),
        "the staged executor makes no call on its journal guard"
    );
    assert!(
        found.flagged().is_empty(),
        "the tree as committed flags nothing: {:#?}",
        found.flagged()
    );

    // The control: the first guarded call made on the executor's file system instead.
    let first = guarded[0].clone();
    let controlled: Vec<(String, String)> = files
        .iter()
        .map(|(name, text)| {
            if *name != first.file {
                return (name.clone(), text.clone());
            }
            let mut chars: Vec<char> = text.chars().collect();
            let end = first.at + GUARD.chars().count();
            chars.splice(first.at..end, "self.fs".chars());
            (name.clone(), chars.into_iter().collect())
        })
        .collect();
    let control = census(&controlled);

    assert_eq!(
        control.flagged().len(),
        1,
        "taking one call off the guard is flagged once: {:#?}",
        control.flagged()
    );
    assert_eq!(control.of(Class::Guarded).len(), guarded.len() - 1);
}
