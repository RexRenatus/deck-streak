//! `plan_sweep` plus `scan` reproduce the engine's media check, G-L5-2 (SPEC-513 A6): over every
//! case's collection, read through `MediaIndex`, the files the sweep deletes are the engine's
//! unused files, and the names the plan lists missing are the engine's missing names but for the
//! images the engine renders from LaTeX, which ADR-531 D4 cites.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_media::{MediaIndex, Source, StoredFile, SweepPolicy, plan};
use serde_json::Value;

/// The committed golden of the engine's media check.
const CHECK: &str = "anki.media.check";

/// A collection of the golden, as a media index: every note's fields and every template as
/// HTML, every note type's CSS as CSS, and the media folder's files, none imported and all added
/// at the epoch.
struct Collection<'a>(&'a Value);

impl MediaIndex for Collection<'_> {
    fn sources(&self) -> Vec<Source<'_>> {
        let mut sources = Vec::new();
        for note in self.0["notes"].as_array().expect("notes") {
            for field in note["fields"].as_array().expect("fields") {
                sources.push(Source::Html(field.as_str().expect("a field")));
            }
        }
        for notetype in self.0["notetypes"].as_array().expect("note types") {
            for template in notetype["templates"].as_array().expect("templates") {
                for side in ["front", "back"] {
                    sources.push(Source::Html(template[side].as_str().expect("a side")));
                }
            }
            sources.push(Source::Css(notetype["css"].as_str().expect("a CSS")));
        }
        sources
    }

    fn stored(&self) -> Vec<StoredFile> {
        self.0["media"]
            .as_array()
            .expect("media")
            .iter()
            .map(|file| StoredFile {
                name: file["name"].as_str().expect("a name").to_owned(),
                added_ms: 0,
                imported: false,
            })
            .collect()
    }
}

/// A list of names from the golden, in byte order.
fn sorted(value: &Value) -> Vec<String> {
    let mut names: Vec<String> = value
        .as_array()
        .expect("an array of names")
        .iter()
        .map(|name| name.as_str().expect("a name").to_owned())
        .collect();
    names.sort();
    names
}

/// The sweep at the case's own instant, with no grace window.
fn policy(collection: &Value) -> SweepPolicy {
    SweepPolicy {
        now_ms: collection["now"].as_i64().expect("an instant") * 1000,
        grace_ms: 0,
    }
}

#[test]
fn plan_sweep_and_scan_reproduce_the_engines_media_check_golden_but_for_latex() {
    let golden = golden::read(&golden::committed(CHECK)).expect("the golden");
    let mut latex_cases = 0;
    let mut latex_names = 0;
    for (index, case) in golden.cases.iter().enumerate() {
        let collection = &case.input["collection"];
        let planned = plan(&Collection(collection), policy(collection));
        assert_eq!(
            planned.delete,
            sorted(&case.output["unused"]),
            "case {index}: the sweep deletes the engine's unused files"
        );
        let missing = sorted(&case.output["missing"]);
        let (rendered, read): (Vec<String>, Vec<String>) = missing
            .into_iter()
            .partition(|name| name.starts_with("latex-"));
        assert_eq!(
            planned.missing, read,
            "case {index}: the plan's missing names are the engine's, LaTeX aside"
        );
        if !rendered.is_empty() {
            assert_eq!(
                case.class.as_deref(),
                Some("latex-notes"),
                "case {index}: only a LaTeX case names a rendered image"
            );
            latex_cases += 1;
            latex_names += rendered.len();
        }
    }
    assert_eq!(golden.cases.len(), 64, "the golden's cases");
    println!(
        "examined {} case(s) of {}: unused equal in all, missing equal in all but {latex_cases} \
         LaTeX case(s), which differ by {latex_names} rendered name(s) cited to ADR-531 D4",
        golden.cases.len(),
        golden.function
    );
    assert_eq!(latex_cases, 8);
}

#[test]
fn a_pinned_file_is_never_unused_and_a_planted_unused_answer_is_caught() {
    let mut golden = golden::read(&golden::committed(CHECK)).expect("the golden");
    let collection = golden.cases[0].input["collection"].clone();
    let planned = plan(&Collection(&collection), policy(&collection));
    assert!(
        planned.kept.iter().any(|(name, _)| name.starts_with('_')),
        "the case's pinned font is kept"
    );
    golden.cases[0].output["unused"] = serde_json::json!(["ds_present.png", "_ds_font.woff2"]);
    assert_ne!(
        planned.delete,
        sorted(&golden.cases[0].output["unused"]),
        "a planted answer that sweeps the pinned font is caught"
    );
}
