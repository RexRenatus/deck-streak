//! Only progression names `xp_settlement`, and only coordination settles (SPEC-072 A12; R9;
//! ADR-072): no other crate's code names the table, no migration but progression's names it, no
//! crate but coordination names the `settle` operation, and inside coordination a caller outside
//! `crates/coordination/src/recompute/` passes the owner's-correction cause and never the
//! recompute's.
//!
//! The census reads every crate's `src` and every migration, prints how many it examined and
//! refuses zero. It asserts the positive artifacts beside the absences: progression's `settle`
//! module and migration name the table, and the fold's XP step calls `settle`. A planted crate, a
//! planted migration and a planted caller are each refused, by name.

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The table the census guards.
const TABLE: &str = "xp_settlement";
/// The context that owns it (docs/CONTEXT-MAP.md).
const OWNER: &str = "progression";
/// The one context that calls `settle`.
const CALLER: &str = "coordination";
/// The request type `settle` takes. A grouped import never spells the operation's path
/// contiguously, and nothing can call `settle` without building this by name.
const REQUEST: &str = "SettleRequest";
/// The crate's name in a path: a source that never names it cannot reach progression's re-exports.
const PROGRESSION_CRATE: &str = "deck_streak_progression";
/// The prefix every member's crate name carries (`crates/habits` is `deck_streak_habits`).
const CRATE_PREFIX: &str = "deck_streak_";
/// The names progression's own re-exports and aliases are followed from: the operation, its
/// request type, and (through `settle`) its module.
const ORIGINALS: [&str; 2] = ["settle", REQUEST];
/// Where a recompute step lives inside coordination.
const RECOMPUTE_DIR: &str = "crates/coordination/src/recompute/";
/// The cause a caller outside the recompute steps passes.
const CORRECTION_CAUSE: &str = "SettleCause::OwnersCorrection";
/// The cause only a recompute step passes.
const RECOMPUTE_CAUSE: &str = "SettleCause::Recompute";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `directory` whose name ends in `extension`, recursively, by path.
fn files(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return found;
    };
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(files(&path, extension));
        } else if path
            .extension()
            .is_some_and(|name| name.to_str() == Some(extension))
        {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Rust `text` as the compiler reads it, by one lexer: every comment (a `//` or doc line, or a
/// `/* */` block, which nests) is the space it stands for, and each literal (a string, a raw, byte
/// or C string, or a character) is kept whole when `literals` is true and emptied when it is
/// false. The census reads the table from code and its literals, where SQL lives, and every name
/// from code alone. A lifetime (`'a`) is code, and `r#name` is left to the tokens.
fn strip(text: &str, literals: bool) -> String {
    let characters: Vec<char> = text.chars().collect();
    let at_char = |index: usize| characters.get(index).copied();
    let mut code = String::new();
    let mut at = 0;
    while let Some(character) = at_char(at) {
        let next = at_char(at + 1);
        if character == '/' && next == Some('/') {
            while at_char(at).is_some_and(|line| line != '\n') {
                at += 1;
            }
            code.push(' ');
            continue;
        }
        if character == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while let Some(opened) = at_char(at) {
                if opened == '/' && at_char(at + 1) == Some('*') {
                    depth += 1;
                    at += 2;
                } else if opened == '*' && at_char(at + 1) == Some('/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    at += 1;
                }
            }
            code.push(' ');
            continue;
        }
        let end = literal_end(&characters, at);
        if let Some(end) = end {
            if literals {
                code.extend(&characters[at..end]);
            } else {
                code.push_str("\"\"");
            }
            at = end;
            continue;
        }
        code.push(character);
        at += 1;
    }
    code
}

/// Where the literal that opens at `at` ends, or `None` when no literal opens there. A literal
/// opens at a quote, or at `r`, `b`, `c`, `br` or `cr` that starts a word and stands before one
/// (or, raw, before `#`s and one); a `'` opens a character only when it closes one character
/// later or escapes, and is otherwise a lifetime's or a label's.
fn literal_end(characters: &[char], at: usize) -> Option<usize> {
    let at_char = |index: usize| characters.get(index).copied();
    let word_start = at == 0 || !at_char(at - 1).is_some_and(|c| c.is_alphanumeric() || c == '_');
    let prefix = ["br", "cr", "r", "b", "c", ""]
        .into_iter()
        .find(|prefix| {
            word_start
                && prefix.chars().zip(&characters[at..]).all(|(a, b)| a == *b)
                && characters.len() > at + prefix.len()
        })
        .unwrap_or_default();
    let open = at + prefix.len();
    let raw = prefix.ends_with('r');
    let hashes = if raw {
        characters[open..].iter().take_while(|c| **c == '#').count()
    } else {
        0
    };
    match at_char(open + hashes)? {
        '"' => {
            let mut end = open + hashes + 1;
            loop {
                match at_char(end) {
                    Some('\\') if !raw => end += 2,
                    Some('"')
                        if characters[end + 1..]
                            .iter()
                            .take(hashes)
                            .filter(|c| **c == '#')
                            .count()
                            == hashes =>
                    {
                        return Some(end + 1 + hashes);
                    }
                    Some(_) => end += 1,
                    None => return Some(characters.len()),
                }
            }
        }
        '\'' if !raw && matches!(prefix, "" | "b") => {
            match (at_char(open + 1), at_char(open + 2)) {
                (Some('\\'), _) => {
                    let mut end = open + 3;
                    while at_char(end).is_some_and(|c| c != '\'') {
                        end += 1;
                    }
                    Some((end + 1).min(characters.len()))
                }
                (Some(_), Some('\'')) => Some(open + 3),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether Rust source text names the table in its code or its literals, outside every comment.
fn names_the_table(text: &str) -> bool {
    text.contains(TABLE) && strip(text, true).contains(TABLE)
}

/// Whether SQL text names the table outside a `--` comment.
fn sql_names_the_table(text: &str) -> bool {
    text.lines()
        .any(|line| line.split("--").next().unwrap_or_default().contains(TABLE))
}

/// The tokens of Rust `text` that a `use` tree is read from: identifiers, and the punctuation that
/// shapes a tree. A path separator is dropped, because the identifiers in a row are the path.
fn tokens(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut word = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
            continue;
        }
        // `r#name` is the raw spelling of `name`, and the compiler reads the two as one name.
        if character == '#' && word == "r" {
            word.clear();
            continue;
        }
        if !word.is_empty() {
            found.push(std::mem::take(&mut word));
        }
        if matches!(
            character,
            '{' | '}' | ',' | ';' | '*' | '(' | ')' | '=' | '<'
        ) {
            found.push(character.to_string());
        }
    }
    found
}

/// One leaf of a `use` tree: the path to it, and the name it is bound under when it is renamed.
#[derive(Clone)]
struct Leaf {
    path: Vec<String>,
    alias: Option<String>,
}

/// Reads the tree that starts at `at` (grouped, nested, with or without `as`) below `prefix`, into
/// `out`, and stops before the `}` or `;` that ends it.
fn use_tree(tokens: &[String], at: &mut usize, prefix: &[String], out: &mut Vec<Leaf>) {
    loop {
        let mut path = prefix.to_vec();
        let mut alias = None;
        let mut grouped = false;
        while let Some(token) = tokens.get(*at) {
            match token.as_str() {
                "," | "}" | ";" => break,
                "{" => {
                    *at += 1;
                    use_tree(tokens, at, &path, out);
                    grouped = true;
                    if tokens.get(*at).is_some_and(|next| next == "}") {
                        *at += 1;
                    }
                }
                "as" => {
                    alias = tokens.get(*at + 1).cloned();
                    *at += 2;
                }
                "*" => {
                    path.push("*".to_owned());
                    *at += 1;
                }
                name => {
                    path.push(name.to_owned());
                    *at += 1;
                }
            }
        }
        // `settle::{self as ledger}` renames the module the group is read below: its path is the
        // group's own prefix, so it is a leaf because it carries a name.
        let renamed_self = alias.is_some() && path.last().is_some_and(|last| last == "self");
        if path.last().is_some_and(|last| last == "self") {
            path.pop();
        }
        if !grouped && (path.len() > prefix.len() || renamed_self) {
            out.push(Leaf { path, alias });
        }
        if tokens.get(*at).is_some_and(|next| next == ",") {
            *at += 1;
        } else {
            return;
        }
    }
}

/// Whether the item whose keyword is `words[index]` is public to another module: `pub` before
/// it, or a `)` that closes `pub(`. An attribute's `)` does not make an item public.
fn public(words: &[String], index: usize) -> bool {
    index > 0
        && match words[index - 1].as_str() {
            "pub" => true,
            ")" => words[..index - 1]
                .iter()
                .rposition(|open| open == "(")
                .is_some_and(|open| open > 0 && words[open - 1] == "pub"),
            _ => false,
        }
}

/// A name a `use` or an `extern crate` binds: its leaf; whether it is public to another module,
/// and whether to another crate (a plain `pub`, the one visibility that crosses a crate); and the
/// inline module (`mod name { .. }`) it sits in, if any.
#[derive(Clone)]
struct Import {
    leaf: Leaf,
    public: bool,
    exported: bool,
    module: Option<String>,
}

/// Every leaf of every `use` tree and every `extern crate` in Rust `words`, of any visibility.
fn imports(words: &[String]) -> Vec<Import> {
    let mut found = Vec::new();
    let mut modules: Vec<(String, usize)> = Vec::new();
    let mut depth = 0_usize;
    for (index, word) in words.iter().enumerate() {
        if word == "{" {
            if index > 1 && words[index - 2] == "mod" {
                modules.push((words[index - 1].clone(), depth));
            }
            depth += 1;
        } else if word == "}" {
            depth = depth.saturating_sub(1);
            if modules.last().is_some_and(|(_, open)| *open == depth) {
                modules.pop();
            }
        }
        // `extern crate x as y;` binds a name exactly as `use x as y;` does.
        let keyword = match word.as_str() {
            "use" => index,
            "crate" if index > 0 && words[index - 1] == "extern" => index - 1,
            _ => continue,
        };
        let mut leaves = Vec::new();
        use_tree(words, &mut (index + 1), &[], &mut leaves);
        let module = modules.last().map(|(name, _)| name.clone());
        found.extend(leaves.into_iter().map(|leaf| Import {
            leaf,
            public: public(words, keyword),
            exported: keyword > 0 && words[keyword - 1] == "pub",
            module: module.clone(),
        }));
    }
    found
}

/// Every leaf of every public `use` (of any visibility but private, at any depth of module) in a
/// source, and every `pub type` alias.
fn reexports(source: &Source) -> Vec<Leaf> {
    let words = &source.words;
    let mut leaves: Vec<Leaf> = source
        .imports
        .iter()
        .filter(|import| import.public)
        .map(|import| import.leaf.clone())
        .collect();
    for (index, word) in words.iter().enumerate() {
        // `pub type Alias<'a> = path::Original<'a>;` names the original as `Alias`.
        if word == "type" && public(words, index) {
            let equals = words[index..].iter().position(|next| next == "=");
            if let (Some(alias), Some(equals)) = (words.get(index + 1), equals) {
                let path: Vec<String> = words[index + equals + 1..]
                    .iter()
                    .take_while(|next| !matches!(next.as_str(), "<" | ";"))
                    .cloned()
                    .collect();
                if !path.is_empty() {
                    leaves.push(Leaf {
                        path,
                        alias: Some(alias.clone()),
                    });
                }
            }
        }
    }
    leaves
}

/// The workspace's members, by path.
fn members(root: &Path) -> Vec<PathBuf> {
    let mut members: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .expect("crates/ is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    members
}

/// A Rust source of a member, read once: its path and text, its code (every comment removed and
/// every literal emptied), the code's words, and every name its `use`s and `extern crate`s bind.
struct Source {
    path: PathBuf,
    text: String,
    code: String,
    words: Vec<String>,
    imports: Vec<Import>,
}

/// A member of the workspace: its directory's name, the names its crate goes by in another
/// crate's paths, and its `src`.
struct Member {
    context: String,
    crate_names: Vec<String>,
    sources: Vec<Source>,
}

/// The workspace at `root`, every member's every source read once.
fn workspace(root: &Path) -> Vec<Member> {
    members(root)
        .into_iter()
        .map(|member| Member {
            context: member
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a UTF-8 crate directory")
                .to_owned(),
            crate_names: member_crate_names(&member),
            sources: files(&member.join("src"), "rs")
                .into_iter()
                .map(|path| {
                    let text = fs::read_to_string(&path).expect("a readable source");
                    let code = strip(&text, false);
                    let words = tokens(&code);
                    let imports = imports(&words);
                    Source {
                        path,
                        text,
                        code,
                        words,
                        imports,
                    }
                })
                .collect(),
        })
        .collect()
}

/// The names progression's `settle`, its request and its module are given by a renaming public
/// `use` or a `pub type` in any member's `src`, each with the original it stands for. A renamed
/// name is followed too, so a chain of renamings ends at the original, in whatever member each
/// link is written.
fn aliases(workspace: &[Member]) -> BTreeMap<String, String> {
    let leaves: Vec<Leaf> = workspace
        .iter()
        .flat_map(|member| &member.sources)
        .flat_map(reexports)
        .collect();
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    loop {
        let before = aliases.len();
        for leaf in &leaves {
            let (Some(last), Some(alias)) = (leaf.path.last(), &leaf.alias) else {
                continue;
            };
            let original = if ORIGINALS.contains(&last.as_str()) {
                last.clone()
            } else if let Some(original) = aliases.get(last) {
                original.clone()
            } else {
                continue;
            };
            aliases.entry(alias.clone()).or_insert(original);
        }
        if aliases.len() == before {
            return aliases;
        }
    }
}

/// A TOML text, read by one reader: every comment is removed outside a string before anything
/// else is read, a key's parts may be bare, quoted or dotted, and a table may be a header or
/// inline (spanning lines, as TOML 1.1 allows).
struct Toml {
    text: Vec<char>,
    at: usize,
}

impl Toml {
    fn new(text: &str) -> Self {
        let mut code = String::new();
        let mut quote: Option<&str> = None;
        let mut rest = text;
        while let Some(character) = rest.chars().next() {
            let opens = ["\"\"\"", "'''", "\"", "'"]
                .into_iter()
                .find(|open| rest.starts_with(open));
            let taken = match quote {
                Some(close) if rest.starts_with(close) => {
                    quote = None;
                    close.len()
                }
                Some(close) if close.starts_with('"') && character == '\\' => {
                    let escaped = rest[1..].chars().next().map_or(0, char::len_utf8);
                    1 + escaped
                }
                None if character == '#' => {
                    rest = &rest[rest.find('\n').unwrap_or(rest.len())..];
                    continue;
                }
                None if opens.is_some() => {
                    quote = opens;
                    opens.map_or(1, str::len)
                }
                _ => character.len_utf8(),
            };
            code.push_str(&rest[..taken]);
            rest = &rest[taken..];
        }
        Self {
            text: code.chars().collect(),
            at: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.text.get(self.at).copied()
    }

    /// Skips spaces, and line ends too when `lines` is true.
    fn blank(&mut self, lines: bool) {
        while self
            .peek()
            .is_some_and(|c| c == ' ' || c == '\t' || c == '\r' || (lines && c == '\n'))
        {
            self.at += 1;
        }
    }

    fn eat(&mut self, expected: char) -> bool {
        let found = self.peek() == Some(expected);
        self.at += usize::from(found);
        found
    }

    /// A string, whole: basic or literal, on one line or three-quoted.
    fn string(&mut self) -> Option<String> {
        let quote = self.peek().filter(|c| *c == '"' || *c == '\'')?;
        let triple = self
            .text
            .get(self.at..self.at + 3)
            .is_some_and(|three| three.iter().all(|c| *c == quote));
        let width = if triple { 3 } else { 1 };
        self.at += width;
        let mut value = String::new();
        loop {
            let character = self.peek()?;
            if self.text[self.at..]
                .iter()
                .take(width)
                .filter(|c| **c == quote)
                .count()
                == width
            {
                self.at += width;
                return Some(value);
            }
            if character == '\\' && quote == '"' {
                self.at += 1;
                value.push(self.peek()?);
            } else {
                value.push(character);
            }
            self.at += 1;
        }
    }

    /// A key: its parts, each bare or quoted, joined by `.`.
    fn key(&mut self) -> Option<Vec<String>> {
        let mut parts = Vec::new();
        loop {
            self.blank(false);
            let part = if matches!(self.peek(), Some('"' | '\'')) {
                self.string()?
            } else {
                let start = self.at;
                while self
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
                {
                    self.at += 1;
                }
                (self.at > start).then(|| self.text[start..self.at].iter().collect())?
            };
            parts.push(part);
            self.blank(false);
            if !self.eat('.') {
                return Some(parts);
            }
        }
    }

    /// Reads the value at the cursor, pushing every string in it by its full key into `out`.
    fn value(&mut self, key: &[String], out: &mut Vec<(Vec<String>, String)>) -> Option<()> {
        match self.peek()? {
            '"' | '\'' => {
                let value = self.string()?;
                out.push((key.to_vec(), value));
            }
            '{' => {
                self.at += 1;
                loop {
                    self.blank(true);
                    if self.eat('}') {
                        break;
                    }
                    self.pair(key, out)?;
                    self.blank(true);
                    if !self.eat(',') {
                        self.blank(true);
                        self.eat('}').then_some(())?;
                        break;
                    }
                }
            }
            '[' => {
                self.at += 1;
                loop {
                    self.blank(true);
                    if self.eat(']') {
                        break;
                    }
                    self.value(key, out)?;
                    self.blank(true);
                    if !self.eat(',') {
                        self.blank(true);
                        self.eat(']').then_some(())?;
                        break;
                    }
                }
            }
            _ => {
                let start = self.at;
                while self
                    .peek()
                    .is_some_and(|c| !matches!(c, ',' | '}' | ']' | '\n' | ' ' | '\t' | '\r'))
                {
                    self.at += 1;
                }
                (self.at > start).then_some(())?;
            }
        }
        Some(())
    }

    /// `key = value`, below the table `table`.
    fn pair(&mut self, table: &[String], out: &mut Vec<(Vec<String>, String)>) -> Option<()> {
        let key = [table.to_vec(), self.key()?].concat();
        self.blank(false);
        self.eat('=').then_some(())?;
        self.blank(false);
        self.value(&key, out)
    }

    /// Every string the text gives, by its full key (`dependencies.prog.package`), up to the first
    /// place this reader cannot read, and that place's offset if there is one: a manifest it cannot
    /// read to its end is refused, rather than let it hide a rename.
    fn strings(mut self) -> (Vec<(Vec<String>, String)>, Option<usize>) {
        let mut found = Vec::new();
        let mut table: Vec<String> = Vec::new();
        loop {
            self.blank(true);
            let read = match self.peek() {
                None => return (found, None),
                Some('[') => {
                    self.at += 1;
                    let array = self.eat('[');
                    self.key().and_then(|key| {
                        self.eat(']').then_some(())?;
                        (!array || self.eat(']')).then_some(())?;
                        table = key;
                        Some(())
                    })
                }
                Some(_) => self.pair(&table, &mut found),
            };
            self.blank(false);
            if read.is_none() || !matches!(self.peek(), None | Some('\n')) {
                return (found, Some(self.at));
            }
        }
    }
}

/// Every string of the manifest (TOML) at `path` that the reader reads, by its full key, and the
/// offset it stops at if it cannot read the manifest to its end; none for a missing file.
fn manifest(path: &Path) -> (Vec<(Vec<String>, String)>, Option<usize>) {
    fs::read_to_string(path).map_or_else(|_| (Vec::new(), None), |text| Toml::new(&text).strings())
}

/// The workspace's manifest and every member's.
fn manifests(root: &Path) -> Vec<PathBuf> {
    std::iter::once(root.join("Cargo.toml"))
        .chain(
            members(root)
                .into_iter()
                .map(|member| member.join("Cargo.toml")),
        )
        .collect()
}

/// A crate or package name as a path writes it: `-` is `_`.
fn path_name(name: &str) -> String {
    name.replace('-', "_")
}

/// The names a member's crate goes by in another crate's paths: its directory's under the
/// workspace's prefix, and its manifest's `[package]` and `[lib]` names.
fn member_crate_names(member: &Path) -> Vec<String> {
    let directory = member
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a UTF-8 crate directory");
    let mut names = vec![path_name(&format!("{CRATE_PREFIX}{directory}"))];
    for (key, value) in manifest(&member.join("Cargo.toml")).0 {
        if key == ["package", "name"] || key == ["lib", "name"] {
            names.push(path_name(&value));
        }
    }
    names
}

/// Every dependency a manifest (the workspace's or a member's) renames by `package`, as
/// (the name a path writes, the package's name as a path writes it): the inline, table and
/// dotted-key forms, under `dependencies`, a target's or the workspace's.
fn manifest_renames(root: &Path) -> Vec<(String, String)> {
    manifests(root)
        .iter()
        .flat_map(|path| manifest(path).0)
        .filter_map(|(key, package)| match key.as_slice() {
            [.., table, name, last] if last == "package" && table.ends_with("dependencies") => {
                Some((path_name(name), path_name(&package)))
            }
            _ => None,
        })
        .collect()
}

/// The name of the module a source file is, or `None` for a crate's root file.
fn module_of(source: &Path) -> Option<String> {
    match source.file_stem()?.to_str()? {
        "lib" | "main" => None,
        "mod" => Some(source.parent()?.file_name()?.to_str()?.to_owned()),
        stem => Some(stem.to_owned()),
    }
}

/// The names a source reaches progression's crate root by, across every member, to a fixpoint:
/// - its own name, and every name a manifest renames a package of one of these names to;
/// - every name an `as` (a `use`, a grouped `{self as x}`, an `extern crate`) binds one to;
/// - and every name that exports one to other crates: a public `use` or `extern crate` that
///   passes through one of these names makes the crate that holds it (at its root) or the module
///   it sits in a name of progression's root too, and so does one that re-exports the operation
///   itself (by a name `aliases` follows) from progression or from a member that names one of
///   these names anywhere in its `src`.
///
/// The set is one for the workspace, not one for each member: a name bound in one member is
/// followed in every member, which fails closed.
fn crate_names(
    root: &Path,
    workspace: &[Member],
    aliases: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    let mut names = BTreeSet::from([PROGRESSION_CRATE.to_owned()]);
    let renames = manifest_renames(root);
    let operation =
        |name: &String| ORIGINALS.contains(&name.as_str()) || aliases.contains_key(name.as_str());
    let mut links: Vec<(String, String)> = Vec::new();
    let mut exports: Vec<(usize, Vec<String>, Leaf)> = Vec::new();
    let mut vocabulary: Vec<(bool, BTreeSet<String>)> = Vec::new();
    for member in workspace {
        let mut spoken = BTreeSet::new();
        for source in &member.sources {
            spoken.extend(source.words.iter().cloned());
            for import in &source.imports {
                if let (Some(last), Some(alias)) = (import.leaf.path.last(), &import.leaf.alias) {
                    links.push((last.clone(), alias.clone()));
                }
                if import.exported {
                    let module = import.module.clone().or_else(|| module_of(&source.path));
                    exports.push((
                        vocabulary.len(),
                        module.map_or_else(|| member.crate_names.clone(), |name| vec![name]),
                        import.leaf.clone(),
                    ));
                }
            }
        }
        vocabulary.push((member.context == OWNER, spoken));
    }
    loop {
        let before = names.len();
        let by_manifest: Vec<String> = renames
            .iter()
            .filter(|(_, package)| names.contains(package))
            .map(|(name, _)| name.clone())
            .collect();
        names.extend(by_manifest);
        for (original, alias) in &links {
            if names.contains(original) {
                names.insert(alias.clone());
            }
        }
        for (member, exported, leaf) in &exports {
            let (owner, spoken) = &vocabulary[*member];
            let reaches = *owner || spoken.iter().any(|word| names.contains(word));
            if leaf.path.iter().any(|segment| names.contains(segment))
                || (reaches && leaf.path.last().is_some_and(operation))
            {
                names.extend(exported.iter().cloned());
            }
        }
        if names.len() == before {
            return names;
        }
    }
}

/// The members whose `src` imports progression's crate, or a module of it, whole with a glob of
/// any visibility (`use prog::*`): each of their files reaches progression's names as
/// `crate::name` or `super::name` without naming the crate.
fn glob_members(workspace: &[Member], crate_names: &BTreeSet<String>) -> BTreeSet<String> {
    workspace
        .iter()
        .filter(|member| {
            member.sources.iter().any(|source| {
                source.imports.iter().any(|import| {
                    import.leaf.path.last().is_some_and(|last| last == "*")
                        && import
                            .leaf
                            .path
                            .iter()
                            .any(|segment| crate_names.contains(segment))
                })
            })
        })
        .map(|member| member.context.clone())
        .collect()
}

/// Whether a source reaches the operation itself through a name that denotes progression's crate:
/// `alias::settle` in a path, a `use` or a grouped `use`, or, in a member that globs the crate's
/// root, `settle` as a word (`crate::settle`).
fn reaches_settle(source: &Source, crate_names: &BTreeSet<String>, globbed: bool) -> bool {
    let step = |pair: &[String]| crate_names.contains(&pair[0]) && pair[1] == ORIGINALS[0];
    (globbed && source.words.iter().any(|word| word == ORIGINALS[0]))
        || source.words.windows(2).any(step)
        || source
            .imports
            .iter()
            .any(|import| import.leaf.path.windows(2).any(step))
}

/// The census of a tree at `root`.
struct Census {
    sources: Vec<String>,
    migrations: Vec<String>,
    naming: BTreeSet<String>,
    calling: BTreeSet<String>,
    refused: Vec<String>,
}

fn census(root: &Path) -> Census {
    let mut census = Census {
        sources: Vec::new(),
        migrations: Vec::new(),
        naming: BTreeSet::new(),
        calling: BTreeSet::new(),
        refused: Vec::new(),
    };
    let workspace = workspace(root);
    let aliases = aliases(&workspace);
    let crate_names = crate_names(root, &workspace, &aliases);
    let globbed = glob_members(&workspace, &crate_names);
    for path in manifests(root) {
        if let (_, Some(at)) = manifest(&path) {
            census.refused.push(format!(
                "{} is a manifest the census cannot read past character {at}, so it may rename \
                 {OWNER}'s crate unseen",
                relative(root, &path)
            ));
        }
    }
    for member in &workspace {
        let context = &member.context;
        for source in &member.sources {
            let name = relative(root, &source.path);
            if names_the_table(&source.text) {
                census.naming.insert(name.clone());
                if context != OWNER {
                    census
                        .refused
                        .push(format!("{name} names {TABLE}, and only {OWNER}'s code may"));
                }
            }
            let direct = source.code.contains(REQUEST)
                || reaches_settle(source, &crate_names, globbed.contains(context));
            // A source that names progression's crate and one of progression's own renamings
            // reaches `settle` without spelling it.
            let words: BTreeSet<&String> = source.words.iter().collect();
            let through: Vec<(&String, &String)> = if context != OWNER
                && !direct
                && (globbed.contains(context)
                    || words.iter().any(|word| crate_names.contains(*word)))
            {
                aliases
                    .iter()
                    .filter(|(alias, _)| words.contains(alias))
                    .collect()
            } else {
                Vec::new()
            };
            if context != OWNER && (direct || !through.is_empty()) {
                census.calling.insert(name.clone());
                if context != CALLER {
                    if direct {
                        census
                            .refused
                            .push(format!("{name} calls settle, and only {CALLER}'s code may"));
                    }
                    for (alias, original) in through {
                        census.refused.push(format!(
                            "{name} calls settle through {alias}, {OWNER}'s alias of {original}, \
                             and only {CALLER}'s code may"
                        ));
                    }
                } else if !name.starts_with(RECOMPUTE_DIR)
                    && (!source.code.contains(CORRECTION_CAUSE)
                        || source.code.contains(RECOMPUTE_CAUSE))
                {
                    census.refused.push(format!(
                        "{name} calls settle outside the recompute steps, and only the owner's \
                         correction may"
                    ));
                }
            }
            census.sources.push(name);
        }
    }
    for migration in files(&root.join("migrations"), "sql") {
        let name = relative(root, &migration);
        let file = migration
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 migration name");
        // `<SPEC number and sequence>_<owning context>_<slug>.sql` (ADR-020).
        let context = file.split('_').nth(1).unwrap_or_default();
        let text = fs::read_to_string(&migration).expect("a readable migration");
        if sql_names_the_table(&text) {
            census.naming.insert(name.clone());
            if context != OWNER {
                census.refused.push(format!(
                    "{name} names {TABLE}, and only {OWNER}'s migrations may"
                ));
            }
        }
        census.migrations.push(name);
    }
    census
}

/// `path` relative to `root`, with `/` separators.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("a path under the root")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Writes `text` to `root`/`path`, making its folders.
fn plant(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().expect("a planted file's folder")).expect("its folder");
    fs::write(file, text).expect("the planted file");
}

#[test]
fn only_progression_writes_xp_settlement_and_only_coordination_settles() {
    let found = census(&root());
    examined("crate source file(s)", found.sources.clone());
    examined("migration(s)", found.migrations.clone());
    assert_eq!(found.refused, Vec::<String>::new());
    // The positive artifacts: progression's `settle` module and its migration name the table, and
    // the fold's XP step calls the operation.
    let writers: BTreeSet<&str> = [
        "crates/progression/src/settle.rs",
        "migrations/007201_progression_xp_settlement.sql",
    ]
    .into_iter()
    .filter(|path| found.naming.contains(*path))
    .collect();
    assert_eq!(
        writers,
        BTreeSet::from([
            "crates/progression/src/settle.rs",
            "migrations/007201_progression_xp_settlement.sql",
        ]),
        "the settle module and its migration name the table; every file that does: {:?}",
        found.naming
    );
    assert!(
        found
            .calling
            .contains("crates/coordination/src/recompute/xp.rs"),
        "the fold's XP step calls settle; every file that does: {:?}",
        found.calling
    );

    // A planted crate that names the table, a planted migration of another context, a planted
    // crate that calls settle, and a planted coordination caller outside the recompute steps that
    // passes the recompute's cause are refused by name; the owner's correction outside the steps,
    // a recompute step, and a mention in a comment are not.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/chest.rs",
        "//! A chest never writes xp_settlement itself.\n\
         fn pay() {\n    let _ = sqlx::query(\"UPDATE xp_settlement SET amount = 0\"); // a debit\n}\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/relight.rs",
        "use deck_streak_progression::settle::settle;\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/grouped.rs",
        "use deck_streak_progression::{settle::{settle as plant_call, SettleRequest as PlantRequest}};\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/recompute/xp.rs",
        "use deck_streak_progression::settle::{SettleCause, settle};\n\
         fn step() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/correction.rs",
        "use deck_streak_progression::settle::{SettleCause, settle};\n\
         fn fix() { let _ = SettleCause::OwnersCorrection; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/shortcut.rs",
        "use deck_streak_progression::settle::{SettleCause, settle};\n\
         fn quick() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/note.rs",
        "// deck_streak_progression::settle is coordination's alone\nfn quiet() {}\n",
    );
    plant(
        planted.path(),
        "migrations/007201_progression_xp_settlement.sql",
        "CREATE TABLE xp_settlement (id INTEGER PRIMARY KEY) STRICT;\n",
    );
    plant(
        planted.path(),
        "migrations/999901_quests_debit.sql",
        "-- a planted migration of another context\nDELETE FROM xp_settlement;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/coordination/src/shortcut.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/quests/src/chest.rs names xp_settlement, and only progression's code may",
            "crates/streaks/src/grouped.rs calls settle, and only coordination's code may",
            "crates/streaks/src/relight.rs calls settle, and only coordination's code may",
            "migrations/999901_quests_debit.sql names xp_settlement, and only progression's \
             migrations may",
        ]
    );
}

#[test]
fn the_census_reads_progressions_own_reexports_as_it_reads_the_other_crates() {
    // Progression re-exports `settle`, its request and its module under other names, one of them
    // through another alias and one inside a nested module. A caller outside coordination that
    // names only the new names never spells `settle` or `SettleRequest`, and is refused by file,
    // alias and original all the same. Progression's own use of the names, coordination's callers
    // by the same rules as before, a mention in a comment, a crate that never imports progression
    // and a re-export that renames nothing of the settlement are not refused.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\n\
         pub use settle::{\n    SettledRow,\n    settle as tally,\n    SettleRequest as TallyRequest,\n    settled_of_day,\n};\n\
         pub use settle as ledger_write;\n\
         pub use self::tally as tally_again;\n\
         pub mod api {\n    pub use super::settle::settle as run_it;\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/inner.rs",
        "use crate::settle::settle as tally;\nfn own() { let _ = tally; }\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/tally_user.rs",
        "use deck_streak_progression::{TallyRequest, tally};\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/module_user.rs",
        "use deck_streak_progression::ledger_write;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/chained_user.rs",
        "use deck_streak_progression::tally_again;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/nested_user.rs",
        "use deck_streak_progression::api::run_it;\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/recompute/xp.rs",
        "use deck_streak_progression::{SettleCause, tally};\n\
         fn step() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/correction.rs",
        "use deck_streak_progression::{SettleCause, tally};\n\
         fn fix() { let _ = SettleCause::OwnersCorrection; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/shortcut.rs",
        "use deck_streak_progression::{SettleCause, tally};\n\
         fn quick() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/note.rs",
        "use deck_streak_progression::SettledRow;\n// tally and ledger_write are coordination's\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/homonym.rs",
        "fn tally() {}\nfn again() { tally(); }\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/coordination/src/shortcut.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/quests/src/chained_user.rs calls settle through tally_again, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/module_user.rs calls settle through ledger_write, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/nested_user.rs calls settle through run_it, progression's alias \
             of settle, and only coordination's code may",
            "crates/streaks/src/tally_user.rs calls settle through TallyRequest, progression's \
             alias of SettleRequest, and only coordination's code may",
            "crates/streaks/src/tally_user.rs calls settle through tally, progression's alias \
             of settle, and only coordination's code may",
        ]
    );
}

#[test]
fn the_census_follows_a_grouped_module_renaming_and_a_chain_read_before_its_link() {
    // `a_chain.rs` is read before `lib.rs`, so its renamings of `tally` and of a crate-visible
    // link are met before either is known. The module is renamed inside a group, by `self`.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/a_chain.rs",
        "pub use crate::tally as tally_early;\npub use crate::crate_link as via_crate;\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\n\
         pub mod a_chain;\n\
         pub use settle::{self as ledger, settle as tally};\n\
         pub(crate) use settle::settle as crate_link;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/early_user.rs",
        "use deck_streak_progression::a_chain::tally_early;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/crate_link_user.rs",
        "use deck_streak_progression::a_chain::via_crate;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/ledger_user.rs",
        "use deck_streak_progression::ledger;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/crate_link_user.rs calls settle through via_crate, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/early_user.rs calls settle through tally_early, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/ledger_user.rs calls settle through ledger, progression's alias \
             of settle, and only coordination's code may",
        ]
    );
}

