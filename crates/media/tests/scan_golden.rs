//! `scan` equals the engine's media names for every case of G-L5-1 (SPEC-513 A4), with each case
//! that differs cited to ADR-531 D4 by its index and its reason, and the grammar holds the
//! engine's reading of hand cases the golden does not draw.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use deck_streak_media::scan;
use deck_streak_media::scan::each_match;
use serde_json::Value;

/// The committed golden of the engine's field references.
const FIELD_REFS: &str = "anki.media.field_refs";

/// The cases ADR-531 D4 cites, by index, and why each differs: the engine names an image it
/// renders from LaTeX, or writes an inline image to a file and names that file; the grammar does
/// neither.
const DIVERGENCES: [(usize, &str); 8] = [
    (24, "inline image"),
    (64, "latex"),
    (65, "latex"),
    (66, "latex"),
    (67, "latex"),
    (68, "latex"),
    (70, "latex"),
    (71, "latex"),
];

/// The golden's answer for one case: the engine's names, each once, in byte order.
fn golden_names(output: &Value) -> BTreeSet<String> {
    output["names"]
        .as_array()
        .expect("an array of names")
        .iter()
        .map(|name| name.as_str().expect("a name").to_owned())
        .collect()
}

/// The grammar's answer for one input, read as the golden reads it.
fn our_names(input: &Value) -> BTreeSet<String> {
    scan(input["html"].as_str().expect("an html input"))
        .into_iter()
        .collect()
}

/// The grammar's answer in the golden's shape.
fn port(input: &Value) -> Value {
    serde_json::json!({ "names": our_names(input) })
}

/// Whether a name is one the engine derives rather than reads: a rendered LaTeX image, or an
/// inline image written to a file.
fn derived(name: &str) -> bool {
    name.starts_with("latex-") || name.starts_with("paste-")
}

#[test]
fn scan_equals_the_engines_field_refs_golden_but_for_the_cited_divergences() {
    let golden = golden::read(&golden::committed(FIELD_REFS)).expect("the golden");
    assert_eq!(golden.cases.len(), 96, "the golden's cases");
    let mut cited = Vec::new();
    for found in golden::mismatches(&golden, port) {
        let index = found.index;
        let ours = our_names(&found.input);
        let theirs = golden_names(&found.expected);
        let Some(reason) = DIVERGENCES
            .iter()
            .find(|(cited, _)| *cited == index)
            .map(|(_, reason)| *reason)
        else {
            panic!("an uncited case differs: {found}");
        };
        let only_theirs: Vec<&String> = theirs.difference(&ours).collect();
        let only_ours: Vec<&String> = ours.difference(&theirs).collect();
        assert!(
            only_theirs.iter().all(|name| derived(name)),
            "case {index}: the engine's extra names are all derived: {only_theirs:?}"
        );
        if reason == "latex" {
            assert!(
                only_ours.is_empty() && golden.cases[index].class.as_deref() == Some("latex"),
                "case {index}: a LaTeX case differs only by the rendered names"
            );
        } else {
            assert!(
                only_ours
                    .iter()
                    .all(|name| name.starts_with("data:image/") && name.contains(";base64,")),
                "case {index}: an inline image differs only by the file the engine wrote"
            );
        }
        cited.push(index);
    }
    assert_eq!(
        cited,
        DIVERGENCES
            .iter()
            .map(|(index, _)| *index)
            .collect::<Vec<_>>(),
        "every cited case differs, and no other"
    );
    println!(
        "examined {} case(s) of {}: {} equal, {} cited to ADR-531 D4",
        golden.cases.len(),
        golden.function,
        golden.cases.len() - cited.len(),
        cited.len()
    );
}

#[test]
fn a_golden_answer_changed_by_hand_is_caught() {
    let mut golden = golden::read(&golden::committed(FIELD_REFS)).expect("the golden");
    golden.cases[0].output = serde_json::json!({ "names": ["ds_planted.png"] });
    let found: Vec<usize> = golden::mismatches(&golden, port)
        .iter()
        .map(|found| found.index)
        .collect();
    assert_eq!(found.first(), Some(&0), "the planted answer is caught");
    println!(
        "the planted case 0 is caught among {} mismatch(es)",
        found.len()
    );
}

