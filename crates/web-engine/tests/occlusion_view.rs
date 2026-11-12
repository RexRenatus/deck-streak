//! SPEC-380 R3, R11, A6 (ADR-391): the web engine's card view carries `withheld`, the engine
//! core's answer whether the question it renders holds an image occlusion mask the app does not
//! draw, and on that answer it empties the view's question, answer and note CSS. `src/wasm.rs`
//! compiles only for `wasm32`, so no native test can run `current_card`: this census reads its
//! source instead, and refuses a view that lacks one `withheld` key bound to the core's rule over
//! the rendered question, or a view that keeps a text a withheld card must not show.

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
const KEY: &str = "\"withheld\":";
/// The core's rule, which `current_card` must call once and nothing else in the module may.
const RULE: &str = "occlusion::masks_not_drawn(";
/// The binding the view's `withheld` names: the core's rule over the question `current_card`
/// renders, compared with every blank removed.
const BINDING: &str = "letwithheld=occlusion::masks_not_drawn(&question.val);";
/// The one door a shown text passes through: empty on a withheld card, the text on any other.
const EMPTIED: &str = "letunless_withheld=|text:String|ifwithheld{String::new()}else{text};";
/// The view's keys whose text a withheld card must not show: its question, answer and note CSS.
const HIDDEN: [&str; 3] = ["question", "answer", "css"];

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

/// The value each `key` of a squeezed view takes, up to the key's closing comma.
fn values(view: &str, key: &str) -> Vec<String> {
    view.match_indices(key)
        .map(|(at, _)| {
            let value = &view[at + key.len()..];
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

/// What `source` owes the shown card's `withheld` and lacks, each named with where.
fn problems(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    match body(source, "current_card") {
        Err(problem) => found.push(format!("current_card: {problem}")),
        Ok(view) => {
            let view = squeezed(view);
            let withheld = values(&view, KEY);
            if withheld.len() != 1 {
                found.push(format!(
                    "current_card's view holds {} `withheld` key(s), not one",
                    withheld.len()
                ));
            }
            for value in withheld {
                if value != "withheld" {
                    found.push(format!(
                        "current_card's view takes `withheld` from `{value}`, not the core's \
                         rule over the rendered question"
                    ));
                }
            }
            if !view.contains(BINDING) {
                found.push(format!(
                    "current_card does not bind `withheld` to the core's rule over the rendered \
                     question (`{BINDING}`)"
                ));
            }
            if !view.contains(EMPTIED) {
                found.push(format!(
                    "current_card has no door that empties a withheld text (`{EMPTIED}`)"
                ));
            }
            for name in HIDDEN {
                let shown = values(&view, &format!("\"{name}\":"));
                if shown.len() != 1 {
                    found.push(format!(
                        "current_card's view holds {} `{name}` key(s), not one",
                        shown.len()
                    ));
                }
                for value in shown {
                    if !value.starts_with("unless_withheld(") {
                        found.push(format!(
                            "current_card's view keeps its `{name}` on a withheld card: `{value}`"
                        ));
                    }
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
    found
}

fn boundary() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wasm.rs");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// `source` with `current_card`'s body `view` given `plant` in its place.
fn planted(source: &str, view: &str, plant: &str) -> String {
    source.replacen(view, plant, 1)
}

#[test]
fn the_shown_card_carries_the_cores_withheld_answer() {
    let source = boundary();
    assert_eq!(
        problems(&source),
        Vec::<String>::new(),
        "the shown card's view does not carry the core's withheld answer, or keeps a withheld text"
    );

    // each plant is refused by name
    let view = body(&source, "current_card").expect("current_card has one body");
    let without: Vec<&str> = view
        .lines()
        .filter(|line| !squeezed(line).contains(KEY))
        .collect();
    let without = planted(&source, view, &without.join("\n"));
    let with_false = planted(
        &source,
        view,
        &view.replacen("\"withheld\": withheld,", "\"withheld\": false,", 1),
    );
    let kept = planted(
        &source,
        view,
        &view.replacen("unless_withheld(question.val)", "question.val", 1),
    );
    let refused = (problems(&without), problems(&with_false), problems(&kept));
    assert!(
        refused
            .0
            .iter()
            .any(|line| line == "current_card's view holds 0 `withheld` key(s), not one")
            && refused.1.iter().any(|line| {
                line.starts_with("current_card's view takes `withheld` from `false`")
            }) && refused.2.iter().any(|line| {
            line == "current_card's view keeps its `question` on a withheld card: `question.val`"
        }),
        "a view without `withheld`, or one that keeps a withheld text, is not refused by name: \
         {refused:?}"
    );

    examined(
        "`withheld` key(s) in current_card's view",
        values(&squeezed(view), KEY),
    );
    examined(
        "hidden text key(s) in current_card's view",
        HIDDEN
            .iter()
            .flat_map(|name| values(&squeezed(view), &format!("\"{name}\":")))
            .collect(),
    );
    examined("planted refusal(s)", vec![refused.0, refused.1, refused.2]);
}
