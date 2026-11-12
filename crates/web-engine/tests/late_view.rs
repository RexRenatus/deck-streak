//! SPEC-376 R4, A8 (ADR-387 D2): the web engine's card view carries `late`, the engine core's
//! answer for the card it shows, and it is judged in one place. `src/wasm.rs` compiles only for
//! `wasm32`, so no native test can run `current_card`: this census reads its source instead, and
//! refuses a view that lacks one `late` key taken from the core's rule, or a module that reads a
//! card's due or the engine's day to judge a due day itself.

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
const KEY: &str = "\"late\":";
/// The core's rule, which the view's `late` must call and nothing else in the module may.
const RULE: &str = "late::past_due_day(";
/// The fields a module would read to judge a due day itself: the card's due, its home deck due
/// and home deck, and the engine day's count and rollover. Only the core's rule reads them.
const OWN_JUDGEMENT: [&str; 5] = [
    "due",
    "original_due",
    "original_deck_id",
    "days_elapsed",
    "next_day_at",
];

/// `text` with every blank removed, so a reflow by rustfmt changes nothing.
fn squeezed(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
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

/// The value each `late` key of a squeezed view takes, up to the key's closing comma.
fn late_values(view: &str) -> Vec<String> {
    view.match_indices(KEY)
        .map(|(at, _)| {
            let value = &view[at + KEY.len()..];
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

/// Whether `source` reads the field `name` of some value: `.name` with no identifier character
/// after it.
fn reads(source: &str, name: &str) -> bool {
    let field = format!(".{name}");
    source.match_indices(&field).any(|(at, _)| {
        source[at + field.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || next == '_'))
    })
}

/// What `source` owes the shown card's `late` and lacks, each named with where.
fn problems(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    match body(source, "current_card") {
        Err(problem) => found.push(format!("current_card: {problem}")),
        Ok(view) => {
            let values = late_values(&squeezed(view));
            if values.len() != 1 {
                found.push(format!(
                    "current_card's view holds {} `late` key(s), not one",
                    values.len()
                ));
            }
            for value in values {
                if !value.starts_with(RULE) {
                    found.push(format!(
                        "current_card's view takes `late` from `{value}`, not the core's rule \
                         `{RULE}`"
                    ));
                }
            }
        }
    }
    let rules = squeezed(source).matches(RULE).count();
    if rules != 1 {
        found.push(format!(
            "src/wasm.rs calls the core's rule {rules} time(s), not once"
        ));
    }
    for field in OWN_JUDGEMENT {
        if reads(source, field) {
            found.push(format!(
                "src/wasm.rs reads `.{field}` and judges a due day itself, beside the core's rule"
            ));
        }
    }
    found
}

fn boundary() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wasm.rs");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// `source` with every line of `view`, `current_card`'s body, that names the `late` key removed,
/// and `extra` set after the view's undo word.
fn planted(source: &str, view: &str, extra: &str) -> String {
    let kept: Vec<&str> = view
        .lines()
        .filter(|line| !squeezed(line).contains(KEY))
        .collect();
    let replanted = kept
        .join("\n")
        .replacen("\"undo\": undo_view(judged),", extra, 1);
    source.replacen(view, &replanted, 1)
}

#[test]
fn the_shown_card_carries_the_cores_late_answer() {
    let source = boundary();
    assert_eq!(
        problems(&source),
        Vec::<String>::new(),
        "the shown card's view does not carry the core's late answer, in one place"
    );

    // each plant is refused by name
    let view = body(&source, "current_card").expect("current_card has one body");
    let with_false = planted(
        &source,
        view,
        "\"undo\": undo_view(judged),\n            \"late\": false,",
    );
    let without = planted(&source, view, "\"undo\": undo_view(judged),");
    let second = format!(
        "{source}\nfn overdue(card: &Card, today: i32) -> bool {{\n    card.due < today\n}}\n"
    );
    let refused = (problems(&with_false), problems(&without), problems(&second));
    assert!(
        refused
            .0
            .iter()
            .any(|line| { line.starts_with("current_card's view takes `late` from `false`") })
            && refused
                .1
                .iter()
                .any(|line| line == "current_card's view holds 0 `late` key(s), not one")
            && refused
                .2
                .iter()
                .any(|line| line.starts_with("src/wasm.rs reads `.due`")),
        "a planted view or a second due-day judgement is not refused by name: {refused:?}"
    );

    examined(
        "`late` key(s) in current_card's view",
        late_values(&squeezed(view)),
    );
    examined("planted refusal(s)", vec![refused.0, refused.1, refused.2]);
}
