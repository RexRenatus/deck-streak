//! The vault adapter reads only the crate's own data: the owned rails and default layout keep
//! exactly the fields the adapter's parsers read, and each parser refuses the owned document
//! without one of them (SPEC-056 A6, A7; ADR-069). The gate's classes are judged beside their
//! parser, in `src/staged.rs` (A8).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout, clippy::panic)]

use std::fs;
use std::path::Path;

use deck_streak_vault::rails::{self, Rails};
use deck_streak_vault::staged::{self, DutyRun, RUN_SCHEMA, RunRefusal};
use serde_json::{Value, json};

/// The owned data file `name`, read from `crates/vault/data/`.
fn owned(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("the owned {} cannot be read: {error}", path.display()))
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// `document` without the field at the JSON pointer `pointer`.
fn without(document: &Value, pointer: &str) -> Value {
    let mut copy = document.clone();
    let (parent, field) = pointer.rsplit_once('/').expect("a pointer names its field");
    copy.pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .and_then(|object| object.remove(field))
        .unwrap_or_else(|| panic!("the owned document holds no {pointer}"));
    copy
}

/// A run record that names its layout by a path, so the layout's parser reads `layout`.
fn parse_with_layout(layout: &str) -> Result<DutyRun, RunRefusal> {
    let record = json!({
        "schema": RUN_SCHEMA,
        "duty": "daily-reading",
        "vault": [],
        "layout": "layout.json",
        "ops": []
    });
    DutyRun::parse(&record.to_string(), |path| {
        (path == "layout.json").then(|| layout.to_owned())
    })
}

#[test]
fn the_rails_parser_refuses_the_owned_rails_without_each_field_it_reads() {
    let text = owned("rails.json");
    assert_eq!(
        rails::VENDORED,
        text,
        "the adapter compiles in the owned rails"
    );
    Rails::from_json(&text).expect("the owned rails read as rails");
    let document: Value = serde_json::from_str(&text).expect("the owned rails are JSON");
    let fields: Vec<String> = document
        .as_object()
        .expect("the owned rails are an object")
        .keys()
        .cloned()
        .collect();
    for field in examined("owned rails fields", fields) {
        let refused = Rails::from_json(&without(&document, &format!("/{field}")).to_string());
        assert!(
            refused.is_err(),
            "the rails read without {field}, so the owned file keeps a field its parser does not read"
        );
    }
}

#[test]
fn the_layout_parser_refuses_the_owned_layout_without_each_field_it_requires() {
    let text = owned("layout.json");
    assert_eq!(
        staged::VENDORED_LAYOUT,
        text,
        "the adapter compiles in the owned layout"
    );
    parse_with_layout(&text).expect("the owned layout reads as a layout");
    let document: Value = serde_json::from_str(&text).expect("the owned layout is JSON");
    let required = vec![
        "/duties",
        "/inbox",
        "/periodic/daily/folder",
        "/periodic/weekly/folder",
    ];
    for pointer in examined("required layout fields", required) {
        let refused = parse_with_layout(&without(&document, pointer).to_string());
        assert!(
            matches!(refused, Err(RunRefusal::Record(_))),
            "the layout read without {pointer}: {refused:?}"
        );
    }
}