/// Plants progression's `settle` module, the file every advisory case reads its names from.
fn plant_settle(root: &Path) {
    plant(
        root,
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
}

#[test]
fn the_census_follows_a_crate_alias() {
    // `prog` is progression's crate under another name, so `use crate::prog::tally` reaches the
    // renamed `settle` without ever spelling the crate's own name.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\npub use settle::settle as tally;\n",
    );
    plant(
        planted.path(),
        "crates/markets/src/prog.rs",
        "pub use deck_streak_progression as prog;\n",
    );
    plant(
        planted.path(),
        "crates/markets/src/via_prog.rs",
        "use crate::prog::tally;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/markets/src/via_prog.rs calls settle through tally, progression's alias of \
             settle, and only coordination's code may",
        ]
    );
}

#[test]
fn the_census_follows_a_type_alias() {
    // `pub type Wrapped<'a> = ...SettleRequest<'a>` names the request under another name.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/wrapped.rs",
        "pub type Wrapped<'a> = crate::settle::SettleRequest<'a>;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/typed.rs",
        "use deck_streak_progression::wrapped::Wrapped;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/typed.rs calls settle through Wrapped, progression's alias of \
             SettleRequest, and only coordination's code may",
        ]
    );
}

#[test]
fn a_private_alias_behind_an_attribute_is_not_a_reexport() {
    // The `)` that closes `#[cfg(test)]` is not the `)` that closes `pub(crate)`: the alias behind
    // it is private, so a caller's own function of the same name is a homonym and stays accepted,
    // while the `pub(crate)` link is followed.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\n\
         #[cfg(test)]\n\
         use settle::settle as gated;\n\
         pub(crate) use settle::settle as crate_link;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/homonym.rs",
        "use deck_streak_progression::SettledRow;\nfn gated() {}\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/link.rs",
        "use deck_streak_progression::crate_link;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/link.rs calls settle through crate_link, progression's alias of \
             settle, and only coordination's code may",
        ]
    );
}

