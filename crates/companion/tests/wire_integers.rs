//! Schema v1 carries integers only (SPEC-501 C1). Every field of every type `src/wire.rs`
//! defines is an integer, a closed enum, a fixed array of them, a `Vec` of them, a type the file
//! itself defines, or one of the two named checkpoint byte fields; anything else is refused by
//! name (default-deny). The census reads the file's text, so a field added in any form is read.
//!
//! Two more censuses read the crate's sources the same way, each with planted sources it must
//! refuse: `Restore`'s doors (R7: its field is private, its one public constructor is
//! `From<CheckpointReply>`, and its one other door is the decoder's crate-private one, which only
//! `decode_request` calls), and the crate root's `#![no_std]` (ADR-519 D5).

// An integration test is test code: its helpers panic on a malformed source, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

/// The schema's source, as committed.
const WIRE: &str = include_str!("../src/wire.rs");

/// The fields the schema holds today; a new field changes this count and this file.
const FIELDS: usize = 49;

/// The two byte fields that may hold a `Vec<u8>`: the companion's own checkpoint, handed back
/// verbatim.
const CHECKPOINT_BYTES: [(&str, &str); 2] = [("CheckpointReply", "bytes"), ("Restore", "bytes")];

/// The integer types a field may hold. `usize` and `isize` are refused: their width is the
/// target's, not the schema's.
const INTEGERS: [&str; 10] = [
    "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128",
];

/// One field: the type it belongs to, its name (a tuple field is named by its index), and its
/// type's text with the whitespace removed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    owner: String,
    name: String,
    ty: String,
}

/// One item the file defines.
#[derive(Debug)]
struct Item {
    name: String,
    is_enum: bool,
    body: String,
}

/// The source with its comments blanked; the file holds no string literal.
fn without_comments(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let code = line.split("//").next().unwrap_or("");
        out.push_str(code);
        out.push('\n');
    }
    out
}

/// Every `struct` and `enum` the source defines, with its braced or parenthesised body.
fn items(source: &str) -> Vec<Item> {
    let code = without_comments(source);
    let tokens: Vec<char> = code.chars().collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let rest: String = tokens[at..].iter().take(7).collect();
        let keyword = if rest.starts_with("struct ") {
            Some((false, 7))
        } else if rest.starts_with("enum ") {
            Some((true, 5))
        } else {
            None
        };
        let starts_word = at == 0 || !(tokens[at - 1].is_alphanumeric() || tokens[at - 1] == '_');
        if let (Some((is_enum, skip)), true) = (keyword, starts_word) {
            let mut cursor = at + skip;
            let mut name = String::new();
            while cursor < tokens.len()
                && (tokens[cursor].is_alphanumeric() || tokens[cursor] == '_')
            {
                name.push(tokens[cursor]);
                cursor += 1;
            }
            while cursor < tokens.len() && tokens[cursor].is_whitespace() {
                cursor += 1;
            }
            let open = tokens[cursor];
            let close = match open {
                '{' => '}',
                '(' => ')',
                _ => {
                    at = cursor;
                    continue;
                }
            };
            let mut depth = 0_i32;
            let mut end = cursor;
            for (index, character) in tokens.iter().enumerate().skip(cursor) {
                if *character == open {
                    depth += 1;
                } else if *character == close {
                    depth -= 1;
                    if depth == 0 {
                        end = index;
                        break;
                    }
                }
            }
            let body: String = tokens[cursor + 1..end].iter().collect();
            found.push(Item {
                name,
                is_enum,
                body,
            });
            at = end;
        }
        at += 1;
    }
    found
}

