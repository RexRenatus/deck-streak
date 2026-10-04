//! The table censuses' shared reader (SPEC-324 R1 to R5, ADR-325). A census that holds a table to
//! its owner reads each line for the name as written; this reader reads every crate's `src` outside
//! the owner as rustc reads its literals, and refuses a file whose literals can assemble the name.
//! It is one file included by path into each census's tests, as the parity oracle's golden reader
//! is (ADR-029):
//!
//! ```text
//! #[path = "../../../tools/table-census/table_census.rs"]
//! mod table_census;
//! ```
//!
//! It lexes each file once and drops its comments, and decodes every literal: strings with every
//! escape and line continuation, raw strings with any number of hashes, byte, raw byte and C
//! strings, characters and byte characters. The words inside `stringify!` are pieces too. It
//! follows `include!` and `#[path]` into a Rust file, and `include_str!` and `include_bytes!` into
//! one piece, by their literal paths. The pieces of two or more characters of every file outside
//! the owner, workspace-wide, are one pool, folded to ASCII lower case, and the reader refuses every
//! file holding a piece on a path that assembles the name: a piece that ends with a prefix of it,
//! pieces equal to the parts between, and a piece that starts with the rest. A file it cannot read,
//! a path it cannot name and a `concat!` argument it cannot read beside part of the name fail
//! closed. An include joined onto `env!("OUT_DIR")` is a build script's output, which it counts and
//! discloses (#585).
//!
//! A piece of one character is judged only in a file's reach (SPEC-331, ADR-331): the file's own
//! pieces, the pieces of every file it includes, transitively, and the pieces of every `const` or
//! `static` item it names by a word or a format placeholder. The same rule covers the name in a
//! reach as in the pool, so one character in an unrelated file completes no path, while a join of
//! named items still does. An item named by a macro's metavariable keeps even its one-character
//! pieces in the pool, because the reader cannot read its name.

// Each including crate calls only the part of the reader its tests need.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

/// One token of Rust source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// An identifier, a keyword or a number.
    Word(String),
    /// A literal's value, decoded as rustc decodes it.
    Literal(String),
    /// Any other character.
    Punct(char),
}

/// The escape at `at`, the character after a backslash: the character it stands for (none for a
/// line continuation, which also skips the whitespace after it) and the index after it.
fn escape(characters: &[(usize, char)], at: usize) -> (Option<char>, usize) {
    let character = |index: usize| characters.get(index).map(|(_, found)| *found);
    match character(at) {
        Some('n') => (Some('\n'), at + 1),
        Some('r') => (Some('\r'), at + 1),
        Some('t') => (Some('\t'), at + 1),
        Some('0') => (Some('\0'), at + 1),
        Some('x') => {
            let digits: String = (at + 1..at + 3).filter_map(character).collect();
            let value = u8::from_str_radix(&digits, 16).map(char::from).ok();
            (value, at + 3)
        }
        Some('u') => {
            let mut after = at + 2;
            let mut digits = String::new();
            while let Some(found) = character(after) {
                after += 1;
                if found == '}' {
                    break;
                }
                if found != '_' {
                    digits.push(found);
                }
            }
            let value = u32::from_str_radix(&digits, 16)
                .ok()
                .and_then(char::from_u32);
            (value, after)
        }
        Some('\n') => {
            let mut after = at + 1;
            while character(after).is_some_and(char::is_whitespace) {
                after += 1;
            }
            (None, after)
        }
        found => (found, at + 1),
    }
}

/// The string whose text starts at `at`, decoded, and the index after its closing quote.
fn string(characters: &[(usize, char)], at: usize) -> (String, usize) {
    let mut value = String::new();
    let mut index = at;
    while let Some(&(_, found)) = characters.get(index) {
        match found {
            '"' => return (value, index + 1),
            '\\' => {
                let (decoded, after) = escape(characters, index + 1);
                value.extend(decoded);
                index = after;
            }
            _ => {
                value.push(found);
                index += 1;
            }
        }
    }
    (value, index)
}