#[test]
fn the_operation_is_matched_as_a_word_not_a_prefix() {
    // `settled_of_day` begins with the operation's path and is another function.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/quests/src/day.rs",
        "use deck_streak_progression::settled_of_day;\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/bare.rs",
        "use deck_streak_progression::settle;\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/direct.rs",
        "use deck_streak_progression::settle::settle;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/streaks/src/bare.rs calls settle, and only coordination's code may",
            "crates/streaks/src/direct.rs calls settle, and only coordination's code may",
        ]
    );
}

/// Round 3's forty-eight forms of binding a name to progression's crate, one per line:
/// `label | the path a caller reaches the crate by | the operation it names | the caller's folder |
/// the caller's first line | the files`, each file `path => text` and files joined by ` || `
/// (`-` is none). `@K` is a crate and `@P` its package, `@N` the bound name, `@M` the member that
/// binds it and `@U` another member. What these tables generate is valid Rust and Cargo under the
/// pinned toolchain: a stratified sample compiles, and the red-first record holds its count.
const FORMS: &str = r#"
F00 as x | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K as @N;\n
F01 as r#x | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K as r#@N;\n
F02 {self as x} | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K::{self as @N};\n
F03 {self as r#x} | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K::{self as r#@N};\n
F04 chain of two | crate::a_link::@N | settle | crates/@M/src/ | - | crates/@M/src/a_link.rs => pub use crate::@N_first as @N;\n || crates/@M/src/lib.rs => pub use @K as @N_first;\n
F05 extern crate as x | @N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => extern crate @K as @N;\n
F06 manifest inline | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies]\n@N = { package = "@P", path = "../x" }\n
F07 manifest table | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies.@N]\npackage = '@P'\npath = '../x'\n
F08 manifest dotted | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies]\n @N.package = "@P"\n @N.path = "../x"\n
F09 re-exported glob of the root | crate | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K::*;\n
B01 {self as x} two groups deep | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use {std::fmt, {@K::{self as @N}}};\n
B02 as x two groups deep | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use {{@K as @N}, std::fmt};\n
B03 {self as r#x} two groups deep | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use {std::io, {@K::{self as r#@N}}};\n
B04 pub(in crate::x) link | super::@N | settle | crates/@M/src/x/ | - | crates/@M/src/lib.rs => pub mod x;\n || crates/@M/src/x.rs => pub(in crate::x) use @K as @N;\n
B05 pub(super) link | crate::x::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => mod x;\n || crates/@M/src/x.rs => pub(super) use @K as @N;\n
B06 pub(super) then pub(crate) chain | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => mod x;\npub(crate) use x::@N_one as @N;\n || crates/@M/src/x.rs => pub(super) use @K as @N_one;\n
B07 chain of three, reverse file order | crate::a_three::@N | settle | crates/@M/src/ | - | crates/@M/src/a_three.rs => pub use crate::b_two::@N_two as @N;\n || crates/@M/src/b_two.rs => pub use crate::@N_one as @N_two;\n || crates/@M/src/lib.rs => pub mod a_three;\npub mod b_two;\npub use @K as @N_one;\n
B08 split, line comment between | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K\n    // the crate, renamed\n    as @N;\n
B09 split, trailing comment | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K // the crate\n    as @N;\n
B10 split in a group, comment between | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K::{\n    // the crate itself\n    self\n    // renamed\n    as @N,\n};\n
B11 block comment before as | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K /* the crate */ as @N;\n
B12 block comment after as | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K as /* renamed */ @N;\n
B13 r#settle through as x | crate::@N | r#settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K as @N;\n
B14 r#settle through the crate's own name | @K | r#settle | crates/@M/src/ | - | -
B15 r#settle through a manifest rename | @N | r#settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies]\n@N = { package = "@P" }\n
B16 extern crate as x in a sub-module | super::@N | settle | crates/@M/src/sub/ | - | crates/@M/src/lib.rs => mod sub;\n || crates/@M/src/sub.rs => extern crate @K as @N;\n
B17 extern crate as r#x in a nested sub-module | crate::sub::deeper::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => mod sub;\n || crates/@M/src/sub.rs => pub(crate) mod deeper {\n    pub(crate) extern crate @K as r#@N;\n}\n
B18 manifest table key, double quotes | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies."@N"]\npackage = "@P"\npath = "../x"\n
B19 manifest table key, single quotes | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies.'@N']\npackage = '@P'\npath = '../x'\n
B20 manifest inline key, double quotes | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies]\n"@N" = { package = "@P", path = "../x" }\n
B21 manifest inline key, single quotes | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies]\n'@N' = { package = '@P', path = '../x' }\n
B22 manifest dotted key, quoted | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies]\n"@N".package = "@P"\n'@N'.path = "../x"\n
B23 manifest target table, quoted cfg | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [target.'cfg(unix)'.dependencies."@N"]\npackage = "@P"\n
B24 manifest table header with a comment | @N | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies.@N] # progression, renamed\npackage = "@P"\npath = "../x"\n
B25 workspace table, quoted key | @N | settle | crates/@M/src/ | - | Cargo.toml => [workspace.dependencies.'@N']\npackage = "@P"\n || crates/@M/Cargo.toml => [dependencies]\n@N = { workspace = true }\n
B26 glob of a use alias of the root | crate | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => use @K as @N;\npub use @N::*;\n
B27 glob of a manifest alias of the root | crate | settle | crates/@M/src/ | - | crates/@M/Cargo.toml => [dependencies]\n@N = { package = "@P" }\n || crates/@M/src/lib.rs => pub use @N::*;\n
B28 glob of an alias through crate:: | crate | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => mod a;\npub use crate::a::@N::*;\n || crates/@M/src/a.rs => pub use @K as @N;\n
B29 glob of an extern-crate alias, grouped | crate | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => extern crate @K as @N;\npub use @N::{*};\n
B30 self:: path to an alias | self::@N | settle | crates/@M/src/ | use @K as @N;\n | -
B31 crate:: path to an alias | crate::@N | settle | crates/@M/src/ | - | crates/@M/src/lib.rs => pub use @K as @N;\n
B32 self:: alias of a crate:: alias | self::@N | settle | crates/@M/src/ | use crate::@N_root as @N;\n | crates/@M/src/lib.rs => pub use @K as @N_root;\n
X01 alias re-exported, caller in another member | deck_streak_@M::@N | settle | crates/@U/src/ | - | crates/@M/Cargo.toml => [package]\nname = "deck-streak-@M"\n || crates/@M/src/lib.rs => pub use @K as @N;\n
X02 glob of the root re-exported, caller in another member | deck_streak_@M | settle | crates/@U/src/ | - | crates/@M/Cargo.toml => [package]\nname = "deck-streak-@M"\n || crates/@M/src/lib.rs => pub use @K::*;\n
X03 glob in a pub module, caller in another member | deck_streak_@M::p | settle | crates/@U/src/ | - | crates/@M/Cargo.toml => [package]\nname = "deck-streak-@M"\n || crates/@M/src/lib.rs => pub mod p {\n    pub use @K::*;\n}\n
X04 glob of the root re-exported, other member globs it | crate | settle | crates/@U/src/ | - | crates/@M/Cargo.toml => [package]\nname = "deck-streak-@M"\n || crates/@M/src/lib.rs => pub use @K::*;\n || crates/@U/src/lib.rs => pub use deck_streak_@M::*;\n
X05 glob of a manifest alias re-exported, caller in another member | deck_streak_@M | settle | crates/@U/src/ | - | crates/@M/Cargo.toml => [package]\nname = "deck-streak-@M"\n[dependencies]\n@N = { package = "@P" }\n || crates/@M/src/lib.rs => pub use @N::*;\n
X06 coordination re-exports the root by glob, caller elsewhere | deck_streak_coordination | settle | crates/@U/src/ | - | crates/coordination/src/lib.rs => pub use @K::*;\n
"#;