/// The body split at its top-level commas, each part trimmed, the empty ones and attributes
/// dropped.
fn top_level(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut current = String::new();
    for character in body.chars() {
        match character {
            '{' | '(' | '[' | '<' => depth += 1,
            '}' | ')' | ']' | '>' => depth -= 1,
            _ => {}
        }
        if character == ',' && depth == 0 {
            parts.push(current.clone());
            current.clear();
        } else {
            current.push(character);
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .map(|part| strip_attributes(part.trim()))
        .filter(|part| !part.is_empty())
        .collect()
}

/// A part with its leading `#[...]` attributes removed.
fn strip_attributes(part: &str) -> String {
    let mut rest = part.trim();
    while let Some(after) = rest.strip_prefix("#[") {
        let close = after.find(']').expect("an attribute closes");
        rest = after[close + 1..].trim();
    }
    rest.to_owned()
}

/// A named field's name and type, with `pub` and its scope removed.
fn named_field(part: &str) -> (String, String) {
    let part = part.strip_prefix("pub(crate)").unwrap_or(part).trim();
    let part = part.strip_prefix("pub ").unwrap_or(part).trim();
    let (name, ty) = part.split_once(':').expect("a named field has a type");
    (name.trim().to_owned(), squeeze(ty))
}

fn squeeze(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Every field of every type of the source, and the names of its closed enums (enums whose
/// every variant is a unit) and of every type it defines.
fn fields(source: &str) -> (Vec<Field>, BTreeSet<String>, BTreeSet<String>) {
    let mut found = Vec::new();
    let mut closed = BTreeSet::new();
    let mut defined = BTreeSet::new();
    for item in items(source) {
        defined.insert(item.name.clone());
        if item.is_enum {
            let mut all_units = true;
            for variant in top_level(&item.body) {
                let head: String = variant
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                let rest = variant[head.len()..].trim();
                let owner = format!("{}::{head}", item.name);
                if let Some(inner) = rest.strip_prefix('{') {
                    all_units = false;
                    let inner = inner.strip_suffix('}').expect("a struct variant closes");
                    for part in top_level(inner) {
                        let (name, ty) = named_field(&part);
                        found.push(Field {
                            owner: owner.clone(),
                            name,
                            ty,
                        });
                    }
                } else if let Some(inner) = rest.strip_prefix('(') {
                    all_units = false;
                    let inner = inner.strip_suffix(')').expect("a tuple variant closes");
                    for (index, part) in top_level(inner).into_iter().enumerate() {
                        found.push(Field {
                            owner: owner.clone(),
                            name: index.to_string(),
                            ty: squeeze(&part),
                        });
                    }
                }
            }
            if all_units {
                closed.insert(item.name);
            }
        } else {
            for part in top_level(&item.body) {
                let (name, ty) = named_field(&part);
                found.push(Field {
                    owner: item.name.clone(),
                    name,
                    ty,
                });
            }
        }
    }
    (found, closed, defined)
}

/// Whether `ty` is an element a fixed array or a `Vec` may hold outside the checkpoint fields:
/// an integer wider than a byte, a closed enum, or a type the file defines.
fn element_allowed(ty: &str, closed: &BTreeSet<String>, defined: &BTreeSet<String>) -> bool {
    (INTEGERS.contains(&ty) && ty != "u8" && ty != "i8")
        || closed.contains(ty)
        || defined.contains(ty)
}

/// Every field whose type is not on the allow-list, by owner, name and type.
fn refused(source: &str) -> (usize, Vec<String>) {
    let (found, closed, defined) = fields(source);
    let mut refusals = Vec::new();
    for field in &found {
        let ty = field.ty.as_str();
        let checkpoint = CHECKPOINT_BYTES
            .iter()
            .any(|(owner, name)| field.owner == *owner && field.name == *name);
        let allowed = if INTEGERS.contains(&ty) || closed.contains(ty) || defined.contains(ty) {
            true
        } else if let Some(inner) = ty.strip_prefix("Vec<").and_then(|t| t.strip_suffix('>')) {
            (checkpoint && inner == "u8") || element_allowed(inner, &closed, &defined)
        } else if let Some(inner) = ty.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            let (element, length) = inner.split_once(';').unwrap_or((inner, ""));
            !length.is_empty()
                && length.chars().all(|c| c.is_ascii_digit() || c == '_')
                && element_allowed(element, &closed, &defined)
        } else {
            false
        };
        if !allowed {
            refusals.push(format!("{}.{}: {}", field.owner, field.name, field.ty));
        }
    }
    (found.len(), refusals)
}

#[test]
fn every_field_of_every_wire_type_is_an_integer_a_closed_enum_or_a_named_checkpoint_byte_field() {
    let (examined, refusals) = refused(WIRE);
    let (_, closed, _) = fields(WIRE);
    println!("examined {examined} field(s); closed enums {closed:?}");
    assert!(examined > 0, "the census read no field");
    assert_eq!(refusals, Vec::<String>::new(), "fields off the allow-list");
    assert_eq!(examined, FIELDS, "the schema's field count");
    assert_eq!(
        closed.into_iter().collect::<Vec<_>>(),
        ["Adapter", "Model", "Status"],
        "the closed enums"
    );
}

#[test]
fn a_planted_string_field_and_a_planted_float_field_are_each_refused() {
    let anchor = "    pub study_day: i32,\n";
    assert!(
        WIRE.contains(anchor),
        "the planting anchor is in the schema"
    );
    let plants = [
        ("pub note_text: String,", "ReviewEvent.note_text: String"),
        ("pub ease: f32,", "ReviewEvent.ease: f32"),
        ("pub tags: Vec<u8>,", "ReviewEvent.tags: Vec<u8>"),
        ("pub width: usize,", "ReviewEvent.width: usize"),
    ];
    for (plant, named) in plants {
        let planted = WIRE.replacen(anchor, &format!("{anchor}    {plant}\n"), 1);
        let (examined, refusals) = refused(&planted);
        assert_eq!(examined, FIELDS + 1, "the planted field is read: {plant}");
        assert_eq!(
            refusals,
            [named.to_owned()],
            "the planted field is refused by name"
        );
    }
    println!("examined {} planted field(s), each refused", plants.len());
}

/// The interface crate's sources, as committed: every file a door to `Restore` could sit in.
const LIB: &str = include_str!("../src/lib.rs");
const CODEC: &str = include_str!("../src/codec.rs");
const DECIDE: &str = include_str!("../src/decide.rs");
const PORT: &str = include_str!("../src/port.rs");

fn sources() -> Vec<(&'static str, String)> {
    vec![
        ("lib.rs", LIB.to_owned()),
        ("wire.rs", WIRE.to_owned()),
        ("codec.rs", CODEC.to_owned()),
        ("decide.rs", DECIDE.to_owned()),
        ("port.rs", PORT.to_owned()),
    ]
}

/// Whether the word `word` starts at `at` in `chars`.
fn word_at(chars: &[char], at: usize, word: &str) -> bool {
    let word: Vec<char> = word.chars().collect();
    let before = at == 0 || !(chars[at - 1].is_alphanumeric() || chars[at - 1] == '_');
    let after = chars
        .get(at + word.len())
        .is_none_or(|next| !(next.is_alphanumeric() || *next == '_'));
    before && after && chars[at..].starts_with(&word)
}

/// Every `impl` block of `code` (comments blanked), nested ones included: its header with the
/// whitespace collapsed and any `where` clause dropped, and its braced body.
fn impls(code: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = code.chars().collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if word_at(&chars, at, "impl") {
            let open = (at..chars.len()).find(|index| chars[*index] == '{');
            if let Some(open) = open {
                let mut depth = 0_i32;
                let mut close = chars.len();
                for (index, character) in chars.iter().enumerate().skip(open) {
                    if *character == '{' {
                        depth += 1;
                    } else if *character == '}' {
                        depth -= 1;
                        if depth == 0 {
                            close = index;
                            break;
                        }
                    }
                }
                let header: String = chars[at..open].iter().collect();
                let header = header.split_whitespace().collect::<Vec<_>>().join(" ");
                let header = header.split(" where ").next().unwrap_or("").to_owned();
                found.push((header, chars[open + 1..close].iter().collect()));
            }
        }
        at += 1;
    }
    found
}

/// The functions an `impl` body declares at its top level: each one's visibility with the
/// whitespace removed (`pub`, `pub(crate)`, or empty), and its name.
fn functions(body: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = body.chars().collect();
    let mut found = Vec::new();
    let mut depth = 0_i32;
    let mut since = 0;
    let mut at = 0;
    while at < chars.len() {
        if depth == 0 && word_at(&chars, at, "fn") {
            let visibility: String = chars[since..at]
                .iter()
                .filter(|c| !c.is_whitespace())
                .collect();
            let name: String = chars[at + 2..]
                .iter()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| c.is_alphanumeric() || **c == '_')
                .collect();
            found.push((visibility, name));
        }
        match chars[at] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    since = at + 1;
                }
            }
            ';' | ']' if depth == 0 => since = at + 1,
            _ => {}
        }
        at += 1;
    }
    found
}

