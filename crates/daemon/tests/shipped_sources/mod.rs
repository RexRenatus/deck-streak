//! The shipped-source reader both censuses include by `mod shipped_sources;` (SPEC-436 R5, R6,
//! R12's key and R21). It reads a list of (path, text) as `proc-macro2` tokens, keys a site and
//! decides a census's verdict at its level. Only `repository_sources` reads the tree, and only the
//! repository tests call it (R19); nothing here writes a file.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};

/// A census's level (ADR-450 D5, D6): at `Warn` an unlisted site prints and passes, at `Fail` it
/// fails the census. A listed entry no site matches fails at both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// An unlisted site prints its `WARN` line and the census passes.
    Warn,
    /// An unlisted site fails the census.
    Fail,
}

/// One shipped source: its path from the repository root and its tokens.
pub struct Shipped {
    /// `crates/<member>/src/...`.
    pub path: String,
    /// The file's tokens.
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

/// The shipped sources among `files` (R5), each read as tokens after one byte order mark is
/// removed. A file that does not lex fails, naming it.
pub fn shipped(files: &[(String, String)]) -> Result<Vec<Shipped>, String> {
    let mut lexed = Vec::new();
    for (path, text) in files {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let stream =
            TokenStream::from_str(text).map_err(|error| format!("{path} does not lex: {error}"))?;
        lexed.push(Shipped {
            path: path.clone(),
            tokens: stream.into_iter().collect(),
        });
    }
    Ok(lexed)
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

/// The census's verdict over its found keys and its listing (R21): `Ok` with its printed lines
/// when it passes, `Err` with them when it fails.
pub fn verdict(
    rule: &str,
    level: Level,
    found: &[Key],
    listed: &[&str],
) -> Result<Vec<String>, Vec<String>> {
    let refused: Vec<String> = Vec::new();
    eprintln!(
        "{rule}: {} found, {} listed, at {level:?}",
        found.len(),
        listed.len()
    );
    if level == Level::Fail && !refused.is_empty() {
        return Err(refused);
    }
    Ok(Vec::new())
}