/// Hand cases of the engine's grammar the golden does not draw, each with the names the engine's
/// expression gives, written out here.
const HAND: [(&str, &[&str]); 17] = [
    (r#"<img src="a&amp;b.png">"#, &["a&b.png"]),
    (r#"<img src="a&eacute;.png">"#, &["a&eacute;.png"]),
    (r#"<img src="a&#x41;&#66;.png">"#, &["aAB.png"]),
    (r#"<img src="a&amp.png">"#, &["a&amp.png"]),
    (r#"<img src="a&nbsp;b.png">"#, &["a b.png"]),
    (r#"<img alt="a>b" src="x.png">"#, &["x.png"]),
    (r"<img alt='a>b' src='y.png'>", &["y.png"]),
    (
        r#"<source src="s.ogg"><audio src=a.mp3 controls>"#,
        &["s.ogg", "a.mp3"],
    ),
    (r#"<object data="o.svg"></object>"#, &["o.svg"]),
    ("<img \u{17f}rc=\"long.png\">", &["long.png"]),
    (r#"<img datasrc="no.png">"#, &[]),
    (r#"<imgx src="no.png">"#, &[]),
    (
        r#"<img src="HTTPS://example.invalid/r.png"><img src="Http://x/r.png">"#,
        &[],
    ),
    ("[sound:]][sound:a.mp3]", &["]", "a.mp3"]),
    ("[sound:line\nbreak.mp3]", &["line\nbreak.mp3"]),
    (r#"[sound:s.mp3]<img src="i.png">"#, &["i.png", "s.mp3"]),
    (r#"<img src=a.png alt="x"><img src=b.png"#, &["a.png"]),
];

#[test]
fn scan_reads_the_engines_grammar_over_hand_cases() {
    for (text, names) in HAND {
        assert_eq!(scan(text), names.to_vec(), "{text:?}");
    }
    println!("examined {} hand case(s)", HAND.len());
}

/// Hand cases of the grammar's edges, each with the names the engine's expression gives: a quoted
/// run that holds only a `>`, an empty quoted value, a `>` inside a quoted value, a match that ends
/// just after a multibyte character, the `&quot;` entity, a signed number, and a scheme one letter
/// off a remote one.
const EDGES: [(&str, &[&str]); 11] = [
    (r#"<img alt=">" src="a.png">"#, &["a.png"]),
    (r#"<img alt=">">" src="a.png">"#, &[]),
    (r#"<img src="">"#, &["\"\""]),
    (r#"<img src="x>" <img src="b.png">"#, &["x>"]),
    (
        "<img src=\"a.png\" alt=\u{e9}><img src=b.png>",
        &["a.png", "b.png"],
    ),
    ("<img src=\u{e9}><img src=b.png>", &["\u{e9}", "b.png"]),
    ("<img src=a.png \u{e9}><img src=b.png>", &["a.png", "b.png"]),
    ("[sound:\u{e9}][sound:b.mp3]", &["\u{e9}", "b.mp3"]),
    (r#"<img src="a&quot;b.png">"#, &["a\"b.png"]),
    (r#"<img src="a&#+65;.png">"#, &["a&#+65;.png"]),
    ("[sound:httpx://a.mp3]", &["httpx://a.mp3"]),
];

#[test]
fn scan_reads_the_grammars_edges_over_hand_cases() {
    for (text, names) in EDGES {
        assert_eq!(scan(text), names.to_vec(), "{text:?}");
    }
    println!("examined {} edge case(s)", EDGES.len());
}

/// Where a match ends, given where it began.
type EndOf = fn(usize) -> usize;

/// Where a match ends, as a scanner that does not advance answers it: where the match began, and
/// at the text's start.
const STALLED_ENDS: [(&str, EndOf); 2] = [
    ("where it began", |start| start),
    ("at the text's start", |_| 0),
];

#[test]
fn each_match_tries_each_occurrence_once_when_a_match_does_not_advance() {
    for (named, end_of) in STALLED_ENDS {
        let mut tried = Vec::new();
        let found = each_match("a<b<c<", "<", |start| {
            tried.push(start);
            assert!(
                tried.len() <= 3,
                "a match ending {named}: tried {tried:?} for 3 occurrences, so the search does not \
                 advance"
            );
            ControlFlow::Continue(Some((start, end_of(start))))
        });
        assert_eq!(found, [1, 3, 5], "a match ending {named}");
        assert_eq!(tried, [1, 3, 5], "a match ending {named}");
    }
    println!("examined {} stalled end(s)", STALLED_ENDS.len());
}
