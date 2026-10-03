//! The generated population of a reserved table name's spellings, which the table censuses' tests
//! plant outside the table's owner and inside it (SPEC-324 A1 to A5, ADR-325). It is one file
//! included by path into each census's tests, as the parity oracle's golden reader is (ADR-029):
//!
//! ```text
//! #[macro_use]
//! #[path = "../../../tools/table-census/population.rs"]
//! mod population;
//! ```
//!
//! Two oracles stand apart from the census's reader. Plain concatenation decides that each split
//! joins to the name, and rustc evaluates each member of the literal family through `spell!`, so
//! neither the population nor its truth is the reader's own work.

// Each including crate calls only the part of the population its tests need.
#![allow(dead_code)]

use std::ffi::CStr;

/// The forms each split of a name is planted in: `concat!`, `+`, `format!`, two constants in one
/// file, a constant in another file, a constant in another crate, a sqlx-style `+` query, an
/// `include_str!` of a text piece and a `#[path]` module holding a piece.
pub const FORMS: usize = 9;

/// One spelling of a name: what it is, and every file that holds a piece of it, each by its path
/// from the root and its text.
pub struct Spelling {
    /// The split and the form, or the family member's text.
    pub label: String,
    /// Every planted file, `(path from the root, text)`.
    pub files: Vec<(String, String)>,
}

impl Spelling {
    /// The same spelling with every file moved into the crate `owner`, whose own code may name the
    /// table, so that a census refuses none of it.
    pub fn in_crate(&self, owner: &str) -> Self {
        Self {
            label: format!("{} in {owner}", self.label),
            files: self
                .files
                .iter()
                .map(|(path, text)| {
                    let (_, rest) = path
                        .strip_prefix("crates/")
                        .and_then(|inner| inner.split_once('/'))
                        .expect("every planted file lies in a crate");
                    (format!("crates/{owner}/{rest}"), text.clone())
                })
                .collect(),
        }
    }
}

/// Every split of `name` the population plants: its `n - 1` splits into two pieces, then its split
/// into single characters.
pub fn splits(name: &str) -> Vec<Vec<String>> {
    let mut found: Vec<Vec<String>> = (1..name.len())
        .map(|at| vec![name[..at].to_owned(), name[at..].to_owned()])
        .collect();
    found.push(name.chars().map(String::from).collect());
    found
}

