//! The shipped-source reader both censuses include by `mod shipped_sources;` (SPEC-436 R5, R6,
//! R12's key and R21). It reads a list of (path, text) as `proc-macro2` tokens, keys a site and
//! decides a census's verdict at its level. Only `repository_sources` reads the tree, and only the
//! repository tests call it (R19); nothing here writes a file.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};

/// A census's level (ADR-450 D5, D6): at `Warn` an unlisted site prints and passes, at `Fail` it
/// fails the census. A listed entry no site matches fails at both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// An unlisted site prints its `WARN` line and the census passes.
    Warn,
    /// An unlisted site fails the census.
    Fail,
}

/// One shipped source: its path from the repository root and its tokens, test code removed.
pub struct Shipped {
    /// `crates/<member>/src/...`.
    pub path: String,
    /// The file's tokens, with every test item dropped (R6).
    pub tokens: Vec<TokenTree>,
}

/// A site's key (R12, R17): its file, its enclosing function, its form and its ordinal among that
/// function's sites of that form, from 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    /// The file, from the repository root.
    pub file: String,
    /// The enclosing function's name.
    pub function: String,
    /// The resolved path, the method form or the macro form.
    pub form: String,
    /// The site's place among its function's sites of its form, from 1.
    pub ordinal: usize,
}

impl fmt::Display for Key {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} {} #{}",
            self.file, self.function, self.form, self.ordinal
        )
    }
}

/// Planted sources in the shape every census function takes (R19).
pub fn planted(files: &[(&str, &str)]) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

/// Every Rust file under each workspace member's `src`, read from `CARGO_MANIFEST_DIR`'s parent
/// and sorted by path. Only the repository tests call it (R19).
pub fn repository_sources() -> Result<Vec<(String, String)>, String> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    let mut pending: Vec<PathBuf> = Vec::new();
    for member in entries(&crates)? {
        let src = member.join("src");
        if src.is_dir() {
            pending.push(src);
        }
    }
    while let Some(directory) = pending.pop() {
        for path in entries(&directory)? {
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let text = fs::read_to_string(&path)
                    .map_err(|error| format!("{} does not read: {error}", path.display()))?;
                found.push((repository_path(&crates, &path)?, text));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// A directory's entries, sorted.
fn entries(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let listing = fs::read_dir(directory)
        .map_err(|error| format!("{} does not list: {error}", directory.display()))?;
    let mut paths = Vec::new();
    for entry in listing {
        paths.push(
            entry
                .map_err(|error| format!("{}: {error}", directory.display()))?
                .path(),
        );
    }
    paths.sort();
    Ok(paths)
}

/// A path under the crates directory, spelt from the repository root with `/`.
fn repository_path(crates: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(crates)
        .map_err(|error| format!("{} is outside the crates: {error}", path.display()))?;
    let parts: Vec<String> = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    Ok(format!("crates/{}", parts.join("/")))
}

/// The shipped sources among `files` (R5): each `crates/<member>/src/**/*.rs`, read as tokens
/// after one byte order mark is removed, less each out-of-line test module a file declares under
/// `#[cfg(test)]`, with every test item dropped (R6). A file that does not lex fails, naming it.
pub fn shipped(files: &[(String, String)]) -> Result<Vec<Shipped>, String> {
    let mut lexed = Vec::new();
    for (path, text) in files {
        if !is_shipped_path(path) {
            continue;
        }
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let stream =
            TokenStream::from_str(text).map_err(|error| format!("{path} does not lex: {error}"))?;
        lexed.push(Shipped {
            path: path.clone(),
            tokens: stream.into_iter().collect(),
        });
    }
    let tests: Vec<String> = lexed
        .iter()
        .flat_map(|source| test_modules(&source.path, &source.tokens))
        .collect();
    Ok(lexed
        .into_iter()
        .filter(|source| !tests.contains(&source.path))
        .map(|source| Shipped {
            tokens: without_tests(&source.tokens),
            path: source.path,
        })
        .collect())
}

/// Whether a path is a member's shipped Rust source, `crates/<member>/src/**/*.rs`.
fn is_shipped_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    parts.len() >= 4
        && parts.first() == Some(&"crates")
        && parts.get(2) == Some(&"src")
        && Path::new(path)
            .extension()
            .is_some_and(|extension| extension == "rs")
}

/// The files of the out-of-line modules a file declares at its top level under `#[cfg(test)]`:
/// `mod x;` read from `x.rs` or `x/mod.rs` in the file's module directory, or the file a
/// `#[path = ".."]` names beside it.
fn test_modules(path: &str, tokens: &[TokenTree]) -> Vec<String> {
    let mut found = Vec::new();
    let mut test = false;
    let mut named: Option<String> = None;
    for (at, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group)
                if group.delimiter() == Delimiter::Bracket
                    && is_punct(before(tokens, at, 1), '#') =>
            {
                let inner = tokens_in(group);
                test |= marks_test_code(&inner);
                if let Some(target) = path_attribute(&inner) {
                    named = Some(target);
                }
            }
            TokenTree::Ident(ident) if ident == "mod" && is_punct(tokens.get(at + 2), ';') => {
                if let (true, Some(TokenTree::Ident(name))) = (test, tokens.get(at + 1)) {
                    found.extend(module_files(path, &name.to_string(), named.as_deref()));
                }
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                test = false;
                named = None;
            }
            TokenTree::Punct(punct) if punct.as_char() == ';' => {
                test = false;
                named = None;
            }
            _ => {}
        }
    }
    found
}