/// The routes by which a holder member (`@M`, and `@V` for a second one) exports progression's
/// root or its operation to a caller in another member: `label | the path from the holder's crate
/// (`@H`) | the operation | the files`. A `control:` route is accepted with progression's crate.
const ROUTES: &str = r#"
a glob of the root at the holder's root | @H | settle | crates/@M/src/lib.rs => pub use @K::*;\n
a glob in a pub mod | @H::p | settle | crates/@M/src/lib.rs => pub mod p {\n    pub use @K::*;\n}\n
a glob in a pub mod's own file | @H::p | settle | crates/@M/src/lib.rs => pub mod p;\n || crates/@M/src/p.rs => pub use @K::*;\n
a glob through a use alias | @H | settle | crates/@M/src/lib.rs => use @K as @N;\npub use @N::*;\n
a glob through a manifest alias | @H | settle | crates/@M/Cargo.toml => [dependencies]\n@N = { package = "@P" }\n || crates/@M/src/lib.rs => pub use @N::*;\n
a pub alias of the root | @H::@N | settle | crates/@M/src/lib.rs => pub use @K as @N;\n
a pub extern crate | @H::@N | settle | crates/@M/src/lib.rs => pub extern crate @K as @N;\n
a pub re-export of a single alias | @H | tally | crates/@M/src/lib.rs => pub use @K::tally;\n
a pub re-export renaming the operation | @H | run | crates/@M/src/lib.rs => pub use @K::settle::settle as run;\n
a renaming re-export in a pub mod | @H::q | run | crates/@M/src/lib.rs => pub mod q {\n    pub use @K::settle::settle as run;\n}\n
a pub mod re-exporting the operation a glob brought | @H::p | settle | crates/@M/src/lib.rs => pub use @K::*;\npub mod p {\n    pub use crate::settle;\n}\n
a chain across two holders, by glob | @H | settle | crates/@V/src/lib.rs => pub use @K::*;\n || crates/@M/src/lib.rs => pub use deck_streak_@V::*;\n
a chain across two holders, by alias | @H::h | settle | crates/@V/src/lib.rs => pub use @K::*;\n || crates/@M/src/lib.rs => pub use deck_streak_@V as h;\n
a recompute step's re-export | @H::recompute::xp | run | crates/@M/src/lib.rs => pub mod recompute;\n || crates/@M/src/recompute/mod.rs => pub mod xp;\n || crates/@M/src/recompute/xp.rs => pub use @K::tally as run;\n
control: a private glob | @H | settle | crates/@M/src/lib.rs => use @K::*;\npub fn settle() -> usize {\n    0\n}\n
"#;

