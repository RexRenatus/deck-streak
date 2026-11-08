//! `rewrite` visits the engine's media reference names for every case of the engine's `media_refs`
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

/// Each name, written back in parentheses, so a test reads where `rewrite` put each answer.
fn wrapped(html: &str) -> String {
    rewrite(html, |name| format!("({name})"))
}

/// Pins `rewrite`'s overlap rule, `place.start < kept`: a place inside an earlier place keeps the
/// earlier answer, and a place starting where the earlier one ends is written too.
#[test]
fn a_place_inside_an_earlier_place_is_skipped_and_one_after_it_is_written() {
    let inside = "<img src=\"[sound:a]\">";
    assert_eq!(
        visited(inside),
        vec!["[sound:a]", "a"],
        "both names are asked"
    );
    assert_eq!(
        wrapped(inside),
        "<img src=\"([sound:a])\">",
        "the sound tag inside the src value keeps the src value's answer"
    );
    let after = "<img src=\"[sound:\">x]";
    assert_eq!(
        visited(after),
        vec!["[sound:", "\">x"],
        "both names are asked"
    );
    assert_eq!(
        wrapped(after),
        "<img src=\"([sound:)(\">x)]",
        "the sound name starting where the src value ends is written after it"
    );
}

/// Pins `push_items`: an attribute item is one character other than `>` (`first != b'>'`), or a
/// run quoted by the character at `at` (`first == quote`), searched from `at + 1`, holding at least
/// one character (`close > at + 1`) and ending after its closing quote (`close + 1`).
#[test]
fn an_attribute_item_is_a_character_or_a_quoted_run_of_at_least_one() {
    assert_eq!(
        visited("<img alt=\"a>b\" src=\"x.png\">"),
        vec!["x.png"],
        "a quoted run carries the search over the > inside it"
    );
    assert_eq!(
        visited("<img x>\" src=y.png><img src=z.png>"),
        vec!["z.png"],
        "no run is quoted by a character that is not a quote, so the first tag ends at its >"
    );
    assert_eq!(
        visited("<img alt=\"a>b\" c>\" src=y.png><img src=z.png>"),
        vec!["z.png"],
        "the run ends after its closing quote, which starts no run of its own"
    );
}

/// Pins `value_at`: a quoted value holds at least one character (`close > at + 1`), else the
/// value is read unquoted; the tag's `>` is searched after the closing quote (`close + 1`); and an
/// unquoted value may end at a space before the tag's `>` (the `Some(b' ')` arm).
#[test]
fn a_value_is_quoted_with_at_least_one_character_or_read_unquoted() {
    assert_eq!(
        visited("<img src=\"\">"),
        vec!["\"\""],
        "an empty quoted value is read unquoted, quotes and all"
    );
    assert_eq!(
        visited("<img src=\"a>\" <img src=b.png>"),
        vec!["a>"],
        "the > inside the quoted value does not end the tag, so the second < lies inside it"
    );
    assert_eq!(visited("<img src=x.png >"), vec!["x.png"]);
    assert_eq!(
        wrapped("<img src=x.png >"),
        "<img src=(x.png) >",
        "an unquoted value ends at the space before the tag's >"
    );
}

/// Pins where each match ends, after the tag's `>` (`end + 1`, `stop + 1`) or the sound tag's `]`
/// (`close + 1`): each is a character boundary, which `each_match` asserts, when a two-byte
/// character comes before it.
#[test]
fn a_match_ends_after_its_closing_character_past_a_two_byte_one() {
    assert_eq!(
        wrapped("<img src=\"x.png\" \u{e9}>"),
        "<img src=\"(x.png)\" \u{e9}>",
        "a quoted value's tag ends after the > that follows a two-byte character"
    );
    assert_eq!(
        wrapped("<img src=\u{e9}>"),
        "<img src=(\u{e9})>",
        "an unquoted two-byte name's tag ends after its >"
    );
    assert_eq!(
        wrapped("<img src=x.png \u{e9}>"),
        "<img src=(x.png) \u{e9}>",
        "an unquoted value's tag ends after the > past the space and a two-byte character"
    );
    assert_eq!(
        wrapped("[sound:\u{e9}]"),
        "[sound:(\u{e9})]",
        "a sound tag ends after the ] that follows its two-byte name"
    );
}

/// Pins `ends_word`: a tag's name is one of the grammar's tags only where a word ends after it.
#[test]
fn a_tag_name_must_end_its_word() {
    assert_eq!(
        visited("<imgx src=a.png><img src=b.png>"),
        vec!["b.png"],
        "imgx is not img"
    );
}
