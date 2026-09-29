//! The coverage gates (SPEC-046 A3 to A6, R6): the anchors, the roster, the list markers, the band
//! and the order the first failure decides in.
//!
//! Every note text, word and reading here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_readings::coverage::{
    ANCHOR_MAX_CHARS, ANCHOR_MIN_USABLE_CHARS, Document, OwnChecks, PackFailure, anchor_for_note,
    check_own, first_failure, is_anchor_usable,
};
use deck_streak_readings::form::Form;
use deck_streak_readings::seed::{Seed, SeedNote, Track};
use deck_streak_readings::state::ReadingGate;

const SECTIONS: [&str; 5] = ["reading", "issue", "rule", "application", "conclusion"];

fn note(id: i64, text: &str) -> SeedNote {
    SeedNote {
        id,
        text: text.to_owned(),
    }
}

fn law_seed() -> Seed {
    Seed {
        card_ids: vec![1, 2, 3, 4],
        notes: vec![
            note(401, "Relevance is a low bar for admission"),
            note(
                402,
                "Hearsay is an out-of-court statement offered for its truth",
            ),
            note(403, "<b>Character</b> evidence is limited"),
            note(404, "short"),
        ],
        new_words: Vec::new(),
    }
}

/// A law reading whose prose holds `prose` in each section, then a retrieval list.
fn law_text(prose: &[(&str, &str)], retrieval: &str) -> String {
    let mut text =
        String::from("---\nschema: v1\nx-new-cards: 4\n---\n\n# A synthetic reading\n\n");
    for (name, body) in prose {
        text.push_str(&format!("## {name} <!-- section:{name} -->\n\n{body}\n\n"));
    }
    text.push_str(&format!(
        "## retrieval <!-- section:retrieval -->\n\n{retrieval}\n"
    ));
    text
}

fn full_prose() -> Vec<(&'static str, String)> {
    SECTIONS
        .iter()
        .map(|name| {
            let body = match *name {
                "reading" => "Relevance is a low bar for admission [@n401]. Hearsay is an out-of-court statement offered for its truth [@n402].",
                "rule" => "Character evidence is limited [@n403]. A short note follows [@n404].",
                _ => "Plain prose with nothing else in it.",
            };
            (*name, body.to_owned())
        })
        .collect()
}