/// How a caller reaches the operation through the path its member binds.
const SHAPES: [(&str, &str); 3] = [
    ("path", "fn call() { let _ = @R::@O(); }\n"),
    (
        "use then call",
        "use @R::@O;\nfn call() { let _ = @O(); }\n",
    ),
    (
        "use as then call",
        "use @R::{@O as s};\nfn call() { let _ = s(); }\n",
    ),
];

/// How a caller's member names a holder's crate: by the crate's own name, or by a name the caller
/// binds to it. `@H` in a route is the name, the file is the caller's member's manifest and the
/// line opens the caller.
const ACCESS: [(&str, &str, &str, &str); 4] = [
    ("by its name", "deck_streak_@M", "", ""),
    (
        "by a manifest rename",
        "@N_h",
        "[dependencies]\n@N_h = { package = \"deck-streak-@M\" }\n",
        "",
    ),
    (
        "by a use alias",
        "@N_h",
        "",
        "use deck_streak_@M as @N_h;\n",
    ),
    (
        "by an extern crate alias",
        "@N_h",
        "",
        "extern crate deck_streak_@M as @N_h;\n",
    ),
];

/// Where a comment may stand in Rust: anywhere a space may, before an item, or at a file's head.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Anywhere,
    Item,
    Head,
}

/// Every kind of Rust comment. Its text (`@T`) carries the operation's path, `as` and a glob, so a
/// comment read as code would bind or call.
const COMMENTS: [(&str, &str, Place); 7] = [
    ("line", "// @T\n", Place::Anywhere),
    ("block", "/* @T */", Place::Anywhere),
    ("nested block", "/* @T /* @T */ @T */", Place::Anywhere),
    ("outer doc line", "/// @T\n", Place::Item),
    ("outer doc block", "/** @T */", Place::Item),
    ("inner doc line", "//! @T\n", Place::Head),
    ("inner doc block", "/*! @T */", Place::Head),
];