/// The `FORMS` spellings of one split, numbered `index`, in the crate `member`; the crate form puts
/// its first piece in the crate `other`. Every file of a spelling holds a piece of it.
pub fn planted(split: &[String], index: usize, member: &str, other: &str) -> Vec<Spelling> {
    let stem = format!("s{index:03}");
    let source = format!("crates/{member}/src");
    let quoted: Vec<String> = split.iter().map(|piece| format!("{piece:?}")).collect();
    let consts = |from: usize| -> String {
        (from..split.len())
            .map(|at| format!("const P{at}: &str = {};\n", quoted[at]))
            .collect()
    };
    let names = |from: usize| -> String {
        (from..split.len())
            .map(|at| format!("P{at}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let joined = |first: &str| -> String {
        format!(
            "{}pub fn joined() -> String {{\n    [{first}, {}].concat()\n}}\n",
            consts(1),
            names(1)
        )
    };
    let first = format!("pub const P0: &str = {};\n", quoted[0]);
    let template = format!("{}{}", split[0], "{}".repeat(split.len() - 1));
    let query = format!("DELETE FROM {}", split[0]);
    let spelling = |form: &str, files: Vec<(String, String)>| Spelling {
        label: format!("{stem} {form} of {}", split.join("|")),
        files,
    };
    vec![
        spelling(
            "concat",
            vec![(
                format!("{source}/{stem}_concat.rs"),
                format!("pub const JOINED: &str = concat!({});\n", quoted.join(", ")),
            )],
        ),
        spelling(
            "plus",
            vec![(
                format!("{source}/{stem}_plus.rs"),
                format!(
                    "pub fn joined() -> String {{\n    String::from({}) + {}\n}}\n",
                    quoted[0],
                    quoted[1..].join(" + ")
                ),
            )],
        ),
        spelling(
            "format",
            vec![(
                format!("{source}/{stem}_format.rs"),
                format!(
                    "pub fn joined() -> String {{\n    format!({template:?}, {})\n}}\n",
                    quoted[1..].join(", ")
                ),
            )],
        ),
        spelling(
            "consts",
            vec![(
                format!("{source}/{stem}_consts.rs"),
                format!(
                    "{}pub fn joined() -> String {{\n    [{}].concat()\n}}\n",
                    consts(0),
                    names(0)
                ),
            )],
        ),
        spelling(
            "file",
            vec![
                (format!("{source}/{stem}_file_pieces.rs"), first.clone()),
                (
                    format!("{source}/{stem}_file.rs"),
                    joined(&format!("super::{stem}_file_pieces::P0")),
                ),
            ],
        ),
        spelling(
            "crate",
            vec![
                (
                    format!("crates/{other}/src/{stem}_crate_pieces.rs"),
                    first.clone(),
                ),
                (
                    format!("{source}/{stem}_crate.rs"),
                    joined(&format!("deck_streak_{other}::{stem}_crate_pieces::P0")),
                ),
            ],
        ),
        spelling(
            "sqlx",
            vec![(
                format!("{source}/{stem}_sqlx.rs"),
                format!(
                    "pub fn erase() {{\n    let _ = sqlx::query!({query:?} + {} + \" WHERE id = 1\");\n}}\n",
                    quoted[1..].join(" + ")
                ),
            )],
        ),
        spelling(
            "text",
            vec![
                (
                    format!("crates/{member}/queries/{stem}_piece.sql"),
                    split[0].clone(),
                ),
                (
                    format!("{source}/{stem}_text.rs"),
                    joined(&format!("include_str!(\"../queries/{stem}_piece.sql\")")),
                ),
            ],
        ),
        spelling(
            "module",
            vec![
                (format!("crates/{member}/shared/{stem}_piece.rs"), first),
                (
                    format!("{source}/{stem}_module.rs"),
                    format!(
                        "#[path = \"../shared/{stem}_piece.rs\"]\nmod piece;\n{}",
                        joined("piece::P0")
                    ),
                ),
            ],
        ),
    ]
}

/// A member of the literal family, numbered `index`, planted in the crate `member`: `text` is its
/// source as `spell!` read it.
pub fn spelled(text: &str, index: usize, member: &str) -> Spelling {
    Spelling {
        label: format!("f{index:03} {text}"),
        files: vec![(
            format!("crates/{member}/src/f{index:03}_spelled.rs"),
            format!("pub fn spelled() {{\n    let _ = {text};\n}}\n"),
        )],
    }
}

/// Splits that come one character short of `name`: each two-piece split with the last character
/// of its first piece or the first character of its second dropped, and the split into single
/// characters with one character dropped that occurs once in the name. A character that occurs
/// twice keeps a piece of its own in the split, and the census may reuse a piece, so dropping one
/// of the two still covers the name.
pub fn near_misses(name: &str) -> Vec<Vec<String>> {
    let mut found: Vec<Vec<String>> = Vec::new();
    for at in 1..name.len() {
        let (head, tail) = name.split_at(at);
        for (first, second) in [(&head[..head.len() - 1], tail), (head, &tail[1..])] {
            if !first.is_empty() && !second.is_empty() {
                found.push(vec![first.to_owned(), second.to_owned()]);
            }
        }
    }
    for (at, character) in name.char_indices() {
        if name.matches(character).count() == 1 {
            found.push(
                name.char_indices()
                    .filter(|(other, _)| *other != at)
                    .map(|(_, kept)| kept.to_string())
                    .collect(),
            );
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The value a family member evaluates to, as text: rustc evaluates the member, and this reads the
/// result whatever its type.
pub trait Text {
    /// The value as text.
    fn text(&self) -> String;
}

impl Text for &str {
    fn text(&self) -> String {
        (*self).to_owned()
    }
}

impl<const N: usize> Text for &[u8; N] {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self[..]).into_owned()
    }
}

impl Text for &CStr {
    fn text(&self) -> String {
        self.to_string_lossy().into_owned()
    }
}

impl<const N: usize> Text for [char; N] {
    fn text(&self) -> String {
        self.iter().collect()
    }
}

impl<const N: usize> Text for [u8; N] {
    fn text(&self) -> String {
        String::from_utf8_lossy(self).into_owned()
    }
}

/// A family member: its source text as written (`stringify!` keeps a literal's text verbatim) and
/// the value rustc gives it.
macro_rules! spell {
    ($($token:tt)+) => {
        (
            stringify!($($token)+),
            $crate::population::Text::text(&($($token)+)),
        )
    };
}
