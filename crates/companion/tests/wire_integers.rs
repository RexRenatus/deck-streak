//! Schema v1 carries integers only (SPEC-501 C1). Every field of every type `src/wire.rs`
//! defines is an integer, a closed enum, a fixed array of them, a `Vec` of them, a type the file
//! itself defines, or one of the two named checkpoint byte fields; anything else is refused by
//! name (default-deny). The census reads the file's text, so a field added in any form is read.

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