/// A comment's text: code, were it read as code.
const COMMENT_TEXT: &str =
    "pub use deck_streak_progression::* ; deck_streak_progression::settle as @N ; SettleRequest";

/// A manifest comment's text: a rename of progression's package, were it read.
const MANIFEST_COMMENT: &str = "# @N.package = \"deck-streak-progression\" [dependencies.@N] package = \"deck-streak-progression\"";

/// Literals that open a caller: each would hide the call that follows it if the lexer misread it.
const LITERALS: [(&str, &str); 10] = [
    (
        "a string holding a block opener",
        "const A: &str = \"/*\";\n",
    ),
    (
        "a string holding a line comment",
        "const A: &str = \"// \";\n",
    ),
    ("an escaped quote", "const A: &str = \"\\\"/*\";\n"),
    (
        "a raw string holding quotes",
        "const A: &str = r#\"a \"b\" /*\"#;\n",
    ),
    ("a byte string", "const A: &[u8] = b\"/*\";\n"),
    ("a C string", "const A: &core::ffi::CStr = c\"/*\";\n"),
    ("a quote character", "const A: char = '\"';\n"),
    ("a quote byte", "const A: u8 = b'\"';\n"),
    (
        "an escaped quote character",
        "const A: (char, &str) = ('\\'', \"/*\");\n",
    ),
    (
        "a lifetime",
        "fn life<'a>(x: &'a str) -> &'a str {\n    x\n}\n",
    ),
];

/// Literals whose text names the operation, the request or a glob of the crate: text in a literal
/// is not code, so each is accepted.
const LITERAL_CONTROLS: [(&str, &str); 6] = [
    (
        "a string naming the operation",
        "const A: &str = \"deck_streak_progression::settle\";\n",
    ),
    (
        "a raw string holding a glob",
        "const A: &str = r#\"pub use deck_streak_progression::*;\"#;\n",
    ),
    (
        "a byte string naming the request",
        "const A: &[u8] = b\"SettleRequest\";\n",
    ),
    (
        "a raw string holding a quote",
        "const A: &str = r##\"\"# deck_streak_progression::settle\"##;\n",
    ),
    (
        "an escaped quote character",
        "const A: (char, &str) = ('\\'', \"deck_streak_progression::settle\");\n",
    ),
    (
        "a string holding a comment",
        "const A: &str = \"/* deck_streak_progression::settle */\";\n",
    ),
];

/// One member or control of the population: its files, and the caller file the census must refuse
/// (a member) or accept (a control).
struct Case {
    axis: &'static str,
    label: String,
    files: Vec<(String, String)>,
    caller: String,
    member: bool,
}

/// A table's rows: its lines, split into fields at ` | `.
fn rows(table: &'static str) -> Vec<Vec<&'static str>> {
    table
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split(" | ").collect())
        .collect()
}

/// A row's files, `path => text` joined by ` || `, with `\n` read as a line end.
fn row_files(field: &str) -> Vec<(String, String)> {
    field
        .split(" || ")
        .filter(|file| *file != "-")
        .filter_map(|file| file.split_once(" => "))
        .map(|(path, text)| (path.to_owned(), text.replace("\\n", "\n")))
        .collect()
}

/// Where a comment may stand in Rust `text`: before each token, and at the end, each with where
/// it is (a file's head, an item's start, or anywhere else).
fn gaps(text: &str) -> Vec<(usize, Place)> {
    let mut found = Vec::new();
    let mut previous = "";
    let mut rest = text.char_indices().peekable();
    while let Some((start, character)) = rest.next() {
        if character.is_whitespace() {
            continue;
        }
        // A comment the form already holds is a space: a gap is on either side of it.
        if text[start..].starts_with("//") || text[start..].starts_with("/*") {
            let end = start + comment_length(&text[start..]);
            while rest.next_if(|(at, _)| *at < end).is_some() {}
            continue;
        }
        let word = |c: char| c.is_alphanumeric() || matches!(c, '_' | '#' | '@');
        let mut end = start + character.len_utf8();
        if word(character) {
            while let Some((at, _)) = rest.next_if(|(_, next)| word(*next)) {
                end = at + 1;
            }
        } else if character == ':' && rest.next_if(|(_, next)| *next == ':').is_some() {
            end += 1;
        }
        let token = &text[start..end];
        let item = matches!(previous, "" | ";" | "{" | "}")
            && matches!(token, "pub" | "use" | "extern" | "mod" | "fn" | "const");
        let place = match (start, item) {
            (0, _) => Place::Head,
            (_, true) => Place::Item,
            _ => Place::Anywhere,
        };
        found.push((start, place));
        previous = token;
    }
    found.push((text.len(), Place::Anywhere));
    found
}

/// The length in bytes of the comment that opens `text` (a `//` line or a nesting `/* */` block).
fn comment_length(text: &str) -> usize {
    if text.starts_with("//") {
        return text.find('\n').unwrap_or(text.len());
    }
    let mut depth = 0_usize;
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with("/*") {
            depth += 1;
            at += 2;
        } else if text[at..].starts_with("*/") {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return at;
            }
        } else {
            at += text[at..].chars().next().map_or(1, char::len_utf8);
        }
    }
    text.len()
}

/// `text` with a comment of every admissible kind in every gap, each labelled.
fn commented(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (at, place) in gaps(text) {
        for (kind, comment, admits) in COMMENTS {
            let fits = admits == Place::Anywhere
                || admits == place
                || (admits == Place::Item && place == Place::Head && at < text.len());
            if fits && !(place == Place::Head && admits == Place::Item && at == text.len()) {
                let comment = comment.replace("@T", COMMENT_TEXT);
                let written = format!("{}{comment}{}", &text[..at], &text[at..]);
                found.push((format!("{kind} at byte {at}"), written));
            }
        }
    }
    found
}

/// A manifest `text` with a comment after each header and each key, on a line of its own, and
/// (TOML 1.1) inside each inline table, made to span lines.
fn manifest_commented(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let kind = if line.starts_with('[') {
            "after a header"
        } else {
            "after a key"
        };
        let mut after = lines
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>();
        after[index] = format!("{line} {MANIFEST_COMMENT}");
        found.push((format!("{kind}, line {index}"), after.join("\n") + "\n"));
        let mut own = lines
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>();
        own.insert(index + 1, MANIFEST_COMMENT.to_owned());
        found.push((
            format!("on its own line, after line {index}"),
            own.join("\n") + "\n",
        ));
    }
    let table = if text.contains("[workspace") {
        "workspace"
    } else {
        "package"
    };
    found.push((
        "a `#` inside a string".to_owned(),
        format!("{text}\n[{table}.metadata.census]\nnote = \"the #1 deck # not a comment\"\n"),
    ));
    if text.contains("{ ") {
        let spanning = text
            .replace("{ ", &format!("{{ {MANIFEST_COMMENT}\n    "))
            .replace(", ", &format!(", {MANIFEST_COMMENT}\n    "))
            .replace(" }", &format!(" {MANIFEST_COMMENT}\n}}"));
        found.push(("inside an inline table spanning lines".to_owned(), spanning));
    }
    found
}

/// The names the tables give a module or an alias. Each case gets its own spelling of them, so
/// that cases planted in one tree never share a name, and a tree answers for each case as a tree
/// of its own would.
const LOCAL_NAMES: [&str; 16] = [
    "a",
    "a_link",
    "a_three",
    "b_two",
    "deeper",
    "h",
    "own",
    "p",
    "q",
    "recompute",
    "run",
    "s",
    "shared",
    "sub",
    "x",
    "xp",
];