/// The files an out-of-line `mod name;` in `path` may be read from.
fn module_files(path: &str, name: &str, named: Option<&str>) -> Vec<String> {
    let (directory, file) = path.rsplit_once('/').unwrap_or(("", path));
    if let Some(target) = named {
        return vec![normalised(&format!("{directory}/{target}"))];
    }
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    let module = if ["lib", "main", "mod"].contains(&stem) {
        directory.to_owned()
    } else {
        format!("{directory}/{stem}")
    };
    vec![
        format!("{module}/{name}.rs"),
        format!("{module}/{name}/mod.rs"),
    ]
}

/// A `/` path with each `.` dropped and each `..` taking the part before it.
fn normalised(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// The file a `#[path = ".."]` attribute's inside names.
fn path_attribute(inner: &[TokenTree]) -> Option<String> {
    match inner {
        [
            TokenTree::Ident(name),
            TokenTree::Punct(equals),
            TokenTree::Literal(value),
        ] if name == "path" && equals.as_char() == '=' => {
            Some(value.to_string().trim_matches('"').to_owned())
        }
        _ => None,
    }
}

/// Whether an attribute's inside marks test code: `test` or `cfg(test)` (R6).
fn marks_test_code(inner: &[TokenTree]) -> bool {
    match inner {
        [TokenTree::Ident(name)] => name == "test",
        [TokenTree::Ident(name), TokenTree::Group(arguments)] => {
            name == "cfg"
                && matches!(tokens_in(arguments).as_slice(), [TokenTree::Ident(only)] if only == "test")
        }
        _ => false,
    }
}

/// The tokens with every item that carries `#[test]` or `#[cfg(test)]` dropped, at every depth
/// (R6). A dropped item ends at its first `;` or brace group, or at its `;` alone once its keyword
/// is `const`, `static`, `let`, `use` or `type`.
fn without_tests(tokens: &[TokenTree]) -> Vec<TokenTree> {
    let mut kept = Vec::new();
    let mut skipping = false;
    let mut to_semicolon = false;
    for (at, token) in tokens.iter().enumerate() {
        if skipping {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == ';' => skipping = false,
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    skipping = to_semicolon;
                }
                TokenTree::Ident(word) if STATEMENT_ITEMS.iter().any(|item| word == item) => {
                    to_semicolon = !is_ident(tokens.get(at + 1), "fn");
                }
                _ => {}
            }
            continue;
        }
        match token {
            TokenTree::Group(group)
                if group.delimiter() == Delimiter::Bracket
                    && is_punct(before(tokens, at, 1), '#')
                    && marks_test_code(&tokens_in(group)) =>
            {
                kept.pop();
                skipping = true;
                to_semicolon = false;
            }
            TokenTree::Group(group) => kept.push(TokenTree::Group(Group::new(
                group.delimiter(),
                without_tests(&tokens_in(group)).into_iter().collect(),
            ))),
            other => kept.push(other.clone()),
        }
    }
    kept
}

/// The item keywords whose item ends at its `;` even when it holds a brace group.
const STATEMENT_ITEMS: [&str; 5] = ["const", "static", "let", "use", "type"];

/// Whether a token is the punctuation `ch`.
pub fn is_punct(token: Option<&TokenTree>, ch: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == ch)
}

/// Whether a token is the identifier `name`.
pub fn is_ident(token: Option<&TokenTree>, name: &str) -> bool {
    matches!(token, Some(TokenTree::Ident(ident)) if ident == name)
}