/// The raw string whose hashes (or quote) start at `at`, and the index after it closes, if `at`
/// opens one.
fn raw_string(characters: &[(usize, char)], at: usize) -> Option<(String, usize)> {
    let character = |index: usize| characters.get(index).map(|(_, found)| *found);
    let hashes = (at..characters.len())
        .take_while(|index| character(*index) == Some('#'))
        .count();
    if character(at + hashes) != Some('"') {
        return None;
    }
    let mut value = String::new();
    let mut index = at + hashes + 1;
    while let Some(found) = character(index) {
        if found == '"' && (1..=hashes).all(|step| character(index + step) == Some('#')) {
            return Some((value, index + 1 + hashes));
        }
        value.push(found);
        index += 1;
    }
    Some((value, index))
}

/// The character literal whose text starts at `at`, after its opening quote, and the index after
/// its closing quote, if it is one rather than a lifetime.
fn character_literal(characters: &[(usize, char)], at: usize) -> Option<(String, usize)> {
    let character = |index: usize| characters.get(index).map(|(_, found)| *found);
    if character(at) == Some('\\') {
        let (decoded, after) = escape(characters, at + 1);
        return (character(after) == Some('\''))
            .then(|| (decoded.into_iter().collect(), after + 1));
    }
    let value = character(at)?;
    (character(at + 1) == Some('\'')).then(|| (value.to_string(), at + 2))
}

/// Rust source as tokens, each with the bytes it spans (rustc's offsets are bytes); comments are
/// dropped, nested block comments included.
pub fn lex(text: &str) -> Vec<(Token, Range<usize>)> {
    let characters: Vec<(usize, char)> = text.char_indices().collect();
    let offset = |index: usize| characters.get(index).map_or(text.len(), |(at, _)| *at);
    let character = |index: usize| characters.get(index).map(|(_, found)| *found);
    let mut tokens = Vec::new();
    let mut at = 0;
    while let Some(found) = character(at) {
        let start = offset(at);
        let next = character(at + 1);
        if found.is_whitespace() {
            at += 1;
        } else if found == '/' && next == Some('/') {
            while character(at).is_some_and(|inside| inside != '\n') {
                at += 1;
            }
        } else if found == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while let Some(inside) = character(at) {
                if inside == '/' && character(at + 1) == Some('*') {
                    depth += 1;
                    at += 2;
                } else if inside == '*' && character(at + 1) == Some('/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    at += 1;
                }
            }
        } else if found == '"' {
            let (value, after) = string(&characters, at + 1);
            tokens.push((Token::Literal(value), start..offset(after)));
            at = after;
        } else if let Some((value, after)) = (found == '\'')
            .then(|| character_literal(&characters, at + 1))
            .flatten()
        {
            tokens.push((Token::Literal(value), start..offset(after)));
            at = after;
        } else if found.is_alphanumeric() || found == '_' {
            let mut after = at;
            while character(after).is_some_and(|inside| inside.is_alphanumeric() || inside == '_') {
                after += 1;
            }
            let word: String = characters[at..after]
                .iter()
                .map(|(_, inside)| *inside)
                .collect();
            let literal = match (word.as_str(), character(after)) {
                ("b" | "c", Some('"')) => Some(string(&characters, after + 1)),
                ("r" | "br" | "cr", Some('"' | '#')) => raw_string(&characters, after),
                ("b", Some('\'')) => character_literal(&characters, after + 1),
                _ => None,
            };
            if let Some((value, end)) = literal {
                tokens.push((Token::Literal(value), start..offset(end)));
                at = end;
            } else {
                tokens.push((Token::Word(word), start..offset(after)));
                at = after;
            }
        } else {
            tokens.push((Token::Punct(found), start..offset(at + 1)));
            at += 1;
        }
    }
    tokens
}

/// Whether `token` opens a delimited group.
fn opens(token: &Token) -> bool {
    matches!(token, Token::Punct('(' | '[' | '{'))
}

/// Whether `token` closes a delimited group.
fn closes(token: &Token) -> bool {
    matches!(token, Token::Punct(')' | ']' | '}'))
}

