//! `rewrite` visits the engine's media reference names for every case of the engine's media_refs
//! golden, in the engine's order, and writes each answer in its reference's place (SPEC-601 A9).

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_media::rewrite;
use serde_json::Value;

/// The committed golden of the engine's media references, the names its own rewrite visits.
const MEDIA_REFS: &str = "anki.render.media_refs";

/// The golden's answer for one case: the engine's names, in its order.
fn golden_names(output: &Value) -> Vec<String> {
    output["names"]
        .as_array()
        .expect("an array of names")
        .iter()
        .map(|name| name.as_str().expect("a name").to_owned())
        .collect()
}

/// The names `rewrite` visits in one input, in its order.
fn visited(html: &str) -> Vec<String> {
    let mut names = Vec::new();
    rewrite(html, |name| {
        names.push(name.to_owned());
        name.to_owned()
    });
    names
}

#[test]
fn rewrite_visits_the_engines_reference_names() {
    let golden = golden::read(&golden::committed(MEDIA_REFS)).expect("the golden");
    let named = golden
        .cases
        .iter()
        .filter(|case| !golden_names(&case.output).is_empty())
        .count();
    assert_eq!(golden.cases.len(), 854, "the golden's cases");
    assert_eq!(named, 61, "the golden's cases that name a media reference");
    let mut divergences = Vec::new();
    for (index, case) in golden.cases.iter().enumerate() {
        let html = case.input["html"].as_str().expect("an html input");
        if visited(html) != golden_names(&case.output) {
            divergences.push(index);
        }
    }
    assert_eq!(
        divergences,
        Vec::<usize>::new(),
        "the cases whose visited names differ from the engine's"
    );
    let written = rewrite("a<img src=\"x.png\">b", |name| format!("ds-{name}"));
    assert_eq!(
        written, "a<img src=\"ds-x.png\">b",
        "the answer is written in the reference's place"
    );
    println!(
        "examined {} case(s) of {}: {} with names, {} divergence(s)",
        golden.cases.len(),
        golden.function,
        named,
        divergences.len()
    );
}