/// Whether a visibility reaches outside the crate: `pub` alone does; `pub(crate)`,
/// `pub(super)` and none do not.
fn is_public(visibility: &str) -> bool {
    visibility.starts_with("pub") && !visibility.starts_with("pub(")
}

/// R7's census over `sources`: every way to build a `Restore` beyond the ones by design, and the
/// number of doors examined. By design, `Restore`'s one field is private, its one trait impl is
/// `From<CheckpointReply>`, its only public function is `bytes`, and its crate-private
/// `decoded` is called once, by `decode_request` in `codec.rs`.
fn restore_doors(sources: &[(&str, String)]) -> (usize, Vec<String>) {
    let mut examined = 0;
    let mut findings = Vec::new();
    let mut calls = Vec::new();
    for (file, source) in sources {
        let code = without_comments(source);
        for (header, body) in impls(&code) {
            if header.ends_with(" for Restore") {
                examined += 1;
                if header != "impl From<CheckpointReply> for Restore" {
                    findings.push(format!("{file}: {header}"));
                }
            } else if header == "impl Restore" {
                for (visibility, name) in functions(&body) {
                    examined += 1;
                    if is_public(&visibility) && name != "bytes" {
                        findings.push(format!("{file}: Restore::{name} is {visibility}"));
                    }
                }
            }
        }
        let count = squeeze(&code).matches("::decoded(").count();
        examined += count;
        if count > 0 {
            calls.push(format!("{file}: {count}"));
        }
        for item in items(source) {
            if item.name == "Restore" {
                for part in top_level(&item.body) {
                    examined += 1;
                    if part.starts_with("pub") {
                        findings.push(format!("{file}: Restore's field is public: {part}"));
                    }
                }
            }
        }
    }
    if calls != ["codec.rs: 1"] {
        findings.push(format!("the decoder's door is called at {calls:?}"));
    }
    (examined, findings)
}

