//! SPEC-388 R5, A10 (ADR-402 D6): the web engine's card view carries `assisted`, the engine core's
//! answer for the shown card's note, and it is read in one place. `src/wasm.rs` compiles only for
//! `wasm32`, so no native test can run `current_card`: this census reads its source instead, and
//! refuses a view that lacks one `assisted` key taken from the core's read of the shown card's
//! note, a read of any other id, or a module that reads a note's tags or matches the tag itself.

// The examined helper prints its count on purpose; clippy.toml's in-test allowances cover only
// `#[test]` bodies.
#![allow(clippy::print_stdout)]

use std::fs;
use std::path::Path;

fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The view's key, compared with every blank removed.
const KEY: &str = "\"assisted\":";
/// The one value the key may take: the binding the core's read answers.
const VALUE: &str = "assisted";
/// The binding `current_card` makes from the core's read of the shown card's note, compared with
/// every blank removed.
const BINDING: &str = "letassisted=dispatcher()?.ai_assisted(card.note_id)";
/// The core's read, which only `current_card` may call.
const READ: &str = ".ai_assisted(";
/// The id the read takes: the shown card's note, never the card's own id.
const NOTE: &str = "card.note_id";
/// The tag, which only the core holds and judges.
const TAG: &str = "ai-assisted";

/// `text` with every blank removed, so a reflow by rustfmt changes nothing.
fn squeezed(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// `source` without its comment lines, so a doc comment's prose names nothing the code does.
fn code(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The body of the one function `name` defines in `source`, braces included. A function
/// defined other than once, or a body that never closes, is refused.
fn body<'a>(source: &'a str, name: &str) -> Result<&'a str, String> {
    let head = format!("fn {name}(");
    let starts: Vec<usize> = source.match_indices(&head).map(|(at, _)| at).collect();
    let [start] = starts.as_slice() else {
        return Err(format!("`{head}` occurs {} times, not once", starts.len()));
    };
    let Some(open) = source[*start..].find('{').map(|at| start + at) else {
        return Err(format!("`{head}` has no body"));
    };
    let mut depth = 0_usize;
    let mut chars = source[open..].char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                while let Some((_, c)) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&source[open..=open + at]);
                }
            }
            _ => {}
        }
    }
    Err(format!("`{head}`'s body never closes"))
}

/// What follows each `marker` in a squeezed `text`, up to the closing comma or bracket that ends
/// it at its own depth.
fn values(text: &str, marker: &str) -> Vec<String> {
    text.match_indices(marker)
        .map(|(at, _)| {
            let value = &text[at + marker.len()..];
            let mut depth = 0_i32;
            value
                .chars()
                .take_while(|c| {
                    match c {
                        '(' | '{' | '[' => depth += 1,
                        ')' | '}' | ']' => depth -= 1,
                        _ => {}
                    }
                    !(depth < 0 || (depth == 0 && *c == ','))
                })
                .collect()
        })
        .collect()
}

/// Whether `source` names the identifier `name`: `name` with no identifier character on either
/// side.
fn names(source: &str, name: &str) -> bool {
    let identifier = |c: char| c.is_alphanumeric() || c == '_';
    source.match_indices(name).any(|(at, _)| {
        source[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !identifier(c))
            && source[at + name.len()..]
                .chars()
                .next()
                .is_none_or(|c| !identifier(c))
    })
}

/// What `source` owes the shown card's `assisted` and lacks, each named with where.
fn problems(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    match body(source, "current_card") {
        Err(problem) => found.push(format!("current_card: {problem}")),
        Ok(view) => {
            let view = squeezed(view);
            let bindings = view.matches(BINDING).count();
            if bindings != 1 {
                found.push(format!(
                    "current_card binds `assisted` from the core's read of the shown card's note \
                     {bindings} time(s), not once"
                ));
            }
            let keys = values(&view, KEY);
            if keys.len() != 1 {
                found.push(format!(
                    "current_card's view holds {} `assisted` key(s), not one",
                    keys.len()
                ));
            }
            for value in keys {
                if value != VALUE {
                    found.push(format!(
                        "current_card's view takes `assisted` from `{value}`, not the core's read \
                         `{VALUE}`"
                    ));
                }
            }
        }
    }
    let code = code(source);
    let reads = values(&squeezed(&code), READ);
    if reads.len() != 1 {
        found.push(format!(
            "src/wasm.rs reads the core's marker {} time(s), not once",
            reads.len()
        ));
    }
    for id in reads {
        if id != NOTE {
            found.push(format!(
                "src/wasm.rs reads the marker of `{id}`, not of the shown card's note `{NOTE}`"
            ));
        }
    }
    if names(&code, "tags") {
        found.push(
            "src/wasm.rs names `tags` and reads a note's tags itself, beside the core's read"
                .to_owned(),
        );
    }
    if code.to_ascii_lowercase().contains(TAG) {
        found.push(format!(
            "src/wasm.rs holds the tag `{TAG}` and matches it itself, beside the core's rule"
        ));
    }
    found
}

fn boundary() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wasm.rs");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// `source` with every line of `view`, `current_card`'s body, that names the `assisted` key
/// removed, and `extra` set after the view's late key.
fn planted(source: &str, view: &str, extra: &str) -> String {
    let kept: Vec<&str> = view
        .lines()
        .filter(|line| !squeezed(line).contains(KEY))
        .collect();
    let replanted = kept
        .join("\n")
        .replacen("\"late\": late::past_due_day(&card, day),", extra, 1);
    source.replacen(view, &replanted, 1)
}

#[test]
fn the_shown_card_carries_the_cores_marker_answer() {
    let source = boundary();
    assert_eq!(
        problems(&source),
        Vec::<String>::new(),
        "the shown card's view does not carry the core's marker answer for its note, in one place"
    );

    // each plant is refused by name
    let view = body(&source, "current_card").expect("current_card has one body");
    let late = "\"late\": late::past_due_day(&card, day),";
    let with_false = planted(
        &source,
        view,
        &format!("{late}\n            \"assisted\": false,"),
    );
    let without = planted(&source, view, late);
    let card_id = source.replacen(".ai_assisted(card.note_id)", ".ai_assisted(card.id)", 1);
    let matcher = format!(
        "{source}\nfn marked(tags: &str) -> bool {{\n    tags.split_whitespace().any(|tag| tag == \
         \"ai-assisted\")\n}}\n"
    );
    let refused = (
        problems(&with_false),
        problems(&without),
        problems(&card_id),
        problems(&matcher),
    );
    assert!(
        refused
            .0
            .iter()
            .any(|line| line.starts_with("current_card's view takes `assisted` from `false`"))
            && refused
                .1
                .iter()
                .any(|line| line == "current_card's view holds 0 `assisted` key(s), not one")
            && refused
                .2
                .iter()
                .any(|line| line.starts_with("src/wasm.rs reads the marker of `card.id`"))
            && refused
                .3
                .iter()
                .any(|line| line.starts_with("src/wasm.rs names `tags`"))
            && refused
                .3
                .iter()
                .any(|line| line.starts_with("src/wasm.rs holds the tag `ai-assisted`")),
        "a planted view, a read of the card's own id or a second tag matcher is not refused by \
         name: {refused:?}"
    );

    examined(
        "`assisted` key(s) in current_card's view",
        values(&squeezed(view), KEY),
    );
    examined(
        "planted refusal(s)",
        vec![refused.0, refused.1, refused.2, refused.3],
    );
}
