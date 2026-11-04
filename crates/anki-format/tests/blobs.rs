//! A6 (SPEC-602 R7): the 8 blob columns as typed views; each case of `anki.blob.encode` encodes to
//! the golden's bytes and decodes to the case's values, and an unchanged view keeps its raw bytes.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::collections::BTreeMap;
use std::fmt::Debug;

use deck_streak_anki_format::blob::{
    Blob, BlobView, ConfigValue, DeckCommonView, DeckConfigView, DeckKindView, FieldConfigView,
    NotetypeConfigView, TagConfigView, TemplateConfigView,
};
use serde_json::Value;
use support::{examined, golden_cases, text};

fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("a hex byte"))
        .collect()
}

/// The case's seed, or none for the default values.
fn seed(case: &Value) -> Option<u64> {
    case["input"]["values"]["seed"].as_u64()
}

fn small(value: u64) -> u32 {
    u32::try_from(value).expect("a small value")
}

/// Each column's default-values case bytes, the base a view is encoded onto.
fn defaults(cases: &[Value]) -> BTreeMap<String, Vec<u8>> {
    cases
        .iter()
        .filter(|case| case["input"]["values"] == "default")
        .map(|case| {
            (
                text(&case["input"]["column"]),
                bytes(&text(&case["output"]["hex"])),
            )
        })
        .collect()
}

/// The view encodes onto `base` as `golden`, and `golden` decodes as the view.
fn both_ways<V: BlobView + Debug>(view: &V, base: &[u8], golden: &[u8]) -> Vec<String> {
    let mut findings = Vec::new();
    match view.encode_onto(base) {
        Ok(encoded) if encoded == golden => {}
        found => findings.push(format!("{}: {view:?} encodes as {found:?}", V::COLUMN)),
    }
    match V::decode(golden) {
        Ok(decoded) if &decoded == view => {}
        found => findings.push(format!("{}: the golden decodes as {found:?}", V::COLUMN)),
    }
    findings
}

fn case_findings(case: &Value, base: &[u8], golden: &[u8]) -> Vec<String> {
    let seed = seed(case);
    match text(&case["input"]["column"]).as_str() {
        "config.val" => {
            let json = seed.map_or_else(|| "1".to_owned(), |n| format!("{{\"seed\":{n}}}"));
            both_ways(&ConfigValue { json }, base, golden)
        }
        "deck_config.config" => {
            let view = seed.map_or(
                DeckConfigView {
                    new_per_day: 20,
                    reviews_per_day: 200,
                },
                |n| DeckConfigView {
                    new_per_day: small(n % 9999),
                    reviews_per_day: small(n % 99_999),
                },
            );
            both_ways(&view, base, golden)
        }
        "decks.common" => {
            let view = seed.map_or(
                DeckCommonView {
                    study_collapsed: true,
                    browser_collapsed: true,
                },
                |n| DeckCommonView {
                    study_collapsed: n % 2 == 1,
                    browser_collapsed: n % 3 == 0,
                },
            );
            both_ways(&view, base, golden)
        }
        "decks.kind" => {
            let view = DeckKindView {
                filtered: false,
                config_id: 1,
                review_limit: seed.map(|n| small(n % 1000)),
                new_limit: seed.map(|n| small(n % 50)),
            };
            both_ways(&view, base, golden)
        }
        "fields.config" => {
            let view = FieldConfigView {
                font_name: "Arial".to_owned(),
                font_size: seed.map_or(20, |n| small(10 + n % 40)),
            };
            both_ways(&view, base, golden)
        }
        "notetypes.config" => {
            let view = NotetypeConfigView {
                kind: 0,
                sort_field: 0,
                css: seed.map_or_else(String::new, |n| format!(".ds-seed-{n} {{ color: black; }}")),
            };
            both_ways(&view, base, golden)
        }
        "tags.config" => both_ways(&TagConfigView {}, base, golden),
        "templates.config" => {
            let view = seed.map_or_else(
                || TemplateConfigView {
                    q_format: "{{Prompt}}{{Answer}}".to_owned(),
                    a_format: "{{Answer}}".to_owned(),
                    browser_font_size: 0,
                },
                |n| TemplateConfigView {
                    q_format: "{{Prompt}}{{Answer}}".to_owned(),
                    a_format: "{{Answer}}".to_owned(),
                    browser_font_size: small(10 + n % 30),
                },
            );
            both_ways(&view, base, golden)
        }
        other => vec![format!("{other}: not a blob column of the reader")],
    }
}

#[test]
fn each_blob_golden_case_encodes_and_decodes_both_ways() {
    let cases = examined("blob case(s)", golden_cases("anki.blob.encode"));
    let defaults = defaults(&cases);
    assert_eq!(defaults.len(), 8, "a default-values case per blob column");
    let mut findings = Vec::new();
    for case in &cases {
        let column = text(&case["input"]["column"]);
        let base = defaults.get(&column).cloned().unwrap_or_default();
        let golden = bytes(&text(&case["output"]["hex"]));
        findings.extend(case_findings(case, &base, &golden));
    }
    assert_eq!(findings, Vec::<String>::new(), "the blob views");
}

/// A blob read and stored again keeps its bytes, even with a field no view models (field 15)
/// and with its fields out of number order.
fn kept<V: BlobView + Debug>(raw: &[u8]) -> Option<String> {
    let mut extended = raw.to_vec();
    extended.extend_from_slice(&[0x78, 0x01]);
    for candidate in [raw.to_vec(), extended] {
        match Blob::<V>::read(&candidate).and_then(|blob| blob.bytes()) {
            Ok(stored) if stored == candidate => {}
            found => {
                return Some(format!(
                    "{}: {candidate:02x?} stored as {found:?}",
                    V::COLUMN
                ));
            }
        }
    }
    None
}

#[test]
fn an_unchanged_view_keeps_its_raw_bytes() {
    let cases = examined("blob case(s)", golden_cases("anki.blob.encode"));
    let mut findings = Vec::new();
    for case in &cases {
        let raw = bytes(&text(&case["output"]["hex"]));
        let finding = match text(&case["input"]["column"]).as_str() {
            "config.val" => {
                // JSON text, not protobuf: the bytes alone.
                match Blob::<ConfigValue>::read(&raw).and_then(|blob| blob.bytes()) {
                    Ok(stored) if stored == raw => None,
                    found => Some(format!("config.val: stored as {found:?}")),
                }
            }
            "deck_config.config" => kept::<DeckConfigView>(&raw),
            "decks.common" => kept::<DeckCommonView>(&raw),
            "decks.kind" => kept::<DeckKindView>(&raw),
            "fields.config" => kept::<FieldConfigView>(&raw),
            "notetypes.config" => kept::<NotetypeConfigView>(&raw),
            "tags.config" => kept::<TagConfigView>(&raw),
            "templates.config" => kept::<TemplateConfigView>(&raw),
            other => Some(format!("{other}: not a blob column")),
        };
        findings.extend(finding);
    }
    // Out of number order: decks.common with field 2 before field 1.
    findings.extend(kept::<DeckCommonView>(&[0x10, 0x01, 0x08, 0x01]));
    assert_eq!(findings, Vec::<String>::new(), "the raw bytes kept");
}