/// A group's tokens, without its delimiters.
pub fn tokens_in(group: &Group) -> Vec<TokenTree> {
    group.stream().into_iter().collect()
}

/// The token `n` places before `at`, if there is one.
pub fn before(tokens: &[TokenTree], at: usize, n: usize) -> Option<&TokenTree> {
    at.checked_sub(n).and_then(|place| tokens.get(place))
}

/// The path `a::b::c` whose last segment is the identifier at `at`, read backwards.
pub fn path_ending_at(tokens: &[TokenTree], at: usize) -> Vec<String> {
    let mut path = Vec::new();
    let mut colons = 0;
    for token in tokens.get(..=at).unwrap_or_default().iter().rev() {
        match token {
            TokenTree::Ident(ident) if colons == 0 && path.is_empty() || colons == 2 => {
                path.push(ident.to_string());
                colons = 0;
            }
            TokenTree::Punct(punct) if punct.as_char() == ':' && !path.is_empty() && colons < 2 => {
                colons += 1;
            }
            _ => break,
        }
    }
    path.reverse();
    path
}

/// The name of the function whose `fn` keyword is at `at`, and whether it is an `async fn`.
pub fn fn_head(tokens: &[TokenTree], at: usize) -> Option<(String, bool)> {
    if !is_ident(tokens.get(at), "fn") {
        return None;
    }
    let Some(TokenTree::Ident(name)) = tokens.get(at + 1) else {
        return None;
    };
    let mut is_async = false;
    for token in tokens.get(..at).unwrap_or_default().iter().rev() {
        let qualifier = match token {
            TokenTree::Ident(word) => FN_QUALIFIERS.iter().any(|qualifier| word == qualifier),
            TokenTree::Literal(_) => true,
            _ => false,
        };
        if !qualifier {
            break;
        }
        is_async |= is_ident(Some(token), "async");
    }
    Some((name.to_string(), is_async))
}

/// The words that may stand before `fn` in a function's head, with an ABI string.
const FN_QUALIFIERS: [&str; 5] = ["async", "const", "unsafe", "extern", "default"];

/// Whether the `const` at `at` opens a `const` item: not a `const fn`, a `const { }` block, a
/// const generic parameter or a `*const` pointer.
pub fn opens_const_item(tokens: &[TokenTree], at: usize) -> bool {
    is_ident(tokens.get(at), "const")
        && !["fn", "unsafe", "async", "extern"]
            .iter()
            .any(|word| is_ident(tokens.get(at + 1), word))
        && !matches!(tokens.get(at + 1), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace)
        && !['<', ',', '*']
            .iter()
            .any(|ch| is_punct(before(tokens, at, 1), *ch))
}

/// The keys of one file's sites, each `(function, form)` in the order the walk met it (R12, R17).
pub fn keyed(file: &str, sites: &[(String, String)]) -> Vec<Key> {
    sites
        .iter()
        .enumerate()
        .map(|(at, site)| Key {
            file: file.to_owned(),
            function: site.0.clone(),
            form: site.1.clone(),
            ordinal: sites[..at]
                .iter()
                .filter(|earlier| *earlier == site)
                .count()
                + 1,
        })
        .collect()
}

/// The census's verdict over its found keys and its listing (R21). Each unlisted site prints
/// `WARN <rule> <key>` and passes at `Warn`, or prints `FAIL <rule> <key>` and fails at `Fail`;
/// each listed entry no site matches prints `STALE <rule> <entry>` and fails at both. `Ok` holds
/// the printed lines when it passes, `Err` when it fails.
pub fn verdict(
    rule: &str,
    level: Level,
    found: &[Key],
    listed: &[&str],
) -> Result<Vec<String>, Vec<String>> {
    let fails = level == Level::Fail;
    let mut failed = false;
    let mut lines = Vec::new();
    for key in found {
        let key = key.to_string();
        if !listed.contains(&key.as_str()) {
            lines.push(format!(
                "{} {rule} {key}",
                if fails { "FAIL" } else { "WARN" }
            ));
            failed |= fails;
        }
    }
    for entry in listed {
        if !found.iter().any(|key| key.to_string() == *entry) {
            lines.push(format!("STALE {rule} {entry}"));
            failed = true;
        }
    }
    for line in &lines {
        eprintln!("{line}");
    }
    if failed { Err(lines) } else { Ok(lines) }
}