/// The index of the token that closes the group `tokens[at]` opens.
fn closing(tokens: &[Token], at: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().skip(at) {
        if opens(token) {
            depth += 1;
        } else if closes(token) {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// The index that closes the macro call `<name>!(..)` at `at`, if `tokens[at]` begins one of the
/// named macros.
fn invocation(tokens: &[Token], at: usize, names: &[&str]) -> Option<usize> {
    match tokens.get(at..at + 3)? {
        [Token::Word(word), Token::Punct('!'), open]
            if names.contains(&word.as_str()) && opens(open) =>
        {
            closing(tokens, at + 2)
        }
        _ => None,
    }
}

/// `tokens` cut at each comma outside every delimited group.
fn arguments(tokens: &[Token]) -> Vec<&[Token]> {
    let mut found = Vec::new();
    let (mut depth, mut start) = (0_usize, 0);
    for (index, token) in tokens.iter().enumerate() {
        if opens(token) {
            depth += 1;
        } else if closes(token) {
            depth = depth.saturating_sub(1);
        } else if *token == Token::Punct(',') && depth == 0 {
            found.push(&tokens[start..index]);
            start = index + 1;
        }
    }
    found.push(&tokens[start..]);
    found
}

/// The literal pieces of a `concat!` call's arguments, nested calls and `stringify!` words
/// included, and whether any argument is a value the reader cannot read.
fn concat_pieces(tokens: &[Token], pieces: &mut Vec<String>) -> bool {
    let mut unread = false;
    for argument in arguments(tokens) {
        match argument {
            [] => {}
            [Token::Literal(value)] => pieces.push(value.clone()),
            [Token::Word(word)] if literal_word(word) => pieces.push(word.clone()),
            // An include by its literal path is a file the reader reads where it follows it.
            [
                Token::Word(word),
                Token::Punct('!'),
                _,
                Token::Literal(_),
                _,
            ] if word == "include_str" || word == "include_bytes" => {}
            [Token::Word(word), Token::Punct('!'), _, inner @ .., _] if word == "concat" => {
                unread |= concat_pieces(inner, pieces);
            }
            [Token::Word(word), Token::Punct('!'), _, inner @ .., _] if word == "stringify" => {
                pieces.extend(inner.iter().filter_map(|token| match token {
                    Token::Word(text) | Token::Literal(text) => Some(text.clone()),
                    Token::Punct(_) => None,
                }));
            }
            _ => unread = true,
        }
    }
    unread
}

/// Whether `word` is a literal written as a word, a number, `true` or `false`, which `concat!` joins
/// as it is written.
fn literal_word(word: &str) -> bool {
    word == "true" || word == "false" || word.starts_with(|first: char| first.is_ascii_digit())
}

/// Whether `piece` ends with a proper prefix of `name` or starts with a proper suffix of it.
fn holds_part_of(name: &str, piece: &str) -> bool {
    (1..name.len()).any(|at| piece.ends_with(&name[..at]) || piece.starts_with(&name[at..]))
}

/// The files holding a piece on a path through the pool that assembles `name`: a piece holding it
/// whole, or a piece that ends with a prefix of it, pieces equal to the parts between and a piece
/// that starts with the rest. Every piece is folded to lower case, and a piece may be reused.
fn covering(name: &str, pool: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    let size = name.len();
    // A start piece ends with the name's first `at` characters, an end piece starts with the rest,
    // and a middle piece is exactly the part that starts at `at`, short of the end.
    let start = |piece: &str, at: usize| piece.ends_with(&name[..at]);
    let end = |piece: &str, at: usize| piece.starts_with(&name[at..]);
    let middle = |piece: &str, at: usize| {
        let after = at + piece.len();
        (!piece.is_empty() && after < size && name[at..].starts_with(piece)).then_some(after)
    };
    // `reached[at]`: a start piece and middles reach the name's first `at` characters.
    let mut reached = vec![false; size + 1];
    for at in 1..size {
        reached[at] |= pool.keys().any(|piece| start(piece, at));
        if reached[at] {
            for piece in pool.keys() {
                if let Some(after) = middle(piece, at) {
                    reached[after] = true;
                }
            }
        }
    }
    // `finished[at]`: middles and an end piece cover the name from its character `at` on.
    let mut finished = vec![false; size + 1];
    for at in (1..size).rev() {
        finished[at] = pool
            .keys()
            .any(|piece| end(piece, at) || middle(piece, at).is_some_and(|after| finished[after]));
    }
    let mut named = BTreeSet::new();
    for (piece, holders) in pool {
        let on_a_path = piece.contains(name)
            || (1..size).any(|at| {
                (start(piece, at) && finished[at])
                    || (reached[at] && end(piece, at))
                    || (reached[at] && middle(piece, at).is_some_and(|after| finished[after]))
            });
        if on_a_path {
            named.extend(holders.iter().cloned());
        }
    }
    named
}

/// `path` with every `.` and `..` resolved by its text, so that two spellings of one file agree.
fn normal(path: &Path) -> PathBuf {
    let mut found = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                found.pop();
            }
            other => found.push(other),
        }
    }
    found
}