/// A case's text for `serial`: its member, holder and caller folders, its bound name and its
/// local names are its own, and `@K`/`@P` are progression's for a member and another crate's for
/// a control.
fn fill(text: &str, serial: usize, member: bool) -> String {
    let (krate, package) = if member {
        (PROGRESSION_CRATE, "deck-streak-progression")
    } else {
        ("deck_streak_other", "deck-streak-other")
    };
    let filled = text
        .replace("@K", krate)
        .replace("@P", package)
        .replace("@N", &format!("bound{serial}"))
        .replace("@M", &format!("habits{serial}"))
        .replace("@U", &format!("quests{serial}"))
        .replace("@V", &format!("markets{serial}"));
    let mut renamed = String::new();
    let mut word = String::new();
    for character in filled.chars().chain(std::iter::once('\0')) {
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
            continue;
        }
        if LOCAL_NAMES.contains(&word.as_str()) {
            word.push('_');
            word.push_str(&serial.to_string());
        }
        renamed.push_str(&std::mem::take(&mut word));
        if character != '\0' {
            renamed.push(character);
        }
    }
    renamed
}

/// The binding population, generated from the tables: every form by every shape; a comment of
/// every kind in every gap of every form's files and of every caller; a comment in every place
/// of every manifest, and a `#` inside a string; a literal before every form's caller; every
/// export route by holder, access and shape, and progression's own module re-export by shape; a
/// file another member compiles by `#[path]`; and a glob member's homonym in another module. Each
/// member has a control that names another crate in the same spelling, but for progression's own
/// module re-export, and the literal, private-glob and own-homonym controls stand alone.
// One generator holds the whole product the test asserts: each axis is a loop over its table.
#[allow(clippy::too_many_lines)]
fn population() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut add = |axis: &'static str,
                   label: String,
                   files: Vec<(String, String)>,
                   caller: (String, String),
                   both: bool| {
        for member in [true, false].into_iter().take(if both { 2 } else { 1 }) {
            let serial = cases.len();
            let mut planted: Vec<(String, String)> = files
                .iter()
                .map(|(path, text)| (fill(path, serial, member), fill(text, serial, member)))
                .collect();
            let caller_path = fill(&caller.0, serial, member);
            planted.push((caller_path.clone(), fill(&caller.1, serial, member)));
            cases.push(Case {
                axis,
                label: label.clone(),
                files: planted,
                caller: caller_path,
                member: member && both,
            });
        }
    };
    let caller = |form: &[&str], shape: &str| {
        let prefix = if form[4] == "-" { "" } else { form[4] };
        let text = format!("{prefix}{shape}")
            .replace("\\n", "\n")
            .replace("@R", form[1])
            .replace("@O", form[2]);
        (format!("{}call.rs", form[3]), text)
    };
    for form in rows(FORMS) {
        let files = row_files(form[5]);
        for (shape, text) in SHAPES {
            let label = format!("{} / {shape}", form[0]);
            add(
                "form",
                label.clone(),
                files.clone(),
                caller(&form, text),
                true,
            );
            let (path, text) = caller(&form, text);
            for (gap, written) in commented(&text) {
                let label = format!("{label} / {gap}");
                add(
                    "comment in a caller",
                    label,
                    files.clone(),
                    (path.clone(), written),
                    true,
                );
            }
        }
        for (index, (path, text)) in files.iter().enumerate() {
            let rust = Path::new(path)
                .extension()
                .is_some_and(|extension| extension == "rs");
            let variants = if rust {
                commented(text)
            } else {
                manifest_commented(text)
            };
            let axis = if rust {
                "comment in a binding"
            } else {
                "comment in a manifest"
            };
            for (gap, written) in variants {
                let mut changed = files.clone();
                changed[index].1 = written;
                let label = format!("{} / {path} / {gap}", form[0]);
                add(axis, label, changed, caller(&form, SHAPES[0].1), true);
            }
        }
        for (literal, text) in LITERALS {
            let (path, call) = caller(&form, SHAPES[0].1);
            let label = format!("{} / {literal}", form[0]);
            add(
                "literal",
                label,
                files.clone(),
                (path, format!("{text}{call}")),
                true,
            );
        }
    }
    for route in rows(ROUTES) {
        let control = route[0].starts_with("control:");
        for holder in ["@M", "coordination"] {
            for (access, name, manifest, line) in ACCESS {
                for (shape, text) in SHAPES {
                    let mut files: Vec<(String, String)> = row_files(route[3])
                        .into_iter()
                        .map(|(path, text)| {
                            (path.replace("@M", holder), text.replace("@M", holder))
                        })
                        .collect();
                    if !manifest.is_empty() {
                        files.push((
                            "crates/@U/Cargo.toml".to_owned(),
                            manifest.replace("@M", holder),
                        ));
                    }
                    let reached = route[1].replace("@H", &name.replace("@M", holder));
                    let call = format!("{}{text}", line.replace("@M", holder))
                        .replace("@R", &reached)
                        .replace("@O", route[2]);
                    let label = format!("{} / {holder} / {access} / {shape}", route[0]);
                    add(
                        "route",
                        label,
                        files,
                        ("crates/@U/src/call.rs".to_owned(), call),
                        !control,
                    );
                }
            }
        }
    }
    for (label, text) in LITERAL_CONTROLS {
        add(
            "literal",
            label.to_owned(),
            Vec::new(),
            ("crates/@U/src/lit.rs".to_owned(), text.to_owned()),
            false,
        );
    }
    add(
        "literal",
        "a string holding a glob before the member's own settle".to_owned(),
        vec![(
            "crates/@U/src/lib.rs".to_owned(),
            "const A: &str = \"pub use deck_streak_progression::*;\";\n".to_owned(),
        )],
        (
            "crates/@U/src/call.rs".to_owned(),
            "fn settle() -> usize {\n    0\n}\nfn call() { let _ = settle(); }\n".to_owned(),
        ),
        false,
    );
    for (shape, text) in SHAPES {
        // A private binding in one member reaches a file that sits in another member's `src` but
        // is compiled into the first by `#[path]`.
        for (binding, reached) in [
            ("use @K as @N;", "crate::@N"),
            ("extern crate @K as @N;", "@N"),
        ] {
            add(
                "a file another member compiles by #[path]",
                format!("{binding} / {shape}"),
                vec![(
                    "crates/@M/src/lib.rs".to_owned(),
                    format!("{binding}\n#[path = \"../../@U/src/shared.rs\"]\nmod shared;\n"),
                )],
                (
                    "crates/@U/src/shared.rs".to_owned(),
                    text.replace("@R", reached).replace("@O", "settle"),
                ),
                true,
            );
        }
        add(
            "a glob member's homonym in another module",
            shape.to_owned(),
            vec![
                (
                    "crates/@M/src/lib.rs".to_owned(),
                    "pub use @K::*;\nmod call;\nmod own;\n".to_owned(),
                ),
                (
                    "crates/@M/src/own.rs".to_owned(),
                    "pub fn tally() -> usize {\n    1\n}\n".to_owned(),
                ),
            ],
            (
                "crates/@M/src/call.rs".to_owned(),
                text.replace("@R", "crate").replace("@O", "tally"),
            ),
            true,
        );
        // A member that imports another of progression's items, not by a glob, and calls its own
        // `settle`: the import opens nothing, so the call is the member's own.
        add(
            "an own homonym beside a plain import",
            shape.to_owned(),
            vec![(
                "crates/@U/src/lib.rs".to_owned(),
                "use deck_streak_progression::Level;\npub fn settle() -> usize {\n    1\n}\nmod call;\n"
                    .to_owned(),
            )],
            (
                "crates/@U/src/call.rs".to_owned(),
                text.replace("@R", "crate").replace("@O", "settle"),
            ),
            false,
        );
    }
    // Progression's own module re-exporting the operation stands without a control: the module's
    // name is a name of the crate wherever it is written (the global set's disclosed refusal), so a
    // control reaching another crate's `p::settle` would be refused.
    for (shape, text) in SHAPES {
        let serial = cases.len();
        let caller = fill("crates/@U/src/call.rs", serial, true);
        let module = "pub mod p {\n    pub use crate::settle::settle;\n}\n";
        cases.push(Case {
            axis: "route",
            label: format!("progression's own pub mod re-exporting the operation / {shape}"),
            files: vec![
                (
                    PROGRESSION_STUB[0].0.to_owned(),
                    fill(&format!("{}{module}", PROGRESSION_STUB[0].1), serial, true),
                ),
                (
                    caller.clone(),
                    fill(
                        &text.replace("@R", "@K::p").replace("@O", "settle"),
                        serial,
                        true,
                    ),
                ),
            ],
            caller,
            member: true,
        });
    }
    cases
}