#[test]
fn restore_is_built_only_from_a_checkpoint_reply_or_by_the_decoder() {
    let (examined, findings) = restore_doors(&sources());
    println!("examined {examined} door(s) to Restore");
    assert_eq!(findings, Vec::<String>::new(), "doors to Restore");
    // One trait impl, two functions, one call of the decoder's door and one field.
    assert_eq!(examined, 5, "the doors read");
}

#[test]
fn a_planted_public_door_to_restore_is_refused() {
    let plants: [(&str, &str, &str, &str); 4] = [
        (
            "wire.rs",
            "    pub(crate) fn decoded(",
            "    pub fn decoded(",
            "wire.rs: Restore::decoded is pub",
        ),
        (
            "wire.rs",
            "impl From<CheckpointReply> for Restore {",
            "impl From<Vec<u8>> for Restore {\n    fn from(bytes: Vec<u8>) -> Self {\n        Self { bytes }\n    }\n}\n\nimpl From<CheckpointReply> for Restore {",
            "wire.rs: impl From<Vec<u8>> for Restore",
        ),
        (
            "wire.rs",
            "pub struct Restore {\n    bytes: Vec<u8>,",
            "pub struct Restore {\n    pub bytes: Vec<u8>,",
            "wire.rs: Restore's field is public: pub bytes: Vec<u8>",
        ),
        (
            "codec.rs",
            "/// A frame's header, read.\n",
            "/// A forged door.\npub fn forge(bytes: Vec<u8>) -> Restore {\n    Restore::decoded(bytes)\n}\n\n/// A frame's header, read.\n",
            "the decoder's door is called at [\"codec.rs: 2\"]",
        ),
    ];
    for (file, anchor, planted, named) in plants {
        let mut planted_sources = sources();
        let source = planted_sources
            .iter_mut()
            .find(|(name, _)| *name == file)
            .map(|(_, source)| source)
            .expect("the planted file is a source");
        assert_eq!(
            source.matches(anchor).count(),
            1,
            "the planting anchor is in {file} once: {anchor}"
        );
        *source = source.replacen(anchor, planted, 1);
        let (_, findings) = restore_doors(&planted_sources);
        assert_eq!(
            findings,
            [named.to_owned()],
            "the planted door is refused by name"
        );
    }
    println!("examined {} planted door(s), each refused", plants.len());
}

/// Whether a crate root declares `#![no_std]` on a line of its own and links no `std` back in
/// (ADR-519 D5).
fn declares_no_std(root: &str) -> bool {
    let code = without_comments(root);
    code.lines().any(|line| line.trim() == "#![no_std]")
        && !squeeze(&code).contains("externcratestd")
}

#[test]
fn the_interface_crate_is_no_std_and_a_root_without_it_is_refused() {
    assert!(
        declares_no_std(LIB),
        "the interface crate's root declares #![no_std]"
    );
    let anchor = "#![no_std]\n";
    assert_eq!(
        LIB.matches(anchor).count(),
        1,
        "the planting anchor is in the root once"
    );
    let plants = [
        LIB.replacen(anchor, "", 1),
        LIB.replacen(anchor, "// #![no_std]\n", 1),
        LIB.replacen(anchor, "#![no_std]\nextern crate std;\n", 1),
    ];
    for planted in &plants {
        assert!(
            !declares_no_std(planted),
            "a planted root is refused: {planted}"
        );
    }
    println!("examined 1 crate root and {} planted root(s)", plants.len());
}