/// Every `.rs` file under `directory`, recursively, by path.
fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Whether `word` could be a Rust identifier, the name a word or a format placeholder writes.
fn identifier(word: &str) -> bool {
    let mut characters = word.chars();
    characters
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && characters.all(|inside| inside.is_alphanumeric() || inside == '_')
}

/// For each token, the name of the `const` or `static` item whose initializer holds it, from the
/// `=` to the `;` that ends the item. An item named by a macro's metavariable (`const $name: ..`)
/// has a name the reader cannot read, so it is recorded as `$`, which no word can name.
fn items_of(tokens: &[Token]) -> Vec<Option<String>> {
    let mut found = vec![None; tokens.len()];
    let word = |index: usize| match tokens.get(index) {
        Some(Token::Word(text)) => Some(text.as_str()),
        _ => None,
    };
    for at in 0..tokens.len() {
        // A lifetime's `'static` declares no item.
        let lifetime = at > 0 && tokens[at - 1] == Token::Punct('\'');
        if !matches!(word(at), Some("const" | "static")) || lifetime {
            continue;
        }
        let mut name_at = at + 1;
        if word(name_at) == Some("mut") {
            name_at += 1;
        }
        let metavariable = tokens.get(name_at) == Some(&Token::Punct('$'))
            && word(name_at + 1).is_some_and(identifier);
        if metavariable {
            name_at += 1;
        }
        let Some(name) = word(name_at).filter(|name| identifier(name) && *name != "fn") else {
            continue;
        };
        let name = if metavariable { "$" } else { name };
        if tokens.get(name_at + 1) != Some(&Token::Punct(':')) {
            continue;
        }
        let mut depth = 0_usize;
        let (mut equals, mut end) = (None, None);
        for (index, token) in tokens.iter().enumerate().skip(name_at + 2) {
            if opens(token) {
                depth += 1;
            } else if closes(token) {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            } else if depth == 0 && *token == Token::Punct('=') && equals.is_none() {
                equals = Some(index);
            } else if depth == 0 && *token == Token::Punct(';') {
                end = Some(index);
                break;
            }
        }
        if let (Some(equals), Some(end)) = (equals, end) {
            for slot in &mut found[equals + 1..end] {
                *slot = Some(name.to_owned());
            }
        }
    }
    found
}

/// How the reader reads an included file: as Rust (`include!`, `#[path]`) or as one piece
/// (`include_str!`, `include_bytes!`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Rust,
    Text,
}