fn law_doc(mutate: impl Fn(&mut Vec<(&'static str, String)>)) -> Document {
    let mut prose = full_prose();
    mutate(&mut prose);
    let borrowed: Vec<(&str, &str)> = prose.iter().map(|(n, b)| (*n, b.as_str())).collect();
    Document::parse(&law_text(
        &borrowed,
        "- What is hearsay?\n- What is relevance?",
    ))
}

fn own(seed: &Seed, document: &Document) -> OwnChecks {
    check_own(&Form::of(Track::Law), seed, document)
}

#[test]
fn a_complete_reading_passes_every_own_gate() {
    let checks = own(&law_seed(), &law_doc(|_| {}));
    assert_eq!(checks.complete, Vec::<String>::new());
    assert_eq!(checks.roster, Vec::<String>::new());
    assert_eq!(checks.anchors, Vec::<String>::new());
    assert_eq!(checks.no_list_markers, Vec::<String>::new());
    assert_eq!(
        checks.advisory.as_deref(),
        Some("anchors_partially_unverifiable:1/4"),
        "the one unusable anchor is excluded and reported"
    );
    assert!(first_failure(&checks, None).is_none());
}

#[test]
fn a_missing_section_fails_the_complete_gate() {
    let doc = law_doc(|prose| prose.retain(|(name, _)| *name != "issue"));
    let checks = own(&law_seed(), &doc);
    assert!(
        !checks.complete.is_empty(),
        "a missing section is a finding"
    );
    let failure = first_failure(&checks, None).expect("a failure");
    assert_eq!(failure.gate, ReadingGate::Complete);
}

#[test]
fn a_reading_missing_a_note_anchor_is_refused() {
    let doc = law_doc(|prose| {
        prose[0].1 = "Relevance is a low bar for admission [@n401]. Hearsay is discussed [@n402]."
            .to_owned();
    });
    let checks = own(&law_seed(), &doc);
    assert!(
        checks.anchors.iter().any(|line| line.contains("n402")),
        "the finding names the note whose anchor is absent: {:?}",
        checks.anchors
    );
    assert!(
        checks
            .anchors
            .iter()
            .all(|line| !line.contains("Hearsay is an out-of-court")),
        "a finding never quotes the note's text"
    );
    let failure = first_failure(&checks, None).expect("a failure");
    assert_eq!(failure.gate, ReadingGate::Anchors);
}

#[test]
fn an_unusable_anchor_is_excluded_not_required() {
    let seed = law_seed();
    let checks = own(&seed, &law_doc(|_| {}));
    assert!(
        checks.anchors.is_empty(),
        "the short note's anchor is not required"
    );
    let all_short = Seed {
        card_ids: vec![1],
        notes: vec![note(1, "tiny")],
        new_words: Vec::new(),
    };
    assert!(
        deck_streak_readings::seed::screen(&all_short, Track::Law).is_err(),
        "a seed whose anchors are all unusable never reaches the model"
    );
    assert!(
        deck_streak_readings::seed::screen(&seed, Track::Law).is_ok(),
        "a seed with some usable anchor is screened in"
    );
}

#[test]
fn a_reading_whose_roster_differs_from_the_seed_is_refused() {
    let seed = law_seed();
    let missing = law_doc(|prose| {
        prose[2].1 = "Character evidence is limited [@n403].".to_owned();
    });
    let checks = own(&seed, &missing);
    assert!(
        checks.roster.iter().any(|line| line.contains("n404")),
        "a seed note left uncited is named: {:?}",
        checks.roster
    );
    let extra = law_doc(|prose| {
        prose[4].1 = "The end [@n999].".to_owned();
    });
    let checks = own(&seed, &extra);
    assert!(
        !checks.roster.is_empty(),
        "a citation outside the seed is refused"
    );
    let failure = first_failure(&checks, None).expect("a failure");
    assert_eq!(failure.gate, ReadingGate::Roster);
}

#[test]
fn a_language_roster_needs_each_new_word_glossed_and_used() {
    let seed = Seed {
        card_ids: vec![1, 2],
        notes: vec![note(1, "a note"), note(2, "another note")],
        new_words: vec!["casa".to_owned(), "perro".to_owned()],
    };
    let text = |reading: &str, glosses: &str| {
        format!(
            "---\nschema: v1\nx-new-words: [\"casa\", \"perro\"]\n---\n\n# T\n\n\
             ## reading <!-- section:reading -->\n\n{reading}\n\n\
             ## glosses <!-- section:glosses -->\n\n{glosses}\n\n\
             ## grammar <!-- section:grammar -->\n\nGrammar prose.\n\n\
             ## pronunciation <!-- section:pronunciation -->\n\nSounds.\n\n\
             ## culture <!-- section:culture -->\n\nCulture.\n\n\
             ## retrieval <!-- section:retrieval -->\n\n- one\n- two\n"
        )
    };
    let form = Form::of(Track::Language);
    let good = Document::parse(&text("La casa y el perro.", "- casa: house\n- perro: dog"));
    let checks = check_own(&form, &seed, &good);
    assert!(checks.roster.is_empty(), "{:?}", checks.roster);
    assert!(
        checks.no_list_markers.is_empty(),
        "the glosses are a list by contract"
    );
    let unglossed = Document::parse(&text("La casa y el perro.", "- casa: house"));
    assert!(
        !check_own(&form, &seed, &unglossed).roster.is_empty(),
        "perro is unglossed"
    );
    let unused = Document::parse(&text("La casa.", "- casa: house\n- perro: dog"));
    assert!(
        !check_own(&form, &seed, &unused).roster.is_empty(),
        "perro is unused"
    );
}

#[test]
fn a_list_marker_in_the_primer_prose_is_refused() {
    for marker in [
        "- item", "* item", "+ item", "1. item", "22) item", "  - item", "\t* item",
    ] {
        let doc = law_doc(|prose| {
            prose[2].1.push_str(&format!("\n{marker}"));
        });
        let checks = own(&law_seed(), &doc);
        assert!(
            !checks.no_list_markers.is_empty(),
            "{marker:?} is a list marker"
        );
    }
    for fine in ["-item", "1.item", "a - b", "12 items", "**bold**"] {
        let doc = law_doc(|prose| {
            prose[2].1.push_str(&format!("\n{fine}"));
        });
        let checks = own(&law_seed(), &doc);
        assert!(
            checks.no_list_markers.is_empty(),
            "{fine:?} is not a list marker"
        );
    }
    let checks = own(&law_seed(), &law_doc(|_| {}));
    assert!(
        checks.no_list_markers.is_empty(),
        "the retrieval list is allowed"
    );
}

#[test]
fn the_first_failure_decides_in_the_gate_order() {
    let pack = |class: &str| PackFailure {
        class: class.to_owned(),
        findings: vec!["a finding".to_owned()],
    };
    let line = || vec!["a finding".to_owned()];
    let all = OwnChecks {
        complete: line(),
        roster: line(),
        anchors: line(),
        no_list_markers: line(),
        advisory: None,
    };
    let order = |own: &OwnChecks, pack: Option<&PackFailure>| {
        first_failure(own, pack).map(|failure| failure.gate)
    };
    let mut checks = all.clone();
    assert_eq!(
        order(&checks, Some(&pack("reading-length"))),
        Some(ReadingGate::Complete)
    );
    checks.complete.clear();
    assert_eq!(
        order(&checks, Some(&pack("reading-length"))),
        Some(ReadingGate::Roster)
    );
    checks.roster.clear();
    assert_eq!(
        order(&checks, Some(&pack("reading-length"))),
        Some(ReadingGate::Anchors)
    );
    checks.anchors.clear();
    assert_eq!(
        order(&checks, Some(&pack("reading-length"))),
        Some(ReadingGate::Band),
        "the band comes before the list markers"
    );
    assert_eq!(
        order(&checks, Some(&pack("output-contract"))),
        Some(ReadingGate::NoListMarkers)
    );
    checks.no_list_markers.clear();
    assert_eq!(
        order(&checks, Some(&pack("output-contract"))),
        Some(ReadingGate::Contract)
    );
    assert_eq!(
        order(&checks, Some(&pack("no-dates"))),
        Some(ReadingGate::Contract)
    );
    assert_eq!(
        order(&checks, Some(&pack("void"))),
        Some(ReadingGate::Contract)
    );
    assert_eq!(
        order(&checks, Some(&pack("examined-nothing"))),
        Some(ReadingGate::Contract)
    );
    assert_eq!(
        order(&checks, Some(&pack("citations-resolve"))),
        Some(ReadingGate::Roster)
    );
    assert_eq!(
        order(&checks, Some(&pack("i1-glosses"))),
        Some(ReadingGate::Roster)
    );
    assert_eq!(order(&checks, None), None);
    let failure = first_failure(&checks, Some(&pack("reading-length"))).expect("a failure");
    assert_eq!(
        failure.findings,
        ["a finding"],
        "the pack's findings travel"
    );
}

#[test]
fn the_anchor_bounds_are_pinned() {
    assert_eq!(ANCHOR_MAX_CHARS, 48);
    assert_eq!(ANCHOR_MIN_USABLE_CHARS, 8);
}

#[test]
fn anchors_match_the_parity_golden() {
    let anchors = golden::each_case("anchor_for_note", |case| {
        let text = case.input["text"].as_str().expect("a note text");
        assert_eq!(
            Some(anchor_for_note(text)),
            case.output.as_str().map(str::to_owned),
            "the anchor of {text:?}"
        );
    });
    assert_eq!(anchors.function, "preread.anchor_for_note");
    let usable = golden::each_case("is_anchor_usable", |case| {
        let anchor = case.input["anchor"].as_str().expect("an anchor");
        assert_eq!(
            Some(is_anchor_usable(anchor)),
            case.output.as_bool(),
            "the usability of {anchor:?}"
        );
    });
    assert_eq!(usable.function, "preread.is_anchor_usable");
}