/// Progression's crate as every population tree holds it: its `settle` module, which names the
/// table and holds the operation and its request, re-exported and renamed at the root.
const PROGRESSION_STUB: [(&str, &str); 2] = [
    (
        "crates/progression/src/lib.rs",
        "pub mod settle;\npub use settle::settle;\npub use settle::settle as tally;\npub struct Level;\n",
    ),
    (
        "crates/progression/src/settle.rs",
        "pub struct SettleRequest;\npub fn settle() -> usize {\n    0\n}\n\
         pub const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    ),
];

/// How many trees the cases that share a tree are planted in, so that threads judge them at once.
const SHARED_TREES: usize = 16;

/// The caller files the census refuses in a tree holding progression and `cases`.
fn judge(cases: &[&Case]) -> Vec<String> {
    let planted = tempfile::tempdir().expect("a temporary directory");
    for (path, text) in PROGRESSION_STUB {
        plant(planted.path(), path, text);
    }
    for case in cases {
        for (path, text) in &case.files {
            plant(planted.path(), path, text);
        }
    }
    census(planted.path())
        .refused
        .iter()
        .filter_map(|line| line.split(' ').next().map(str::to_owned))
        .collect()
}

/// Plants `cases` and returns the caller files the census refuses. A case that writes the
/// workspace's manifest, coordination's files or progression's is planted in a tree of its own; the rest share
/// a few trees, where every folder and name is the case's own. Threads judge the trees at once.
fn refused_callers(cases: &[Case]) -> BTreeSet<String> {
    let solo = |case: &&Case| {
        case.files.iter().any(|(path, _)| {
            path == "Cargo.toml"
                || path.starts_with("crates/coordination/")
                || path.starts_with("crates/progression/")
        })
    };
    let shared: Vec<&Case> = cases.iter().filter(|case| !solo(case)).collect();
    let mut trees: Vec<Vec<&Case>> = shared
        .chunks(shared.len().div_ceil(SHARED_TREES).max(1))
        .map(<[&Case]>::to_vec)
        .collect();
    trees.extend(cases.iter().filter(solo).map(|case| vec![case]));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut refused = Vec::new();
                    while let Some(tree) =
                        trees.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                    {
                        refused.extend(judge(tree));
                    }
                    refused
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a judging thread"))
            .collect()
    })
}

#[test]
fn the_census_refuses_every_member_of_the_binding_population() {
    // The class: a source outside progression and coordination whose path reaches progression's
    // `settle`, by any binding form and any member's export, past any comment and literal. The
    // population is generated from the tables above: every member is a caller by construction
    // and is refused, and every control (another crate in the same spelling, a private glob, a
    // literal) is accepted.
    let started = std::time::Instant::now();
    let cases = population();
    let refused = refused_callers(&cases);
    let mut axes: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for case in &cases {
        let counts = axes.entry(case.axis).or_default();
        if case.member {
            counts.0 += 1;
        } else {
            counts.1 += 1;
        }
    }
    for (axis, (members, controls)) in &axes {
        println!("axis {axis}: {members} member(s), {controls} control(s)");
    }
    let members: Vec<&Case> = cases.iter().filter(|case| case.member).collect();
    let controls: Vec<&Case> = cases.iter().filter(|case| !case.member).collect();
    println!("class members: examined {}", members.len());
    println!("class controls: examined {}", controls.len());
    println!("class population judged in {:?}", started.elapsed());
    let escaping: Vec<String> = members
        .iter()
        .filter(|case| !refused.contains(&case.caller))
        .map(|case| format!("{}: {}", case.axis, case.label))
        .collect();
    let wrongly_refused: Vec<String> = controls
        .iter()
        .filter(|case| refused.contains(&case.caller))
        .map(|case| format!("{}: {}", case.axis, case.label))
        .collect();
    for member in escaping.iter().take(40) {
        println!("escapes: {member}");
    }
    for control in wrongly_refused.iter().take(40) {
        println!("wrongly refused: {control}");
    }
    println!(
        "class members escaping: {}; class controls refused: {}",
        escaping.len(),
        wrongly_refused.len()
    );
    assert_eq!(
        escaping.first(),
        None,
        "members that escape the census: {} of {}",
        escaping.len(),
        members.len()
    );
    assert_eq!(
        wrongly_refused.first(),
        None,
        "controls the census refuses: {} of {}",
        wrongly_refused.len(),
        controls.len()
    );
    // Every manifest in the population is TOML, so the census reads each to its end.
    let unread: Vec<&String> = refused
        .iter()
        .filter(|name| name.ends_with("Cargo.toml"))
        .collect();
    assert_eq!(
        unread.first(),
        None,
        "manifests the census cannot read: {}",
        unread.len()
    );
    assert!(members.len() > FORMS.lines().count() * SHAPES.len());
}

#[test]
fn the_census_reads_a_raw_identifier_as_its_plain_name() {
    // `r#raw_tally` and `raw_tally` are one name to the compiler, so a caller may import the alias
    // in either spelling.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    for (path, text) in [
        (
            "crates/progression/src/lib.rs",
            "pub mod settle;\npub use settle::settle as r#raw_tally;\n",
        ),
        (
            "crates/quests/src/plain.rs",
            "use deck_streak_progression::raw_tally;\n",
        ),
        (
            "crates/quests/src/raw.rs",
            "use deck_streak_progression::r#raw_tally;\n",
        ),
    ] {
        plant(planted.path(), path, text);
    }
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/plain.rs calls settle through raw_tally, progression's alias of \
             settle, and only coordination's code may",
            "crates/quests/src/raw.rs calls settle through raw_tally, progression's alias of \
             settle, and only coordination's code may",
        ]
    );
}

/// The refusal of a planted `crates/{file}` that reaches the renamed `settle` as `tally`.
fn through_tally(file: &str) -> String {
    format!(
        "crates/{file} calls settle through tally, progression's alias of settle, and only \
         coordination's code may"
    )
}

#[test]
fn the_census_follows_a_crate_alias_however_it_is_written() {
    // Each member reaches progression's crate by a name other than its own: by a glob, by an
    // `extern crate`, renamed inside a group, through a chain read before its link, and in raw
    // spelling. A member whose glob is another crate's, or which imports progression's crate
    // without a glob, keeps its own `tally`.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    for (path, text) in [
        (
            "crates/progression/src/lib.rs",
            "pub mod settle;\npub use settle::settle as tally;\n",
        ),
        (
            "crates/habits/src/lib.rs",
            "pub use deck_streak_progression::*;\n",
        ),
        ("crates/habits/src/via_glob.rs", "use crate::tally;\n"),
        (
            "crates/economy/src/lib.rs",
            "extern crate deck_streak_progression as ext_prog;\n",
        ),
        ("crates/economy/src/via_extern.rs", "use ext_prog::tally;\n"),
        (
            "crates/markets/src/lib.rs",
            "pub use deck_streak_progression::{self as grouped_prog};\n",
        ),
        (
            "crates/markets/src/via_grouped.rs",
            "use crate::grouped_prog::tally;\n",
        ),
        (
            "crates/quests/src/a_link.rs",
            "pub use crate::first_prog as second_prog;\n",
        ),
        (
            "crates/quests/src/lib.rs",
            "pub use deck_streak_progression as first_prog;\n",
        ),
        (
            "crates/quests/src/via_chain.rs",
            "use crate::second_prog::tally;\n",
        ),
        ("crates/readings/src/lib.rs", "use std::io::*;\n"),
        ("crates/readings/src/own.rs", "fn tally() {}\n"),
        (
            "crates/streaks/src/lib.rs",
            "pub use deck_streak_progression as r#raw_prog;\n",
        ),
        (
            "crates/streaks/src/via_raw.rs",
            "use crate::raw_prog::tally;\n",
        ),
        (
            "crates/vault/src/lib.rs",
            "use deck_streak_progression::SettledRow;\n",
        ),
        ("crates/vault/src/own.rs", "fn tally() {}\n"),
    ] {
        plant(planted.path(), path, text);
    }
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            through_tally("economy/src/via_extern.rs"),
            through_tally("habits/src/via_glob.rs"),
            through_tally("markets/src/via_grouped.rs"),
            through_tally("quests/src/via_chain.rs"),
            through_tally("streaks/src/via_raw.rs"),
        ]
    );
}

#[test]
fn the_census_follows_a_crate_renamed_by_a_manifest() {
    // A manifest's `package` binds progression's package to another name: in the workspace's own
    // table, and in a member's inline, table (with a `-` in its key) and dotted-key forms. A
    // package whose name only begins with progression's is another package, and a manifest the
    // reader cannot read to its end is refused.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    for (path, text) in [
        (
            "crates/progression/src/lib.rs",
            "pub mod settle;\npub use settle::settle as tally;\n",
        ),
        (
            "Cargo.toml",
            "[workspace.dependencies]\n\
             workspace_prog = { package = \"deck-streak-progression\", path = \"crates/progression\" }\n",
        ),
        (
            "crates/analytics/src/via_workspace.rs",
            "use workspace_prog::tally;\n",
        ),
        (
            "crates/focus/Cargo.toml",
            "[dependencies]\n\
             focus_prog = { package = \"deck-streak-progression\", path = \"../progression\" }\n",
        ),
        ("crates/focus/src/via_inline.rs", "use focus_prog::tally;\n"),
        (
            "crates/insights/Cargo.toml",
            "[dependencies.insights-prog]\n\
             package = 'deck-streak-progression'\n\
             path = '../progression'\n",
        ),
        (
            "crates/insights/src/via_table.rs",
            "use insights_prog::tally;\n",
        ),
        (
            "crates/markets/Cargo.toml",
            "[dependencies]\n\
             dotted_prog.package = \"deck-streak-progression\"\n\
             dotted_prog.path = \"../progression\"\n",
        ),
        (
            "crates/markets/src/via_dotted.rs",
            "use dotted_prog::tally;\n",
        ),
        (
            "crates/vault/Cargo.toml",
            "[dependencies]\n\
             vault_other = { package = \"deck-streak-progression-extra\", path = \"../extra\" }\n",
        ),
        ("crates/vault/src/own.rs", "use vault_other::tally;\n"),
        (
            "crates/wallet/Cargo.toml",
            "[dependencies\nwallet_prog.package = 1\n",
        ),
    ] {
        plant(planted.path(), path, text);
    }
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/wallet/Cargo.toml is a manifest the census cannot read past character 13, so it \
             may rename progression's crate unseen"
                .to_owned(),
            through_tally("analytics/src/via_workspace.rs"),
            through_tally("focus/src/via_inline.rs"),
            through_tally("insights/src/via_table.rs"),
            through_tally("markets/src/via_dotted.rs"),
        ]
    );
}