/// One census's reading of a tree: the pool, the files a word or a whole piece names the table
/// in, what it refused and what it counted.
struct Reader<'a> {
    root: PathBuf,
    table: &'a str,
    owner: &'a str,
    pool: BTreeMap<String, BTreeSet<String>>,
    named: BTreeSet<String>,
    refused: BTreeSet<String>,
    seen: BTreeSet<(PathBuf, Kind)>,
    files: usize,
    pieces: usize,
    words: usize,
    includes: usize,
    out_dir: usize,
    /// Every file's own pieces, of every length, folded: the first part of its reach.
    local: BTreeMap<String, BTreeSet<String>>,
    /// The pieces of each `const` or `static` item, by its name: the file defining it, the piece.
    items: BTreeMap<String, BTreeSet<(String, String)>>,
    /// The words each file writes, and the names its format strings' placeholders write.
    mentions: BTreeMap<String, BTreeSet<String>>,
    /// The files each file includes, recorded for every includer.
    included: BTreeMap<String, BTreeSet<String>>,
    /// The item whose initializer holds the token being read.
    item: Option<String>,
    /// The one-character pieces kept out of the pool, judged only in a reach.
    held: usize,
}

impl Reader<'_> {
    /// `path` relative to the root, with `/` separators, or whole when it lies outside.
    fn name_of(&self, path: &Path) -> String {
        path.strip_prefix(&self.root).map_or_else(
            |_| path.display().to_string(),
            |inner| {
                inner
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            },
        )
    }

    /// Adds a literal's value to the pool, cut at each brace so that a `format!` string's text
    /// around its placeholders is a piece of its own. A piece of one character stays out of the
    /// pool, and is judged only in the reach of a file that holds, includes or names it.
    fn piece(&mut self, file: &str, value: &str) {
        let template = value.contains(['{', '}']);
        for part in value.split(['{', '}']).filter(|part| !part.is_empty()) {
            self.pieces += 1;
            self.reach_piece(file, part, template);
            if part.chars().count() < 2 && self.item.as_deref() != Some("$") {
                self.held += 1;
                continue;
            }
            self.pool
                .entry(part.to_ascii_lowercase())
                .or_default()
                .insert(file.to_owned());
        }
    }

    /// Records `part`, a piece of `file`, for the reaches: among the file's own pieces, among the
    /// pieces of the item being read, and, cut from a format string, its placeholder's name as a
    /// name the file writes.
    fn reach_piece(&mut self, file: &str, part: &str, template: bool) {
        let folded = part.to_ascii_lowercase();
        if template {
            let name = part.split(':').next().unwrap_or_default().trim();
            if identifier(name) {
                self.mentions
                    .entry(file.to_owned())
                    .or_default()
                    .insert(name.to_owned());
            }
        }
        if let Some(item) = &self.item {
            self.items
                .entry(item.clone())
                .or_default()
                .insert((file.to_owned(), folded.clone()));
        }
        self.local
            .entry(file.to_owned())
            .or_default()
            .insert(folded);
    }

    /// The pool of `file`'s reach: its own pieces, the pieces of every file it includes,
    /// transitively, and the pieces of every item it or an included file names, each held by the
    /// file that holds it.
    fn reach(&self, file: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut pool: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut pending = vec![file.to_owned()];
        let mut visited = BTreeSet::new();
        while let Some(next) = pending.pop() {
            if !visited.insert(next.clone()) {
                continue;
            }
            for piece in self.local.get(&next).into_iter().flatten() {
                pool.entry(piece.clone()).or_default().insert(next.clone());
            }
            pending.extend(self.included.get(&next).into_iter().flatten().cloned());
        }
        for holder in &visited {
            for name in self.mentions.get(holder).into_iter().flatten() {
                for (defined, piece) in self.items.get(name).into_iter().flatten() {
                    pool.entry(piece.clone())
                        .or_default()
                        .insert(defined.clone());
                }
            }
        }
        pool
    }

    /// Counts a word, records it as a name its file writes, and names its file when it holds the
    /// table's name in any case.
    fn word(&mut self, file: &str, word: &str) {
        self.words += 1;
        self.mentions
            .entry(file.to_owned())
            .or_default()
            .insert(word.to_owned());
        if word.to_ascii_lowercase().contains(self.table) {
            self.named.insert(file.to_owned());
        }
    }

    /// Reads the file at `path`, an include of `file`, by its literal path `written`.
    fn follow(&mut self, file: &str, path: &Path, written: &str, kind: Kind) {
        let Ok(bytes) = fs::read(path) else {
            self.refused.insert(format!(
                "{file} includes {written}, which the census cannot read for {}",
                self.table
            ));
            return;
        };
        let reached = self.name_of(path);
        self.included
            .entry(file.to_owned())
            .or_default()
            .insert(reached);
        if !self.seen.insert((path.to_path_buf(), kind)) {
            return;
        }
        let included = self.name_of(path);
        // An included file's text holds no item of the includer's.
        self.item = None;
        let text = String::from_utf8_lossy(&bytes);
        match kind {
            Kind::Rust => self.read(path, &included, &text),
            Kind::Text => self.piece(&included, &text),
        }
    }

    /// Refuses an include of `file` whose path is not one literal, unless it is joined onto
    /// `env!("OUT_DIR")`, which is counted and disclosed.
    fn unnamed(&mut self, file: &str, path: &[Token]) {
        if let [
            Token::Word(concat),
            Token::Punct('!'),
            Token::Punct('('),
            Token::Word(env),
            Token::Punct('!'),
            Token::Punct('('),
            Token::Literal(variable),
            ..,
        ] = path
            && concat == "concat"
            && env == "env"
            && variable == "OUT_DIR"
        {
            self.out_dir += 1;
            return;
        }
        self.refused.insert(format!(
            "{file} includes a file the census cannot name, so it cannot read it for {}",
            self.table
        ));
    }

    /// Reads `text`, the Rust file at `path` named `file`.
    fn read(&mut self, path: &Path, file: &str, text: &str) {
        let tokens: Vec<Token> = lex(text).into_iter().map(|(token, _)| token).collect();
        let items = items_of(&tokens);
        // Every file read has a reach, also one that holds no piece of its own (SPEC-331 R2).
        self.local.entry(file.to_owned()).or_default();
        let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut stringified = 0..0;
        let mut at = 0;
        while let Some(token) = tokens.get(at) {
            self.item.clone_from(&items[at]);
            if let Some(close) =
                invocation(&tokens, at, &["include", "include_str", "include_bytes"])
            {
                self.includes += 1;
                let kind = if *token == Token::Word("include".to_owned()) {
                    Kind::Rust
                } else {
                    Kind::Text
                };
                match &tokens[at + 3..close] {
                    [Token::Literal(written)] => {
                        self.follow(file, &normal(&folder.join(written)), written, kind);
                    }
                    path => self.unnamed(file, path),
                }
                at = close + 1;
                continue;
            }
            if let [
                Token::Punct('#'),
                Token::Punct('['),
                Token::Word(attribute),
                Token::Punct('='),
            ] = tokens.get(at..at + 4).unwrap_or_default()
                && attribute == "path"
            {
                self.includes += 1;
                if let Some([Token::Literal(written), Token::Punct(']')]) =
                    tokens.get(at + 4..at + 6)
                {
                    self.follow(file, &normal(&folder.join(written)), written, Kind::Rust);
                    at += 6;
                } else {
                    self.unnamed(file, &[]);
                    at += 4;
                }
                continue;
            }
            if let Some(close) = invocation(&tokens, at, &["concat"]) {
                let mut pieces = Vec::new();
                if concat_pieces(&tokens[at + 3..close], &mut pieces)
                    && pieces
                        .iter()
                        .any(|piece| holds_part_of(self.table, &piece.to_ascii_lowercase()))
                {
                    self.refused.insert(format!(
                        "{file} joins a value the census cannot read beside part of {}, and only \
                         {}'s code may",
                        self.table, self.owner
                    ));
                }
            }
            if let Some(close) = invocation(&tokens, at, &["stringify"]) {
                stringified = at + 3..close;
            }
            match token {
                Token::Literal(value) => self.piece(file, value),
                Token::Word(word) if stringified.contains(&at) || literal_word(word) => {
                    self.piece(file, word);
                }
                Token::Word(word) => self.word(file, word),
                Token::Punct(_) => {}
            }
            at += 1;
        }
    }
}

/// What the shared reader refuses for `table`, owned by the crate `owner`, in the tree at `root`:
/// every crate's `src` outside the owner, but for the files of `already` (each census's own reader
/// named them), read once. The refusals are sorted, one per file and reason.
pub fn refusals(root: &Path, table: &str, owner: &str, already: &BTreeSet<String>) -> Vec<String> {
    let mut reader = Reader {
        root: normal(root),
        table,
        owner,
        pool: BTreeMap::new(),
        named: BTreeSet::new(),
        refused: BTreeSet::new(),
        seen: BTreeSet::new(),
        files: 0,
        pieces: 0,
        words: 0,
        includes: 0,
        out_dir: 0,
        local: BTreeMap::new(),
        items: BTreeMap::new(),
        mentions: BTreeMap::new(),
        included: BTreeMap::new(),
        item: None,
        held: 0,
    };
    let mut members: Vec<PathBuf> = fs::read_dir(reader.root.join("crates"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    for member in &members {
        if member.file_name().is_some_and(|name| name == owner) {
            continue;
        }
        for source in rust_files(&member.join("src")) {
            let file = reader.name_of(&source);
            if already.contains(&file) || !reader.seen.insert((source.clone(), Kind::Rust)) {
                continue;
            }
            reader.files += 1;
            let text = fs::read(&source)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            reader.read(&source, &file, &text);
        }
    }
    println!(
        "table census: examined {} member(s), {} file(s) outside {owner}, {} piece(s) ({} of one \
         character, pooled only in a reach), {} word(s), {} include(s) ({} joined onto OUT_DIR, \
         disclosed) for {table}",
        members.len(),
        reader.files,
        reader.pieces,
        reader.held,
        reader.words,
        reader.includes,
        reader.out_dir
    );
    assert!(
        !members.is_empty(),
        "table census: examined 0 members for {table}: the population is empty, so nothing was \
         judged"
    );
    // The pool covers the name with pieces of two or more characters; each file's reach covers it
    // with pieces of every length, one character included (SPEC-331 R3).
    let mut covered = covering(table, &reader.pool);
    for file in reader.local.keys() {
        covered.extend(covering(table, &reader.reach(file)));
    }
    let mut refused = reader.refused;
    for file in covered.union(&reader.named) {
        refused.insert(format!(
            "{file} spells {table} from literals, joined or in another case, and only {owner}'s \
             code may"
        ));
    }
    refused.into_iter().collect()
}

/// Whether the token at `at` begins an item: the file's start, or after `;`, a brace or an
/// attribute's `]`, with a `pub` or `pub(..)` before it.
fn item_start(tokens: &[(Token, Range<usize>)], at: usize) -> bool {
    let mut before = at;
    if before > 0 && tokens[before - 1].0 == Token::Punct(')') {
        let open = (0..before - 1)
            .rev()
            .find(|index| tokens[*index].0 == Token::Punct('('));
        if let Some(open) = open
            && open > 0
            && tokens[open - 1].0 == Token::Word("pub".to_owned())
        {
            before = open - 1;
        }
    } else if before > 0 && tokens[before - 1].0 == Token::Word("pub".to_owned()) {
        before -= 1;
    }
    before == 0 || matches!(tokens[before - 1].0, Token::Punct(';' | '{' | '}' | ']'))
}

/// The bytes of every `use` declaration in `text`, from `use` through its `;`, an import or a
/// re-export, in a module, a function body or a macro's body alike.
pub fn use_declarations(text: &str) -> Vec<Range<usize>> {
    let tokens = lex(text);
    let mut found = Vec::new();
    for (at, (token, span)) in tokens.iter().enumerate() {
        if *token != Token::Word("use".to_owned()) || !item_start(&tokens, at) {
            continue;
        }
        let mut depth = 0_usize;
        let end = tokens[at..]
            .iter()
            .find_map(|(inside, bytes)| match inside {
                Token::Punct('{') => {
                    depth += 1;
                    None
                }
                Token::Punct('}') => {
                    depth = depth.saturating_sub(1);
                    None
                }
                Token::Punct(';') if depth == 0 => Some(bytes.end),
                _ => None,
            });
        if let Some(end) = end {
            found.push(span.start..end);
        }
    }
    found
}
